//! The guarded write flow, end to end against `fake_pager`.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::Path;

use md5::{Digest, Md5};

use crate::backup::{BackupInfo, backup};
use crate::device::{authorize_read, connect, hex_upper};
use crate::fake_pager::FakePager;
use crate::write::{PC_DB, PC_ZIP, RADIO_DB, WriteSet, files_set, program, restore_set};

const GUID: &str = "0e8c1a52-6f1b-4d1e-9a43-5b7f2c8d9e10";

/// An SQLite 2 database with one row, as bytes.
fn db(tag: &str) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.db");
    let c = sqlite2::Connection::open(&path).unwrap();
    c.execute_batch(&format!("CREATE TABLE t(a); INSERT INTO t VALUES('{tag}');")).unwrap();
    drop(c);
    fs::read(path).unwrap()
}

fn zip_of(pc: &[u8]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default();
    w.start_file(format!("{GUID}.db"), opts).unwrap();
    w.write_all(pc).unwrap();
    w.add_directory(format!("{GUID}/"), opts).unwrap();
    w.finish().unwrap().into_inner()
}

fn md5(d: &[u8]) -> Vec<u8> {
    hex_upper(&Md5::digest(d)).into_bytes()
}

/// The five profile files for a profile tagged `tag`.
fn profile(tag: &str) -> Vec<(&'static str, Vec<u8>)> {
    let pc = db(&format!("pc-{tag}"));
    let radio = db(&format!("radio-{tag}"));
    let zip = zip_of(&pc);
    vec![
        (PC_DB, pc),
        (RADIO_DB, radio.clone()),
        ("/defpara/Applications/PPS/u3db.db.md5", md5(&radio)),
        (PC_ZIP, zip.clone()),
        ("/defpara/Applications/PPS/u3db_PC.zip.md5", md5(&zip)),
    ]
}

fn pager_files(tag: &str) -> BTreeMap<String, Vec<u8>> {
    let mut files: BTreeMap<String, Vec<u8>> = profile(tag).into_iter().map(|(p, d)| (p.to_string(), d)).collect();
    files.insert("/root/Applications/u3db/u3db.db".into(), db("live"));
    files.insert("/root/Applications/u3db/keytab".into(), b"KEY MATERIAL".to_vec());
    files.insert("/root/Applications/u3db/SuperPassword.conf".into(), b"0123456789ABCDEF0123456789ABCDEF".to_vec());
    files
}

fn take_backup(fake: &FakePager, root: &Path) -> anyhow::Result<BackupInfo> {
    let (mut c, info) = connect(fake.addr)?;
    let r = authorize_read(&mut c, None).and_then(|()| backup(&mut c, &info, root, &mut |_| {}));
    c.bye();
    r
}

/// The source backup's profile replaced by `tag`'s, as an edit would.
fn edit(_src: &Path, tag: &str) -> WriteSet {
    let mut set = files_set(profile(tag));
    set.require_unchanged_pager = true;
    set
}

fn run(fake: &FakePager, src: &Path, set: &WriteSet, root: &Path) -> anyhow::Result<crate::write::WriteResult> {
    program(fake.addr, None, src, set, root, &mut |_| {})
}

fn pps_file(fake: &FakePager, p: &str) -> Vec<u8> {
    fake.with(|s| s.files[p].clone())
}

#[test]
fn edit_is_written_and_verified() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    let set = edit(&src.path, "b");
    let result = run(&fake, &src.path, &set, root.path()).unwrap();

    assert_eq!(result.verified.len(), 5);
    let written = &set.files[0].1;
    assert_eq!(&pps_file(&fake, PC_DB), written);
    assert_eq!(fake.with(|s| (s.commits, s.aborts)), (1, 0));
    assert_ne!(result.safety_backup.path, src.path, "the safety backup gets its own folder");
    let new = result.new_backup.expect("a backup of the written profile");
    assert_eq!(&fs::read(new.path.join(PC_DB.trim_start_matches('/'))).unwrap(), written);
}

#[test]
fn backups_leave_out_credentials() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let b = take_backup(&fake, root.path()).unwrap();
    let u3db = b.path.join("root/Applications/u3db");
    assert!(u3db.join("u3db.db").is_file());
    assert!(!u3db.join("keytab").exists());
    assert!(!u3db.join("SuperPassword.conf").exists());
    assert!(!fake.with(|s| s.reads.iter().any(|p| p.ends_with("keytab") || p.ends_with("SuperPassword.conf"))), "never even read");
    let manifest = fs::read_to_string(b.path.join("MANIFEST.txt")).unwrap();
    assert!(manifest.contains("keytab  (holds credentials, not backed up)"), "{manifest}");
}

#[test]
fn backup_refuses_unsafe_listing_names() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    fake.with(|s| s.extra_listing.push("../../../escaped".into()));
    let err = take_backup(&fake, &root.path().join("backups")).unwrap_err().to_string();
    assert!(err.contains("unsafe file name"), "{err}");
    assert!(!root.path().join("escaped").exists());
    assert!(fake.with(|s| s.reads.is_empty()), "nothing is read once the listing is refused");
}

#[test]
fn another_pager_is_refused_before_anything_is_sent() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    fake.with(|s| s.serial = "G5OTHER0002".into());
    let err = run(&fake, &src.path, &edit(&src.path, "b"), root.path()).unwrap_err().to_string();
    assert!(err.contains("Serial"), "{err}");
    assert_eq!(fake.with(|s| s.write_businesses), 0);
}

#[test]
fn edit_refuses_a_pager_changed_since_the_backup() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    fake.with(|s| s.files.extend(pager_files("changed")));
    let err = run(&fake, &src.path, &edit(&src.path, "b"), root.path()).unwrap_err().to_string();
    assert!(err.contains("changed since this backup"), "{err}");
    assert_eq!(fake.with(|s| s.write_businesses), 0);
}

#[test]
fn restore_refuses_an_incomplete_safety_backup() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    fake.with(|s| s.files.remove("/defpara/Applications/PPS/u3db_PC.zip.md5"));
    let err = format!("{:#}", run(&fake, &src.path, &restore_set(&src.path).unwrap(), root.path()).unwrap_err());
    assert!(err.contains("doesn't hold the whole profile"), "{err}");
    assert_eq!(fake.with(|s| s.write_businesses), 0);
}

#[test]
fn failed_file_write_aborts_and_names_the_safety_backup() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    let before = pps_file(&fake, PC_DB);
    fake.with(|s| s.fail_write = Some(PC_ZIP.into()));
    let err = format!("{:#}", run(&fake, &src.path, &edit(&src.path, "b"), root.path()).unwrap_err());
    assert!(err.contains("restore the safety backup"), "{err}");
    assert_eq!(fake.with(|s| (s.commits, s.aborts)), (0, 1));
    assert_eq!(pps_file(&fake, PC_DB), before, "nothing applied");
}

#[test]
fn read_back_mismatch_is_reported() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    fake.with(|s| s.corrupt_commit = true);
    let err = format!("{:#}", run(&fake, &src.path, &edit(&src.path, "b"), root.path()).unwrap_err());
    assert!(err.contains("read-back mismatch") && err.contains("safety backup"), "{err}");
}

#[test]
fn read_back_from_a_swapped_pager_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    fake.with(|s| s.serial_after_commit = Some("G5OTHER0002".into()));
    let err = format!("{:#}", run(&fake, &src.path, &edit(&src.path, "b"), root.path()).unwrap_err());
    assert!(err.contains("a different pager answered"), "{err}");
    assert_eq!(fake.with(|s| s.reads.iter().filter(|p| p.ends_with("u3db_PC.db")).count()), 2, "source and safety backups only, no read-back");
}

#[test]
fn restore_checks_the_backup_first() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    restore_set(&src.path).unwrap();

    // The standalone profile DB must be the zip's.
    let pc = src.path.join(PC_DB.trim_start_matches('/'));
    let good = fs::read(&pc).unwrap();
    fs::write(&pc, db("other")).unwrap();
    let err = format!("{:#}", restore_set(&src.path).unwrap_err());
    assert!(err.contains("not the profile DB inside"), "{err}");

    // A database cut off after its header doesn't open.
    fs::write(&pc, &good[..sqlite2::FILE_HEADER.len()]).unwrap();
    assert!(restore_set(&src.path).is_err());
}

#[test]
fn restore_replaces_a_damaged_profile() {
    // A pager whose zip no longer matches its sidecar: an edit refuses it,
    // but a restore is how it gets fixed.
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    fake.with(|s| s.files.insert("/defpara/Applications/PPS/u3db_PC.zip.md5".into(), b"00000000000000000000000000000000".to_vec()));

    let err = format!("{:#}", run(&fake, &src.path, &edit(&src.path, "b"), root.path()).unwrap_err());
    assert!(err.contains("doesn't hold the whole profile"), "{err}");
    assert_eq!(fake.with(|s| s.write_businesses), 0);

    run(&fake, &src.path, &restore_set(&src.path).unwrap(), root.path()).unwrap();
    assert_eq!(fake.with(|s| s.commits), 1);
    assert_eq!(pps_file(&fake, "/defpara/Applications/PPS/u3db_PC.zip.md5"), fs::read(src.path.join("defpara/Applications/PPS/u3db_PC.zip.md5")).unwrap());
}

#[test]
fn edit_refuses_when_its_safety_backup_wouldnt_restore() {
    let root = tempfile::tempdir().unwrap();
    let fake = FakePager::start(pager_files("a"));
    let src = take_backup(&fake, root.path()).unwrap();
    // u3db_PC.db no longer matches the zip's copy, on the pager and in the source backup alike.
    let other = db("other");
    fake.with(|s| s.files.insert(PC_DB.into(), other.clone()));
    fs::write(src.path.join(PC_DB.trim_start_matches('/')), &other).unwrap();
    let err = format!("{:#}", run(&fake, &src.path, &edit(&src.path, "b"), root.path()).unwrap_err());
    assert!(err.contains("couldn't be restored from its safety backup"), "{err}");
    assert_eq!(fake.with(|s| s.write_businesses), 0);
}
