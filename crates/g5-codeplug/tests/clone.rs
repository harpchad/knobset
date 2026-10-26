//! The clone guard against private fixtures (skipped without them): a result that
//! differs from the source anywhere but the target's identity fields is refused.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::Profile;
use g5_codeplug::clone::{check_clone, identity, keep_identity, kept_fields, missing_protocols};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Any two backups with profiles that convert (the guard doesn't care which pagers).
fn two_backups() -> Option<(PathBuf, PathBuf)> {
    let root = std::env::var_os("G5_BACKUPS").map(PathBuf::from).unwrap_or(repo().join("backups"));
    let mut dirs: Vec<PathBuf> = fs::read_dir(root)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| fs::read_to_string(p.join("MANIFEST.txt")).is_ok_and(|m| m.contains("DBV1.41b") || m.contains("DBV1.41c")))
        .collect();
    dirs.sort();
    Some((dirs.first()?.clone(), dirs.last()?.clone()))
}

#[test]
fn guard_accepts_only_the_target_identity() {
    let Some((src, tgt)) = two_backups() else { return eprintln!("skipping: fixtures not available") };
    let source = Profile::from_backup(&src).unwrap();
    let target = Profile::from_backup(&tgt).unwrap();
    target.db.execute_batch("UPDATE tabMember_New SET UserDeviceID = 'User-555', DMRAddr = '31337' WHERE RadioNo = '99999'; UPDATE tabPagerSUID SET P25TAddr = '7654321';").unwrap();

    let fresh = || {
        let w = Profile::from_backup(&src).unwrap();
        keep_identity(&w, &target).unwrap();
        w
    };
    let w = fresh();
    let mut changed = check_clone(&w.db, &source.db, &target.db).unwrap();
    changed.sort();
    assert!(changed.contains(&"tabMember_New".to_string()) && changed.contains(&"tabPagerSUID".to_string()), "{changed:?}");
    let id = identity(&w).unwrap();
    assert_eq!((id.radio_number.as_str(), id.pager_suid.as_str()), ("User-555", "7654321"));
    assert!(id.addresses.iter().any(|(l, a)| l == "DMR" && a == "31337"));
    let kept = kept_fields(&identity(&source).unwrap(), &id);
    assert!(kept.iter().any(|k| k.label == "Radio number" && k.differs));

    // Anything else that differs from the source is refused.
    for sql in [
        "UPDATE tabZone_New SET Name = 'Tampered'",
        "UPDATE tabMember_New SET AlertMode = '4' WHERE RadioNo = '99999'",
        "UPDATE tabPagerSUID SET P25TAddr = '1'",
        "UPDATE tabMember_New SET UserDeviceID = 'User-001' WHERE RadioNo = '99999'",
        "DELETE FROM tabAccessPassword WHERE ID = 2",
    ] {
        let w = fresh();
        w.db.execute_batch(sql).unwrap();
        assert!(check_clone(&w.db, &source.db, &target.db).is_err(), "guard missed: {sql}");
    }
    // A result that never took the target's identity is refused too.
    let untouched = Profile::from_backup(&src).unwrap();
    assert!(check_clone(&untouched.db, &source.db, &target.db).is_err());
}

#[test]
fn missing_protocols_names_what_the_target_lacks() {
    let Some((src, tgt)) = two_backups() else { return eprintln!("skipping: fixtures not available") };
    let source = Profile::from_backup(&src).unwrap();
    let target = Profile::from_backup(&tgt).unwrap();
    source.db.execute_batch("UPDATE tabProtocolMode SET DMRC = '1'").unwrap();
    target.db.execute_batch("UPDATE tabProtocolMode SET DMRC = '0'").unwrap();
    assert_eq!(missing_protocols(&source, &target), ["DMR"]);
    assert!(missing_protocols(&target, &source).is_empty());
}
