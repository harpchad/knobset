//! Building what to write, and writing it safely.
//!
//! Every write goes through `program`, which:
//! 1. requires the backup to come from this exact pager (model, serial, radio DB version),
//! 2. verifies the backup's own MD5 sidecars and file formats,
//! 3. takes a fresh safety backup,
//! 4. for edits, refuses if the pager changed since the source backup,
//! 5. programs the files in PPS order inside one write business,
//! 6. reconnects after the pager's reboot and verifies every file byte for byte.

use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::Variant;
use g5_codeplug::source::{BACKUP_PROFILE_ZIP, BACKUP_RADIO_DB};
use g5_codeplug::summary::Summary;
use g5_codeplug::{Profile, Templates};
use g5_proto::DeviceInfo;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

use crate::backup::{BackupInfo, backup, manifest_field};
use crate::device::{Pager, authorize_read, authorize_write, connect, hex_upper};
use crate::Progress;

pub const PC_DB: &str = "/defpara/Applications/PPS/u3db_PC.db";
pub const RADIO_DB: &str = "/defpara/Applications/PPS/u3db.db";
pub const PC_ZIP: &str = "/defpara/Applications/PPS/u3db_PC.zip";

/// Files PPS programs for a profile, in the order it sends them (data before its .md5).
pub const PROFILE_FILES: &[&str] = &[
    PC_DB,
    RADIO_DB,
    "/defpara/Applications/PPS/u3db.db.md5",
    PC_ZIP,
    "/defpara/Applications/PPS/u3db_PC.zip.md5",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnobRename {
    pub zone: String,
    pub knob: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenameApplied {
    pub zone: String,
    pub knob: i64,
    pub old: String,
    pub new: String,
}

/// New talkgroup list for a trunked knob, by `GroupID`: priority list in order, then the rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnobTalkgroupEdit {
    pub zone: String,
    pub knob: i64,
    pub priority: Vec<String>,
    pub normal: Vec<String>,
}

/// Everything the user changed, applied to a backup's profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Edits {
    pub renames: Vec<KnobRename>,
    pub talkgroups: Vec<KnobTalkgroupEdit>,
    /// Complete desired state of edited P25 systems (applied first).
    pub systems: Vec<g5_codeplug::system_edit::SystemEdit>,
    /// Systems to add, complete.
    pub new_systems: Vec<g5_codeplug::system_edit::NewSystem>,
    /// Systems copied from another backup (added like new systems).
    pub copied_systems: Vec<g5_codeplug::system_edit::SystemCopy>,
    /// `P25TrunkingID`s of systems to remove (refused while a knob uses one).
    pub deleted_systems: Vec<String>,
    /// Systems put on knobs (empty knob positions, or trunked knobs moving system).
    pub knob_systems: Vec<g5_codeplug::knob_system::KnobSystemEdit>,
    /// Analog conventional channels put on empty knob positions.
    pub conv_knobs: Vec<g5_codeplug::conv_knob::ConvKnobEdit>,
    /// Knobs to make empty. With a new knob on the same position in the same
    /// batch, this converts the knob's type.
    pub removed_knobs: Vec<g5_codeplug::knob_remove::KnobRemoval>,
    /// In-place edits of analog conventional knobs.
    pub conv_updates: Vec<g5_codeplug::knob_remove::ConvKnobUpdate>,
    /// Scan knobs on empty positions, or new member lists for existing ones.
    pub scan_knobs: Vec<g5_codeplug::scan_knob::ScanKnobEdit>,
    /// DMR and P25 conventional knobs on empty positions.
    pub digital_knobs: Vec<g5_codeplug::digital_knob::DigitalKnobEdit>,
    /// Zones to add (appended, applied before knob edits so those can use them).
    pub new_zones: Vec<g5_codeplug::zone_edit::NewZone>,
    pub zone_renames: Vec<g5_codeplug::zone_edit::ZoneRename>,
    /// IDs of empty zones to delete.
    pub deleted_zones: Vec<String>,
    /// Every zone ID in the new order, if the order changes.
    pub zone_order: Option<Vec<String>>,
    /// Knobs copied to empty positions (applied before scan edits, so a scan can list a copy).
    pub knob_copies: Vec<g5_codeplug::knob_copy::KnobCopy>,
    /// Knobs moved to empty positions or swapped: applied last, checked as a renumbering.
    pub knob_moves: Vec<g5_codeplug::knob_copy::KnobMove>,
    /// Device-wide settings by key, in the profile's encoding (notes/pager-settings.md).
    pub settings: std::collections::BTreeMap<String, i64>,
}

impl Edits {
    pub fn is_empty(&self) -> bool {
        self.renames.is_empty()
            && self.talkgroups.is_empty()
            && self.systems.is_empty()
            && self.new_systems.is_empty()
            && self.copied_systems.is_empty()
            && self.deleted_systems.is_empty()
            && self.knob_systems.is_empty()
            && self.conv_knobs.is_empty()
            && self.scan_knobs.is_empty()
            && self.digital_knobs.is_empty()
            && self.removed_knobs.is_empty()
            && self.conv_updates.is_empty()
            && self.new_zones.is_empty()
            && self.zone_renames.is_empty()
            && self.deleted_zones.is_empty()
            && self.zone_order.is_none()
            && self.knob_copies.is_empty()
            && self.knob_moves.is_empty()
            && self.settings.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TalkgroupRef {
    pub tgid: i64,
    pub name: String,
}

/// What changed on one knob's talkgroup list.
#[derive(Debug, Clone, Serialize)]
pub struct TalkgroupChange {
    pub zone: String,
    pub knob: i64,
    pub knob_name: String,
    pub added: Vec<TalkgroupRef>,
    pub removed: Vec<TalkgroupRef>,
    /// Same talkgroups, but the priority list or order changed.
    pub priority_changed: bool,
    pub count: usize,
}

/// Files to write plus a description of the change.
#[derive(Debug)]
pub struct WriteSet {
    pub files: Vec<(&'static str, Vec<u8>)>,
    pub renames: Vec<RenameApplied>,
    pub talkgroups: Vec<TalkgroupChange>,
    pub systems: Vec<g5_codeplug::system_edit::SystemChange>,
    pub knob_systems: Vec<g5_codeplug::knob_system::KnobSystemChange>,
    pub conv_knobs: Vec<g5_codeplug::conv_knob::ConvKnobChange>,
    pub zones: Vec<g5_codeplug::zone_edit::ZoneChange>,
    pub scan_knobs: Vec<g5_codeplug::scan_knob::ScanKnobChange>,
    pub removed_knobs: Vec<g5_codeplug::knob_remove::KnobRemovalChange>,
    pub conv_updates: Vec<g5_codeplug::knob_remove::ConvKnobUpdateChange>,
    pub digital_knobs: Vec<g5_codeplug::digital_knob::DigitalKnobChange>,
    pub knob_copies: Vec<g5_codeplug::knob_copy::KnobCopyChange>,
    pub knob_moves: Vec<g5_codeplug::knob_copy::KnobMoveChange>,
    pub settings: Vec<g5_codeplug::settings::SettingChange>,
    /// Converted by PPS's rules but never checked on a pager.
    pub experimental: Vec<String>,
    pub changed_tables: Vec<String>,
    /// Edits must not overwrite changes made after the source backup.
    pub require_unchanged_pager: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteResult {
    pub safety_backup: BackupInfo,
    pub verified: Vec<String>,
    /// A backup of the pager as written, to keep editing from. `None` when
    /// that backup failed (the write itself still succeeded).
    pub new_backup: Option<BackupInfo>,
}

/// Whether two backups hold the same profile (all five profile files equal).
/// A safety backup of a write that never programmed matches the backup it
/// was taken from, so editing can continue from either.
pub fn same_profile(a: &Path, b: &Path) -> bool {
    match (load_backup_files(a), load_backup_files(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn rel(p: &str) -> &str {
    p.trim_start_matches('/')
}

/// Load the backup's five profile files and verify their MD5 sidecars.
///
/// A firmware update migrates the radio DB in place and leaves its sidecar
/// stale. That one mismatch is accepted when the radio DB is exactly what the
/// backup read off the pager, and the sidecar is corrected in the result.
pub(crate) fn load_backup_files(src: &Path) -> Result<Vec<(&'static str, Vec<u8>)>> {
    let mut files = Vec::new();
    for p in PROFILE_FILES {
        let local = src.join(rel(p));
        files.push((*p, fs::read(&local).with_context(|| format!("reading {}", local.display()))?));
    }
    for i in 0..files.len() {
        let Some(target) = files[i].0.strip_suffix(".md5") else { continue };
        let md5 = hex_upper(&Md5::digest(&files.iter().find(|(p, _)| *p == target).unwrap().1));
        if String::from_utf8_lossy(&files[i].1).trim().to_uppercase() == md5 {
            continue;
        }
        if target != RADIO_DB || md5_read_from_pager(src, target).as_deref() != Some(md5.as_str()) {
            bail!("{target} in the backup does not match its .md5");
        }
        files[i].1 = md5.into_bytes();
    }
    if !files[0].1.starts_with(sqlite2::FILE_HEADER) || !files[1].1.starts_with(sqlite2::FILE_HEADER) {
        bail!("backup databases are not SQLite 2.1 files");
    }
    Ok(files)
}

/// The MD5 the backup's manifest recorded for `path` as read off the pager.
fn md5_read_from_pager(src: &Path, path: &str) -> Option<String> {
    let manifest = fs::read_to_string(src.join("MANIFEST.txt")).ok()?;
    let read = manifest.split("\nFiles read:").nth(1)?.split("\nSidecar checks:").next()?;
    read.lines().find_map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
        [_, md5, p] if p == path => Some(md5.to_uppercase()),
        _ => None,
    })
}

/// The backup's files as they are (a restore), apart from a corrected stale radio DB sidecar.
pub fn restore_set(src: &Path) -> Result<WriteSet> {
    Ok(WriteSet { files: load_backup_files(src)?, renames: Vec::new(), talkgroups: Vec::new(), systems: Vec::new(), knob_systems: Vec::new(), conv_knobs: Vec::new(), zones: Vec::new(), scan_knobs: Vec::new(), removed_knobs: Vec::new(), conv_updates: Vec::new(), digital_knobs: Vec::new(), knob_copies: Vec::new(), knob_moves: Vec::new(), settings: Vec::new(), experimental: Vec::new(), changed_tables: Vec::new(), require_unchanged_pager: false })
}

/// The backup's profile with `edits` applied, regenerated for the pager.
pub fn edit_set(src: &Path, templates: &Path, edits: &Edits) -> Result<WriteSet> {
    if edits.is_empty() {
        bail!("no changes to write");
    }
    if let Some(id) = edits.deleted_systems.iter().find(|id| {
        edits.systems.iter().any(|e| &e.system_id == *id) || edits.new_systems.iter().any(|n| &n.system.system_id == *id) || edits.copied_systems.iter().any(|c| &c.system.system_id == *id)
    }) {
        bail!("system {id} is both deleted and edited in this batch");
    }
    if let Some(k) = edits.knob_systems.iter().find(|k| edits.talkgroups.iter().any(|t| t.zone == k.zone && t.knob == k.knob)) {
        bail!("{} knob {} has both a system change and a separate talkgroup edit", k.zone, k.knob);
    }
    let touched = |zone: &str, knob: i64| {
        edits.knob_systems.iter().any(|k| k.zone == zone && k.knob == knob)
            || edits.talkgroups.iter().any(|t| t.zone == zone && t.knob == knob)
            || edits.renames.iter().any(|r| r.zone == zone && r.knob == knob)
    };
    // A removal pairs only with one new knob on the same position (a conversion).
    // Renames apply after new knobs, so a new trunked knob can take one.
    for r in &edits.removed_knobs {
        let same = |z: &str, k: i64| z == r.zone && k == r.knob;
        let adds = edits.conv_knobs.iter().filter(|c| same(&c.zone, c.knob)).count()
            + edits.knob_systems.iter().filter(|c| same(&c.zone, c.knob)).count()
            + edits.digital_knobs.iter().filter(|c| same(&c.zone, c.knob)).count()
            + edits.scan_knobs.iter().filter(|c| same(&c.zone, c.knob)).count();
        let trunked = edits.knob_systems.iter().any(|c| same(&c.zone, c.knob));
        let others = (!trunked && edits.renames.iter().any(|x| same(&x.zone, x.knob)))
            || edits.talkgroups.iter().any(|x| same(&x.zone, x.knob))
            || edits.conv_updates.iter().any(|x| same(&x.zone, x.knob))
            || edits.removed_knobs.iter().filter(|x| same(&x.zone, x.knob)).count() > 1;
        if adds > 1 || others {
            bail!("{} knob {} is removed and changed in other ways in the same batch", r.zone, r.knob);
        }
    }
    if let Some(u) = edits.conv_updates.iter().find(|u| {
        edits.knob_systems.iter().any(|k| k.zone == u.zone && k.knob == u.knob)
            || edits.conv_knobs.iter().any(|k| k.zone == u.zone && k.knob == u.knob)
            || edits.scan_knobs.iter().any(|k| k.zone == u.zone && k.knob == u.knob)
            || edits.conv_updates.iter().filter(|k| k.zone == u.zone && k.knob == u.knob).count() > 1
    }) {
        bail!("{} knob {} has a conventional edit and another change in the same batch", u.zone, u.knob);
    }
    if let Some(c) = edits.conv_knobs.iter().find(|c| touched(&c.zone, c.knob)) {
        bail!("{} knob {} gets a conventional channel and another change in the same batch", c.zone, c.knob);
    }
    if let Some(d) = edits.digital_knobs.iter().find(|d| touched(&d.zone, d.knob) || edits.conv_knobs.iter().any(|k| k.zone == d.zone && k.knob == d.knob)) {
        bail!("{} knob {} gets a digital channel and another change in the same batch", d.zone, d.knob);
    }
    if let Some(c) = edits.scan_knobs.iter().find(|c| touched(&c.zone, c.knob) || edits.conv_knobs.iter().any(|k| k.zone == c.zone && k.knob == c.knob)) {
        bail!("{} knob {} gets a scan list and another change in the same batch", c.zone, c.knob);
    }
    // A copy or move names positions as they are before this batch: a position it
    // uses carries no other knob edit, and a moved one is neither copied nor moved twice.
    let others: Vec<(&str, i64)> = edits
        .renames
        .iter()
        .map(|x| (x.zone.as_str(), x.knob))
        .chain(edits.talkgroups.iter().map(|x| (x.zone.as_str(), x.knob)))
        .chain(edits.knob_systems.iter().map(|x| (x.zone.as_str(), x.knob)))
        .chain(edits.conv_knobs.iter().map(|x| (x.zone.as_str(), x.knob)))
        .chain(edits.removed_knobs.iter().map(|x| (x.zone.as_str(), x.knob)))
        .chain(edits.conv_updates.iter().map(|x| (x.zone.as_str(), x.knob)))
        .chain(edits.scan_knobs.iter().map(|x| (x.zone.as_str(), x.knob)))
        .chain(edits.digital_knobs.iter().map(|x| (x.zone.as_str(), x.knob)))
        .collect();
    let busy = |z: &str, k: i64| others.iter().any(|(a, b)| *a == z && *b == k);
    for c in &edits.knob_copies {
        if busy(&c.zone, c.knob) {
            bail!("{} knob {} is copied and changed in the same batch; write the change first", c.zone, c.knob);
        }
        let targets = edits.knob_copies.iter().filter(|x| x.to_zone == c.to_zone && x.to_knob == c.to_knob).count();
        if busy(&c.to_zone, c.to_knob) || targets > 1 || edits.knob_copies.iter().any(|x| x.zone == c.to_zone && x.knob == c.to_knob) {
            bail!("{} knob {} gets a copy and another change in the same batch", c.to_zone, c.to_knob);
        }
    }
    let moved: Vec<(&str, i64)> = edits.knob_moves.iter().flat_map(|m| [(m.zone.as_str(), m.knob), (m.to_zone.as_str(), m.to_knob)]).collect();
    for &(z, k) in &moved {
        let copied = edits.knob_copies.iter().any(|c| (c.zone == z && c.knob == k) || (c.to_zone == z && c.to_knob == k));
        if busy(z, k) || copied || moved.iter().filter(|x| **x == (z, k)).count() > 1 {
            bail!("{z} knob {k} is moved and changed in other ways in the same batch; write the move on its own first");
        }
    }
    let knob_or_system_edits = !edits.renames.is_empty()
        || !edits.talkgroups.is_empty()
        || !edits.systems.is_empty()
        || !edits.new_systems.is_empty()
        || !edits.copied_systems.is_empty()
        || !edits.deleted_systems.is_empty()
        || !edits.knob_systems.is_empty()
        || !edits.conv_knobs.is_empty()
        || !edits.scan_knobs.is_empty()
        || !edits.removed_knobs.is_empty()
        || !edits.conv_updates.is_empty()
        || !edits.digital_knobs.is_empty()
        || !edits.knob_copies.is_empty()
        || !edits.knob_moves.is_empty();
    if (edits.zone_order.is_some() || !edits.deleted_zones.is_empty()) && knob_or_system_edits {
        bail!("write zone reorders and zone deletes on their own, without knob or system changes in the same batch");
    }
    let mut files = load_backup_files(src)?;
    let db_version = manifest_field(src, "Radio DB version:").context("backup has no radio DB version")?;
    let variant = Variant::for_db_version(&db_version).with_context(|| format!("writing is not supported for {db_version} yet"))?;
    let templates = Templates::new(templates)?;
    let profile = Profile::from_backup(src)?;
    let before = Summary::read(&profile)?;

    // Zones first, so knob edits can name a new or renamed zone. Then systems
    // (their talkgroup deletes cascade into knob lists), then knob
    // assignments, then deletes, so a knob can leave a system being deleted.
    let now = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
    let mut zones = Vec::new();
    for z in &edits.new_zones {
        zones.push(g5_codeplug::zone_edit::create_zone(&profile, z, &now)?);
    }
    for r in &edits.zone_renames {
        zones.push(g5_codeplug::zone_edit::rename_zone(&profile, r, &now)?);
    }
    let mut systems = Vec::new();
    for n in &edits.new_systems {
        systems.push(g5_codeplug::system_edit::create_system(&profile, &templates, n, &now)?);
    }
    for c in &edits.copied_systems {
        systems.push(g5_codeplug::system_edit::copy_system(&profile, &templates, c, &now)?);
    }
    for e in &edits.systems {
        let ch = g5_codeplug::system_edit::apply_system_edit(&profile, &templates, e, &now)?;
        if !ch.is_empty() {
            systems.push(ch);
        }
    }
    // Removals before new knobs, so a conversion's new knob finds an empty position.
    let mut removed_knobs = Vec::new();
    for r in &edits.removed_knobs {
        removed_knobs.push(g5_codeplug::knob_remove::remove_knob(&profile, r, &now)?);
    }
    let mut knob_systems = Vec::new();
    for k in &edits.knob_systems {
        knob_systems.push(g5_codeplug::knob_system::assign_knob_system(&profile, k, &now)?);
    }
    let mut conv_updates = Vec::new();
    for u in &edits.conv_updates {
        conv_updates.push(g5_codeplug::knob_remove::update_conv_knob(&profile, &templates, u, &now)?);
    }
    let mut conv_knobs = Vec::new();
    for c in &edits.conv_knobs {
        conv_knobs.push(g5_codeplug::conv_knob::add_conv_knob(&profile, &templates, c, &now)?);
    }
    let mut digital_knobs = Vec::new();
    for d in &edits.digital_knobs {
        digital_knobs.push(g5_codeplug::digital_knob::add_digital_knob(&profile, &templates, d, &now)?);
    }
    let mut knob_copies = Vec::new();
    for c in &edits.knob_copies {
        knob_copies.push(g5_codeplug::knob_copy::copy_knob(&profile, c, &now)?);
    }
    // After conventional knobs and copies, so a scan can list channels added in this batch.
    let mut scan_knobs = Vec::new();
    for s in &edits.scan_knobs {
        scan_knobs.push(g5_codeplug::scan_knob::set_scan_knob(&profile, s, &now)?);
    }
    for id in &edits.deleted_systems {
        systems.push(g5_codeplug::system_edit::delete_system(&profile, id, &now)?);
    }
    for id in &edits.deleted_zones {
        zones.push(g5_codeplug::zone_edit::delete_zone(&profile, id, &now)?);
    }
    if let Some(order) = &edits.zone_order {
        zones.push(g5_codeplug::zone_edit::reorder_zones(&profile, order, &now)?);
    }

    let mut renames = Vec::new();
    for r in &edits.renames {
        let old = g5_codeplug::edit::rename_knob(&profile, &r.zone, r.knob, &r.name)?;
        renames.push(RenameApplied { zone: r.zone.clone(), knob: r.knob, old, new: r.name.clone() });
    }
    let mut talkgroups = Vec::new();
    for t in &edits.talkgroups {
        let list = g5_codeplug::edit::KnobTalkgroups { priority: t.priority.clone(), normal: t.normal.clone() };
        g5_codeplug::edit::set_knob_talkgroups(&profile, &t.zone, t.knob, &list)?;
        talkgroups.push(describe_talkgroup_change(&before, &edits.zone_renames, t));
    }
    let settings = g5_codeplug::settings::apply(&profile, &edits.settings, variant, &now)?;

    // Moves go last and are converted on their own: what they change on top of
    // the other edits must be a pure renumbering.
    let mut temps = TempFiles(Vec::new());
    let backup_radio = src.join(BACKUP_RADIO_DB);
    let (mid, mut experimental) = convert_temp(&profile, &templates, variant, "g5-edit", &mut temps)?;
    let diffs = table_diffs(&mid, &backup_radio)?;
    let mut knob_moves = Vec::new();
    for m in &edits.knob_moves {
        knob_moves.push(g5_codeplug::knob_copy::move_knob(&profile, m, &now)?);
    }
    let (out, move_diffs, all) = if knob_moves.is_empty() {
        (mid, Vec::new(), diffs.clone())
    } else {
        let (out, exp) = convert_temp(&profile, &templates, variant, "g5-move", &mut temps)?;
        experimental = exp;
        let (m, a) = (table_diffs(&out, &mid)?, table_diffs(&out, &backup_radio)?);
        (out, m, a)
    };
    let changed: Vec<String> = all.iter().map(|d| d.table.clone()).collect();
    let stage: Vec<String> = diffs.iter().map(|d| d.table.clone()).collect();
    let radio = fs::read(&out)?;
    drop(temps);
    let mut allowed: Vec<&str> = Vec::new();
    if !edits.renames.is_empty() {
        allowed.extend(g5_codeplug::edit::RENAME_TABLES);
    }
    if !edits.talkgroups.is_empty() || systems.iter().any(|c| c.tgs_changed + c.tgs_removed > 0) {
        allowed.extend(g5_codeplug::edit::TALKGROUP_EDIT_TABLES);
    }
    if systems.iter().any(|c| c.tgs_settings_changed > 0) {
        allowed.extend(g5_codeplug::edit::TALKGROUP_SETTINGS_TABLES);
    }
    // Creating or deleting a system no knob uses changes nothing on the radio.
    // A pure TG Scan/Monitor switch keeps the system and list (MODE_SWITCH_TABLES below).
    let assigned = knob_systems.iter().any(|k| k.list_changed);
    if systems.iter().any(|c| !c.created && !c.deleted) || assigned {
        allowed.extend(g5_codeplug::edit::SYSTEM_EDIT_TABLES);
    }
    if assigned {
        allowed.extend(g5_codeplug::edit::TALKGROUP_EDIT_TABLES);
    }
    if knob_systems.iter().any(|k| k.new_knob) {
        allowed.extend(g5_codeplug::edit::KNOB_CHANNEL_TABLES);
    }
    if !conv_knobs.is_empty() {
        allowed.extend(g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES);
    }
    if !zones.is_empty() {
        allowed.extend(g5_codeplug::edit::ZONE_TABLES);
    }
    if scan_knobs.iter().any(|s| s.new_knob) {
        allowed.extend(g5_codeplug::edit::SCAN_KNOB_TABLES);
    }
    if !scan_knobs.is_empty() {
        allowed.extend(g5_codeplug::edit::SCAN_MEMBER_TABLES);
    }
    for r in &removed_knobs {
        use g5_codeplug::knob_remove::KnobKind;
        match r.kind {
            KnobKind::Trunked => {
                allowed.extend(g5_codeplug::edit::KNOB_CHANNEL_TABLES);
                allowed.extend(g5_codeplug::edit::TALKGROUP_EDIT_TABLES);
                allowed.extend(g5_codeplug::edit::SYSTEM_EDIT_TABLES);
            }
            KnobKind::Conventional => {
                allowed.extend(g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES);
                allowed.extend(g5_codeplug::edit::SCAN_MEMBER_TABLES);
            }
            KnobKind::Digital => {
                allowed.extend(g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES);
                allowed.extend(g5_codeplug::edit::DMR_KNOB_TABLES);
                allowed.extend(g5_codeplug::edit::P25C_KNOB_TABLES);
                allowed.extend(g5_codeplug::edit::TALKGROUP_EDIT_TABLES);
            }
            KnobKind::Scan => allowed.extend(g5_codeplug::edit::SCAN_KNOB_TABLES),
        }
    }
    if !conv_updates.is_empty() {
        allowed.extend(g5_codeplug::edit::CONV_EDIT_TABLES);
    }
    for d in &digital_knobs {
        allowed.extend(g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES.iter().filter(|t| **t != "tabRecvSaList"));
        allowed.extend(if d.dmr { g5_codeplug::edit::DMR_KNOB_TABLES } else { g5_codeplug::edit::P25C_KNOB_TABLES });
        // A new conventional group renumbers every trunking talkgroup contact.
        allowed.extend(g5_codeplug::edit::TALKGROUP_EDIT_TABLES);
    }
    if knob_systems.iter().any(|k| k.mode_switched) {
        allowed.extend(g5_codeplug::edit::MODE_SWITCH_TABLES);
    }
    if zones.iter().any(|z| z.renumbers) {
        // Renumbering moves channel IDs but must not add or drop a row
        // anywhere except the zone table itself.
        allowed.extend(g5_codeplug::edit::ZONE_RENUMBER_TABLES);
        if let Some(d) = diffs.iter().find(|d| d.table != "tabZone" && d.left_rows != d.right_rows) {
            bail!("renumbering zones changed the number of rows in {} ({} -> {}); refusing", d.table, d.right_rows, d.left_rows);
        }
    }
    for c in &knob_copies {
        use g5_codeplug::knob_remove::KnobKind;
        match c.kind {
            KnobKind::Trunked => {
                allowed.extend(g5_codeplug::edit::KNOB_CHANNEL_TABLES);
                allowed.extend(g5_codeplug::edit::TRUNKED_COPY_TABLES);
            }
            KnobKind::Conventional => allowed.extend(g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES),
            KnobKind::Digital => {
                allowed.extend(g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES);
                allowed.extend(g5_codeplug::edit::DMR_KNOB_TABLES);
                allowed.extend(g5_codeplug::edit::P25C_KNOB_TABLES);
            }
            KnobKind::Scan => allowed.extend(g5_codeplug::edit::SCAN_KNOB_TABLES),
        }
    }
    for c in &settings {
        allowed.extend(g5_codeplug::settings::radio_tables(&c.key));
    }
    if let Some(t) = stage.iter().find(|t| !allowed.contains(&t.as_str())) {
        bail!("the edit unexpectedly changed {t} in the radio DB; refusing");
    }
    // A move renumbers the channels it touches, as a zone reorder does, and nothing else.
    if let Some(d) = move_diffs.iter().find(|d| d.table != "tabZone" && d.left_rows != d.right_rows) {
        bail!("moving knobs changed the number of rows in {} ({} -> {}); refusing", d.table, d.right_rows, d.left_rows);
    }
    let renumbered = |t: &str| g5_codeplug::edit::ZONE_RENUMBER_TABLES.contains(&t) || g5_codeplug::edit::DIGITAL_RENUMBER_TABLES.contains(&t);
    if let Some(d) = move_diffs.iter().find(|d| !renumbered(&d.table)) {
        bail!("moving knobs unexpectedly changed {} in the radio DB; refusing", d.table);
    }

    let replacements = [
        (PC_DB, fs::read(&profile.db_path)?),
        (RADIO_DB, radio),
        (PC_ZIP, g5_codeplug::edit::repack_zip(&src.join(BACKUP_PROFILE_ZIP), &profile, &now)?),
    ];
    for (target, bytes) in replacements {
        files.iter_mut().find(|(p, _)| *p == target).unwrap().1 = bytes;
    }
    for target in [RADIO_DB, PC_ZIP] {
        let md5 = hex_upper(&Md5::digest(&files.iter().find(|(p, _)| *p == target).unwrap().1)).into_bytes();
        let sidecar = format!("{target}.md5");
        files.iter_mut().find(|(p, _)| *p == sidecar).unwrap().1 = md5;
    }
    if renames.is_empty() && talkgroups.is_empty() && systems.is_empty() && knob_systems.is_empty() && conv_knobs.is_empty() && zones.is_empty() && scan_knobs.is_empty() && removed_knobs.is_empty() && conv_updates.is_empty() && digital_knobs.is_empty() && knob_copies.is_empty() && knob_moves.is_empty() && settings.is_empty() {
        bail!("no changes to write");
    }
    Ok(WriteSet { files, renames, talkgroups, systems, knob_systems, conv_knobs, zones, scan_knobs, removed_knobs, conv_updates, digital_knobs, knob_copies, knob_moves, settings, experimental, changed_tables: changed, require_unchanged_pager: true })
}

fn describe_talkgroup_change(before: &Summary, renames: &[g5_codeplug::zone_edit::ZoneRename], t: &KnobTalkgroupEdit) -> TalkgroupChange {
    // The edit names the zone as it is after this batch's zone renames.
    let zone_id = renames.iter().find(|r| r.name.trim() == t.zone).map(|r| r.zone_id.as_str());
    let knob = before
        .zones
        .iter()
        .find(|z| zone_id.map_or(z.name == t.zone, |id| z.id == id))
        .and_then(|z| z.knobs.iter().find(|k| k.index == t.knob));
    let system = knob.and_then(|k| before.systems.iter().find(|s| Some(&s.id) == k.system_id.as_ref()));
    let lookup = |gid: &str| {
        system
            .and_then(|s| s.talkgroup_list.iter().find(|g| g.group_id == gid))
            .map(|g| TalkgroupRef { tgid: g.tgid, name: g.name.clone() })
            .unwrap_or(TalkgroupRef { tgid: 0, name: gid.to_string() })
    };
    let old: Vec<(String, bool)> = knob.map(|k| k.talkgroup_list.iter().map(|g| (g.group_id.clone(), g.priority)).collect()).unwrap_or_default();
    let new: Vec<(String, bool)> = t.priority.iter().map(|g| (g.clone(), true)).chain(t.normal.iter().map(|g| (g.clone(), false))).collect();
    let has = |list: &[(String, bool)], g: &str| list.iter().any(|(x, _)| x == g);
    TalkgroupChange {
        zone: t.zone.clone(),
        knob: t.knob,
        knob_name: knob.map(|k| k.name.clone()).unwrap_or_default(),
        added: new.iter().filter(|(g, _)| !has(&old, g)).map(|(g, _)| lookup(g)).collect(),
        removed: old.iter().filter(|(g, _)| !has(&new, g)).map(|(g, _)| lookup(g)).collect(),
        priority_changed: {
            let pri = |l: &[(String, bool)]| l.iter().filter(|(_, p)| *p).map(|(g, _)| g.clone()).collect::<Vec<_>>();
            pri(&old) != pri(&new)
        },
        count: new.len(),
    }
}

/// Temporary radio DBs, removed when dropped.
struct TempFiles(Vec<std::path::PathBuf>);

impl Drop for TempFiles {
    fn drop(&mut self) {
        for f in &self.0 {
            let _ = fs::remove_file(f);
        }
    }
}

/// Convert the working copy to a temporary radio DB, refusing unverified
/// features. Returns its path and the experimental features it uses.
fn convert_temp(profile: &Profile, templates: &Templates, variant: Variant, prefix: &str, temps: &mut TempFiles) -> Result<(std::path::PathBuf, Vec<String>)> {
    let out = tempfile_path(prefix);
    temps.0.push(out.clone());
    let report = g5_codeplug::convert141::convert(profile, templates, variant, &out)?;
    if !report.unverified.is_empty() {
        bail!("profile uses features not yet verified for writing: {}", report.unverified.join(", "));
    }
    Ok((out, report.experimental))
}

/// Tables of radio DB `ours` that differ from `theirs`.
fn table_diffs(ours: &Path, theirs: &Path) -> Result<Vec<g5_codeplug::compare::TableDiff>> {
    let a = sqlite2::Connection::open(ours)?;
    let b = sqlite2::Connection::open(theirs)?;
    Ok(compare(&a, &b)?.into_iter().filter(|d| d.status != TableStatus::Same).collect())
}

pub(crate) fn tempfile_path(prefix: &str) -> std::path::PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}-{}-{n}.db", std::process::id()))
}

/// PPS's time zone string: POSIX sign convention (UTC-5 is `GMT+05:00:00`).
pub fn pps_zone_and_time() -> (String, String) {
    let now = jiff::Zoned::now();
    let secs = now.offset().seconds();
    let a = secs.unsigned_abs();
    let zone = format!("GMT{}{:02}:{:02}:{:02}", if secs < 0 { '+' } else { '-' }, a / 3600, a / 60 % 60, a % 60);
    (zone, now.strftime("%Y%m%d%H%M%S").to_string())
}

/// What the write checks compare: a pager's model, serial and radio DB version, as the
/// handshake reports them or as a backup's MANIFEST.txt recorded them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PagerId {
    pub model: String,
    pub serial: String,
    pub db_version: String,
}

impl PagerId {
    pub fn of(info: &DeviceInfo) -> Self {
        Self { model: info.model_number.clone().unwrap_or_default(), serial: info.serial_number.clone(), db_version: info.u3db_version.clone() }
    }

    pub fn of_backup(src: &Path) -> Result<Self> {
        let field = |label: &str| manifest_field(src, label).with_context(|| format!("backup MANIFEST.txt has no {label} line"));
        Ok(Self { model: field("Model:")?, serial: field("Serial:")?, db_version: field("Radio DB version:")? })
    }
}

/// The source backup must describe this exact pager.
pub fn check_backup_matches(info: &DeviceInfo, src: &Path) -> Result<()> {
    check_pager_matches(&PagerId::of(info), src)
}

/// [`check_backup_matches`] for a pager described by its [`PagerId`].
pub fn check_pager_matches(pager: &PagerId, src: &Path) -> Result<()> {
    let got = PagerId::of_backup(src)?;
    for (label, got, have) in [("Model", &got.model, &pager.model), ("Serial", &got.serial, &pager.serial), ("Radio DB version", &got.db_version, &pager.db_version)] {
        if got != have {
            bail!("this backup's {label} is {got:?} but the attached pager reports {have:?}");
        }
    }
    Ok(())
}

/// Write `set` to the attached pager, safely. See the module docs.
pub fn program(
    addr: SocketAddr,
    password: Option<&str>,
    src: &Path,
    set: &WriteSet,
    backups_root: &Path,
    progress: &mut dyn FnMut(Progress),
) -> Result<WriteResult> {
    progress(Progress::step("Connecting"));
    let (mut client, info) = connect(addr)?;
    let result = program_on(&mut client, &info, addr, password, src, set, backups_root, progress);
    client.bye();
    result
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn program_on(
    client: &mut Pager,
    info: &DeviceInfo,
    addr: SocketAddr,
    password: Option<&str>,
    src: &Path,
    set: &WriteSet,
    backups_root: &Path,
    progress: &mut dyn FnMut(Progress),
) -> Result<WriteResult> {
    check_backup_matches(info, src)?;
    authorize_read(client, password)?;
    progress(Progress::step("Safety backup"));
    let safety = backup(client, info, backups_root, progress)?;
    if set.require_unchanged_pager {
        let source = load_backup_files(src)?;
        let current = load_backup_files(&safety.path)?;
        if source.iter().zip(&current).any(|(a, b)| a.1 != b.1) {
            bail!("the pager's profile changed since this backup was taken; open the newest backup ({}) and edit that instead", safety.id);
        }
    }

    authorize_write(client, password)?;
    let (zone, now) = pps_zone_and_time();
    let refs: Vec<(&str, &[u8])> = set.files.iter().map(|(p, d)| (*p, d.as_slice())).collect();
    let total: usize = refs.iter().map(|(_, d)| d.len()).sum();
    let offsets: Vec<usize> = refs.iter().scan(0, |acc, (_, d)| { let o = *acc; *acc += d.len(); Some(o) }).collect();
    client
        .program_files_with_progress(&refs, Some((&zone, &now)), &mut |i, sent, _| {
            progress(Progress::of(format!("Writing {}", refs[i].0), offsets[i] + sent, total));
        })
        .context("programming")?;

    progress(Progress::step("Pager is rebooting to apply the profile"));
    let back = read_back_after_commit(addr, password)?;
    let mut verified = Vec::new();
    for ((path, data), got) in set.files.iter().zip(&back) {
        if data != got {
            bail!("read-back mismatch on {path}; the safety backup {} has the previous files", safety.id);
        }
        verified.push(path.to_string());
    }
    progress(Progress::step("Verified"));
    // Back up the pager as written, so the next change can start from it.
    progress(Progress::step("Backing up the new profile"));
    let new_backup = (|| -> Result<BackupInfo> {
        let (mut c, info) = connect(addr)?;
        let r = authorize_read(&mut c, password).and_then(|()| backup(&mut c, &info, backups_root, progress));
        c.bye();
        r
    })()
    .map_err(|e| tracing::warn!("post-write backup failed: {e:#}"))
    .ok();
    Ok(WriteResult { safety_backup: safety, verified, new_backup })
}

/// The pager reboots a few seconds after a commit; reconnect and read the profile files.
fn read_back_after_commit(addr: SocketAddr, password: Option<&str>) -> Result<Vec<Vec<u8>>> {
    std::thread::sleep(Duration::from_secs(10));
    let mut last = None;
    for attempt in 1..=12 {
        let result = (|| -> Result<Vec<Vec<u8>>> {
            let (mut c, _) = connect(addr)?;
            authorize_read(&mut c, password)?;
            let files = c.read_files(PROFILE_FILES)?;
            c.bye();
            Ok(files)
        })();
        match result {
            Ok(f) => return Ok(f),
            Err(e) => {
                tracing::debug!("read-back attempt {attempt} failed: {e:#}");
                last = Some(e);
                std::thread::sleep(Duration::from_secs(5));
            }
        }
    }
    Err(last.unwrap()).context("could not read the profile back after programming")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A backup folder whose radio DB sidecar holds `sidecar`, with "Files read"
    /// recording `read_md5` for the radio DB.
    fn backup_with(sidecar: &str, read_md5: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let db = [sqlite2::FILE_HEADER, b"radio".as_slice()].concat();
        let pc = [sqlite2::FILE_HEADER, b"profile".as_slice()].concat();
        let zip = b"zip".to_vec();
        let zip_md5 = hex_upper(&Md5::digest(&zip));
        for (p, data) in [
            (PC_DB, pc),
            (RADIO_DB, db),
            ("/defpara/Applications/PPS/u3db.db.md5", sidecar.as_bytes().to_vec()),
            (PC_ZIP, zip),
            ("/defpara/Applications/PPS/u3db_PC.zip.md5", zip_md5.into_bytes()),
        ] {
            let local = dir.path().join(rel(p));
            fs::create_dir_all(local.parent().unwrap()).unwrap();
            fs::write(local, data).unwrap();
        }
        let manifest = format!("Radio DB version: DBV1.41e\n\nFiles read:\n  {:>10}  {read_md5}  {RADIO_DB}\n\nSidecar checks:\n  {RADIO_DB}: MISMATCH\n", 21);
        fs::write(dir.path().join("MANIFEST.txt"), manifest).unwrap();
        dir
    }

    fn radio_md5() -> String {
        hex_upper(&Md5::digest([sqlite2::FILE_HEADER, b"radio".as_slice()].concat()))
    }

    #[test]
    fn stale_radio_db_sidecar_is_corrected_when_the_pager_had_it() {
        let dir = backup_with("4B53DA1452701F19B955AD7E1E3AF55A", &radio_md5());
        let files = load_backup_files(dir.path()).unwrap();
        let sidecar = &files.iter().find(|(p, _)| p.ends_with("u3db.db.md5")).unwrap().1;
        assert_eq!(String::from_utf8_lossy(sidecar), radio_md5());
    }

    #[test]
    fn radio_db_changed_after_backup_is_refused() {
        let dir = backup_with("4B53DA1452701F19B955AD7E1E3AF55A", "00000000000000000000000000000000");
        let err = load_backup_files(dir.path()).unwrap_err().to_string();
        assert!(err.contains("does not match its .md5"), "{err}");
    }
}
