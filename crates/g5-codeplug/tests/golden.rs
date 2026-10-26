//! Golden-file test: regenerate each real pager's radio DB from the profile
//! stored on it, and require an exact logical match with what PPS uploaded.
//!
//! Fixtures are private (real profiles, unit IDs) and never committed. The
//! test looks for `g5 backup` folders in `G5_BACKUPS` (default `<repo>/backups`)
//! and Unication templates in `G5_TEMPLATES` (default: the extracted PPS
//! installer under `<repo>/extract`). It skips when either is missing.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::{Profile, Templates};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn env_or(var: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).unwrap_or(default)
}

fn db_version(backup: &Path) -> Option<String> {
    let m = fs::read_to_string(backup.join("MANIFEST.txt")).ok()?;
    m.lines().find_map(|l| l.strip_prefix("Radio DB version:").map(|v| v.trim().to_string()))
}

#[test]
fn regenerates_uploaded_radio_db() {
    let templates_dir = env_or("G5_TEMPLATES", repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"));
    let backups_dir = env_or("G5_BACKUPS", repo().join("backups"));
    let Ok(templates) = Templates::new(&templates_dir) else {
        eprintln!("skipping: no templates at {}", templates_dir.display());
        return;
    };
    let Ok(entries) = fs::read_dir(&backups_dir) else {
        eprintln!("skipping: no backups at {}", backups_dir.display());
        return;
    };

    let mut checked = 0;
    let mut failures = Vec::new();
    for dir in entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_dir()) {
        let Some(variant) = db_version(&dir).as_deref().and_then(Variant::for_db_version) else { continue };
        let label = format!("{} ({})", dir.file_name().unwrap().to_string_lossy().split('-').next().unwrap(), variant.db_version());
        // A stale sidecar means a firmware update migrated the radio DB in
        // place; no converter made it, so it isn't a reference.
        if fs::read_to_string(dir.join("MANIFEST.txt")).is_ok_and(|m| m.contains("/PPS/u3db.db: MISMATCH")) {
            eprintln!("{label}: skipped, radio DB migrated by a firmware update");
            continue;
        }
        let profile = Profile::from_backup(&dir).expect("load profile");
        let out = tempfile::NamedTempFile::new().unwrap();
        let report = convert(&profile, &templates, variant, out.path()).unwrap_or_else(|e| panic!("{label}: {e}"));
        eprintln!("{label}: {} statements, unverified: {:?}", report.statements, report.unverified);

        let ours = sqlite2::Connection::open(out.path()).unwrap();
        let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();

        // Sanity: the bare template must differ, or the comparison proves nothing.
        let template = sqlite2::Connection::open(&templates.radio_template(variant.db_version())).unwrap();
        let differing = compare(&template, &theirs).unwrap().iter().filter(|d| d.status != TableStatus::Same).count();
        assert!(differing > 50, "{label}: template differs in only {differing} tables; comparison looks insensitive");

        for d in compare(&ours, &theirs).unwrap() {
            if d.status != TableStatus::Same {
                failures.push(format!("{label}: {} ours={} theirs={} {:?}", d.table, d.left_rows, d.right_rows, d.status));
            }
        }
        checked += 1;
    }
    if checked == 0 {
        eprintln!("skipping: no DBV1.41b-e backups found");
    }
    assert!(failures.is_empty(), "{} table mismatches:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn rename_knob_changes_only_the_name() {
    let templates_dir = env_or("G5_TEMPLATES", repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"));
    let backups_dir = env_or("G5_BACKUPS", repo().join("backups"));
    let (Ok(templates), Ok(entries)) = (Templates::new(&templates_dir), fs::read_dir(&backups_dir)) else {
        eprintln!("skipping: fixtures not available");
        return;
    };
    let Some(dir) = entries.filter_map(|e| e.ok().map(|e| e.path())).find(|p| db_version(p).as_deref().and_then(Variant::for_db_version).is_some()) else {
        eprintln!("skipping: no DBV1.41b/c backup");
        return;
    };
    let variant = Variant::for_db_version(&db_version(&dir).unwrap()).unwrap();
    let profile = Profile::from_backup(&dir).unwrap();
    let summary = g5_codeplug::summary::Summary::read(&profile).unwrap();
    let zone = &summary.zones[0];
    let knob = zone.knobs.last().unwrap();
    let old = g5_codeplug::edit::rename_knob(&profile, &zone.name, knob.index, "G5 TEST").unwrap();
    assert_eq!(old, knob.name);

    let out = tempfile::NamedTempFile::new().unwrap();
    convert(&profile, &templates, variant, out.path()).unwrap();
    let ours = sqlite2::Connection::open(out.path()).unwrap();
    let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
    let changed: Vec<String> = compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect();
    assert_eq!(changed, ["tabChannel", "tabReceivingMode"], "only the knob's name rows may change");

    // The repacked zip must load and carry the new name.
    let zip = g5_codeplug::edit::repack_zip(&dir.join(g5_codeplug::source::BACKUP_PROFILE_ZIP), &profile, "2026-10-18 12:00:00").unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), &zip).unwrap();
    let reread = Profile::from_zip(tmp.path()).unwrap();
    let s2 = g5_codeplug::summary::Summary::read(&reread).unwrap();
    assert!(s2.zones[0].knobs.iter().any(|k| k.index == knob.index && k.name == "G5 TEST"));
    assert_eq!(s2.systems.len(), summary.systems.len(), "system DBs survive the repack");
}

/// First DBV1.41b/c backup plus its variant, or None when fixtures are missing.
fn fixture() -> Option<(PathBuf, Templates, Variant)> {
    let templates = Templates::new(env_or("G5_TEMPLATES", repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"))).ok()?;
    let dir = fs::read_dir(env_or("G5_BACKUPS", repo().join("backups")))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| db_version(p).as_deref().and_then(Variant::for_db_version).is_some())?;
    let variant = Variant::for_db_version(&db_version(&dir).unwrap()).unwrap();
    Some((dir, templates, variant))
}

fn changed_tables(profile: &Profile, templates: &Templates, variant: Variant, dir: &Path) -> Vec<(String, usize, usize)> {
    let out = tempfile::NamedTempFile::new().unwrap();
    convert(profile, templates, variant, out.path()).unwrap();
    let ours = sqlite2::Connection::open(out.path()).unwrap();
    let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
    compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| (d.table, d.left_rows, d.right_rows)).collect()
}

/// A trunked knob and its current list, split the way PPS stores it.
fn first_trunked_knob(profile: &Profile) -> (String, i64, g5_codeplug::edit::KnobTalkgroups, String) {
    let s = g5_codeplug::summary::Summary::read(profile).unwrap();
    for z in &s.zones {
        for k in &z.knobs {
            if k.system_id.is_some() && k.talkgroup_list.len() >= 2 {
                let list = g5_codeplug::edit::KnobTalkgroups {
                    priority: k.talkgroup_list.iter().filter(|t| t.priority).map(|t| t.group_id.clone()).collect(),
                    normal: k.talkgroup_list.iter().filter(|t| !t.priority).map(|t| t.group_id.clone()).collect(),
                };
                return (z.name.clone(), k.index, list, k.system_id.clone().unwrap());
            }
        }
    }
    panic!("no trunked knob with 2+ talkgroups");
}

#[test]
fn rewriting_a_knob_list_unchanged_is_a_no_op() {
    let Some((dir, templates, variant)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let profile = Profile::from_backup(&dir).unwrap();
    let (zone, knob, list, _) = first_trunked_knob(&profile);
    g5_codeplug::edit::set_knob_talkgroups(&profile, &zone, knob, &list).unwrap();
    assert_eq!(changed_tables(&profile, &templates, variant, &dir), Vec::<(String, usize, usize)>::new());
}

#[test]
fn talkgroup_edits_change_only_contact_tables() {
    let Some((dir, templates, variant)) = fixture() else { return eprintln!("skipping: fixtures not available") };

    // Remove the last talkgroup from the knob.
    let profile = Profile::from_backup(&dir).unwrap();
    let (zone, knob, mut list, _) = first_trunked_knob(&profile);
    if list.normal.pop().is_none() {
        list.priority.pop();
    }
    g5_codeplug::edit::set_knob_talkgroups(&profile, &zone, knob, &list).unwrap();
    let removed = changed_tables(&profile, &templates, variant, &dir);
    eprintln!("remove one: {removed:?}");

    // Add a talkgroup from the same system that no knob uses.
    let profile = Profile::from_backup(&dir).unwrap();
    let (zone, knob, mut list, sys_id) = first_trunked_knob(&profile);
    let s = g5_codeplug::summary::Summary::read(&profile).unwrap();
    let used: std::collections::HashSet<String> = s.zones.iter().flat_map(|z| z.knobs.iter()).flat_map(|k| k.talkgroup_list.iter().map(|t| t.group_id.clone())).collect();
    let sys = s.systems.iter().find(|x| x.id == sys_id).unwrap();
    let new = sys.talkgroup_list.iter().find(|t| !used.contains(&t.group_id) && t.tgid != 0 && t.tgid != 65535).expect("an unused talkgroup");
    list.normal.push(new.group_id.clone());
    g5_codeplug::edit::set_knob_talkgroups(&profile, &zone, knob, &list).unwrap();
    let added = changed_tables(&profile, &templates, variant, &dir);
    eprintln!("add one ({} {}): {added:?}", new.tgid, new.name);

    for (t, _, _) in removed.iter().chain(&added) {
        assert!(g5_codeplug::edit::TALKGROUP_EDIT_TABLES.contains(&t.as_str()), "unexpected table {t}");
    }
}

#[test]
fn priority_reorder_changes_only_the_receive_list() {
    let Some((dir, templates, variant)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let profile = Profile::from_backup(&dir).unwrap();
    let (zone, knob, list, _) = first_trunked_knob(&profile);
    // Move every talkgroup onto the priority list, reversed.
    let mut all: Vec<String> = list.priority.iter().chain(&list.normal).cloned().collect();
    all.reverse();
    g5_codeplug::edit::set_knob_talkgroups(&profile, &zone, knob, &g5_codeplug::edit::KnobTalkgroups { priority: all, normal: vec![] }).unwrap();
    let changed = changed_tables(&profile, &templates, variant, &dir);
    eprintln!("priority reorder: {changed:?}");
    for (t, _, _) in &changed {
        assert!(g5_codeplug::edit::TALKGROUP_EDIT_TABLES.contains(&t.as_str()), "unexpected table {t}");
    }
}

#[test]
fn rejects_invalid_lists() {
    let Some((dir, _, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let profile = Profile::from_backup(&dir).unwrap();
    let (zone, knob, list, _) = first_trunked_knob(&profile);
    let set = |l: &g5_codeplug::edit::KnobTalkgroups| g5_codeplug::edit::set_knob_talkgroups(&profile, &zone, knob, l);
    assert!(set(&g5_codeplug::edit::KnobTalkgroups::default()).is_err(), "empty list");
    let first = list.priority.iter().chain(&list.normal).next().unwrap().clone();
    assert!(set(&g5_codeplug::edit::KnobTalkgroups { priority: vec![first.clone()], normal: vec![first] }).is_err(), "duplicate");
    assert!(set(&g5_codeplug::edit::KnobTalkgroups { priority: vec![], normal: vec!["no-such-group".into()] }).is_err(), "unknown group");
}
