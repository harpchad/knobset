//! Cloning one pager's backup onto another pager, offline, against private fixtures
//! (skipped without them). The "attached pager" is the target's newest backup.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::Profile;
use g5_codeplug::clone::{IDENTITY_RADIO_TABLES, identity};
use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::Variant;
use g5_codeplug::source::{BACKUP_PROFILE_ZIP, BACKUP_RADIO_DB};
use g5_core::backup::{BackupInfo, list_backups};
use g5_core::clone::{check_clone_compatible, clone_set};
use g5_core::write::{PC_DB, PC_ZIP, PagerId, RADIO_DB, check_pager_matches, restore_set};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn templates() -> Option<PathBuf> {
    let tpl = std::env::var_os("G5_TEMPLATES").map(PathBuf::from).unwrap_or(repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"));
    tpl.join("Data_G4A.db").is_file().then_some(tpl)
}

/// The newest supported backup of each pager.
fn newest_per_pager() -> Vec<BackupInfo> {
    let root = std::env::var_os("G5_BACKUPS").map(PathBuf::from).unwrap_or(repo().join("backups"));
    let mut out: Vec<BackupInfo> = Vec::new();
    for b in list_backups(&root) {
        if Variant::for_db_version(&b.db_version).is_some() && !out.iter().any(|x| x.serial == b.serial) {
            out.push(b);
        }
    }
    out
}

fn m21(m: &str) -> &str {
    &m[..21.min(m.len())]
}

/// (source, target) pairs of different pagers, same model or not.
fn pairs(same_model: bool) -> Vec<(BackupInfo, BackupInfo)> {
    let all = newest_per_pager();
    let mut out = Vec::new();
    for a in &all {
        for b in &all {
            if a.serial != b.serial && (m21(&a.model) == m21(&b.model)) == same_model {
                out.push((a.clone(), b.clone()));
            }
        }
    }
    out
}

fn file<'a>(set: &'a g5_core::write::WriteSet, path: &str) -> &'a [u8] {
    &set.files.iter().find(|(p, _)| *p == path).unwrap().1
}

fn profile_of(zip: &[u8]) -> (Profile, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    (Profile::from_zip(tmp.path()).unwrap(), tmp)
}

fn diffs(a: &sqlite2::Connection, b: &sqlite2::Connection) -> Vec<String> {
    compare(a, b).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect()
}

/// Copy a backup folder (never touching the original).
fn copy_backup(src: &Path, dest: &Path) {
    for e in walk(src) {
        let rel = e.strip_prefix(src).unwrap();
        let to = dest.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(&e, &to).unwrap();
    }
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).unwrap().filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() { out.extend(walk(&p)) } else { out.push(p) }
    }
    out
}

fn md5_hex(b: &[u8]) -> String {
    use md5::Digest;
    md5::Md5::digest(b).iter().map(|x| format!("{x:02X}")).collect()
}

/// Give a copied backup's profile a different identity, as another pager's would have.
fn retag_identity(dir: &Path) {
    let p = Profile::from_backup(dir).unwrap();
    p.db.execute_batch(
        "UPDATE tabMember_New SET UserDeviceID = 'User-777', MemberName = 'Clone Test', P25Addr = '4242' WHERE RadioNo = '99999';
         UPDATE tabPagerSUID SET P25TAddr = '1234567';
         UPDATE tabAccessPassword SET Enable = '1', AccessPassword = '0123456789ABCDEF0123456789ABCDEF';", // gitleaks:allow (synthetic hash)
    )
    .unwrap();
    let zip = g5_codeplug::edit::repack_zip(&dir.join(BACKUP_PROFILE_ZIP), &p, "2026-10-20 10:00:00").unwrap();
    fs::write(dir.join(BACKUP_PROFILE_ZIP), &zip).unwrap();
    fs::write(dir.join(format!("{BACKUP_PROFILE_ZIP}.md5")), md5_hex(&zip)).unwrap();
    fs::copy(&p.db_path, dir.join(PC_DB.trim_start_matches('/'))).unwrap();
}

fn set_manifest(dir: &Path, label: &str, value: &str) {
    let m = fs::read_to_string(dir.join("MANIFEST.txt")).unwrap();
    let m: Vec<String> = m.lines().map(|l| if l.starts_with(label) { format!("{label} {value}") } else { l.to_string() }).collect();
    fs::write(dir.join("MANIFEST.txt"), m.join("\n") + "\n").unwrap();
}

#[test]
fn compatibility_rules() {
    let id = |model: &str, serial: &str, db: &str| PagerId { model: model.into(), serial: serial.into(), db_version: db.into() };
    let m = "G5B67BF-SXVXEN1401UNI-0002-180625";
    assert!(check_clone_compatible(&id(m, "S1", "DBV1.41c"), &id(m, "S2", "DBV1.41c")).is_ok());
    // Only the first 21 characters count, as in PPS.
    assert!(check_clone_compatible(&id(m, "S1", "DBV1.41c"), &id("G5B67BF-SXVXEN1401UNI-0003-190101", "S2", "DBV1.41c")).is_ok());
    let v = check_clone_compatible(&id(m, "S1", "DBV1.41c"), &id(m, "S2", "DBV1.41b")).unwrap();
    assert_eq!(v.db_version(), "DBV1.41b", "regenerates for the target's version");
    let e = check_clone_compatible(&id(m, "S1", "DBV1.41c"), &id(m, "S1", "DBV1.41c")).unwrap_err().to_string();
    assert!(e.contains("both are pager"), "{e}");
    let e = check_clone_compatible(&id("G5D66BE-SXVXENB702UNI-A001-220104", "S1", "DBV1.41b"), &id(m, "S2", "DBV1.41c")).unwrap_err().to_string();
    assert!(e.contains("same model") && e.contains("hardware D vs B") && e.contains("protocols 66 vs 67"), "{e}");
    let e = check_clone_compatible(&id("G5B67BF-SXUCEN1401UNI", "S1", "DBV1.41c"), &id(m, "S2", "DBV1.41c")).unwrap_err().to_string();
    assert!(e.contains("bands SXUC vs SXVX"), "{e}");
    assert!(check_clone_compatible(&id(m, "S1", "DBV2.06"), &id(m, "S2", "DBV1.41c")).is_err());
    let v = check_clone_compatible(&id(m, "S1", "DBV1.41c"), &id(m, "S2", "DBV1.41e")).unwrap();
    assert_eq!(v.db_version(), "DBV1.41e", "regenerates for an updated target");
    assert!(check_clone_compatible(&id(m, "S1", "DBV1.41c"), &id(m, "S2", "DBV1.41f")).is_err());
}

#[test]
fn clone_onto_another_pager_of_the_same_model() {
    let Some(tpl) = templates() else { return eprintln!("skipping: no templates") };
    let pairs = pairs(true);
    if pairs.is_empty() {
        return eprintln!("skipping: no two pagers of the same model in the fixtures");
    }
    for (src, tgt) in &pairs {
        let (set, sum) = clone_set(&src.path, &tgt.path, &tpl).unwrap_or_else(|e| panic!("{} -> {}: {e:#}", src.id, tgt.id));
        eprintln!("{} -> {}: target radio tables changed {:?}", src.id, tgt.id, set.changed_tables);
        assert!(set.require_unchanged_pager);
        assert_eq!(set.files.len(), 5);
        // The pager-dependent check runs against the target backup: the target passes, the source doesn't.
        assert!(check_pager_matches(&PagerId::of_backup(&tgt.path).unwrap(), &tgt.path).is_ok());
        assert!(check_pager_matches(&PagerId::of_backup(&src.path).unwrap(), &tgt.path).is_err());

        let (out, _t1) = profile_of(file(&set, PC_ZIP));
        let source = Profile::from_backup(&src.path).unwrap();
        let target = Profile::from_backup(&tgt.path).unwrap();
        assert_eq!(identity(&out).unwrap(), identity(&target).unwrap(), "identity must stay the target's");
        assert_eq!(sum.after.zones, g5_codeplug::summary::Summary::read(&source).unwrap().zones.len());
        assert_eq!(sum.before.zones, g5_codeplug::summary::Summary::read(&target).unwrap().zones.len());
        assert!(sum.kept.iter().any(|k| k.label == "Radio number" && k.value == identity(&target).unwrap().radio_number));
        assert_eq!(fs::read(&out.db_path).unwrap(), file(&set, PC_DB), "u3db_PC.db must equal the zip's root DB");
        assert_eq!(file(&set, &format!("{RADIO_DB}.md5")), md5_hex(file(&set, RADIO_DB)).as_bytes());
        assert_eq!(file(&set, &format!("{PC_ZIP}.md5")), md5_hex(file(&set, PC_ZIP)).as_bytes());

        if sum.identity_tables.is_empty() && src.db_version == tgt.db_version {
            // Same identity on both pagers: the source's files, byte for byte.
            let restore = restore_set(&src.path).unwrap();
            assert_eq!(set.files, restore.files, "expected the source's files verbatim");
        } else {
            let tmp = tempfile::NamedTempFile::new().unwrap();
            fs::write(tmp.path(), file(&set, RADIO_DB)).unwrap();
            let (reference, _r) = source_radio_for(&src.path, &tgt.db_version, &tpl);
            let theirs = sqlite2::Connection::open(&reference).unwrap();
            for t in diffs(&sqlite2::Connection::open(tmp.path()).unwrap(), &theirs) {
                assert!(IDENTITY_RADIO_TABLES.contains(&t.as_str()), "{t} changed");
            }
        }
    }
}

#[test]
fn clone_keeps_a_different_target_identity() {
    let Some(tpl) = templates() else { return eprintln!("skipping: no templates") };
    let Some((src, tgt)) = pairs(true).into_iter().next() else { return eprintln!("skipping: no two pagers of the same model") };
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join(&tgt.id);
    copy_backup(&tgt.path, &target);
    retag_identity(&target);
    let want = identity(&Profile::from_backup(&target).unwrap()).unwrap();
    assert_eq!(want.radio_number, "User-777");

    let (set, sum) = clone_set(&src.path, &target, &tpl).unwrap();
    let mut tables = sum.identity_tables.clone();
    tables.sort();
    assert_eq!(tables, ["tabAccessPassword", "tabMember_New", "tabPagerSUID"]);
    assert!(sum.kept.iter().any(|k| k.label == "Radio number" && k.value == "User-777" && k.differs));
    assert!(sum.kept.iter().any(|k| k.label == "Write password" && k.value == "on" && k.differs));
    assert!(!format!("{sum:?}").contains("0123456789ABCDEF0123456789ABCDEF"), "password hashes must never reach the summary");

    // Profile: the source's, with the target's identity.
    let (out, _t) = profile_of(file(&set, PC_ZIP));
    assert_eq!(identity(&out).unwrap(), want);
    let source = Profile::from_backup(&src.path).unwrap();
    let mut changed = diffs(&out.db, &source.db);
    changed.sort();
    assert_eq!(changed, ["tabAccessPassword", "tabMember_New", "tabPagerSUID"]);
    let s_sum = g5_codeplug::summary::Summary::read(&source).unwrap();
    let o_sum = g5_codeplug::summary::Summary::read(&out).unwrap();
    assert_eq!(serde_json_like(&o_sum), serde_json_like(&s_sum), "zones, knobs and systems must be the source's");

    // Radio DB: the source's except where the identity reaches.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), file(&set, RADIO_DB)).unwrap();
    let radio = sqlite2::Connection::open(tmp.path()).unwrap();
    let (reference, _r) = source_radio_for(&src.path, &tgt.db_version, &tpl);
    let mut changed = diffs(&radio, &sqlite2::Connection::open(&reference).unwrap());
    changed.sort();
    eprintln!("radio tables carrying the identity: {changed:?}");
    assert_eq!(changed, ["tabAccessPassword", "tabContactAddr", "tabContactAddrAndAlertTone", "tabContacts", "tabRadioBaseSetting"]);
    let rn = radio.query("SELECT RadioNo FROM tabRadioBaseSetting", &[]).unwrap().rows;
    assert_eq!(rn[0][0].as_deref(), Some("User-777"));
}

/// The source's radio DB as the target's version would have it: the backup's own
/// when the versions match, else the source profile regenerated.
fn source_radio_for(src: &Path, db_version: &str, tpl: &Path) -> (PathBuf, Option<tempfile::NamedTempFile>) {
    if PagerId::of_backup(src).unwrap().db_version == db_version {
        return (src.join(BACKUP_RADIO_DB), None);
    }
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let variant = Variant::for_db_version(db_version).unwrap();
    g5_codeplug::convert141::convert(&Profile::from_backup(src).unwrap(), &g5_codeplug::Templates::new(tpl).unwrap(), variant, tmp.path()).unwrap();
    (tmp.path().to_path_buf(), Some(tmp))
}

fn serde_json_like(s: &g5_codeplug::summary::Summary) -> String {
    format!("{:?}{:?}{}", s.zones, s.systems, s.conventional_freqs)
}

#[test]
fn clone_regenerates_for_a_target_on_another_radio_db_version() {
    let Some(tpl) = templates() else { return eprintln!("skipping: no templates") };
    let Some((src, tgt)) = pairs(true).into_iter().find(|(s, _)| s.db_version == "DBV1.41c") else { return eprintln!("skipping: no 1.41c pair") };
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join(&tgt.id);
    copy_backup(&tgt.path, &target);
    set_manifest(&target, "Radio DB version:", "DBV1.41b");
    let (set, sum) = clone_set(&src.path, &target, &tpl).unwrap();
    assert!(sum.regenerated);
    let want = tempfile::NamedTempFile::new().unwrap();
    g5_codeplug::convert141::convert(&Profile::from_backup(&src.path).unwrap(), &g5_codeplug::Templates::new(&tpl).unwrap(), g5_codeplug::convert141::Variant::B, want.path()).unwrap();
    let got = tempfile::NamedTempFile::new().unwrap();
    fs::write(got.path(), file(&set, RADIO_DB)).unwrap();
    assert!(diffs(&sqlite2::Connection::open(got.path()).unwrap(), &sqlite2::Connection::open(want.path()).unwrap()).is_empty());
    assert_ne!(file(&set, RADIO_DB), fs::read(src.path.join(BACKUP_RADIO_DB)).unwrap(), "a 1.41b target must not get the 1.41c DB");
}

#[test]
fn clone_between_models_is_refused() {
    let Some(tpl) = templates() else { return eprintln!("skipping: no templates") };
    let pairs = pairs(false);
    if pairs.is_empty() {
        return eprintln!("skipping: no two models in the fixtures");
    }
    for (src, tgt) in pairs {
        let e = clone_set(&src.path, &tgt.path, &tpl).unwrap_err().to_string();
        assert!(e.contains("same model"), "{} -> {}: {e}", src.id, tgt.id);
    }
}

#[test]
fn clone_refuses_a_target_backup_with_a_broken_sidecar() {
    let Some(tpl) = templates() else { return eprintln!("skipping: no templates") };
    let Some((src, tgt)) = pairs(true).into_iter().next() else { return eprintln!("skipping: no two pagers of the same model") };
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join(&tgt.id);
    copy_backup(&tgt.path, &target);
    fs::write(target.join(format!("{BACKUP_PROFILE_ZIP}.md5")), "00000000000000000000000000000000").unwrap();
    assert!(clone_set(&src.path, &target, &tpl).is_err());
}
