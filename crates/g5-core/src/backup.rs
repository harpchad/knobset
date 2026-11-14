//! Pager backups: taking them and listing them.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use g5_proto::DeviceInfo;
use md5::{Digest, Md5};
use serde::Serialize;

use crate::Progress;
use crate::device::{Pager, describe, hex_upper};

/// Directories PPS reads profiles from.
pub const PROFILE_DIRS: &[&str] = &["/defpara/Applications/PPS/", "/root/Applications/u3db/"];
/// Skip anything bigger than this (profiles are a few MB at most).
const MAX_BACKUP_FILE: u32 = 64 * 1024 * 1024;
/// Files in the profile directories that hold credentials (the pager's key
/// material and its super password's MD5). Nothing reads them back and
/// nothing writes them, so backups leave them out.
const NOT_BACKED_UP: &[&str] = &["keytab", "SuperPassword.conf"];

/// A listing name the backup can store: one plain file name. The pager is
/// on the far side of a network link, so a name like `../x` is refused
/// rather than joined onto the backup folder. Backups move between
/// systems, so Windows' rules apply everywhere: no `:*?"<>|`, no trailing
/// dot or space, and no device names like `CON` or `COM1`.
fn safe_name(name: &str) -> bool {
    let mut c = Path::new(name).components();
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && stem.as_bytes()[3].is_ascii_digit());
    matches!((c.next(), c.next()), (Some(std::path::Component::Normal(_)), None))
        && !name.contains(['\\', '\0', ':', '*', '?', '"', '<', '>', '|'])
        && !name.chars().any(char::is_control)
        && !name.ends_with(['.', ' '])
        && !device
}

/// A backup folder on disk, described by its MANIFEST.txt.
#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    /// Folder name (stable ID within the backups root).
    pub id: String,
    pub path: PathBuf,
    pub model: String,
    pub serial: String,
    pub sw_version: String,
    pub db_version: String,
    /// `YYYYMMDD-HHMMSSZ` (UTC) from the folder name; older backups have no `Z` and are local time.
    pub taken: String,
    /// When the backup was taken, as an RFC 3339 UTC instant (empty if the folder name has no stamp).
    pub taken_at: String,
    /// The pager's own MD5 sidecars matched when the backup was taken.
    pub sidecars_ok: bool,
    /// The only mismatch is the radio DB's sidecar, which a firmware update
    /// leaves stale until the next write. Writing from this backup is fine.
    pub radio_db_sidecar_stale: bool,
}

pub fn manifest_field(backup: &Path, label: &str) -> Option<String> {
    let m = fs::read_to_string(backup.join("MANIFEST.txt")).ok()?;
    m.lines().find_map(|l| l.strip_prefix(label).map(|v| v.trim().to_string()))
}

impl BackupInfo {
    pub fn read(dir: &Path) -> Option<Self> {
        let manifest = fs::read_to_string(dir.join("MANIFEST.txt")).ok()?;
        let field = |label: &str| manifest.lines().find_map(|l| l.strip_prefix(label).map(|v| v.trim().to_string())).unwrap_or_default();
        let id = dir.file_name()?.to_string_lossy().into_owned();
        let taken = id.rsplitn(3, '-').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("-");
        let sidecars_ok = manifest.contains("Sidecar checks:") && !manifest.contains("MISMATCH");
        let mismatched: Vec<&str> = manifest.split("\nSidecar checks:").nth(1).unwrap_or("").lines().filter_map(|l| l.trim().strip_suffix(": MISMATCH")).collect();
        let radio_db_sidecar_stale = mismatched == [crate::write::RADIO_DB];
        let taken_at = taken_instant(&taken).map(|t| t.to_string()).unwrap_or_default();
        Some(Self {
            path: dir.to_path_buf(),
            model: field("Model:"),
            serial: field("Serial:"),
            sw_version: field("Software:"),
            db_version: field("Radio DB version:"),
            taken,
            taken_at,
            sidecars_ok,
            radio_db_sidecar_stale,
            id,
        })
    }
}

/// All backups under `root`, newest first.
pub fn list_backups(root: &Path) -> Vec<BackupInfo> {
    let mut out: Vec<BackupInfo> = fs::read_dir(root)
        .map(|rd| rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).filter_map(|e| BackupInfo::read(&e.path())).collect())
        .unwrap_or_default();
    out.sort_by(|a, b| b.taken_at.cmp(&a.taken_at).then_with(|| b.taken.cmp(&a.taken)));
    out
}

/// The instant a folder-name stamp names. New stamps end in `Z` and are UTC. Older ones were
/// written in whatever zone jiff found on the machine (local time on desktop, UTC on iPadOS,
/// where the app can't read the system zone), so they're read back the same way.
fn taken_instant(stamp: &str) -> Option<jiff::Timestamp> {
    let (civil, tz) = match stamp.strip_suffix('Z') {
        Some(s) => (s, jiff::tz::TimeZone::UTC),
        None => (stamp, jiff::tz::TimeZone::system()),
    };
    let dt = jiff::civil::DateTime::strptime("%Y%m%d-%H%M%S", civil).ok()?;
    Some(dt.to_zoned(tz).ok()?.timestamp())
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect()
}

/// Read every file in the profile directories into a new timestamped folder under `root`.
/// The caller must already have passed the read-access check.
pub fn backup(client: &mut Pager, info: &DeviceInfo, root: &Path, progress: &mut dyn FnMut(Progress)) -> Result<BackupInfo> {
    let model = info.model_number.as_deref().unwrap_or("unknown");
    fs::create_dir_all(root).with_context(|| format!("creating {}", root.display()))?;
    // Folder names have one-second resolution. A second backup in the same
    // second (a safety backup right after "Back up now") waits for the next
    // one instead of writing into the first backup's folder.
    let mut tries = 0;
    let dir = loop {
        // UTC, marked with a Z: iPadOS apps can't read the system time zone, so local time isn't reliable.
        let stamp = jiff::Timestamp::now().strftime("%Y%m%d-%H%M%SZ");
        let dir = root.join(format!("{}-{}-{stamp}", sanitize(model), sanitize(&info.serial_number)));
        match fs::create_dir(&dir) {
            Ok(()) => break dir,
            // A few seconds at most: a clock set back onto an old backup's
            // stamp must not hang the pager session.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && tries < 30 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(e).with_context(|| format!("creating {}", dir.display())),
        }
    };
    let r = fill(client, info, &dir, progress);
    // A backup that stopped partway (the pager dropped off, a read failed) is no backup: drop its folder.
    if r.is_err() && !dir.join("MANIFEST.txt").exists() {
        let _ = fs::remove_dir_all(&dir);
    }
    r
}

fn fill(client: &mut Pager, info: &DeviceInfo, dir: &Path, progress: &mut dyn FnMut(Progress)) -> Result<BackupInfo> {
    progress(Progress::step("Listing files"));
    let mut manifest = describe(info);
    manifest.push('\n');
    let mut wanted = Vec::new();
    for d in PROFILE_DIRS {
        match client.list_dir(d) {
            Ok(entries) => {
                let _ = writeln!(manifest, "{d}");
                for e in entries {
                    if !safe_name(&e.name) {
                        bail!("the pager listed an unsafe file name in {d}: {:?}", e.name);
                    }
                    if NOT_BACKED_UP.contains(&e.name.as_str()) {
                        let _ = writeln!(manifest, "  {:>10}  {}  (holds credentials, not backed up)", e.size, e.name);
                        continue;
                    }
                    let _ = writeln!(manifest, "  {:>10}  {}", e.size, e.name);
                    if e.size <= MAX_BACKUP_FILE {
                        wanted.push(format!("{d}{}", e.name));
                    }
                }
            }
            Err(e) => {
                let _ = writeln!(manifest, "{d}  (not listed: {e})");
            }
        }
    }
    if wanted.is_empty() {
        fs::write(dir.join("MANIFEST.txt"), &manifest)?;
        bail!("no profile files found; listing saved to {}", dir.display());
    }

    let paths: Vec<&str> = wanted.iter().map(String::as_str).collect();
    let total = paths.len();
    let files = client
        .read_files_with_progress(&paths, &mut |i| progress(Progress::of(format!("Reading {}", paths[i]), i + 1, total)))
        .context("reading files")?;
    let _ = writeln!(manifest, "\nFiles read:");
    for (path, data) in paths.iter().zip(&files) {
        let local = dir.join(path.trim_start_matches('/'));
        fs::create_dir_all(local.parent().unwrap())?;
        fs::write(&local, data)?;
        let _ = writeln!(manifest, "  {:>10}  {}  {path}", data.len(), hex_upper(&Md5::digest(data)));
    }
    let _ = writeln!(manifest, "\nSidecar checks:");
    for (path, data) in paths.iter().zip(&files) {
        let Some(target) = path.strip_suffix(".md5") else { continue };
        let Some(i) = paths.iter().position(|p| *p == target) else { continue };
        let ok = String::from_utf8_lossy(data).trim().to_uppercase() == hex_upper(&Md5::digest(&files[i]));
        let _ = writeln!(manifest, "  {target}: {}", if ok { "OK" } else { "MISMATCH" });
    }
    fs::write(dir.join("MANIFEST.txt"), &manifest)?;
    BackupInfo::read(dir).context("reading the new backup's manifest")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_names_must_be_plain_file_names() {
        for ok in ["u3db.db", "u3db_PC.zip.md5", "a b.dat", "CONFIG.txt", "COMPANY.db"] {
            assert!(safe_name(ok), "{ok}");
        }
        for bad in ["", ".", "..", "../x", "a/b", "/etc/passwd", "a\\b", "..\\x", "a\0b", "u3db.db.", "u3db.db ", "a:b", "x?.db", "CON", "nul.db", "COM1.txt", "lpt9"] {
            assert!(!safe_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn reads_manifest_and_sorts_newest_first() {
        let root = tempfile::tempdir().unwrap();
        for (stamp, sidecar) in [("20261016-091200", "OK"), ("20261017-165926Z", "MISMATCH")] {
            let dir = root.path().join(format!("G5D66BE-SXVXENB702UNI-A001-220104-G5TEST0001-{stamp}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("MANIFEST.txt"),
                format!("Model:            G5D66BE-SXVXENB702UNI-A001-220104\nSerial:           G5TEST0001\nSoftware:         V1.33\nRadio DB version: DBV1.41b\n\nSidecar checks:\n  /defpara/Applications/PPS/u3db.db: {sidecar}\n"),
            )
            .unwrap();
        }
        fs::create_dir_all(root.path().join("not-a-backup")).unwrap();
        let list = list_backups(root.path());
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].taken, "20261017-165926Z");
        assert_eq!(list[0].taken_at, "2026-10-17T16:59:26Z");
        assert!(list[1].taken_at.starts_with("2026-10-1"));
        assert!(!list[0].sidecars_ok);
        assert!(list[0].radio_db_sidecar_stale);
        assert!(list[1].sidecars_ok);
        assert!(!list[1].radio_db_sidecar_stale);
        assert_eq!(list[1].serial, "G5TEST0001");
        assert_eq!(list[1].db_version, "DBV1.41b");
    }
}
