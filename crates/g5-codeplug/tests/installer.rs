//! Template import against the real PPS download (skipped without it; never
//! redistributed). Expects the import to match a reference `DataBase` folder
//! byte for byte.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::installer::import_installer;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn env_or(var: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).unwrap_or(default)
}

fn files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    out.sort();
    out
}

#[test]
fn import_matches_reference_database() {
    let installer = env_or("G5_PPS_INSTALLER", repo().join("Unication_Pager_PPS_Setup_V00-04-00-48.zip"));
    let reference = env_or("G5_TEMPLATES", repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"));
    if !installer.is_file() || !reference.is_dir() {
        return eprintln!("skipping: PPS installer or reference templates not available");
    }
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("DataBase");
    let r = import_installer(&installer, &dest).unwrap();

    let (got, want) = (files(&dest), files(&reference));
    assert_eq!(got, want);
    assert_eq!(r.files, want.len());
    for rel in &want {
        assert!(fs::read(dest.join(rel)).unwrap() == fs::read(reference.join(rel)).unwrap(), "{} differs", rel.display());
    }
    // A second import replaces the first in place and leaves no staging dir.
    import_installer(&installer, &dest).unwrap();
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
}

#[test]
fn rejects_files_that_are_not_installers() {
    let tmp = tempfile::tempdir().unwrap();
    let bogus = tmp.path().join("notes.txt");
    fs::write(&bogus, "hello").unwrap();
    let dest = tmp.path().join("DataBase");
    let err = import_installer(&bogus, &dest).unwrap_err().to_string();
    assert!(err.contains("not a PPS installer"), "{err}");
    assert!(!dest.exists());
}
