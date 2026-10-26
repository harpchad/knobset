//! Writing one pager's backup to a different pager of the same model (`notes/cloning.md`).
//!
//! The write set is built against the target's own newest backup, so `program`'s
//! safety properties hold unchanged: the attached pager must be the target, a
//! safety backup comes first, the target must still hold that backup's profile,
//! and every file is read back. The guard is exact: the result is the source
//! profile with the target's identity fields restored, and its radio DB differs
//! from the source's only in the tables those fields reach.

use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use anyhow::{Context, Result, bail};
use g5_codeplug::clone::{IDENTITY_RADIO_TABLES, KeptField, check_clone, identity, keep_identity, kept_fields, missing_protocols};
use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::source::{BACKUP_PROFILE_ZIP, BACKUP_RADIO_DB};
use g5_codeplug::summary::Summary;
use g5_codeplug::system_edit::model_band_codes;
use g5_codeplug::{Profile, Templates};
use md5::{Digest, Md5};
use serde::Serialize;

use crate::Progress;
use crate::device::{connect, hex_upper};
use crate::write::{PC_DB, PC_ZIP, PagerId, RADIO_DB, WriteResult, WriteSet, load_backup_files, program_on, restore_set, tempfile_path};

/// One side of a clone, for display.
#[derive(Debug, Clone, Serialize)]
pub struct CloneSide {
    pub backup_id: String,
    pub pager: PagerId,
    /// The profile's name (`tabOrganization_New.OrgName`).
    pub profile: String,
}

/// Sizes of a profile, to show what a clone replaces.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProfileCounts {
    pub zones: usize,
    pub knobs: usize,
    pub systems: usize,
    pub talkgroups: usize,
    pub conventional_freqs: usize,
}

impl ProfileCounts {
    fn of(s: &Summary) -> Self {
        Self {
            zones: s.zones.len(),
            knobs: s.zones.iter().map(|z| z.knobs.len()).sum(),
            systems: s.systems.len(),
            talkgroups: s.systems.iter().map(|x| x.talkgroups).sum(),
            conventional_freqs: s.conventional_freqs,
        }
    }
}

/// What a clone does to the target pager.
#[derive(Debug, Clone, Serialize)]
pub struct CloneSummary {
    pub source: CloneSide,
    pub target: CloneSide,
    /// Identity fields the target keeps, and whether the source's differ.
    pub kept: Vec<KeptField>,
    /// The target now, and after the write.
    pub before: ProfileCounts,
    pub after: ProfileCounts,
    pub zones_added: Vec<String>,
    pub zones_removed: Vec<String>,
    pub systems_added: Vec<String>,
    pub systems_removed: Vec<String>,
    /// The radio DB was regenerated for the target's radio DB version (the source's differs).
    pub regenerated: bool,
    /// Profile identity tables restored from the target (empty when both pagers' identities match).
    pub identity_tables: Vec<String>,
}

/// The model as PPS compares it: the first 21 characters (`ProgramController.cs:79-80,107`).
fn model21(m: &str) -> &str {
    m.get(..21).unwrap_or(m)
}

/// Whether a profile from `source` can be written to `target`. Returns the target's
/// converter variant. Pure, so tests can call it with any two pager descriptions.
pub fn check_clone_compatible(source: &PagerId, target: &PagerId) -> Result<Variant> {
    if source.serial == target.serial {
        bail!("both are pager {}; to write a backup back to the pager it came from, open it and write, or restore it", target.serial);
    }
    let (s, t) = (model21(&source.model), model21(&target.model));
    if s != t {
        let part = |m: &str, r: std::ops::Range<usize>| m.get(r).unwrap_or("?").to_string();
        let mut why = Vec::new();
        for (label, r) in [("series", 0..2), ("hardware", 2..3), ("protocols", 3..5), ("software", 5..7), ("bands", 8..12), ("language", 12..14), ("revision", 14..21)] {
            if part(s, r.clone()) != part(t, r.clone()) {
                why.push(format!("{label} {} vs {}", part(s, r.clone()), part(t, r)));
            }
        }
        bail!(
            "the source pager is a {s} and the target a {t} ({}); Knobset only writes a profile to another pager of the same model",
            why.join(", ")
        );
    }
    Variant::for_db_version(&source.db_version).with_context(|| format!("the source's radio DB version {} isn't supported", source.db_version))?;
    Variant::for_db_version(&target.db_version).with_context(|| format!("the target's radio DB version {} isn't supported", target.db_version))
}

fn radio_diffs(ours: &Path, theirs: &Path) -> Result<Vec<String>> {
    let (a, b) = (sqlite2::Connection::open(ours)?, sqlite2::Connection::open(theirs)?);
    Ok(compare(&a, &b)?.into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect())
}

fn side(dir: &Path, p: &Profile) -> Result<CloneSide> {
    Ok(CloneSide {
        backup_id: dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        pager: PagerId::of_backup(dir)?,
        profile: Summary::read(p)?.name,
    })
}

/// `source`'s profile adjusted for the pager `target` is a backup of, to write to that
/// pager with [`program_clone`]. `target` should be the target's newest backup: the
/// write refuses if the pager changed since.
pub fn clone_set(source: &Path, target: &Path, templates: &Path) -> Result<(WriteSet, CloneSummary)> {
    let (src_id, tgt_id) = (PagerId::of_backup(source)?, PagerId::of_backup(target)?);
    let variant = check_clone_compatible(&src_id, &tgt_id)?;
    let src_variant = Variant::for_db_version(&src_id.db_version).unwrap();
    load_backup_files(source).context("source backup")?;
    let mut set = restore_set(target).context("target backup")?;
    let templates = Templates::new(templates)?;

    let original = Profile::from_backup(source)?;
    let work = Profile::from_backup(source)?;
    let theirs = Profile::from_backup(target)?;
    let (before, after) = (Summary::read(&theirs)?, Summary::read(&original)?);
    for (p, id, which) in [(&original, &src_id, "source"), (&theirs, &tgt_id, "target")] {
        let m = Summary::read(p)?.model;
        if model21(&m) != model21(&id.model) {
            bail!("the {which} backup's profile is for a {m}, not the pager's own model {}", model21(&id.model));
        }
    }
    let missing = missing_protocols(&original, &theirs);
    if !missing.is_empty() {
        bail!("the source profile uses {} and the target's profile doesn't allow it", missing.join(", "));
    }
    let bands = model_band_codes(&tgt_id.model);
    for r in g5_codeplug::row::load_table(&original.db, "tabP25TrunkingSecurity") {
        let code = r.s("BandTypeCode");
        if code.as_bytes().chunks(2).any(|c| !bands.iter().any(|b| b.as_bytes() == c)) {
            bail!("system {} is in band {code}, which the target ({}) doesn't have", r.s("SystemName"), bands.join(", "));
        }
    }

    // The source's own radio DB must regenerate exactly, or nothing below proves anything.
    let (src_regen, out, ref_db) = (tempfile_path("g5-clone-src"), tempfile_path("g5-clone-out"), tempfile_path("g5-clone-ref"));
    let cleanup = || {
        for f in [&src_regen, &out, &ref_db] {
            let _ = fs::remove_file(f);
        }
    };
    // (radio DB, identity tables restored, radio tables changed on the target, experimental)
    type Built = (Vec<u8>, Vec<String>, Vec<String>, Vec<String>);
    let r = (|| -> Result<Built> {
        convert(&original, &templates, src_variant, &src_regen)?;
        let off = radio_diffs(&src_regen, &source.join(BACKUP_RADIO_DB))?;
        if !off.is_empty() {
            bail!("Knobset can't regenerate the source backup's radio DB exactly ({} differ); refusing to clone it", off.join(", "));
        }
        keep_identity(&work, &theirs)?;
        let identity_tables = check_clone(&work.db, &original.db, &theirs.db)?;
        let report = convert(&work, &templates, variant, &out)?;
        if !report.unverified.is_empty() {
            bail!("profile uses features not yet verified for writing: {}", report.unverified.join(", "));
        }
        let reference = if variant == src_variant {
            source.join(BACKUP_RADIO_DB)
        } else {
            convert(&original, &templates, variant, &ref_db)?;
            ref_db.clone()
        };
        let diffs = radio_diffs(&out, &reference)?;
        if let Some(t) = diffs.iter().find(|t| !IDENTITY_RADIO_TABLES.contains(&t.as_str())) {
            bail!("keeping the target's identity unexpectedly changed {t} in the radio DB; refusing");
        }
        if identity_tables.is_empty() && !diffs.is_empty() {
            bail!("the radio DB differs from the source's in {} although no identity field differs; refusing", diffs.join(", "));
        }
        let on_target = radio_diffs(&out, &target.join(BACKUP_RADIO_DB))?;
        // Same version and nothing to restore: the source's own radio DB, byte for byte.
        let radio = if identity_tables.is_empty() && variant == src_variant { fs::read(source.join(BACKUP_RADIO_DB))? } else { fs::read(&out)? };
        Ok((radio, identity_tables, on_target, report.experimental))
    })();
    cleanup();
    let (radio, identity_tables, on_target, experimental) = r?;

    // With nothing to restore, the source's files go out verbatim.
    let now = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
    let (pc_db, pc_zip) = if identity_tables.is_empty() {
        (fs::read(source.join(PC_DB.trim_start_matches('/')))?, fs::read(source.join(BACKUP_PROFILE_ZIP))?)
    } else {
        (fs::read(&work.db_path)?, g5_codeplug::edit::repack_zip(&source.join(BACKUP_PROFILE_ZIP), &work, &now)?)
    };
    for (target, bytes) in [(PC_DB, pc_db), (RADIO_DB, radio), (PC_ZIP, pc_zip)] {
        set.files.iter_mut().find(|(p, _)| *p == target).unwrap().1 = bytes;
    }
    for target in [RADIO_DB, PC_ZIP] {
        let md5 = hex_upper(&Md5::digest(&set.files.iter().find(|(p, _)| *p == target).unwrap().1)).into_bytes();
        let sidecar = format!("{target}.md5");
        set.files.iter_mut().find(|(p, _)| *p == sidecar).unwrap().1 = md5;
    }
    set.changed_tables = on_target;
    set.experimental = experimental;
    set.require_unchanged_pager = true;

    let names = |s: &Summary, z: bool| -> Vec<String> { if z { s.zones.iter().map(|x| x.name.clone()).collect() } else { s.systems.iter().map(|x| x.name.clone()).collect() } };
    let minus = |a: Vec<String>, b: &[String]| a.into_iter().filter(|x| !b.contains(x)).collect::<Vec<_>>();
    let summary = CloneSummary {
        source: side(source, &original)?,
        target: side(target, &theirs)?,
        kept: kept_fields(&identity(&original)?, &identity(&theirs)?),
        before: ProfileCounts::of(&before),
        after: ProfileCounts::of(&after),
        zones_added: minus(names(&after, true), &names(&before, true)),
        zones_removed: minus(names(&before, true), &names(&after, true)),
        systems_added: minus(names(&after, false), &names(&before, false)),
        systems_removed: minus(names(&before, false), &names(&after, false)),
        regenerated: variant != src_variant,
        identity_tables,
    };
    Ok((set, summary))
}

/// Write a [`clone_set`] to the attached pager, which must be the target. Same steps as
/// `write::program`, after checking the live pager against the source's model.
pub fn program_clone(
    addr: SocketAddr,
    password: Option<&str>,
    summary: &CloneSummary,
    target: &Path,
    set: &WriteSet,
    backups_root: &Path,
    progress: &mut dyn FnMut(Progress),
) -> Result<WriteResult> {
    progress(Progress::step("Connecting"));
    let (mut client, info) = connect(addr)?;
    let result = check_clone_compatible(&summary.source.pager, &PagerId::of(&info))
        .and_then(|_| program_on(&mut client, &info, addr, password, target, set, backups_root, progress));
    client.bye();
    result
}
