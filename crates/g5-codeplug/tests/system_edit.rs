//! System-editing tests against private real-pager fixtures (skipped without them).
//! Each edit must change only the radio-DB tables `notes/system-editing.md` 9.7 allows.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::summary::Summary;
use g5_codeplug::system_edit::{
    ALL_GROUP_ID, ControlChannelEdit, NewSystem, SiteEdit, SystemEdit, TalkgroupEdit, apply_system_edit, create_system, current_state, delete_system,
    profile_bands,
};
use g5_codeplug::{Profile, Templates};

const NOW: &str = "2026-10-18 12:00:00";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn env_or(var: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).unwrap_or(default)
}

fn fixture() -> Option<(PathBuf, Templates, Variant)> {
    let templates = Templates::new(env_or("G5_TEMPLATES", repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"))).ok()?;
    let version = |p: &Path| {
        fs::read_to_string(p.join("MANIFEST.txt")).ok()?.lines().find_map(|l| l.strip_prefix("Radio DB version:").map(|v| v.trim().to_string()))
    };
    let dir = fs::read_dir(env_or("G5_BACKUPS", repo().join("backups")))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| version(p).as_deref().and_then(Variant::for_db_version).is_some())?;
    let variant = Variant::for_db_version(&version(&dir)?)?;
    Some((dir, templates, variant))
}

fn changed(profile: &Profile, templates: &Templates, variant: Variant, dir: &Path) -> Vec<String> {
    let out = tempfile::NamedTempFile::new().unwrap();
    convert(profile, templates, variant, out.path()).unwrap();
    let ours = sqlite2::Connection::open(out.path()).unwrap();
    let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
    compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect()
}

/// A system that some knob uses, plus one of its talkgroups on a knob with 2+ entries.
fn used_system(p: &Profile) -> (String, String) {
    let s = Summary::read(p).unwrap();
    for z in &s.zones {
        for k in &z.knobs {
            if let (Some(sid), true) = (&k.system_id, k.talkgroup_list.len() >= 2) {
                return (sid.clone(), k.talkgroup_list.last().unwrap().group_id.clone());
            }
        }
    }
    panic!("no knob with 2+ talkgroups");
}

fn assert_only(changed: &[String], allowed: &[&str]) {
    for t in changed {
        assert!(allowed.contains(&t.as_str()), "unexpected change in {t} (changed: {changed:?})");
    }
}

#[test]
fn unchanged_edit_is_a_no_op() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, _) = used_system(&p);
    let state = current_state(&p, &sid).unwrap();
    let ch = apply_system_edit(&p, &tpl, &state, NOW).unwrap();
    assert!(ch.is_empty(), "{ch:?}");
    assert_eq!(changed(&p, &tpl, v, &dir), Vec::<String>::new());
}

#[test]
fn properties_change_only_signaling_and_alias() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, _) = used_system(&p);
    let mut e = current_state(&p, &sid).unwrap();
    e.name = "Knobset Test".into();
    e.wacn ^= 1;
    let ch = apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    assert_eq!(ch.properties_changed, ["name", "WACN"]);
    let c = changed(&p, &tpl, v, &dir);
    assert_only(&c, &["tabSystemAlias", "tabP25TrunkingSignaling"]);
    assert_eq!(c.len(), 2);
}

#[test]
fn site_edits_change_only_sites() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, _) = used_system(&p);
    let mut e = current_state(&p, &sid).unwrap();
    let free = (0..=255).find(|n| !e.sites.iter().any(|s| s.rfss == 9 && s.site == *n)).unwrap();
    e.sites.push(SiteEdit { id: None, name: "New Site".into(), rfss: 9, site: free });
    e.sites[0].name = "Renamed".into();
    e.sites.rotate_right(1); // new site first: a reorder
    let ch = apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    assert_eq!((ch.sites_added, ch.sites_changed, ch.sites_reordered), (1, 1, true));
    assert_eq!(changed(&p, &tpl, v, &dir), ["tabP25TrunkingSites"]);
    // Reloads with the new order.
    assert_eq!(current_state(&p, &sid).unwrap().sites[0].name, "New Site");
}

#[test]
fn control_channel_edits_change_only_control_channels() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, _) = used_system(&p);
    let mut e = current_state(&p, &sid).unwrap();
    let base = e.control_channels[0].rx_hz;
    let new = (1..).map(|i| base + i * 12_500).find(|f| !e.control_channels.iter().any(|c| c.rx_hz == *f)).unwrap();
    e.control_channels.push(ControlChannelEdit { id: None, rx_hz: new, note: "added".into() });
    let ch = apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    assert_eq!(ch.ccs_added, 1);
    assert_eq!(changed(&p, &tpl, v, &dir), ["tabP25TrunkingControlChannels"]);
}

#[test]
fn talkgroup_edits_change_only_contacts_and_cascade_deletes() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let contact_tables = g5_codeplug::edit::TALKGROUP_EDIT_TABLES;

    // Rename a talkgroup that is on a knob.
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, gid) = used_system(&p);
    let mut e = current_state(&p, &sid).unwrap();
    e.talkgroups.iter_mut().find(|t| t.group_id == gid).unwrap().name = "TG Renamed".into();
    apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    let c = changed(&p, &tpl, v, &dir);
    assert_eq!(c, ["tabContacts"]);

    // Delete it: removed from the knob, contact tables only.
    let p = Profile::from_backup(&dir).unwrap();
    let mut e = current_state(&p, &sid).unwrap();
    e.talkgroups.retain(|t| t.group_id != gid);
    let ch = apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    assert_eq!(ch.tgs_removed, 1);
    assert!(ch.knob_entries_removed >= 1);
    assert_only(&changed(&p, &tpl, v, &dir), contact_tables);
    assert!(!Summary::read(&p).unwrap().zones.iter().flat_map(|z| &z.knobs).any(|k| k.talkgroup_list.iter().any(|t| t.group_id == gid)));

    // Add one nobody uses: no radio change at all.
    let p = Profile::from_backup(&dir).unwrap();
    let mut e = current_state(&p, &sid).unwrap();
    let tgid = (1..=65535).find(|n| !e.talkgroups.iter().any(|t| t.tgid == *n)).unwrap();
    e.talkgroups.push(TalkgroupEdit { group_id: "0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e6f".into(), tgid, name: "Brand New".into(), note: String::new(), ..Default::default() });
    let ch = apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    assert_eq!(ch.tgs_added, 1);
    assert_eq!(changed(&p, &tpl, v, &dir), Vec::<String>::new());

    // New rows get PPS's editor defaults, or its RadioReference importer's (silent, no voice storage).
    let p = Profile::from_backup(&dir).unwrap();
    let mut e = current_state(&p, &sid).unwrap();
    let taken: Vec<i64> = e.talkgroups.iter().map(|t| t.tgid).collect();
    let mut free = (1..=65535).filter(|n| !taken.contains(n));
    for (gid, imported) in [("0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e60", false), ("0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e61", true)] {
        e.talkgroups.push(TalkgroupEdit { group_id: gid.into(), tgid: free.next().unwrap(), name: gid[..8].into(), note: String::new(), imported, ..Default::default() });
    }
    apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    let r = p
        .system_db(&sid)
        .unwrap()
        .query("SELECT RecordFlag, AlertMode, AlertToneID, KeyStrapOption FROM tabGroup_New WHERE GroupID LIKE '0b9a6c1e-%' ORDER BY GroupID", &[])
        .unwrap();
    let got: Vec<Vec<&str>> = r.rows.iter().map(|row| row.iter().map(|v| v.as_deref().unwrap_or("")).collect()).collect();
    assert_eq!(got, [["1", "1", "9999", "2"], ["0", "3", "9999", "1"]]);
}

/// `AlertMode, AlertToneID, RecordFlag, BackgroundColor, KeyStrapOption` of one talkgroup row.
fn settings_row(p: &Profile, sid: &str, gid: &str) -> Vec<String> {
    let r = p
        .system_db(sid)
        .unwrap()
        .query("SELECT AlertMode, AlertToneID, RecordFlag, BackgroundColor, KeyStrapOption FROM tabGroup_New WHERE GroupID = ?", &[Some(gid)])
        .unwrap();
    r.rows[0].iter().map(|v| v.clone().unwrap_or_default()).collect()
}

/// One backup per radio DB version (pager A runs 1.41b, B 1.41c, C 1.41e).
fn fixture_per_version() -> Vec<(PathBuf, Templates, Variant)> {
    let Some((_, tpl, _)) = fixture() else { return Vec::new() };
    let mut seen = Vec::new();
    let mut dirs: Vec<PathBuf> = fs::read_dir(env_or("G5_BACKUPS", repo().join("backups"))).unwrap().filter_map(|e| e.ok().map(|e| e.path())).collect();
    dirs.sort();
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(manifest) = fs::read_to_string(dir.join("MANIFEST.txt")) else { continue };
        // Firmware-migrated radio DBs (stale sidecar) weren't made by a converter.
        if manifest.contains("/PPS/u3db.db: MISMATCH") {
            continue;
        }
        let Some(ver) = manifest.lines().find_map(|l| l.strip_prefix("Radio DB version:").map(|v| v.trim().to_string())) else { continue };
        let Some(v) = Variant::for_db_version(&ver) else { continue };
        if !seen.contains(&ver) {
            seen.push(ver);
            out.push((dir, Templates::new(&tpl.dir).unwrap(), v));
        }
    }
    out
}

#[test]
fn talkgroup_settings_change_only_their_radio_tables() {
    let all = fixture_per_version();
    if all.is_empty() {
        return eprintln!("skipping: fixtures not available");
    }
    for (dir, tpl, v) in all {
        eprintln!("talkgroup settings on {:?}", v);
        settings_on(&dir, &tpl, v);
    }
}

fn settings_on(dir: &Path, tpl: &Templates, v: Variant) {
    let p0 = Profile::from_backup(dir).unwrap();
    let (sid, gid) = used_system(&p0);
    let cur = current_state(&p0, &sid).unwrap().talkgroups.into_iter().find(|t| t.group_id == gid).unwrap();
    let before = settings_row(&p0, &sid, &gid);
    let mode = cur.alert_mode.unwrap() % 4 + 1;
    let bg = (cur.background.unwrap() + 1) % 8;
    let storage = !cur.voice_storage.unwrap();
    let tone = if cur.alert_tone.as_deref() == Some("5") { "6" } else { "5" };
    type Case = (&'static str, Box<dyn Fn(&mut TalkgroupEdit)>, usize, String, &'static [&'static str]);
    let cases: Vec<Case> = vec![
        ("alert mode", Box::new(move |t| t.alert_mode = Some(mode)), 0, mode.to_string(), &["tabRecvBehaviorByContactID"]),
        ("alert tone", Box::new(move |t| t.alert_tone = Some(tone.into())), 1, tone.to_string(), &["tabContactAddrAndAlertTone", "tabRecvToneForContacts"]),
        ("chirp", Box::new(|t| t.alert_tone = Some("10000".into())), 1, "10000".into(), &["tabContactAddrAndAlertTone", "tabRecvToneForContacts"]),
        ("voice storage", Box::new(move |t| t.voice_storage = Some(storage)), 2, if storage { "1" } else { "0" }.into(), &["tabRecvBehaviorByContactID", "tabRecvVoiceCallRecordByContactID"]),
        ("background", Box::new(move |t| t.background = Some(bg)), 3, bg.to_string(), &["tabContactOfBackground"]),
    ];
    for (what, f, col, want, tables) in cases {
        let p = Profile::from_backup(dir).unwrap();
        let mut e = current_state(&p, &sid).unwrap();
        f(e.talkgroups.iter_mut().find(|t| t.group_id == gid).unwrap());
        let ch = apply_system_edit(&p, tpl, &e, NOW).unwrap();
        assert_eq!((ch.tgs_settings_changed, ch.tgs_changed), (1, 0), "{what}");
        let mut after = before.clone();
        after[col] = want;
        assert_eq!(settings_row(&p, &sid, &gid), after, "{what}: only its column changes");
        assert_eq!(changed(&p, tpl, v, dir), tables, "{what}");
    }

    // Tone "None" writes no tone rows at all; from a tone, both tone tables lose a row.
    let p = Profile::from_backup(dir).unwrap();
    let mut e = current_state(&p, &sid).unwrap();
    let t = e.talkgroups.iter_mut().find(|t| t.group_id == gid).unwrap();
    t.alert_tone = Some(if cur.alert_tone.as_deref() == Some("0") { "9999" } else { "0" }.into());
    apply_system_edit(&p, tpl, &e, NOW).unwrap();
    assert_eq!(changed(&p, tpl, v, dir), ["tabContactAddrAndAlertTone", "tabRecvToneForContacts"]);

    // All four at once on a talkgroup no knob uses: profile only.
    let p = Profile::from_backup(dir).unwrap();
    let used: Vec<String> = Summary::read(&p).unwrap().zones.iter().flat_map(|z| &z.knobs).flat_map(|k| k.talkgroup_list.iter().map(|t| t.group_id.clone())).collect();
    let mut e = current_state(&p, &sid).unwrap();
    let t = e.talkgroups.iter_mut().find(|t| !used.contains(&t.group_id)).expect("a talkgroup on no knob");
    let free = t.group_id.clone();
    (t.alert_mode, t.alert_tone, t.voice_storage, t.background) = (Some(4), Some("10000".into()), Some(true), Some(6));
    let ch = apply_system_edit(&p, tpl, &e, NOW).unwrap();
    assert_eq!(ch.tgs_settings_changed, 1);
    assert_eq!(settings_row(&p, &sid, &free)[..4], ["4", "10000", "1", "6"]);
    assert_eq!(changed(&p, tpl, v, dir), Vec::<String>::new());
}

#[test]
fn bulk_talkgroup_settings() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, _) = used_system(&p);
    let mut e = current_state(&p, &sid).unwrap();
    let want = (Some(4), Some("10000".to_string()), Some(true), Some(2));
    let differs = e.talkgroups.iter().filter(|t| (t.alert_mode, t.alert_tone.clone(), t.voice_storage, t.background) != want).count();
    for t in &mut e.talkgroups {
        (t.alert_mode, t.alert_tone, t.voice_storage, t.background) = want.clone();
    }
    let ch = apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    assert_eq!((ch.tgs_settings_changed, ch.tgs_changed, ch.tgs_added, ch.tgs_removed), (differs, 0, 0, 0));
    let c = changed(&p, &tpl, v, &dir);
    assert_only(&c, g5_codeplug::edit::TALKGROUP_SETTINGS_TABLES);
    assert!(c.iter().any(|t| t == "tabRecvBehaviorByContactID"), "{c:?}");
    let back = current_state(&p, &sid).unwrap();
    assert!(back.talkgroups.iter().all(|t| (t.alert_mode, t.alert_tone.clone(), t.voice_storage, t.background) == want));
    // Names, TGIDs and order are untouched.
    let names = |s: &SystemEdit| s.talkgroups.iter().map(|t| (t.group_id.clone(), t.tgid, t.name.clone())).collect::<Vec<_>>();
    assert_eq!(names(&back), names(&e));
    // Applying the same state again is a no-op.
    assert!(apply_system_edit(&p, &tpl, &back, NOW).unwrap().is_empty());
}

#[test]
fn new_talkgroup_settings_override_defaults() {
    let Some((dir, tpl, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, _) = used_system(&p);
    let mut e = current_state(&p, &sid).unwrap();
    let taken: Vec<i64> = e.talkgroups.iter().map(|t| t.tgid).collect();
    let mut free = (1..=65535).filter(|n| !taken.contains(n));
    let mut add = |gid: &str, imported: bool, alert_mode: Option<i64>, voice_storage: Option<bool>| {
        e.talkgroups.push(TalkgroupEdit { group_id: gid.into(), tgid: free.next().unwrap(), name: gid[..8].into(), imported, alert_mode, voice_storage, ..Default::default() })
    };
    add("0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e70", true, Some(1), None);
    add("0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e71", true, None, None);
    add("0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e72", false, Some(2), Some(false));
    apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    // An imported row keeps the importer's KeyStrapOption 1 and silence unless overridden.
    assert_eq!(settings_row(&p, &sid, "0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e70"), ["1", "9999", "0", "0", "1"]);
    assert_eq!(settings_row(&p, &sid, "0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e71"), ["3", "9999", "0", "0", "1"]);
    assert_eq!(settings_row(&p, &sid, "0b9a6c1e-3f2d-4c8a-9e7b-1a2b3c4d5e72"), ["2", "9999", "0", "0", "2"]);
}

#[test]
fn talkgroup_settings_validation_keeps_stored_values() {
    let Some((dir, tpl, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, gid) = used_system(&p);
    let base = current_state(&p, &sid).unwrap();
    let try_tg = |f: &dyn Fn(&mut TalkgroupEdit)| {
        let mut e = base.clone();
        f(e.talkgroups.iter_mut().find(|t| t.group_id == gid).unwrap());
        apply_system_edit(&p, &tpl, &e, NOW)
    };
    assert!(try_tg(&|t| t.alert_mode = Some(5)).is_err(), "alert mode range");
    assert!(try_tg(&|t| t.alert_mode = Some(0)).is_err(), "alert mode 0 isn't offered");
    assert!(try_tg(&|t| t.alert_tone = Some("9".into())).is_err(), "only Tone 1..8");
    assert!(try_tg(&|t| t.alert_tone = Some("6f1d2c3b-4a59-4e87-9d6c-5b4a3f2e1d0c".into())).is_err(), "custom tones aren't set by Knobset");
    assert!(try_tg(&|t| t.background = Some(8)).is_err(), "background range");
    assert_eq!(current_state(&p, &sid).unwrap(), base, "refused edits leave the working copy untouched");

    // A custom tone (or any other value PPS wrote) already on a row passes through untouched.
    let custom = "6f1d2c3b-4a59-4e87-9d6c-5b4a3f2e1d0c";
    p.system_db(&sid).unwrap().execute("UPDATE tabGroup_New SET AlertToneID = ?, AlertMode = '0' WHERE GroupID = ?", &[Some(custom), Some(&gid)]).unwrap();
    let mut e = current_state(&p, &sid).unwrap();
    let t = e.talkgroups.iter_mut().find(|t| t.group_id == gid).unwrap();
    assert_eq!((t.alert_tone.as_deref(), t.alert_mode), (Some(custom), Some(0)));
    t.background = Some((t.background.unwrap() + 1) % 8);
    let ch = apply_system_edit(&p, &tpl, &e, NOW).unwrap();
    assert_eq!(ch.tgs_settings_changed, 1);
    assert_eq!(settings_row(&p, &sid, &gid)[..2], ["0", custom]);
}

#[test]
fn rejects_invalid_edits() {
    let Some((dir, tpl, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (sid, _) = used_system(&p);
    let base = current_state(&p, &sid).unwrap();
    let try_edit = |f: &dyn Fn(&mut SystemEdit)| {
        let mut e = base.clone();
        f(&mut e);
        apply_system_edit(&p, &tpl, &e, NOW)
    };
    assert!(try_edit(&|e| e.control_channels.push(ControlChannelEdit { id: None, rx_hz: 50_000_000, note: String::new() })).is_err(), "out of band");
    assert!(try_edit(&|e| {
        let s = e.sites[0].clone();
        e.sites.push(SiteEdit { id: None, name: "Dup".into(), ..s });
    })
    .is_err(), "duplicate RFSS/site");
    assert!(try_edit(&|e| e.sites.clear()).is_err(), "no sites");
    assert!(try_edit(&|e| e.name = "x".repeat(15)).is_err(), "long name");
    assert!(try_edit(&|e| e.wacn = 0x100000).is_err(), "WACN range");
    assert!(try_edit(&|e| e.talkgroups.push(TalkgroupEdit { group_id: "NOT-A-GUID".into(), tgid: 1, name: "x".into(), note: String::new(), ..Default::default() })).is_err(), "bad new id");

    // Deleting every talkgroup on a knob must be refused, and leave the working copy untouched.
    let s = Summary::read(&p).unwrap();
    let knob = s.zones.iter().flat_map(|z| &z.knobs).find(|k| k.system_id.as_deref() == Some(&sid)).unwrap();
    let on_knob: Vec<String> = knob.talkgroup_list.iter().map(|t| t.group_id.clone()).collect();
    assert!(try_edit(&|e| e.talkgroups.retain(|t| !on_knob.contains(&t.group_id))).is_err(), "empties a knob");
    assert_eq!(current_state(&p, &sid).unwrap(), base, "a refused edit must roll back");
}

const NEW_ID: &str = "6f1d2c3b-4a59-4e87-9d6c-5b4a3f2e1d0c";

fn new_system(p: &Profile, tpl: &Templates) -> NewSystem {
    let band = profile_bands(p, tpl).unwrap().into_iter().next().expect("model has a band");
    let lo = band.ranges[0].0;
    NewSystem {
        band: band.code,
        system: SystemEdit {
            system_id: NEW_ID.into(),
            name: "Knobset New".into(),
            wacn: 0xBEE00,
            sysid: 0x3A1,
            sites: vec![
                SiteEdit { id: None, name: "North".into(), rfss: 5, site: 0x18 },
                SiteEdit { id: None, name: "South".into(), rfss: 5, site: 0x19 },
            ],
            control_channels: vec![
                ControlChannelEdit { id: None, rx_hz: lo + 12_500, note: "primary".into() },
                ControlChannelEdit { id: None, rx_hz: lo + 25_000, note: String::new() },
            ],
            talkgroups: vec![
                TalkgroupEdit { group_id: "a1b2c3d4-0000-4000-8000-000000000001".into(), tgid: 1001, name: "FD Dispatch".into(), note: String::new(), ..Default::default() },
                TalkgroupEdit { group_id: "a1b2c3d4-0000-4000-8000-000000000002".into(), tgid: 1002, name: "PD Main".into(), note: "busy".into(), ..Default::default() },
            ],
        },
    }
}

#[test]
fn create_system_writes_a_complete_unused_system() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let before = Summary::read(&p).unwrap().systems.len();
    let n = new_system(&p, &tpl);
    let ch = create_system(&p, &tpl, &n, NOW).unwrap();
    assert!(ch.created);
    assert_eq!((ch.sites_added, ch.ccs_added, ch.tgs_added), (2, 2, 2));

    // A system no knob uses never reaches the radio DB.
    assert_eq!(changed(&p, &tpl, v, &dir), Vec::<String>::new());

    // Reads back as written, with the template's "All" talkgroup first.
    let st = current_state(&p, NEW_ID).unwrap();
    assert_eq!((st.name.as_str(), st.wacn, st.sysid), ("Knobset New", 0xBEE00, 0x3A1));
    assert_eq!(st.sites.iter().map(|s| (s.rfss, s.site)).collect::<Vec<_>>(), [(5, 0x18), (5, 0x19)]);
    assert_eq!(st.control_channels.len(), 2);
    assert_eq!(st.talkgroups.iter().map(|t| t.group_id.as_str()).collect::<Vec<_>>()[0], ALL_GROUP_ID);
    assert_eq!(st.talkgroups.len(), 3);

    // Index row and system DB rows per notes/system-creation.md 7.2.
    let s = Summary::read(&p).unwrap();
    assert_eq!(s.systems.len(), before + 1);
    let idx = g5_codeplug::row::load_table(&p.db, "tabP25TrunkingSecurity").into_iter().find(|r| r.s("P25TrunkingID") == NEW_ID).unwrap();
    for (col, want) in [
        ("DBFileName", NEW_ID.to_string()),
        ("SystemName", "Knobset New".into()),
        ("WacnID", 0xBEE00.to_string()),
        ("SiteCount", "2".into()),
        ("ControlCHCount", "2".into()),
        ("TGIDCount", "3".into()),
        ("SystemCreateType", "2".into()),
        ("BandTypeCode", n.band.clone()),
    ] {
        assert_eq!(idx.s(col), want, "{col}");
    }
    let sys = p.system_db(NEW_ID).unwrap();
    let row = g5_codeplug::row::load_table(&sys, "tabP25TrunkingSystem").remove(0);
    for (col, want) in [("ID", NEW_ID), ("SystemCreateType", "2"), ("ConverageType", "0"), ("FullSpectrumScanRanges", "1"), ("ErrorRateThreshold", "5"), ("SUID", "")] {
        assert_eq!(row.s(col), want, "{col}");
    }
    assert_eq!(g5_codeplug::row::load_table(&sys, "tabRFBandTypeInfo")[0].s("Code"), n.band);
}

#[test]
fn create_system_rejects_bad_input_and_cleans_up() {
    let Some((dir, tpl, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let base = new_system(&p, &tpl);
    let existing = Summary::read(&p).unwrap().systems[0].name.clone();
    let index_len = || g5_codeplug::row::load_table(&p.db, "tabP25TrunkingSecurity").len();
    let n0 = index_len();
    let try_new = |f: &dyn Fn(&mut NewSystem)| {
        let mut n = base.clone();
        f(&mut n);
        create_system(&p, &tpl, &n, NOW)
    };
    assert!(try_new(&|n| n.band = "NX".into()).is_err(), "band not on this model");
    assert!(try_new(&|n| n.system.system_id = "NOT-A-GUID".into()).is_err(), "bad id");
    // These fail after the file and index row exist: both must be undone.
    assert!(try_new(&|n| n.system.name = existing.clone()).is_err(), "duplicate name");
    assert!(try_new(&|n| n.system.control_channels.clear()).is_err(), "no control channel");
    assert!(try_new(&|n| n.system.control_channels[0].rx_hz = 50_000_000).is_err(), "out of band");
    assert_eq!(index_len(), n0);
    assert!(!p.folder.join(format!("{NEW_ID}.db")).exists(), "failed create left its file");
    // And a clean retry still works.
    create_system(&p, &tpl, &base, NOW).unwrap();
}

#[test]
fn delete_system_refuses_used_and_removes_unused() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (used, _) = used_system(&p);
    let err = delete_system(&p, &used, NOW).unwrap_err().to_string();
    assert!(err.contains("knob"), "{err}");
    assert!(p.system_db(&used).is_ok(), "refused delete must keep the file");

    create_system(&p, &tpl, &new_system(&p, &tpl), NOW).unwrap();
    let ch = delete_system(&p, NEW_ID, NOW).unwrap();
    assert!(ch.deleted);
    assert!(!p.folder.join(format!("{NEW_ID}.db")).exists());
    assert!(!Summary::read(&p).unwrap().systems.iter().any(|s| s.id == NEW_ID));
    assert_eq!(changed(&p, &tpl, v, &dir), Vec::<String>::new());
}

#[test]
fn repack_adds_created_and_drops_deleted_system_dbs() {
    let Some((dir, tpl, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let entries = |zip: &[u8]| -> Vec<String> {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(zip)).unwrap();
        (0..z.len()).map(|i| z.by_index(i).unwrap().name().unwrap().into_owned()).collect()
    };
    let original = dir.join(g5_codeplug::source::BACKUP_PROFILE_ZIP);
    let p = Profile::from_backup(&dir).unwrap();
    create_system(&p, &tpl, &new_system(&p, &tpl), NOW).unwrap();
    let a = g5_codeplug::edit::repack_zip(&original, &p, NOW).unwrap();
    let before = entries(&fs::read(&original).unwrap());
    let after = entries(&a);
    assert_eq!(after.len(), before.len() + 1);
    assert_eq!(after[..before.len()], before[..], "existing entries keep their order");
    assert!(after.last().unwrap().ends_with(&format!("/{NEW_ID}.db")));

    let a_path = tempfile::NamedTempFile::new().unwrap();
    fs::write(a_path.path(), &a).unwrap();
    let q = Profile::from_zip(a_path.path()).unwrap();
    assert_eq!(current_state(&q, NEW_ID).unwrap().name, "Knobset New");
    delete_system(&q, NEW_ID, NOW).unwrap();
    let b = g5_codeplug::edit::repack_zip(a_path.path(), &q, NOW).unwrap();
    assert_eq!(entries(&b), before, "deleting the new system restores the original entry list");
}
