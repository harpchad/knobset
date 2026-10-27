//! Where profiles and Unication templates come from.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use sqlite2::Connection;

use crate::Error;

/// Path of the profile zip inside a `g5 backup` folder.
pub const BACKUP_PROFILE_ZIP: &str = "defpara/Applications/PPS/u3db_PC.zip";
/// Path of the radio DB PPS uploaded, inside a `g5 backup` folder.
pub const BACKUP_RADIO_DB: &str = "defpara/Applications/PPS/u3db.db";

/// A PPS profile: `<GUID>.db` plus its `<GUID>/` resource folder (one DB per
/// P25 system, tone/prompt catalogs, custom wavs).
pub struct Profile {
    pub db: Connection,
    /// The `<GUID>.db` file `db` is open on (a private temp copy when loaded from a zip).
    pub db_path: PathBuf,
    pub folder: PathBuf,
    pub folder_files: HashSet<String>,
    _tmp: Option<tempfile::TempDir>,
}

impl Profile {
    /// Load from a `u3db_PC.zip` (what PPS stores on the pager).
    pub fn from_zip(zip_path: &Path) -> Result<Self, Error> {
        let tmp = tempfile::tempdir()?;
        let file = fs::File::open(zip_path)?;
        let mut zip = zip::ZipArchive::new(file)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let name = entry.name().map(|n| n.into_owned()).unwrap_or_default();
            let Some(rel) = entry.enclosed_name() else {
                return Err(Error::Invalid(format!("unsafe path in zip: {name}")));
            };
            if entry.is_symlink() {
                return Err(Error::Invalid(format!("symlink in zip: {name}")));
            }
            let dest = tmp.path().join(rel);
            if entry.is_dir() {
                fs::create_dir_all(&dest)?;
            } else {
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p)?;
                }
                io::copy(&mut entry, &mut fs::File::create(&dest)?)?;
            }
        }
        let root_db = fs::read_dir(tmp.path())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .find(|p| p.is_file() && p.extension().is_some_and(|x| x == "db"))
            .ok_or_else(|| Error::Invalid("no <GUID>.db at the root of the profile zip".into()))?;
        let folder = root_db.with_extension("");
        let mut p = Self::from_parts(&root_db, &folder)?;
        p._tmp = Some(tmp);
        Ok(p)
    }

    /// Load from a `g5 backup` folder.
    pub fn from_backup(dir: &Path) -> Result<Self, Error> {
        Self::from_zip(&dir.join(BACKUP_PROFILE_ZIP))
    }

    pub fn from_parts(db: &Path, folder: &Path) -> Result<Self, Error> {
        if !sqlite2::is_sqlite2(db) {
            return Err(Error::Invalid(format!("{} is not an SQLite 2.1 database", db.display())));
        }
        let folder_files = match fs::read_dir(folder) {
            Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect(),
            Err(_) => HashSet::new(),
        };
        Ok(Self { db: Connection::open(db)?, db_path: db.to_path_buf(), folder: folder.to_path_buf(), folder_files, _tmp: None })
    }

    /// True when `db` is a private temporary copy (safe to edit).
    pub fn is_working_copy(&self) -> bool {
        self._tmp.is_some()
    }

    /// Open the per-system DB `<folder>/<id>.db`.
    pub fn system_db(&self, id: &str) -> Result<Connection, Error> {
        let p = self.folder.join(format!("{id}.db"));
        if !p.is_file() {
            return Err(Error::Invalid(format!("profile references missing system DB {id}.db")));
        }
        Ok(Connection::open(&p)?)
    }
}

/// Unication's shipped template DBs: the `DataBase` folder of a PPS install
/// (or extracted installer). We never redistribute these.
#[derive(Debug, Clone)]
pub struct Templates {
    pub dir: PathBuf,
}

impl Templates {
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self, Error> {
        let t = Self { dir: dir.into() };
        if !t.data_db().is_file() {
            return Err(Error::Templates(t.dir.clone()));
        }
        Ok(t)
    }

    /// PPS reference data (band ranges, menu and function catalogs).
    pub fn data_db(&self) -> PathBuf {
        self.dir.join("Data_G4A.db")
    }

    /// Empty per-system DB PPS copies for every new P25 trunking system.
    pub fn system_template(&self) -> PathBuf {
        self.dir.join("P25TSystem.db")
    }

    /// Radio DB template for a reported U3DB version, e.g. `DBV1.41c`.
    pub fn radio_template(&self, db_version: &str) -> PathBuf {
        self.dir.join("DB_Base").join(format!("DB_Base_G4A.{db_version}.db"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn zip_at(dir: &Path, build: impl FnOnce(&mut zip::ZipWriter<fs::File>, SimpleFileOptions)) -> std::path::PathBuf {
        let path = dir.join("u3db_PC.zip");
        let mut w = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        build(&mut w, SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored));
        w.finish().unwrap();
        path
    }

    #[test]
    fn zip_entries_must_stay_inside_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        // Absolute names are safe too: enclosed_name re-roots them inside.
        for name in ["../escape.db", "a/../../escape.db", "..\\escape.db"] {
            let path = zip_at(dir.path(), |w, o| {
                w.start_file(name, o).unwrap();
                w.write_all(b"x").unwrap();
            });
            let err = Profile::from_zip(&path).err().map(|e| e.to_string()).unwrap_or_default();
            assert!(err.contains("unsafe path"), "{name}: {err}");
        }
    }

    #[test]
    fn zip_symlinks_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = zip_at(dir.path(), |w, o| w.add_symlink("u3db.db", "/etc/passwd", o).unwrap());
        let err = Profile::from_zip(&path).err().map(|e| e.to_string()).unwrap_or_default();
        assert!(err.contains("symlink"), "{err}");
    }
}
