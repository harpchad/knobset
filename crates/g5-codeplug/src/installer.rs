//! Pull Unication's template DBs out of the PPS installer the user downloaded.
//!
//! The download is a zip holding an Inno Setup exe, which carries
//! `PPS_Setup.msi`, whose embedded CAB holds the `DataBase` folder. Any of the
//! three layers is accepted. Only the folder holding `Data_G4A.db` is kept
//! (PPS also ships an unrelated `ProgramLog/DataBase`).

use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

use crate::{Error, Templates};

const DATA_DB: &str = "Data_G4A.db";
const INNER_MSI: &str = "pps_setup.msi";
/// MSI directory chains are shallow; anything deeper is a malformed table.
const MAX_DIR_DEPTH: usize = 32;

/// What [`import_installer`] wrote.
#[derive(Debug, Clone)]
pub struct Imported {
    pub templates: Templates,
    pub files: usize,
    pub bytes: u64,
}

/// Extract the template folder from `src` (PPS zip, Inno exe, or MSI) into
/// `dest`, replacing any previous import only once the new one is complete.
pub fn import_installer(src: &Path, dest: &Path) -> Result<Imported, Error> {
    let msi = find_msi(fs::read(src)?)?;
    let name = dest.file_name().ok_or_else(|| Error::Installer("destination has no folder name".into()))?;
    let staging = dest.with_file_name(format!(".{}.partial", name.to_string_lossy()));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    let (files, bytes) = match extract_database(msi, &staging).and_then(|n| Templates::new(&staging).map(|_| n)) {
        Ok(n) => n,
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
    };
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::rename(&staging, dest)?;
    Ok(Imported { templates: Templates::new(dest)?, files, bytes })
}

/// Peel the zip and Inno layers until the MSI is reached.
fn find_msi(bytes: Vec<u8>) -> Result<Vec<u8>, Error> {
    match bytes.get(..4) {
        Some([0xD0, 0xCF, 0x11, 0xE0]) => Ok(bytes),
        Some([b'P', b'K', 3, 4]) => {
            let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
            let idx = (0..zip.len())
                .find(|&i| zip.by_index(i).is_ok_and(|e| e.name().is_ok_and(|n| is_installer_name(&n))))
                .ok_or_else(|| Error::Installer("the zip has no PPS installer in it".into()))?;
            let mut inner = Vec::new();
            zip.by_index(idx)?.read_to_end(&mut inner)?;
            find_msi(inner)
        }
        Some([b'M', b'Z', ..]) => {
            let inst = innospect::InnoInstaller::from_bytes(&bytes).map_err(|e| Error::Installer(e.to_string()))?;
            let file = inst
                .files()
                .iter()
                .find(|f| f.destination.to_lowercase().ends_with(INNER_MSI))
                .ok_or_else(|| Error::Installer("this installer doesn't contain PPS_Setup.msi".into()))?;
            inst.extract_to_vec(file).map_err(|e| Error::Installer(e.to_string()))
        }
        _ => Err(Error::Installer("not a PPS installer (expected the .zip, .exe, or .msi from Unication)".into())),
    }
}

fn is_installer_name(name: &str) -> bool {
    let n = name.to_lowercase();
    !n.starts_with("__macosx/") && (n.ends_with(".exe") || n.ends_with(".msi"))
}

/// MSI names are `short|long`; directory names may also be `target:source`.
fn long_name(s: &str) -> &str {
    let target = s.split(':').next().unwrap_or(s);
    target.rsplit('|').next().unwrap_or(target)
}

fn col(row: &msi::Row, name: &str) -> Result<String, Error> {
    row[name].as_str().map(str::to_string).ok_or_else(|| Error::Installer(format!("MSI row is missing {name}")))
}

/// Write every file under the `DataBase` folder (the one holding
/// `Data_G4A.db`) into `out`. Returns (files, bytes).
fn extract_database(msi_bytes: Vec<u8>, out: &Path) -> Result<(usize, u64), Error> {
    let mut pkg = msi::Package::open(Cursor::new(msi_bytes))?;

    let mut dirs: HashMap<String, (Option<String>, String)> = HashMap::new();
    for row in pkg.select_rows(msi::Select::table("Directory"))? {
        let parent = row["Directory_Parent"].as_str().map(str::to_string);
        dirs.insert(col(&row, "Directory")?, (parent, long_name(&col(&row, "DefaultDir")?).to_string()));
    }
    let dir_path = |id: &str| -> Result<PathBuf, Error> {
        let mut parts = Vec::new();
        let mut id = id.to_string();
        for _ in 0..MAX_DIR_DEPTH {
            let Some((parent, name)) = dirs.get(&id) else { break };
            let Some(parent) = parent.as_ref().filter(|p| **p != id) else { return Ok(parts.iter().rev().collect()) };
            if name != "." {
                parts.push(safe_component(name)?);
            }
            id = parent.clone();
        }
        Err(Error::Installer(format!("MSI directory {id} doesn't resolve")))
    };

    let mut comp_dir = HashMap::new();
    for row in pkg.select_rows(msi::Select::table("Component"))? {
        comp_dir.insert(col(&row, "Component")?, col(&row, "Directory_")?);
    }
    // File key (the name inside the CAB) -> path relative to the install root.
    let mut paths = HashMap::new();
    for row in pkg.select_rows(msi::Select::table("File"))? {
        let comp = col(&row, "Component_")?;
        let dir = comp_dir.get(&comp).ok_or_else(|| Error::Installer(format!("MSI component {comp} has no directory")))?;
        let name = safe_component(long_name(&col(&row, "FileName")?))?;
        paths.insert(col(&row, "File")?, dir_path(dir)?.join(name));
    }
    let root = paths
        .values()
        .find(|p| p.file_name().is_some_and(|n| n == DATA_DB))
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::Installer(format!("this installer has no {DATA_DB}")))?;

    let mut cabs = Vec::new();
    for row in pkg.select_rows(msi::Select::table("Media"))? {
        if let Some(c) = row["Cabinet"].as_str() {
            cabs.push(c.strip_prefix('#').ok_or_else(|| Error::Installer(format!("external cabinet {c} isn't supported")))?.to_string());
        }
    }

    let (mut files, mut bytes) = (0, 0);
    for name in cabs {
        let mut buf = Vec::new();
        pkg.read_stream(&name)?.read_to_end(&mut buf)?;
        let mut cab = cab::Cabinet::new(Cursor::new(buf))?;
        let keys: Vec<String> = cab.folder_entries().flat_map(|f| f.file_entries().map(|e| e.name().to_string())).collect();
        for key in keys {
            let Some(rel) = paths.get(&key).and_then(|p| p.strip_prefix(&root).ok()) else { continue };
            let dest = out.join(rel);
            if let Some(p) = dest.parent() {
                fs::create_dir_all(p)?;
            }
            bytes += std::io::copy(&mut cab.read_file(&key)?, &mut fs::File::create(&dest)?)?;
            files += 1;
        }
    }
    Ok((files, bytes))
}

/// One plain path component; MSI tables are user-supplied input.
fn safe_component(name: &str) -> Result<String, Error> {
    let mut c = Path::new(name).components();
    match (c.next(), c.next()) {
        (Some(Component::Normal(_)), None) if !name.contains('\\') => Ok(name.to_string()),
        _ => Err(Error::Installer(format!("unsafe path in MSI: {name}"))),
    }
}
