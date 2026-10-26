//! Edits to a profile working copy, and repacking it the way the pager stores it.

use std::io::{Cursor, Write};
use std::path::Path;

use crate::row::{load_table, sel};
use crate::{Error, Profile};

/// PPS limits aliases to 14 characters.
pub const MAX_NAME: usize = 14;

/// Rename the channel at `knob` in the zone called `zone`.
/// Only valid on a working copy (e.g. a profile loaded from a zip).
pub fn rename_knob(p: &Profile, zone: &str, knob: i64, name: &str) -> Result<String, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(Error::Invalid(format!("name must be 1 to {MAX_NAME} characters")));
    }
    if !name.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        return Err(Error::Invalid("name must be printable ASCII".into()));
    }
    let zones = load_table(&p.db, "tabZone_New");
    let matches = sel(&zones, "Name", zone);
    let [z] = matches.as_slice() else {
        return Err(Error::Invalid(format!("expected exactly one zone named {zone:?}, found {}", matches.len())));
    };
    let zid = z.s("ID").to_string();
    let modes = load_table(&p.db, "tabReceivingMode_NewG4");
    let k = modes
        .iter()
        .find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == knob && r.i("CurrentMode") != 0)
        .ok_or_else(|| Error::Invalid(format!("zone {zone:?} has no active knob {knob}")))?;
    let old = k.s("ChannelName").to_string();
    let n = p.db.execute(
        "UPDATE tabReceivingMode_NewG4 SET ChannelName = ? WHERE ZoneID = ? AND KnobIndex = ?",
        &[Some(name), Some(&zid), Some(k.s("KnobIndex"))],
    )?;
    if n != 1 {
        return Err(Error::Invalid(format!("rename touched {n} rows, expected 1")));
    }
    Ok(old)
}

/// Rebuild `u3db_PC.zip` from the profile's working copy: same entries, order
/// and timestamps as `original`; every file whose working-copy bytes differ
/// (the root `<GUID>.db`, edited per-system DBs, ...) is replaced, the rest is
/// copied through untouched. Per-system DBs (`<GUID>/<id>.db`) that were
/// created are appended, stamped `now` (`yyyy-MM-dd HH:mm:ss`); ones that were
/// deleted are dropped. Nothing else can be added or removed.
pub fn repack_zip(original: &Path, p: &Profile, now: &str) -> Result<Vec<u8>, Error> {
    enum Action {
        Copy,
        Replace(Vec<u8>),
        Drop,
    }
    let root = p.db_path.parent().ok_or_else(|| Error::Invalid("bad profile path".into()))?;
    let folder = p.folder.file_name().and_then(|n| n.to_str()).ok_or_else(|| Error::Invalid("bad profile folder".into()))?.to_string();
    let is_system_db = |name: &str| name.strip_prefix(&format!("{folder}/")).is_some_and(|f| !f.contains('/') && f.ends_with(".db"));
    let mut src = zip::ZipArchive::new(std::fs::File::open(original)?)?;

    let mut actions = Vec::with_capacity(src.len());
    let mut names = std::collections::HashSet::new();
    for i in 0..src.len() {
        let mut entry = src.by_index(i)?;
        let name = entry.name()?.into_owned();
        names.insert(name.clone());
        let local = entry.enclosed_name().map(|n| root.join(n));
        let action = match local {
            Some(path) if entry.is_file() && path.is_file() => {
                let bytes = std::fs::read(&path)?;
                let mut old = Vec::with_capacity(entry.size() as usize);
                std::io::Read::read_to_end(&mut entry, &mut old)?;
                if old != bytes { Action::Replace(bytes) } else { Action::Copy }
            }
            Some(_) if entry.is_file() && is_system_db(&name) => Action::Drop,
            _ => Action::Copy,
        };
        actions.push(action);
    }
    let db_name = p.db_path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    if !names.contains(&db_name) {
        return Err(Error::Invalid(format!("{db_name} not found in the original zip")));
    }
    let mut added: Vec<String> = match std::fs::read_dir(&p.folder) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().to_str().map(|f| format!("{folder}/{f}")))
            .filter(|n| is_system_db(n) && !names.contains(n))
            .collect(),
        Err(_) => Vec::new(),
    };
    added.sort();

    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (i, action) in actions.into_iter().enumerate() {
        let entry = src.by_index_raw(i)?;
        match action {
            Action::Copy => out.raw_copy_file(entry)?,
            Action::Drop => {}
            Action::Replace(bytes) => {
                let opts = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated)
                    .last_modified_time(entry.last_modified().unwrap_or_default());
                let name = entry.name()?.into_owned();
                drop(entry);
                out.start_file(name, opts)?;
                out.write_all(&bytes)?;
            }
        }
    }
    let stamp = zip_time(now);
    for name in added {
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).last_modified_time(stamp);
        out.start_file(name.as_str(), opts)?;
        out.write_all(&std::fs::read(root.join(&name))?)?;
    }
    Ok(out.finish()?.into_inner())
}

/// `yyyy-MM-dd HH:mm:ss` as a zip timestamp (the 1980 epoch if unparsable).
fn zip_time(now: &str) -> zip::DateTime {
    let n: Vec<u16> = now.split(|c: char| !c.is_ascii_digit()).filter_map(|x| x.parse().ok()).collect();
    match n[..] {
        [y, mo, d, h, mi, s, ..] => zip::DateTime::from_date_and_time(y, mo as u8, d as u8, h as u8, mi as u8, s as u8).unwrap_or_default(),
        _ => zip::DateTime::default(),
    }
}

/// PPS limit for a trunked knob's receive list (priority + non-priority).
pub const MAX_KNOB_TALKGROUPS: usize = 64;

/// The talkgroup list of a trunked knob: `priority` in priority order, then `normal`.
#[derive(Debug, Clone, Default)]
pub struct KnobTalkgroups {
    pub priority: Vec<String>,
    pub normal: Vec<String>,
}

const ACK_COLS: [&str; 9] = [
    "VoiceRecvAck", "VoiceReadAck", "VoiceManualAck", "TextRecvAck", "TextReadAck", "TextManualAck", "ImageRecvAck", "ImageReadAck",
    "ImageManualAck",
];

/// Replace a trunked knob's talkgroup list the way PPS's TrunkingRxGroupListControl.Save does:
/// delete the knob's `tabChannelGroup` rows, write the priority list as SeqNo 1..n and the rest
/// as SeqNo 0, with SigType '' and OffDuty/Priority/OutOfRange/OTA 0. Ack flags are kept for
/// talkgroups that were already on the knob, 0 for new ones. Returns the number of rows written.
pub fn set_knob_talkgroups(p: &Profile, zone: &str, knob: i64, list: &KnobTalkgroups) -> Result<usize, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let zones = load_table(&p.db, "tabZone_New");
    let matches = sel(&zones, "Name", zone);
    let [z] = matches.as_slice() else {
        return Err(Error::Invalid(format!("expected exactly one zone named {zone:?}, found {}", matches.len())));
    };
    let zid = z.s("ID").to_string();
    let knobs = load_table(&p.db, "tabP25TrunkingSystemKnob");
    let k = knobs
        .iter()
        .find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == knob)
        .ok_or_else(|| Error::Invalid(format!("{zone:?} knob {knob} is not a P25 trunked knob")))?;
    let ch = k.s("ChannelID").to_string();
    refuse_sub_features(p, &ch)?;
    check_knob_list(p, k.s("P25TrunkingID"), k.s("ATGID"), list)?;
    p.db.execute_batch("BEGIN;")?;
    match write_knob_list(p, &ch, list) {
        Ok(n) => {
            p.db.execute_batch("COMMIT;")?;
            Ok(n)
        }
        Err(e) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

/// Knob features Knobset can't carry through a talkgroup-list rewrite yet.
pub(crate) fn refuse_sub_features(p: &Profile, channel_id: &str) -> Result<(), Error> {
    for (table, what) in [("tabP25TChannelSubGroup", "sub-groups"), ("tabP25TCallAlertSUIDGroup", "call alert"), ("tabChannelRevertGroup", "revert talkgroups")] {
        if !sel(&load_table(&p.db, table), "ChannelID", channel_id).is_empty() {
            return Err(Error::Unsupported(format!("editing a knob that uses {what}")));
        }
    }
    Ok(())
}

/// PPS's checks on a knob's receive list: 1..64 talkgroups of `system_id`, no
/// duplicates, no two with the same TGID, none equal to the announcement TG `atg`.
pub(crate) fn check_knob_list(p: &Profile, system_id: &str, atg: &str, list: &KnobTalkgroups) -> Result<(), Error> {
    let all: Vec<&String> = list.priority.iter().chain(&list.normal).collect();
    if all.is_empty() {
        return Err(Error::Invalid("a trunked knob needs at least one talkgroup".into()));
    }
    if all.len() > MAX_KNOB_TALKGROUPS {
        return Err(Error::Invalid(format!("{} talkgroups; a knob holds at most {MAX_KNOB_TALKGROUPS}", all.len())));
    }
    let sys = p.system_db(system_id)?;
    let groups = load_table(&sys, "tabGroup_New");
    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_tgids = std::collections::HashMap::new();
    for gid in &all {
        if !seen_ids.insert(gid.as_str()) {
            return Err(Error::Invalid(format!("talkgroup {gid} is listed twice")));
        }
        let g = groups.iter().find(|g| g.s("GroupID") == gid.as_str()).ok_or_else(|| Error::Invalid(format!("talkgroup {gid} is not in this knob's system")))?;
        let tgid = g.s("P25TAddr").to_string();
        if !atg.is_empty() && atg != "0" && tgid == atg {
            return Err(Error::Invalid(format!("{} uses the knob's announcement talkgroup ID {tgid}", g.s("GroupName"))));
        }
        if let Some(other) = seen_tgids.insert(tgid.clone(), g.s("GroupName").to_string()) {
            return Err(Error::Invalid(format!("{other} and {} have the same TGID {tgid}", g.s("GroupName"))));
        }
    }
    Ok(())
}

/// Replace the `tabChannelGroup` rows of `channel_id` (no transaction; the caller owns it).
pub(crate) fn write_knob_list(p: &Profile, channel_id: &str, list: &KnobTalkgroups) -> Result<usize, Error> {
    let ch = channel_id;
    let existing = sel(&load_table(&p.db, "tabChannelGroup"), "ChannelID", ch).into_iter().cloned().collect::<Vec<_>>();
    p.db.execute("DELETE FROM tabChannelGroup WHERE ChannelID = ?", &[Some(ch)])?;
    let mut n = 0;
    let rows = list.priority.iter().enumerate().map(|(i, g)| (g, (i + 1).to_string())).chain(list.normal.iter().map(|g| (g, "0".to_string())));
    for (gid, seq) in rows {
        let old = existing.iter().find(|r| r.s("GroupID") == gid.as_str());
        let acks: Vec<String> = ACK_COLS.iter().map(|c| old.and_then(|r| r.get(c)).unwrap_or("0").to_string()).collect();
        let mut params: Vec<Option<&str>> = vec![Some(ch), Some(gid), Some(""), Some(&seq), Some("0"), Some("0"), Some("0"), Some("0")];
        params.extend(acks.iter().map(|a| Some(a.as_str())));
        p.db.execute(
            "INSERT INTO tabChannelGroup(ChannelID,GroupID,SigType,SeqNo,OffDuty,Priority,OutOfRange,OTA,VoiceRecvAck,VoiceReadAck,VoiceManualAck,TextRecvAck,TextReadAck,TextManualAck,ImageRecvAck,ImageReadAck,ImageManualAck) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            &params,
        )?;
        n += 1;
    }
    Ok(n)
}

/// Radio-DB tables a talkgroup-list edit may change (the contact list is renumbered).
/// Measured on real profiles by removing and adding talkgroups (tests/golden.rs).
pub const TALKGROUP_EDIT_TABLES: &[&str] = &[
    "tabContactAddr",
    "tabContactAddrAndAlertTone",
    "tabContactOfBackground",
    "tabContactOfChan",
    "tabContactOfRelayCtl",
    "tabContacts",
    "tabRadioGroup",
    "tabRecvBehaviorByContactID",
    "tabRecvGroupList",
    "tabRecvToneForContacts",
    "tabRecvVoiceCallRecordByContactID",
];

/// Radio-DB tables a talkgroup's alert mode, alert tone, voice storage or
/// background colour may change, with no contact renumbering
/// (notes/talkgroup-settings.md 4). Custom tones would add `tabTone` and
/// `tabToneFileDetail`; Knobset doesn't set them.
pub const TALKGROUP_SETTINGS_TABLES: &[&str] =
    &["tabRecvBehaviorByContactID", "tabRecvToneForContacts", "tabContactAddrAndAlertTone", "tabRecvVoiceCallRecordByContactID", "tabContactOfBackground"];

/// Radio-DB tables a knob rename may change.
pub const RENAME_TABLES: &[&str] = &["tabChannel", "tabReceivingMode"];

/// Radio-DB tables a system-properties, site or control-channel edit may change.
pub const SYSTEM_EDIT_TABLES: &[&str] = &["tabSystemAlias", "tabP25TrunkingSignaling", "tabP25TrunkingSites", "tabP25TrunkingControlChannels"];

/// Radio-DB tables a knob gaining a radio channel may change (notes/system-creation.md 7.4).
pub const KNOB_CHANNEL_TABLES: &[&str] = &[
    "tabReceivingMode",
    "tabZoneChannelList",
    "tabChannelAnnouncement",
    "tabChannel",
    "tabChannelAdvInfo",
    "tabFunctionOnOffOfChan",
    "tabFunctionParamOfChan",
    "tabVoiceCallSettings",
    "tabResetModeSetting",
    "tabR2RModeSetting",
    "tabAlertModeSetting",
    "tabRecvVoiceCallRecordByChannel",
    "tabDispatchChannel",
    "tabP25TSetting",
    "tabKMFSystemByChannel",
    "tabCipherKeyAssignment",
    "tabP25OTARFeatureControlByChannel",
    "tabP25OTARFeatureControl",
    "tabZone",
];

/// Radio-DB tables a zone create, rename or delete-last may change.
pub const ZONE_TABLES: &[&str] = &["tabZone"];

/// Radio-DB tables keyed by channel ID (zone position x 100 + knob), which a
/// zone reorder or a middle-zone delete renumbers (notes/zones-and-scan.md 9.1).
pub const ZONE_RENUMBER_TABLES: &[&str] = &[
    "tabZone",
    "tabReceivingMode",
    "tabZoneChannelList",
    "tabChannelAnnouncement",
    "tabChannel",
    "tabChannelAdvInfo",
    "tabFunctionOnOffOfChan",
    "tabFunctionParamOfChan",
    "tabVoiceCallSettings",
    "tabResetModeSetting",
    "tabR2RModeSetting",
    "tabAlertModeSetting",
    "tabRecvVoiceCallRecordByChannel",
    "tabDispatchChannel",
    "tabP25TSetting",
    "tabKMFSystemByChannel",
    "tabCipherKeyAssignment",
    "tabP25OTARFeatureControlByChannel",
    "tabP25OTARFeatureControl",
    "tabRepeaterSettingOfChan",
    "tabContactOfChan",
    "tabRecvGroupList",
    "tabRecvBehaviorByContactID",
    "tabRecvSaList",
    "tabScanListSettings",
    // DMR and P25 conventional channels' signaling rows are keyed by channel too.
    "tabDMRSignaling",
    "tabDMRRepeaterSetting",
    "tabP25Signaling",
    "tabP25RepeaterSetting",
];

/// Radio-DB tables an in-place edit of an analog conventional knob may change
/// (notes/knob-editing.md 8.3).
pub const CONV_EDIT_TABLES: &[&str] =
    &["tabChannel", "tabChannelAdvInfo", "tabRecvSaList", "tabAlertModeSetting", "tabRecvVoiceCallRecordByChannel", "tabChannelAnnouncement"];

/// Radio-DB tables a TG Scan / TG Monitor switch may change (notes/knob-editing.md 8.4).
pub const MODE_SWITCH_TABLES: &[&str] = &["tabReceivingMode", "tabP25TSetting", "tabChannelAnnouncement"];

/// Radio-DB tables a DMR conventional knob adds beyond the analog channel
/// tables (notes/digital-conventional.md 9.7). New talkgroups also renumber
/// the contacts (`TALKGROUP_EDIT_TABLES`).
pub const DMR_KNOB_TABLES: &[&str] = &["tabDMRSignaling", "tabDMRRepeaterSetting"];

/// Radio-DB tables a P25 conventional knob adds beyond the analog channel
/// tables, including the per-channel encryption rows a profile with keys gets.
pub const P25C_KNOB_TABLES: &[&str] =
    &["tabP25Signaling", "tabP25RepeaterSetting", "tabKMFSystemByChannel", "tabCipherKeyAssignment", "tabP25OTARFeatureControlByChannel", "tabP25OTARFeatureControl"];

/// Radio-DB tables a new scan knob may change (notes/zones-and-scan.md 9.6).
pub const SCAN_KNOB_TABLES: &[&str] = &[
    "tabReceivingMode",
    "tabChannelAnnouncement",
    "tabScanListSettings",
    "tabFunctionParamOfChan",
    "tabFunctionOnOffOfChan",
    "tabResetModeSetting",
    "tabAlertModeSetting",
    "tabRecvVoiceCallRecordByChannel",
    "tabZone",
];

/// Radio-DB tables a scan member-list edit may change.
pub const SCAN_MEMBER_TABLES: &[&str] = &["tabScanListSettings"];

/// Radio-DB tables a knob gaining an analog conventional channel may change
/// (notes/conventional-knobs.md 8.4): no P25, encryption or contact tables.
/// `tabRecvSaList` changes only with a tone, `tabZone` only when the zone had
/// no active knob.
pub const CONV_KNOB_CHANNEL_TABLES: &[&str] = &[
    "tabReceivingMode",
    "tabZoneChannelList",
    "tabChannelAnnouncement",
    "tabChannel",
    "tabChannelAdvInfo",
    "tabFunctionOnOffOfChan",
    "tabFunctionParamOfChan",
    "tabVoiceCallSettings",
    "tabResetModeSetting",
    "tabR2RModeSetting",
    "tabAlertModeSetting",
    "tabRecvVoiceCallRecordByChannel",
    "tabRepeaterSettingOfChan",
    "tabDispatchChannel",
    "tabContactOfChan",
    "tabRecvGroupList",
    "tabRecvBehaviorByContactID",
    "tabRecvSaList",
    "tabZone",
];

/// Receive-list rows a copied trunked knob adds beyond `KNOB_CHANNEL_TABLES`.
/// The contact list itself stays: the copy lists talkgroups the source
/// already made contacts (notes/knob-copy-move.md 5).
pub const TRUNKED_COPY_TABLES: &[&str] = &["tabContactOfChan", "tabRecvGroupList", "tabRecvBehaviorByContactID"];

/// Radio-DB tables keyed by channel ID that digital conventional knobs add to
/// `ZONE_RENUMBER_TABLES`; a knob move renumbers them too.
pub const DIGITAL_RENUMBER_TABLES: &[&str] = &["tabDMRSignaling", "tabDMRRepeaterSetting", "tabP25Signaling", "tabP25RepeaterSetting"];
