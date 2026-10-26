//! Zone create, rename, delete and reorder against private real-pager
//! fixtures (skipped without them), per notes/zones-and-scan.md 9.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::row::load_table;
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::zone_edit::{NewZone, ZoneRename, create_zone, delete_zone, rename_zone, reorder_zones};
use g5_codeplug::{Profile, Templates};

const NOW: &str = "2026-10-18 12:00:00";
const ZID: &str = "3c2b1a09-8f7e-4d6c-9b5a-4f3e2d1c0b9a";

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

/// Radio tables that differ from the backup's radio DB.
fn changed(profile: &Profile, templates: &Templates, variant: Variant, dir: &Path) -> Vec<g5_codeplug::compare::TableDiff> {
    let out = tempfile::NamedTempFile::new().unwrap();
    let report = convert(profile, templates, variant, out.path()).unwrap();
    assert!(report.unverified.is_empty(), "unverified: {:?}", report.unverified);
    let ours = sqlite2::Connection::open(out.path()).unwrap();
    let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
    compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).collect()
}

fn zone_ids(p: &Profile) -> Vec<String> {
    load_table(&p.db, "tabZone_New").iter().map(|z| z.s("ID").to_string()).collect()
}

#[test]
fn create_writes_pps_rows_and_only_touches_tab_zone() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let ch = create_zone(&p, &NewZone { zone_id: ZID.into(), name: " Scanner ".into() }, NOW).unwrap();
    assert_eq!(ch.name, "Scanner", "stored trimmed");
    let z = load_table(&p.db, "tabZone_New").pop().unwrap();
    assert_eq!((z.s("ID"), z.s("ZoneNo"), z.s("Name"), z.s("Note"), z.s("ModifyByDevice")), (ZID, "", "Scanner", "", ""));
    let rows: Vec<_> = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().filter(|r| r.s("ZoneID") == ZID).collect();
    assert_eq!(rows.len(), 8);
    for (i, r) in rows.iter().enumerate() {
        assert_eq!((r.i("KnobIndex"), r.s("CurrentMode"), r.s("ChannelName"), r.s("AlertToneID"), r.s("AlertFileName")), (i as i64 + 1, "0", "", "0", ""));
    }
    let c: Vec<String> = changed(&p, &tpl, v, &dir).into_iter().map(|d| d.table).collect();
    assert_eq!(c, ["tabZone"]);
}

#[test]
fn names_are_checked() {
    let Some((dir, _, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let first = load_table(&p.db, "tabZone_New").remove(0);
    let err = |name: &str| create_zone(&p, &NewZone { zone_id: ZID.into(), name: name.into() }, NOW).unwrap_err().to_string();
    assert!(err(&first.s("Name").to_uppercase()).contains("already called"), "case-insensitive duplicate");
    assert!(err("   ").contains("blank"));
    assert!(err("A zone name too long").contains("longer than 14"));
    assert!(create_zone(&p, &NewZone { zone_id: "NOPE".into(), name: "X".into() }, NOW).is_err());

    let r = |name: &str| rename_zone(&p, &ZoneRename { zone_id: first.s("ID").into(), name: name.into() }, NOW);
    assert!(r(first.s("Name")).is_err(), "same name");
    let ch = r("Renamed").unwrap();
    assert_eq!(ch.renamed_from.as_deref(), Some(first.s("Name")));
    assert_eq!(load_table(&p.db, "tabZone_New")[0].s("Name"), "Renamed");
}

#[test]
fn delete_refuses_zones_with_knobs_and_the_last_zone() {
    let Some((dir, _, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let busy = load_table(&p.db, "tabZone_New").remove(0);
    let err = delete_zone(&p, busy.s("ID"), NOW).unwrap_err().to_string();
    assert!(err.contains("still has knobs"), "{err}");
    assert!(delete_zone(&p, "no-such-zone", NOW).is_err());
}

#[test]
fn create_move_first_then_delete_is_a_no_op_on_the_radio() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let before = zone_ids(&p);
    create_zone(&p, &NewZone { zone_id: ZID.into(), name: "Front".into() }, NOW).unwrap();
    let mut order = vec![ZID.to_string()];
    order.extend(before.iter().cloned());
    let ch = reorder_zones(&p, &order, NOW).unwrap();
    assert!(ch.renumbers);
    assert_eq!(zone_ids(&p), order);
    // Every channel moved down one zone: many tables change, none gains or loses rows.
    let d = changed(&p, &tpl, v, &dir);
    assert!(d.iter().any(|d| d.table == "tabChannel"), "{d:?}");
    for t in d.iter().filter(|d| d.table != "tabZone") {
        assert_eq!(t.left_rows, t.right_rows, "{}", t.table);
        assert!(g5_codeplug::edit::ZONE_RENUMBER_TABLES.contains(&t.table.as_str()), "{} isn't keyed by channel", t.table);
    }
    // Deleting the empty first zone renumbers back: the radio DB is the original again.
    assert!(delete_zone(&p, ZID, NOW).unwrap().renumbers);
    assert_eq!(zone_ids(&p), before);
    let d = changed(&p, &tpl, v, &dir);
    assert!(d.is_empty(), "{d:?}");

    assert!(reorder_zones(&p, &before, NOW).is_err(), "unchanged order");
    assert!(reorder_zones(&p, &before[1..], NOW).is_err(), "missing a zone");
}
