//! Removing knobs, editing conventional knobs, and switching TG Scan/Monitor
//! against private real-pager fixtures (skipped without them), per
//! notes/knob-editing.md 8.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::conv_knob::{DcsCode, Squelch};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::edit::{CONV_EDIT_TABLES, CONV_KNOB_CHANNEL_TABLES, KNOB_CHANNEL_TABLES, MODE_SWITCH_TABLES, SCAN_KNOB_TABLES, SCAN_MEMBER_TABLES, SYSTEM_EDIT_TABLES, TALKGROUP_EDIT_TABLES};
use g5_codeplug::knob_remove::{ConvKnobUpdate, KnobKind, KnobRemoval, remove_knob, update_conv_knob};
use g5_codeplug::knob_system::{KnobSystemEdit, MODE_TG_MONITOR, MODE_TG_SCAN, assign_knob_system};
use g5_codeplug::row::load_table;
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::summary::{Knob, Summary};
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
    let report = convert(profile, templates, variant, out.path()).unwrap();
    assert!(report.unverified.is_empty(), "unverified: {:?}", report.unverified);
    let ours = sqlite2::Connection::open(out.path()).unwrap();
    let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
    compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect()
}

fn assert_only(c: &[String], allowed: &[&[&str]]) {
    for t in c {
        assert!(allowed.iter().any(|a| a.contains(&t.as_str())), "unexpected change in {t} (changed: {c:?})");
    }
}

/// The real priority scan's zone, the scan knob, and its member knobs (priority first).
fn priority_scan(p: &Profile) -> (String, Knob, Vec<Knob>) {
    let s = Summary::read(p).unwrap();
    for z in s.zones {
        if let Some(scan) = z.knobs.iter().find(|k| k.mode == 36) {
            let members = scan.scan_members.iter().map(|m| z.knobs.iter().find(|k| k.channel_id.as_deref() == Some(m)).unwrap().clone()).collect();
            return (z.name.clone(), scan.clone(), members);
        }
    }
    panic!("no priority scan");
}

fn knob(p: &Profile, zone: &str, index: i64) -> Option<Knob> {
    Summary::read(p).unwrap().zones.into_iter().find(|z| z.name == zone)?.knobs.into_iter().find(|k| k.index == index)
}

#[test]
fn removing_a_scan_member_keeps_the_scan_valid() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, scan, members) = priority_scan(&p);
    assert!(members.len() >= 4, "the real priority scan has 4 channels");
    let (prio, plain) = (&members[0], &members[1]);
    let freq = load_table(&p.db, "tabSingleConvCHKnob").into_iter().find(|c| c.s("ChannelID") == plain.channel_id.as_deref().unwrap()).unwrap().s("ConvFreqID").to_string();

    let ch = remove_knob(&p, &KnobRemoval { zone: zone.clone(), knob: plain.index }, NOW).unwrap();
    assert_eq!(ch.kind, KnobKind::Conventional);
    assert_eq!(ch.scans_changed.len(), 1);
    assert!(ch.priority_moved.is_empty());
    assert!(knob(&p, &zone, plain.index).is_none(), "position is empty");
    let after = knob(&p, &zone, scan.index).unwrap();
    assert_eq!(after.scan_members.len(), members.len() - 1);
    assert_eq!(after.scan_priority, scan.scan_priority);
    assert!(load_table(&p.db, "tabConvFreqList").iter().all(|f| f.s("FreqID") != freq), "the knob's own frequency row goes");
    let c = changed(&p, &tpl, v, &dir);
    assert!(c.iter().any(|t| t == "tabScanListSettings"), "{c:?}");
    assert_only(&c, &[CONV_KNOB_CHANNEL_TABLES, SCAN_MEMBER_TABLES]);

    // Removing the priority channel moves the priority to the next member.
    let ch = remove_knob(&p, &KnobRemoval { zone: zone.clone(), knob: prio.index }, NOW).unwrap();
    assert_eq!(ch.priority_moved.len(), 1);
    let after = knob(&p, &zone, scan.index).unwrap();
    assert_eq!(after.scan_priority.as_ref(), after.scan_members.first());
    assert!(!after.scan_members.contains(prio.channel_id.as_ref().unwrap()));
    // Two members left: removing either would leave one, which is refused.
    assert_eq!(after.scan_members.len(), 2);
    let next = members.iter().find(|m| after.scan_members.contains(m.channel_id.as_ref().unwrap())).unwrap();
    let err = remove_knob(&p, &KnobRemoval { zone: zone.clone(), knob: next.index }, NOW).unwrap_err().to_string();
    assert!(err.contains("fewer than 2"), "{err}");
}

#[test]
fn removing_trunked_and_scan_knobs() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let (zone, k) = s.zones.iter().find_map(|z| z.knobs.iter().find(|k| k.system_id.is_some()).map(|k| (z.name.clone(), k.clone()))).unwrap();
    let ch = remove_knob(&p, &KnobRemoval { zone: zone.clone(), knob: k.index }, NOW).unwrap();
    assert_eq!(ch.kind, KnobKind::Trunked);
    assert!(load_table(&p.db, "tabChannelGroup").iter().all(|r| r.s("ChannelID") != k.channel_id.as_deref().unwrap()), "its talkgroup rows go");
    assert_only(&changed(&p, &tpl, v, &dir), &[KNOB_CHANNEL_TABLES, TALKGROUP_EDIT_TABLES, SYSTEM_EDIT_TABLES]);

    let p = Profile::from_backup(&dir).unwrap();
    let (zone, scan, members) = priority_scan(&p);
    let ch = remove_knob(&p, &KnobRemoval { zone: zone.clone(), knob: scan.index }, NOW).unwrap();
    assert_eq!(ch.kind, KnobKind::Scan);
    assert!(knob(&p, &zone, members[0].index).is_some(), "members stay");
    assert!(load_table(&p.db, "tabMultiCHScanList").iter().all(|r| r.i("KnobIndex") != scan.index || r.s("ZoneID") != load_table(&p.db, "tabZone_New").iter().find(|z| z.s("Name") == zone).unwrap().s("ID")));
    assert_only(&changed(&p, &tpl, v, &dir), &[SCAN_KNOB_TABLES]);
    assert!(remove_knob(&p, &KnobRemoval { zone, knob: scan.index }, NOW).unwrap_err().to_string().contains("already empty"));
}

#[test]
fn conventional_edit_in_place() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, _, members) = priority_scan(&p);
    let m = &members[0];
    let c = load_table(&p.db, "tabSingleConvCHKnob").into_iter().find(|c| c.s("ChannelID") == m.channel_id.as_deref().unwrap()).unwrap();
    let fid = c.s("ConvFreqID").to_string();
    let freqs_before = load_table(&p.db, "tabConvFreqList").len();
    let u = ConvKnobUpdate {
        zone: zone.clone(),
        knob: m.index,
        rx_hz: Some(154_430_000),
        squelch: Some(Squelch::Dcs { code: DcsCode::try_from("265".to_string()).unwrap(), inverted: false }),
        // Unchanged values are skipped, changed ones applied.
        alert_mode: Some(if c.s("AlertMode") == "1" { 3 } else { 1 }),
        record_flag: Some(c.i("RecordFlag")),
        ..Default::default()
    };
    let ch = update_conv_knob(&p, &tpl, &u, NOW).unwrap();
    assert_eq!(ch.changed.len(), 3, "{:?}", ch.changed);
    let f = load_table(&p.db, "tabConvFreqList").into_iter().find(|f| f.s("FreqID") == fid).unwrap();
    assert_eq!((f.s("RxFreq"), f.s("TxFreq")), ("154430000", "154430000"), "unshared row edited in place");
    assert_eq!(load_table(&p.db, "tabConvFreqList").len(), freqs_before);
    let tone = load_table(&p.db, "tabRecvSaList").into_iter().filter(|r| r.s("ChannelID") == m.channel_id.as_deref().unwrap()).collect::<Vec<_>>();
    assert_eq!(tone.len(), 1);
    assert_eq!((tone[0].s("SaType"), tone[0].s("cdCodeWord")), ("2", "181"));
    assert_eq!(knob(&p, &zone, m.index).unwrap().target.as_deref(), Some("154.43000 MHz"));
    assert_only(&changed(&p, &tpl, v, &dir), &[CONV_EDIT_TABLES]);
    assert!(update_conv_knob(&p, &tpl, &ConvKnobUpdate { zone, knob: m.index, ..Default::default() }, NOW).unwrap_err().to_string().contains("unchanged"));
}

#[test]
fn tg_scan_and_monitor_switch() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let (zone, k) = s.zones.iter().find_map(|z| z.knobs.iter().find(|k| k.mode == MODE_TG_SCAN).map(|k| (z.name.clone(), k.clone()))).unwrap();
    let ch_id = k.channel_id.clone().unwrap();
    let list = |k: &Knob| (k.talkgroup_list.iter().filter(|t| t.priority).map(|t| t.group_id.clone()).collect::<Vec<_>>(), k.talkgroup_list.iter().filter(|t| !t.priority).map(|t| t.group_id.clone()).collect::<Vec<_>>());
    let (priority, normal) = list(&k);
    let edit = |mode| KnobSystemEdit { zone: zone.clone(), knob: k.index, system_id: k.system_id.clone().unwrap(), mode, channel_id: None, name: None, voice_prompt: 0, priority: priority.clone(), normal: normal.clone() };
    let ch = assign_knob_system(&p, &edit(MODE_TG_MONITOR), NOW).unwrap();
    assert!(ch.mode_switched);
    let sub: Vec<_> = load_table(&p.db, "tabP25TSubGroupEnable").into_iter().filter(|r| r.s("ChannelID") == ch_id).collect();
    assert_eq!(sub.len(), 1);
    assert_eq!((sub[0].s("GroupID"), sub[0].s("Enable2TSubGroup")), ("", "1"));
    assert_eq!(knob(&p, &zone, k.index).unwrap().mode, MODE_TG_MONITOR);
    let c = changed(&p, &tpl, v, &dir);
    assert!(c.iter().any(|t| t == "tabP25TSetting"), "{c:?}");
    assert_only(&c, &[MODE_SWITCH_TABLES, SYSTEM_EDIT_TABLES, TALKGROUP_EDIT_TABLES]);

    // And back: the sub-group row goes, the radio DB is the original again.
    let ch = assign_knob_system(&p, &edit(MODE_TG_SCAN), NOW).unwrap();
    assert!(ch.mode_switched);
    assert!(load_table(&p.db, "tabP25TSubGroupEnable").iter().all(|r| r.s("ChannelID") != ch_id));
    assert!(changed(&p, &tpl, v, &dir).is_empty());
}

#[test]
fn removal_clears_a_second_priority_and_a_mode_switch_keeps_talkgroup_rows() {
    let Some((dir, _, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, scan, members) = priority_scan(&p);
    let zid = load_table(&p.db, "tabZone_New").into_iter().find(|z| z.s("Name") == zone).unwrap().s("ID").to_string();
    let second = members[1].channel_id.clone().unwrap();
    p.db.execute("UPDATE tabMultiChannelScan SET SecondPriorityCH = ? WHERE ZoneID = ? AND KnobIndex = ?", &[Some(&second), Some(&zid), Some(&scan.index.to_string())]).unwrap();
    remove_knob(&p, &KnobRemoval { zone: zone.clone(), knob: members[1].index }, NOW).unwrap();
    let row = load_table(&p.db, "tabMultiChannelScan").into_iter().find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == scan.index).unwrap();
    assert_eq!(row.s("SecondPriorityCH"), "");

    // Pure TG Scan -> Monitor: the knob's tabChannelGroup rows stay exactly as they were.
    let s = Summary::read(&p).unwrap();
    let (tz, k) = s.zones.iter().find_map(|z| z.knobs.iter().find(|k| k.mode == MODE_TG_SCAN).map(|k| (z.name.clone(), k.clone()))).unwrap();
    let ch = k.channel_id.clone().unwrap();
    let snapshot = |p: &Profile| load_table(&p.db, "tabChannelGroup").into_iter().filter(|r| r.s("ChannelID") == ch).map(|r| format!("{r:?}")).collect::<Vec<_>>();
    let before = snapshot(&p);
    let pri: Vec<String> = k.talkgroup_list.iter().filter(|t| t.priority).map(|t| t.group_id.clone()).collect();
    let norm: Vec<String> = k.talkgroup_list.iter().filter(|t| !t.priority).map(|t| t.group_id.clone()).collect();
    let e = KnobSystemEdit { zone: tz, knob: k.index, system_id: k.system_id.clone().unwrap(), mode: MODE_TG_MONITOR, channel_id: None, name: None, voice_prompt: 0, priority: pri, normal: norm };
    let c = assign_knob_system(&p, &e, NOW).unwrap();
    assert!(c.mode_switched && !c.list_changed);
    assert_eq!(snapshot(&p), before);
}
