//! Putting a P25 trunking system on a knob, per `notes/system-creation.md` 7.4:
//! an empty knob position becomes a trunked knob, or a trunked knob moves to
//! another system. Converting a conventional or scan knob is not supported
//! yet (it has to unpick scan-list memberships and priority references).

use serde::{Deserialize, Serialize};

use crate::edit::{KnobTalkgroups, check_knob_list, refuse_sub_features, write_knob_list};
use crate::row::{Row, load_table, sel};
use crate::system_edit::{check_name, is_guid};
use crate::{Error, Profile};

/// `tabReceivingMode_NewG4.CurrentMode` for a trunked talkgroup-scan knob.
pub const MODE_TG_SCAN: i64 = 16;
/// `CurrentMode` for a trunked talkgroup-monitor knob.
pub const MODE_TG_MONITOR: i64 = 48;
/// PPS's built-in knob voice prompt for each trunked mode (Knobset defaults to none).
pub const fn builtin_prompt(mode: i64) -> i64 {
    if mode == MODE_TG_MONITOR { 11 } else { 6 }
}
const ZERO_KEY: &str = "000000000000";

/// Put `system_id` on a knob, with its new talkgroup list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnobSystemEdit {
    pub zone: String,
    pub knob: i64,
    pub system_id: String,
    /// `MODE_TG_SCAN` or `MODE_TG_MONITOR`. A trunked knob in the other mode
    /// switches (notes/knob-editing.md 8.4).
    pub mode: i64,
    /// New lowercase GUID for an empty knob's channel; ignored for a trunked knob.
    #[serde(default)]
    pub channel_id: Option<String>,
    /// Alias for an empty knob (required there); trunked knobs keep theirs (rename separately).
    #[serde(default)]
    pub name: Option<String>,
    /// Knob voice prompt for an empty knob: 0 (none) or the mode's built-in prompt.
    #[serde(default)]
    pub voice_prompt: i64,
    pub priority: Vec<String>,
    pub normal: Vec<String>,
}

/// What an assignment did.
#[derive(Debug, Clone, Serialize)]
pub struct KnobSystemChange {
    pub zone: String,
    pub knob: i64,
    pub knob_name: String,
    /// The knob was an empty position before.
    pub new_knob: bool,
    pub mode: i64,
    pub old_system: Option<String>,
    pub new_system: String,
    pub talkgroups: usize,
    /// A trunked knob switched between TG Scan and TG Monitor.
    pub mode_switched: bool,
    /// The knob's system or talkgroup list changed (not just its mode).
    pub list_changed: bool,
}

fn zone_id(p: &Profile, zone: &str) -> Result<String, Error> {
    let zones = load_table(&p.db, "tabZone_New");
    let matches = sel(&zones, "Name", zone);
    let [z] = matches.as_slice() else {
        return Err(Error::Invalid(format!("expected exactly one zone named {zone:?}, found {}", matches.len())));
    };
    Ok(z.s("ID").to_string())
}

fn system_name(p: &Profile, id: &str) -> Option<String> {
    load_table(&p.db, "tabP25TrunkingSecurity").into_iter().find(|r| r.s("P25TrunkingID") == id).map(|r| r.s("SystemName").to_string())
}

/// The first P25 trunking protocol setting (type 9), as PPS's knob editor picks it.
fn trunking_protocol(p: &Profile) -> Result<String, Error> {
    load_table(&p.db, "tabProtocolSetting")
        .into_iter()
        .find(|r| r.s("ProtocolType") == "9")
        .map(|r| r.s("ID").to_string())
        .ok_or_else(|| Error::Unsupported("a profile without a P25 trunking protocol setting (add one in Unication PPS first)".into()))
}

/// The `tabP25TrunkingSystemKnob` row PPS saves for a new trunked knob with untouched controls.
fn default_knob_row(cols: &[String], zone_id: &str, knob: i64, channel: &str, system: &str, protocol: &str) -> Result<Vec<String>, Error> {
    let knob = knob.to_string();
    cols.iter()
        .map(|c| {
            Ok(match c.as_str() {
                "ZoneID" => zone_id,
                "KnobIndex" => &knob,
                "ChannelID" => channel,
                "P25TrunkingID" => system,
                "ProtocolSettingID" => protocol,
                "TxPowerLevel" => "2",
                "HangTime" => "50",
                "PriorityGroup" | "VoiceBuffering" | "AlertMode" | "RecordFlag" | "AlwaysUnreadReminderAlert" => "1",
                "MaxMSELSamplingTime" => "2",
                "DelayNTime" | "TgidStayTime" => "30",
                "DelayNTimeEx" | "CarrierOffDelayTime" => "5",
                "EncryptionKeyID" | "KMFSystemID" | "PrivateEncryptionKeyID" | "ATGEncryptionKeyID" | "DynRegroupEncryptionKeyID" => ZERO_KEY,
                "DynRegroupATGEncryptionKeyID" => "",
                "AudioSquelch" | "OpenSpeakerLogic" | "DispatchCHFlag" | "ATGID" | "RFPerformance" | "PushToListen" | "Privacy" | "VoiceBufferPrompt"
                | "VoiceBufferingForceDrop" | "DutyStatus" | "MSELCallSamplingFlag" | "ResetMode" | "EnableTgidStayTime" | "AlwaysUnreadVoiceMessage"
                | "EncryptedCallFlag" | "EncryptionBy" | "KeyStrapOption" | "PrivateKeyStrapOption" | "ATGKeyStrapOption" | "UserSelectableKeyset"
                | "UserSelectableKey" | "DynRegroupKeyStrapOption" | "DynRegroupATGKeyStrapOption" | "DelayNFeature" | "EnableCarrierOffTime" => "0",
                other => return Err(Error::Unsupported(format!("unknown knob column {other}"))),
            }
            .to_string())
        })
        .collect()
}

/// Apply `e` to a working copy. `now` is the local time for `tabOrganization_New.FinishDate`.
pub fn assign_knob_system(p: &Profile, e: &KnobSystemEdit, now: &str) -> Result<KnobSystemChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    if ![MODE_TG_SCAN, MODE_TG_MONITOR].contains(&e.mode) {
        return Err(Error::Invalid(format!("mode {} isn't a trunked talkgroup mode", e.mode)));
    }
    let zid = zone_id(p, &e.zone)?;
    let recv = load_table(&p.db, "tabReceivingMode_NewG4");
    let r: &Row = recv
        .iter()
        .find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == e.knob)
        .ok_or_else(|| Error::Invalid(format!("{:?} has no knob position {}", e.zone, e.knob)))?;
    let new_system = system_name(p, &e.system_id).ok_or_else(|| Error::Invalid(format!("system {} is not in this profile", e.system_id)))?;
    p.system_db(&e.system_id)?;
    let list = KnobTalkgroups { priority: e.priority.clone(), normal: e.normal.clone() };
    let knobs = load_table(&p.db, "tabP25TrunkingSystemKnob");
    let existing = knobs.iter().find(|k| k.s("ZoneID") == zid && k.i("KnobIndex") == e.knob);
    let current = r.i("CurrentMode");

    let (old_system, knob_name, channel, protocol) = match (current, existing) {
        (0, None) => {
            let name = e.name.as_deref().ok_or_else(|| Error::Invalid("a new knob needs a name".into()))?;
            check_name("Knob", name)?;
            if ![0, builtin_prompt(e.mode)].contains(&e.voice_prompt) {
                return Err(Error::Invalid(format!("voice prompt {} isn't available for this mode", e.voice_prompt)));
            }
            let ch = e.channel_id.as_deref().filter(|c| is_guid(c)).ok_or_else(|| Error::Invalid("a new knob needs a lowercase GUID channel ID".into()))?;
            check_knob_list(p, &e.system_id, "0", &list)?;
            (None, name.to_string(), ch.to_string(), Some(trunking_protocol(p)?))
        }
        (m, Some(k)) if m == MODE_TG_SCAN || m == MODE_TG_MONITOR => {
            if m != e.mode {
                if load_table(&p.db, "tabP25TCallAlertEnable").iter().any(|r| r.s("ChannelID") == k.s("ChannelID") && r.s("EnableCallAlert") == "1") {
                    return Err(Error::Unsupported("switching a knob with call alerts".into()));
                }
                // PPS would quietly reset these for TG Monitor; refuse non-defaults instead.
                for (col, want) in [
                    ("ResetMode", "0"),
                    ("DelayNTime", "30"),
                    ("EnableTgidStayTime", "0"),
                    ("TgidStayTime", "30"),
                    ("DelayNFeature", "0"),
                    ("DelayNTimeEx", "5"),
                    ("EnableCarrierOffTime", "0"),
                    ("CarrierOffDelayTime", "5"),
                ] {
                    if e.mode == MODE_TG_MONITOR && k.get(col).is_some_and(|v| v != want) {
                        return Err(Error::Unsupported(format!("switching to TG Monitor with a non-default {col}; reset it in PPS first")));
                    }
                }
                // A custom prompt stays; built-ins (6, 8, 11) and none take the caller's choice.
                if ![0, builtin_prompt(e.mode)].contains(&e.voice_prompt) {
                    return Err(Error::Invalid(format!("voice prompt {} isn't available for this mode", e.voice_prompt)));
                }
            }
            let ch = k.s("ChannelID").to_string();
            refuse_sub_features(p, &ch)?;
            check_knob_list(p, &e.system_id, k.s("ATGID"), &list)?;
            (system_name(p, k.s("P25TrunkingID")), r.s("ChannelName").to_string(), ch, None)
        }
        (0, Some(_)) => return Err(Error::Invalid(format!("{:?} knob {} is empty but has a leftover trunking row", e.zone, e.knob))),
        _ => return Err(Error::Unsupported("putting a P25 system on a conventional or scan knob".into())),
    };

    let list_changed = match existing {
        None => true,
        Some(k) => {
            let rows = sel(&load_table(&p.db, "tabChannelGroup"), "ChannelID", &channel).into_iter().cloned().collect::<Vec<_>>();
            let mut pri: Vec<&Row> = rows.iter().filter(|r| r.i("SeqNo") > 0).collect();
            pri.sort_by_key(|r| r.i("SeqNo"));
            let cur_pri: Vec<String> = pri.iter().map(|r| r.s("GroupID").to_string()).collect();
            let cur_norm: Vec<String> = rows.iter().filter(|r| r.i("SeqNo") <= 0).map(|r| r.s("GroupID").to_string()).collect();
            k.s("P25TrunkingID") != e.system_id || cur_pri != list.priority || cur_norm != list.normal
        }
    };
    p.db.execute_batch("BEGIN;")?;
    let res = (|| -> Result<usize, Error> {
        if let Some(protocol) = &protocol {
            p.db.execute(
                "UPDATE tabReceivingMode_NewG4 SET CurrentMode = ?, ChannelName = ?, AlertToneID = ?, AlertFileName = coalesce(AlertFileName, '') WHERE ZoneID = ? AND KnobIndex = ?",
                &[Some(&e.mode.to_string()), Some(&knob_name), Some(&e.voice_prompt.to_string()), Some(&zid), Some(r.s("KnobIndex"))],
            )?;
            let cols = p.db.table_columns("tabP25TrunkingSystemKnob")?;
            let row = default_knob_row(&cols, &zid, e.knob, &channel, &e.system_id, protocol)?;
            let params: Vec<Option<&str>> = row.iter().map(|v| Some(v.as_str())).collect();
            p.db.execute(&format!("INSERT INTO tabP25TrunkingSystemKnob VALUES({})", vec!["?"; row.len()].join(",")), &params)?;
            if e.mode == MODE_TG_MONITOR {
                p.db.execute("INSERT INTO tabP25TSubGroupEnable(ChannelID,GroupID,Enable2TSubGroup) VALUES(?,'','1')", &[Some(&channel)])?;
            }
        } else {
            p.db.execute("UPDATE tabP25TrunkingSystemKnob SET P25TrunkingID = ? WHERE ChannelID = ?", &[Some(&e.system_id), Some(&channel)])?;
            if current != e.mode {
                let tone = r.s("AlertToneID");
                let builtin = ["", "0", "6", "8", "11"].contains(&tone);
                let tone = if builtin { e.voice_prompt.to_string() } else { tone.to_string() };
                p.db.execute(
                    "UPDATE tabReceivingMode_NewG4 SET CurrentMode = ?, AlertToneID = ? WHERE ZoneID = ? AND KnobIndex = ?",
                    &[Some(&e.mode.to_string()), Some(&tone), Some(&zid), Some(r.s("KnobIndex"))],
                )?;
                p.db.execute("DELETE FROM tabP25TSubGroupEnable WHERE ChannelID = ?", &[Some(&channel)])?;
                if e.mode == MODE_TG_MONITOR {
                    p.db.execute("INSERT INTO tabP25TSubGroupEnable(ChannelID,GroupID,Enable2TSubGroup) VALUES(?,'','1')", &[Some(&channel)])?;
                }
            }
        }
        // A pure mode switch leaves the knob's talkgroup rows as they are.
        let n = if list_changed { write_knob_list(p, &channel, &list)? } else { list.priority.len() + list.normal.len() };
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(n)
    })();
    match res {
        Ok(n) => {
            p.db.execute_batch("COMMIT;")?;
            Ok(KnobSystemChange { zone: e.zone.clone(), knob: e.knob, knob_name, new_knob: existing.is_none(), mode: e.mode, old_system, new_system, talkgroups: n, mode_switched: existing.is_some() && current != e.mode, list_changed })
        }
        Err(err) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(err)
        }
    }
}
