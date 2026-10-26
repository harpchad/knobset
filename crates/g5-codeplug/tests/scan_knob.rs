//! Scan knobs against private real-pager fixtures (skipped without them),
//! per notes/zones-and-scan.md 9.6 and 9.7.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::edit::{SCAN_KNOB_TABLES, SCAN_MEMBER_TABLES};
use g5_codeplug::row::load_table;
use g5_codeplug::scan_knob::{MODE_NORMAL_SCAN, MODE_PRIORITY_SCAN, ScanKnobEdit, set_scan_knob};
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::summary::Summary;
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

/// The real priority scan (PPS defaults) and its zone, plus an empty position in that zone.
fn real_priority_scan(p: &Profile) -> (String, i64, g5_codeplug::summary::Knob) {
    let s = Summary::read(p).unwrap();
    for z in s.zones {
        if let Some(k) = z.knobs.iter().find(|k| k.mode == MODE_PRIORITY_SCAN)
            && let Some(&empty) = z.empty_knobs.iter().find(|&&e| e <= 8)
        {
            return (z.name.clone(), empty, k.clone());
        }
    }
    panic!("no zone with a priority scan and an empty position");
}

#[test]
fn priority_scan_matches_the_real_one() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, empty, real) = real_priority_scan(&p);
    let zid = load_table(&p.db, "tabZone_New").into_iter().find(|z| z.s("Name") == zone).unwrap().s("ID").to_string();
    // Members in a different order, priority last: Knobset puts the priority first, as PPS does.
    let mut members = real.scan_members.clone();
    members.rotate_left(1);
    let e = ScanKnobEdit { zone: zone.clone(), knob: empty, mode: MODE_PRIORITY_SCAN, members, priority: real.scan_priority.clone(), name: Some("Knobset Scan".into()), voice_prompt: 0 };
    let ch = set_scan_knob(&p, &e, NOW).unwrap();
    assert!(ch.new_knob);
    assert_eq!(ch.members, real.scan_members);

    let rows = load_table(&p.db, "tabMultiChannelScan");
    let ours = rows.iter().find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == empty).unwrap();
    let theirs = rows.iter().find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == real.index).unwrap();
    for c in p.db.table_columns("tabMultiChannelScan").unwrap() {
        if c != "KnobIndex" {
            assert_eq!(ours.s(&c), theirs.s(&c), "{c}");
        }
    }
    let k = Summary::read(&p).unwrap().zones.into_iter().find(|z| z.name == zone).unwrap().knobs.into_iter().find(|k| k.index == empty).unwrap();
    assert_eq!((k.mode, k.scan_members.clone(), k.scan_priority.clone()), (MODE_PRIORITY_SCAN, real.scan_members.clone(), real.scan_priority.clone()));
    let c = changed(&p, &tpl, v, &dir);
    assert!(c.iter().any(|t| t == "tabScanListSettings"), "{c:?}");
    assert!(c.iter().all(|t| SCAN_KNOB_TABLES.contains(&t.as_str())), "{c:?}");
}

#[test]
fn member_edit_only_touches_the_scan_list() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let (zone, k) = s.zones.iter().find_map(|z| z.knobs.iter().find(|k| k.mode == MODE_NORMAL_SCAN).map(|k| (z.name.clone(), k.clone()))).expect("a normal scan");
    let mut members = k.scan_members.clone();
    members.pop();
    members.reverse();
    let ch = set_scan_knob(&p, &ScanKnobEdit { zone: zone.clone(), knob: k.index, mode: MODE_NORMAL_SCAN, members: members.clone(), priority: None, name: None, voice_prompt: 0 }, NOW).unwrap();
    assert!(!ch.new_knob);
    let after = Summary::read(&p).unwrap().zones.into_iter().find(|z| z.name == zone).unwrap().knobs.into_iter().find(|x| x.index == k.index).unwrap();
    assert_eq!(after.scan_members, members);
    assert_eq!(changed(&p, &tpl, v, &dir), SCAN_MEMBER_TABLES);
}

#[test]
fn refusals() {
    let Some((dir, _, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, empty, real) = real_priority_scan(&p);
    let s = Summary::read(&p).unwrap();
    let trunked = s.zones.iter().flat_map(|z| z.knobs.iter()).find(|k| k.system_id.is_some()).unwrap().channel_id.clone().unwrap();
    let base = ScanKnobEdit { zone, knob: empty, mode: MODE_NORMAL_SCAN, members: real.scan_members.clone(), priority: None, name: Some("Scan".into()), voice_prompt: 0 };
    let err = |e: ScanKnobEdit| set_scan_knob(&p, &e, NOW).unwrap_err().to_string();
    assert!(err(ScanKnobEdit { members: real.scan_members[..1].to_vec(), ..base.clone() }).contains("2 to 16"));
    assert!(err(ScanKnobEdit { members: vec![real.scan_members[0].clone(), real.scan_members[0].clone()], ..base.clone() }).contains("twice"));
    assert!(err(ScanKnobEdit { members: vec![real.scan_members[0].clone(), trunked], ..base.clone() }).contains("isn't a conventional knob"));
    assert!(err(ScanKnobEdit { mode: MODE_PRIORITY_SCAN, ..base.clone() }).contains("priority channel"));
    assert!(err(ScanKnobEdit { priority: Some(real.scan_members[0].clone()), ..base.clone() }).contains("only a priority scan"));
    assert!(err(ScanKnobEdit { name: None, ..base.clone() }).contains("needs a name"));
    assert!(err(ScanKnobEdit { voice_prompt: 4, ..base.clone() }).contains("voice prompt"));
    assert!(err(ScanKnobEdit { knob: real.index, ..base }).contains("switching a scan knob"));
}
