//! Editing an existing P25 trunking system: properties, sites, control
//! channels and talkgroups. Behavior follows `notes/system-editing.md`
//! (PPS's editors), with two deliberate differences: each database is
//! written in one transaction, and deleting a talkgroup also removes it from
//! this system's knob lists instead of leaving orphans.
//!
//! The caller passes the complete desired state of the system; rows are
//! matched to existing ones by ID, so unchanged rows are rewritten verbatim.
//!
//! Creating and deleting systems follows `notes/system-creation.md`: unlike
//! PPS, a new system is written whole (no placeholder control channel, no
//! orphan file on failure), and a system a knob uses can't be deleted.

use std::collections::{HashMap, HashSet};
use std::fs;

use serde::{Deserialize, Serialize};
use sqlite2::Connection;

use crate::row::{Row, load_table, sel};
use crate::{Error, Profile, Templates};

pub const MAX_SITES: usize = 256;
pub const MAX_CONTROL_CHANNELS: usize = 256;
pub const MAX_TALKGROUPS: usize = 8192;
pub const MAX_NAME: usize = 14;
pub const MAX_CC_NOTE: usize = 64;
/// PPS disables "add system" at this many systems in a profile.
pub const MAX_SYSTEMS: usize = 1024;
/// GroupID of the shared "All" talkgroup (same in every system).
pub const ALL_GROUP_ID: &str = "0000000000000000";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteEdit {
    /// Existing row ID (`tabP25TrunkingSite.ID`), or None for a new site.
    pub id: Option<String>,
    pub name: String,
    pub rfss: i64,
    pub site: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlChannelEdit {
    /// Existing row ID (`tabP25TrunkingControlCH.ID`), or None for a new channel.
    pub id: Option<i64>,
    pub rx_hz: i64,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalkgroupEdit {
    /// Existing GroupID, or a new lowercase GUID chosen by the caller (so knob
    /// edits in the same batch can refer to it).
    pub group_id: String,
    pub tgid: i64,
    pub name: String,
    #[serde(default)]
    pub note: String,
    /// Added by a RadioReference import: a new row gets the defaults of PPS's
    /// importer (silent, no voice storage) instead of its talkgroup editor's.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub imported: bool,
    /// The settings below are `None` to keep the stored value (or, for a new
    /// row, the editor's or importer's default). `AlertMode`: 1 tone, 2 vibrate,
    /// 3 none, 4 tone + vibrate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert_mode: Option<i64>,
    /// `AlertToneID`, one of `ALERT_TONES`; a custom tone ID only as already stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alert_tone: Option<String>,
    /// `RecordFlag` (PPS's Voice Storage checkbox: 1 on, 0 off).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_storage: Option<bool>,
    /// `BackgroundColor`: 0 off, 1 green, 2 blue, 3 cyan, 4 red, 5 yellow, 6 magenta, 7 white.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<i64>,
}

/// Alert tones PPS's talkgroup editor offers besides custom ones
/// (`View/View.Common/NameValueVModel.cs:175-201`): none, default, chirp, Tone 1..8.
pub const ALERT_TONES: &[&str] = &["0", "9999", "10000", "1", "2", "3", "4", "5", "6", "7", "8"];

fn num(r: &Row, col: &str) -> Option<i64> {
    r.s(col).trim().parse().ok()
}

/// A setting may keep whatever the row already holds; a new value must be one PPS offers.
fn check_settings(t: &TalkgroupEdit, old: Option<&Row>) -> Result<(), Error> {
    let what = &t.name;
    if let Some(m) = t.alert_mode
        && old.and_then(|r| num(r, "AlertMode")) != Some(m)
        && !(1..=4).contains(&m)
    {
        return Err(Error::Invalid(format!("talkgroup {what:?}: alert mode must be 1 to 4")));
    }
    if let Some(tone) = &t.alert_tone
        && old.map(|r| r.s("AlertToneID")) != Some(tone.as_str())
        && !ALERT_TONES.contains(&tone.as_str())
    {
        return Err(Error::Invalid(format!("talkgroup {what:?}: alert tone {tone:?} isn't a built-in tone")));
    }
    if let Some(bg) = t.background
        && old.and_then(|r| num(r, "BackgroundColor")) != Some(bg)
        && !(0..=7).contains(&bg)
    {
        return Err(Error::Invalid(format!("talkgroup {what:?}: background colour must be 0 to 7")));
    }
    Ok(())
}

/// Write `t`'s explicit settings into `v`; true if any differs from `old`.
fn apply_settings(v: &mut [String], cols: &[String], t: &TalkgroupEdit, old: Option<&Row>) -> bool {
    let mut changed = false;
    if let Some(m) = t.alert_mode
        && old.and_then(|r| num(r, "AlertMode")) != Some(m)
    {
        set(v, cols, "AlertMode", m.to_string());
        changed = true;
    }
    if let Some(tone) = &t.alert_tone
        && old.map(|r| r.s("AlertToneID")) != Some(tone.as_str())
    {
        set(v, cols, "AlertToneID", tone.clone());
        changed = true;
    }
    if let Some(on) = t.voice_storage
        && old.and_then(|r| num(r, "RecordFlag")).map(|f| f > 0) != Some(on)
    {
        set(v, cols, "RecordFlag", if on { "1" } else { "0" });
        changed = true;
    }
    if let Some(bg) = t.background
        && old.and_then(|r| num(r, "BackgroundColor")) != Some(bg)
    {
        set(v, cols, "BackgroundColor", bg.to_string());
        changed = true;
    }
    changed
}

/// Complete desired state of one system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemEdit {
    /// `P25TrunkingID` of the system.
    pub system_id: String,
    pub name: String,
    pub wacn: i64,
    pub sysid: i64,
    /// In the desired order (site order reaches the pager).
    pub sites: Vec<SiteEdit>,
    /// Stored in ID order; new channels are appended with fresh IDs.
    pub control_channels: Vec<ControlChannelEdit>,
    /// Existing talkgroups keep their stored order; new ones are appended.
    pub talkgroups: Vec<TalkgroupEdit>,
}

/// A system to add to the profile: its complete state plus the band it lives in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewSystem {
    /// One of the pager model's 2-letter band codes (`profile_bands`).
    pub band: String,
    /// `system_id` is a new lowercase GUID chosen by the caller.
    pub system: SystemEdit,
}

/// A system copied from another backup's profile, PPS's "Add System from Resources"
/// (notes/data-model.md 5.2): the source's system DB file with a new ID, so its sites,
/// control channels and talkgroups keep every setting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemCopy {
    /// Backup folder holding the system (the app sends a backup ID; the backend resolves it).
    pub backup: std::path::PathBuf,
    /// `P25TrunkingID` of the system in that backup.
    pub source_id: String,
    /// Desired state: `system_id` is a new lowercase GUID, row IDs are the source's.
    pub system: SystemEdit,
}

/// What an edit did, for display and for the write guard.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemChange {
    pub system_id: String,
    pub name: String,
    /// The system is new (the counts below are its contents).
    pub created: bool,
    /// The system was removed from the profile.
    pub deleted: bool,
    pub properties_changed: Vec<String>,
    pub sites_added: usize,
    pub sites_removed: usize,
    pub sites_changed: usize,
    pub sites_reordered: bool,
    pub ccs_added: usize,
    pub ccs_removed: usize,
    pub ccs_changed: usize,
    pub tgs_added: usize,
    pub tgs_removed: usize,
    pub tgs_changed: usize,
    /// Existing talkgroups whose alert mode, alert tone, voice storage or
    /// background colour changed (counted apart from `tgs_changed`, which
    /// covers name, TGID and note, because they touch fewer radio tables).
    pub tgs_settings_changed: usize,
    /// Knob receive-list entries removed because their talkgroup was deleted.
    pub knob_entries_removed: usize,
    /// Knobs left with no talkgroups by the cascade (programming would fail).
    pub empty_knobs: Vec<String>,
}

impl SystemChange {
    pub fn is_empty(&self) -> bool {
        !self.created
            && !self.deleted
            && self.properties_changed.is_empty()
            && self.sites_added + self.sites_removed + self.sites_changed + self.ccs_added + self.ccs_removed + self.ccs_changed + self.tgs_added + self.tgs_removed + self.tgs_changed + self.tgs_settings_changed == 0
            && !self.sites_reordered
    }
}

/// Inclusive Rx ranges (Hz, whole-MHz bounds) for a band-type code like `SX` or `SXVX`.
pub fn band_ranges(templates: &Templates, code: &str) -> Result<Vec<(i64, i64)>, Error> {
    if code.is_empty() {
        return Ok(vec![(136_000_000, 870_000_000)]);
    }
    let data = Connection::open(&templates.data_db())?;
    let info = load_table(&data, "tabRFBandTypeInfo");
    let ranges = load_table(&data, "tabRFBandRanges");
    let mut out = Vec::new();
    for part in code.as_bytes().chunks(2) {
        let c = String::from_utf8_lossy(part);
        let Some(t) = info.iter().find(|r| r.s("Code") == c) else {
            return Err(Error::Invalid(format!("unknown band code {c}")));
        };
        for r in ranges.iter().filter(|r| r.s("BandTypeID") == t.s("ID") && r.s("RxTx") != "1") {
            out.push((r.i("MinFreq") / 1_000_000 * 1_000_000, r.i("MaxFreq") / 1_000_000 * 1_000_000));
        }
    }
    Ok(out)
}

/// A band a system can be created in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Band {
    pub code: String,
    pub name: String,
    /// e.g. `(763~776, 851~870 MHz)`.
    pub comment: String,
    pub ranges: Vec<(i64, i64)>,
}

/// Band codes in a model number like `G5D66BE-SXVXENB702UNI`: positions 8-9
/// and 10-11, `XX` meaning none (data-model 4.2).
pub fn model_band_codes(model: &str) -> Vec<String> {
    [8..10, 10..12].into_iter().filter_map(|r| model.get(r)).filter(|c| *c != "XX" && c.bytes().all(|b| b.is_ascii_uppercase())).map(String::from).collect()
}

/// The bands new systems can use in this profile (its model's first and second band).
pub fn profile_bands(p: &Profile, templates: &Templates) -> Result<Vec<Band>, Error> {
    let model = load_table(&p.db, "tabModel").first().map(|r| r.s("Model").to_string()).unwrap_or_default();
    let data = Connection::open(&templates.data_db())?;
    let info = load_table(&data, "tabRFBandTypeInfo");
    let mut out = Vec::new();
    for code in model_band_codes(&model) {
        let Some(r) = info.iter().find(|r| r.s("Code") == code) else { continue };
        out.push(Band { ranges: band_ranges(templates, &code)?, name: r.s("Name").into(), comment: r.s("Comment").into(), code });
    }
    Ok(out)
}

pub(crate) fn is_guid(s: &str) -> bool {
    s.len() == 36
        && s.char_indices().all(|(i, c)| if [8, 13, 18, 23].contains(&i) { c == '-' } else { c.is_ascii_digit() || ('a'..='f').contains(&c) })
}

pub(crate) fn check_name(what: &str, name: &str) -> Result<(), Error> {
    if name.trim().is_empty() {
        return Err(Error::Invalid(format!("{what} name can't be blank")));
    }
    if name.chars().count() > MAX_NAME {
        return Err(Error::Invalid(format!("{what} name {name:?} is longer than {MAX_NAME} characters")));
    }
    if !name.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        return Err(Error::Invalid(format!("{what} name {name:?} must be printable ASCII")));
    }
    Ok(())
}

/// Smallest positive integer not already used (PPS compares the values as strings).
fn next_id(used: &HashSet<String>) -> i64 {
    (1..).find(|n: &i64| !used.contains(&n.to_string())).unwrap()
}

/// `DELETE FROM t; INSERT ...` with every value as quoted text, columns in table order.
fn rewrite(db: &Connection, table: &str, rows: &[Vec<String>]) -> Result<(), Error> {
    db.execute(&format!("DELETE FROM {table}"), &[])?;
    for r in rows {
        let ph = vec!["?"; r.len()].join(",");
        let params: Vec<Option<&str>> = r.iter().map(|v| Some(v.as_str())).collect();
        db.execute(&format!("INSERT INTO {table} VALUES({ph})"), &params)?;
    }
    Ok(())
}

fn row_values(r: &Row, cols: &[String]) -> Vec<String> {
    cols.iter().map(|c| r.s(c).to_string()).collect()
}

fn set(vals: &mut [String], cols: &[String], col: &str, v: impl Into<String>) {
    if let Some(i) = cols.iter().position(|c| c.eq_ignore_ascii_case(col)) {
        vals[i] = v.into();
    }
}

/// Apply `edit` to a working copy of the profile. `templates` supplies band
/// ranges; `now` is the local time for `tabOrganization_New.FinishDate`
/// (`yyyy-MM-dd HH:mm:ss`).
pub fn apply_system_edit(p: &Profile, templates: &Templates, edit: &SystemEdit, now: &str) -> Result<SystemChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let index = load_table(&p.db, "tabP25TrunkingSecurity");
    let idx = index
        .iter()
        .find(|r| r.s("P25TrunkingID") == edit.system_id)
        .ok_or_else(|| Error::Invalid(format!("system {} is not in this profile", edit.system_id)))?;
    let sys = p.system_db(&edit.system_id)?;
    let sys_row = load_table(&sys, "tabP25TrunkingSystem").into_iter().next().ok_or_else(|| Error::Invalid("system DB has no system row".into()))?;
    let full_spectrum = sys_row.s("FullSpectrumScanFlag") == "1";

    // ---------- validate ----------
    check_name("System", &edit.name)?;
    if index.iter().any(|r| r.s("P25TrunkingID") != edit.system_id && r.s("SystemName").trim() == edit.name.trim()) {
        return Err(Error::Invalid(format!("another system is already called {:?}", edit.name)));
    }
    if !(0..=0xFFFFF).contains(&edit.wacn) {
        return Err(Error::Invalid("WACN must be 0 to FFFFF".into()));
    }
    if !(0..=0xFFF).contains(&edit.sysid) {
        return Err(Error::Invalid("System ID must be 0 to FFF".into()));
    }

    if edit.sites.is_empty() || edit.sites.len() > MAX_SITES {
        return Err(Error::Invalid(format!("a system needs 1 to {MAX_SITES} sites")));
    }
    let old_sites = load_table(&sys, "tabP25TrunkingSite");
    let mut names = HashSet::new();
    let mut pairs = HashSet::new();
    for s in &edit.sites {
        check_name("Site", &s.name)?;
        if !names.insert(s.name.clone()) {
            return Err(Error::Invalid(format!("two sites are called {:?}", s.name)));
        }
        if !(0..=255).contains(&s.rfss) || !(0..=255).contains(&s.site) {
            return Err(Error::Invalid(format!("site {:?}: RFSS and Site ID must be 0 to FF", s.name)));
        }
        if !pairs.insert((s.rfss, s.site)) {
            return Err(Error::Invalid(format!("two sites use RFSS {:X} / Site {:X}", s.rfss, s.site)));
        }
        if let Some(id) = &s.id
            && !old_sites.iter().any(|r| r.s("ID") == id)
        {
            return Err(Error::Invalid(format!("unknown site {id}")));
        }
    }

    if edit.control_channels.len() > MAX_CONTROL_CHANNELS {
        return Err(Error::Invalid(format!("at most {MAX_CONTROL_CHANNELS} control channels")));
    }
    if edit.control_channels.is_empty() && !full_spectrum {
        return Err(Error::Invalid("a system needs at least one control channel (or full spectrum scan)".into()));
    }
    let old_ccs = load_table(&sys, "tabP25TrunkingControlCH");
    let ranges = band_ranges(templates, idx.s("BandTypeCode"))?;
    let mut freqs = HashSet::new();
    for c in &edit.control_channels {
        if !ranges.iter().any(|(lo, hi)| (*lo..=*hi).contains(&c.rx_hz)) {
            return Err(Error::Invalid(format!("{:.5} MHz is outside this pager's bands", c.rx_hz as f64 / 1e6)));
        }
        if !freqs.insert(c.rx_hz) {
            return Err(Error::Invalid(format!("{:.5} MHz is listed twice", c.rx_hz as f64 / 1e6)));
        }
        if c.note.chars().count() > MAX_CC_NOTE {
            return Err(Error::Invalid(format!("control channel notes are at most {MAX_CC_NOTE} characters")));
        }
        if let Some(id) = c.id
            && !old_ccs.iter().any(|r| r.s("ID") == id.to_string())
        {
            return Err(Error::Invalid(format!("unknown control channel {id}")));
        }
    }

    if edit.talkgroups.is_empty() || edit.talkgroups.len() > MAX_TALKGROUPS {
        return Err(Error::Invalid(format!("a system needs 1 to {MAX_TALKGROUPS} talkgroups")));
    }
    let old_tgs = load_table(&sys, "tabGroup_New");
    let old_ids: HashSet<&str> = old_tgs.iter().map(|r| r.s("GroupID")).collect();
    let mut ids = HashSet::new();
    for t in &edit.talkgroups {
        // Rows the user didn't touch are left alone even if they predate these rules.
        let unchanged = old_tgs.iter().any(|r| {
            r.s("GroupID") == t.group_id && r.s("GroupName") == t.name && r.s("P25TAddr") == t.tgid.to_string() && r.s("Note") == t.note
        });
        if !unchanged {
            check_name("Talkgroup", &t.name)?;
            if !(1..=65535).contains(&t.tgid) {
                return Err(Error::Invalid(format!("talkgroup {:?}: TGID must be 1 to 65535", t.name)));
            }
        }
        if !ids.insert(t.group_id.as_str()) {
            return Err(Error::Invalid(format!("talkgroup {} is listed twice", t.group_id)));
        }
        if !old_ids.contains(t.group_id.as_str()) && !is_guid(&t.group_id) {
            return Err(Error::Invalid(format!("new talkgroup IDs must be lowercase GUIDs, got {}", t.group_id)));
        }
        check_settings(t, old_tgs.iter().find(|r| r.s("GroupID") == t.group_id))?;
    }

    let mut change = SystemChange { system_id: edit.system_id.clone(), name: edit.name.clone(), ..Default::default() };
    for (label, old, new) in [
        ("name", sys_row.s("Name").to_string(), edit.name.clone()),
        ("WACN", sys_row.s("WacnID").to_string(), edit.wacn.to_string()),
        ("System ID", sys_row.s("SystemID").to_string(), edit.sysid.to_string()),
    ] {
        if old != new {
            change.properties_changed.push(label.to_string());
        }
    }

    // ---------- system DB ----------
    sys.execute_batch("BEGIN;")?;
    let r = (|| -> Result<(), Error> {
        sys.execute(
            "UPDATE tabP25TrunkingSystem SET Name = ?, WacnID = ?, SystemID = ?",
            &[Some(&edit.name), Some(&edit.wacn.to_string()), Some(&edit.sysid.to_string())],
        )?;

        // Sites in the desired order.
        let cols = sys.table_columns("tabP25TrunkingSite")?;
        let mut used: HashSet<String> = old_sites.iter().map(|r| r.s("ID").to_string()).collect();
        let mut rows = Vec::new();
        for s in &edit.sites {
            let mut vals = match s.id.as_ref().and_then(|id| old_sites.iter().find(|r| r.s("ID") == id)) {
                Some(old) => {
                    let v = row_values(old, &cols);
                    if old.s("Name") != s.name || old.s("RFSSID") != s.rfss.to_string() || old.s("SiteID") != s.site.to_string() {
                        change.sites_changed += 1;
                    }
                    v
                }
                None => {
                    let id = next_id(&used).to_string();
                    used.insert(id.clone());
                    change.sites_added += 1;
                    let mut v = vec![String::new(); cols.len()];
                    set(&mut v, &cols, "ID", id);
                    v
                }
            };
            set(&mut vals, &cols, "Name", s.name.clone());
            set(&mut vals, &cols, "RFSSID", s.rfss.to_string());
            set(&mut vals, &cols, "SiteID", s.site.to_string());
            rows.push(vals);
        }
        let kept: Vec<&str> = edit.sites.iter().filter_map(|s| s.id.as_deref()).collect();
        change.sites_removed = old_sites.iter().filter(|r| !kept.contains(&r.s("ID"))).count();
        let old_order: Vec<&str> = old_sites.iter().map(|r| r.s("ID")).filter(|id| kept.contains(id)).collect();
        // Reordered if existing sites changed relative order, or a new site sits before an existing one.
        let last_existing = edit.sites.iter().rposition(|s| s.id.is_some());
        let new_before_existing = edit.sites.iter().position(|s| s.id.is_none()).zip(last_existing).is_some_and(|(n, e)| n < e);
        change.sites_reordered = old_order != kept || new_before_existing;
        rewrite(&sys, "tabP25TrunkingSite", &rows)?;

        // Control channels: existing rows updated in place, new rows appended, ID order kept.
        let cols = sys.table_columns("tabP25TrunkingControlCH")?;
        let mut used: HashSet<String> = old_ccs.iter().map(|r| r.s("ID").to_string()).collect();
        let mut rows: Vec<(i64, Vec<String>)> = Vec::new();
        for c in &edit.control_channels {
            let vals = match c.id.and_then(|id| old_ccs.iter().find(|r| r.s("ID") == id.to_string())) {
                Some(old) => {
                    let mut v = row_values(old, &cols);
                    if old.s("RxFreq") != c.rx_hz.to_string() || old.s("Note") != c.note {
                        change.ccs_changed += 1;
                    }
                    set(&mut v, &cols, "RxFreq", c.rx_hz.to_string());
                    set(&mut v, &cols, "Note", c.note.clone());
                    v
                }
                None => {
                    let id = next_id(&used);
                    used.insert(id.to_string());
                    change.ccs_added += 1;
                    let mut v = vec![String::new(); cols.len()];
                    set(&mut v, &cols, "ID", id.to_string());
                    set(&mut v, &cols, "RxFreq", c.rx_hz.to_string());
                    set(&mut v, &cols, "RxBandWidth", "1");
                    set(&mut v, &cols, "TxBandWidth", "1");
                    set(&mut v, &cols, "Note", c.note.clone());
                    v
                }
            };
            let id = vals[cols.iter().position(|c| c == "ID").unwrap()].parse().unwrap_or(0);
            rows.push((id, vals));
        }
        rows.sort_by_key(|(id, _)| *id);
        let kept_cc: HashSet<String> = rows.iter().map(|(id, _)| id.to_string()).collect();
        change.ccs_removed = old_ccs.iter().filter(|r| !kept_cc.contains(r.s("ID"))).count();
        rewrite(&sys, "tabP25TrunkingControlCH", &rows.into_iter().map(|(_, v)| v).collect::<Vec<_>>())?;

        // Links to deleted sites or channels go away; the rest keep their order.
        let site_ids: HashSet<String> = sys.query("SELECT ID FROM tabP25TrunkingSite", &[])?.rows.into_iter().filter_map(|mut r| r[0].take()).collect();
        let links = load_table(&sys, "tabP25TrunkingSiteControlCH");
        let lcols = sys.table_columns("tabP25TrunkingSiteControlCH")?;
        let keep: Vec<Vec<String>> = links
            .iter()
            .filter(|l| site_ids.contains(l.s("SiteID")) && kept_cc.contains(l.s("ControlChannelID")))
            .map(|l| row_values(l, &lcols))
            .collect();
        if keep.len() != links.len() {
            rewrite(&sys, "tabP25TrunkingSiteControlCH", &keep)?;
        }

        // Talkgroups: stored order for existing rows, new rows appended.
        let cols = sys.table_columns("tabGroup_New")?;
        let wanted: HashMap<&str, &TalkgroupEdit> = edit.talkgroups.iter().map(|t| (t.group_id.as_str(), t)).collect();
        let mut used_no: HashSet<String> = old_tgs.iter().map(|r| r.s("GroupNo").to_string()).collect();
        let first_key = load_table(&p.db, "tabEncryptionKeyList").first().map(|r| r.s("ID").to_string()).unwrap_or_else(|| "000000000000".into());
        let mut rows = Vec::new();
        for old in &old_tgs {
            let Some(t) = wanted.get(old.s("GroupID")) else {
                change.tgs_removed += 1;
                continue;
            };
            let mut v = row_values(old, &cols);
            if old.s("GroupName") != t.name || old.s("P25TAddr") != t.tgid.to_string() || old.s("Note") != t.note {
                change.tgs_changed += 1;
            }
            set(&mut v, &cols, "GroupName", t.name.clone());
            set(&mut v, &cols, "P25TAddr", t.tgid.to_string());
            set(&mut v, &cols, "Note", t.note.clone());
            if apply_settings(&mut v, &cols, t, Some(old)) {
                change.tgs_settings_changed += 1;
            }
            rows.push(v);
        }
        for t in edit.talkgroups.iter().filter(|t| !old_ids.contains(t.group_id.as_str())) {
            change.tgs_added += 1;
            let no = next_id(&used_no).to_string();
            used_no.insert(no.clone());
            let mut v = vec![String::new(); cols.len()];
            // Editor: P25TGroupListControl.cs:201-217. Importer: RadioReferencehandler.cs:553-562.
            let (record, alert, strap) = if t.imported { ("0", "3", "1") } else { ("1", "1", "2") };
            for (col, val) in [
                ("GroupID", t.group_id.clone()),
                ("GroupName", t.name.clone()),
                ("GroupNo", no),
                ("Note", t.note.clone()),
                ("RecordFlag", record.into()),
                ("Type", "0".into()),
                ("Relay", "0".into()),
                ("Priority", "0".into()),
                ("DutyStatus", "0".into()),
                ("AlertMode", alert.into()),
                ("AlertToneID", "9999".into()),
                ("BackgroundColor", "0".into()),
                ("EncryptionKeyID", first_key.clone()),
                ("KeyStrapOption", strap.into()),
                ("P25TAddr", t.tgid.to_string()),
            ] {
                set(&mut v, &cols, col, val);
            }
            apply_settings(&mut v, &cols, t, None);
            rows.push(v);
        }
        rewrite(&sys, "tabGroup_New", &rows)?;
        Ok(())
    })();
    if let Err(e) = r {
        let _ = sys.execute_batch("ROLLBACK;");
        return Err(e);
    }

    // ---------- profile: knob cascade, index row, finish date ----------
    let deleted: Vec<String> = old_tgs.iter().map(|r| r.s("GroupID").to_string()).filter(|g| !ids.contains(g.as_str())).collect();
    p.db.execute_batch("BEGIN;")?;
    let r = (|| -> Result<(), Error> {
        let knobs = load_table(&p.db, "tabP25TrunkingSystemKnob");
        let channels: Vec<&Row> = sel(&knobs, "P25TrunkingID", &edit.system_id);
        if !deleted.is_empty() {
            for k in &channels {
                let ch = k.s("ChannelID");
                for table in ["tabChannelGroup", "tabChannelRevertGroup", "tabP25TChannelSubGroup", "tabP25TCallAlertSUIDGroup"] {
                    for g in &deleted {
                        let n = p.db.execute(&format!("DELETE FROM {table} WHERE ChannelID = ? AND GroupID = ?"), &[Some(ch), Some(g)])?;
                        if table == "tabChannelGroup" {
                            change.knob_entries_removed += n;
                        }
                    }
                }
                // Renumber the priority list 1..n in its current order.
                let cg = sel(&load_table(&p.db, "tabChannelGroup"), "ChannelID", ch).into_iter().cloned().collect::<Vec<_>>();
                let mut pri: Vec<&Row> = cg.iter().filter(|r| r.i("SeqNo") > 0).collect();
                pri.sort_by_key(|r| r.i("SeqNo"));
                for (i, r) in pri.iter().enumerate() {
                    let want = (i + 1).to_string();
                    if r.s("SeqNo") != want {
                        p.db.execute("UPDATE tabChannelGroup SET SeqNo = ? WHERE ChannelID = ? AND GroupID = ?", &[Some(&want), Some(ch), Some(r.s("GroupID"))])?;
                    }
                }
                if cg.is_empty() {
                    let z = k.s("ZoneID");
                    let zname = load_table(&p.db, "tabZone_New").iter().find(|r| r.s("ID") == z).map(|r| r.s("Name").to_string()).unwrap_or_default();
                    change.empty_knobs.push(format!("{zname} {}", k.s("KnobIndex")));
                }
            }
            if !change.empty_knobs.is_empty() {
                return Err(Error::Invalid(format!(
                    "deleting those talkgroups would leave knob(s) {} with none; give them another talkgroup first",
                    change.empty_knobs.join(", ")
                )));
            }
        }
        let count = |t: &str| -> Result<String, Error> {
            Ok(sys.query(&format!("SELECT count(*) FROM {t}"), &[])?.rows[0][0].clone().unwrap_or_default())
        };
        let (sites, ccs, tgs, subs, inds) =
            (count("tabP25TrunkingSite")?, count("tabP25TrunkingControlCH")?, count("tabGroup_New")?, count("tabSubGroup")?, count("tabMember_New")?);
        p.db.execute(
            "UPDATE tabP25TrunkingSecurity SET SystemName = ?, WacnID = ?, SystemID = ?, SiteCount = ?, ControlCHCount = ?, TGIDCount = ?, SubGroupCount = ?, IndividualCount = ? WHERE P25TrunkingID = ?",
            &[Some(&edit.name), Some(&edit.wacn.to_string()), Some(&edit.sysid.to_string()), Some(&sites), Some(&ccs), Some(&tgs), Some(&subs), Some(&inds), Some(&edit.system_id)],
        )?;
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(())
    })();
    match r {
        Ok(()) => {
            sys.execute_batch("COMMIT;")?;
            p.db.execute_batch("COMMIT;")?;
            Ok(change)
        }
        Err(e) => {
            let _ = sys.execute_batch("ROLLBACK;");
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

/// The current state of a system as a `SystemEdit` (the starting point for an editor).
pub fn current_state(p: &Profile, system_id: &str) -> Result<SystemEdit, Error> {
    let sys = p.system_db(system_id)?;
    let row = load_table(&sys, "tabP25TrunkingSystem").into_iter().next().ok_or_else(|| Error::Invalid("system DB has no system row".into()))?;
    Ok(SystemEdit {
        system_id: system_id.to_string(),
        name: row.s("Name").into(),
        wacn: row.i("WacnID"),
        sysid: row.i("SystemID"),
        sites: load_table(&sys, "tabP25TrunkingSite")
            .iter()
            .map(|r| SiteEdit { id: Some(r.s("ID").into()), name: r.s("Name").into(), rfss: r.i("RFSSID"), site: r.i("SiteID") })
            .collect(),
        control_channels: load_table(&sys, "tabP25TrunkingControlCH")
            .iter()
            .map(|r| ControlChannelEdit { id: Some(r.i("ID")), rx_hz: r.i("RxFreq"), note: r.s("Note").into() })
            .collect(),
        talkgroups: load_table(&sys, "tabGroup_New")
            .iter()
            .map(|r| TalkgroupEdit {
                group_id: r.s("GroupID").into(),
                tgid: num(r, "P25TAddr").unwrap_or(0),
                name: r.s("GroupName").into(),
                note: r.s("Note").into(),
                imported: false,
                alert_mode: num(r, "AlertMode"),
                alert_tone: r.get("AlertToneID").map(String::from),
                voice_storage: num(r, "RecordFlag").map(|f| f > 0),
                background: num(r, "BackgroundColor"),
            })
            .collect(),
    })
}

/// Add a new system to a working copy: copy Unication's empty system DB,
/// write its system and band rows and the profile index row, then fill in
/// sites, control channels and talkgroups through `apply_system_edit`. On any
/// failure the file and index row are removed again.
pub fn create_system(p: &Profile, templates: &Templates, new: &NewSystem, now: &str) -> Result<SystemChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let edit = &new.system;
    let id = edit.system_id.as_str();
    if !is_guid(id) {
        return Err(Error::Invalid(format!("new system IDs must be lowercase GUIDs, got {id}")));
    }
    let index = load_table(&p.db, "tabP25TrunkingSecurity");
    if index.len() >= MAX_SYSTEMS {
        return Err(Error::Invalid(format!("a profile holds at most {MAX_SYSTEMS} systems")));
    }
    let path = p.folder.join(format!("{id}.db"));
    if index.iter().any(|r| r.s("P25TrunkingID") == id) || path.exists() {
        return Err(Error::Invalid(format!("system {id} already exists")));
    }
    check_name("System", &edit.name)?;
    let bands = profile_bands(p, templates)?;
    if !bands.iter().any(|b| b.code == new.band) {
        let codes: Vec<&str> = bands.iter().map(|b| b.code.as_str()).collect();
        return Err(Error::Invalid(format!("band {:?} isn't one of this pager's bands ({})", new.band, codes.join(", "))));
    }
    let data = Connection::open(&templates.data_db())?;
    let band_row = load_table(&data, "tabRFBandTypeInfo").into_iter().find(|r| r.s("Code") == new.band).ok_or_else(|| Error::Invalid(format!("unknown band code {}", new.band)))?;
    let template = templates.system_template();
    if !template.is_file() {
        return Err(Error::Templates(templates.dir.clone()));
    }

    fs::create_dir_all(&p.folder)?;
    fs::copy(&template, &path)?;
    let r = (|| -> Result<SystemChange, Error> {
        {
            let sys = Connection::open(&path)?;
            sys.execute_batch("BEGIN;")?;
            let cols = sys.table_columns("tabP25TrunkingSystem")?;
            let mut v = vec![String::new(); cols.len()];
            for (col, val) in [
                ("ID", id.to_string()),
                ("SystemCreateType", "2".into()),
                ("Name", edit.name.clone()),
                ("ConverageType", "0".into()),
                ("WacnID", edit.wacn.to_string()),
                ("SystemID", edit.sysid.to_string()),
                ("FullSpectrumScanFlag", "0".into()),
                ("FullSpectrumScanRanges", "1".into()),
                ("SiteTrunkingFlag", "0".into()),
                ("ErrorRateThreshold", "5".into()),
            ] {
                set(&mut v, &cols, col, val);
            }
            let bcols = sys.table_columns("tabRFBandTypeInfo")?;
            let bv = row_values(&band_row, &bcols);
            let res = rewrite(&sys, "tabP25TrunkingSystem", &[v]).and_then(|()| rewrite(&sys, "tabRFBandTypeInfo", &[bv]));
            match res {
                Ok(()) => sys.execute_batch("COMMIT;")?,
                Err(e) => {
                    let _ = sys.execute_batch("ROLLBACK;");
                    return Err(e);
                }
            }
        }
        let cols = p.db.table_columns("tabP25TrunkingSecurity")?;
        let mut v = vec![String::new(); cols.len()];
        for (col, val) in [
            ("P25TrunkingID", id.to_string()),
            ("DBFileName", id.to_string()),
            ("SystemName", edit.name.clone()),
            ("WacnID", edit.wacn.to_string()),
            ("SystemID", edit.sysid.to_string()),
            ("SiteCount", "0".into()),
            ("ControlCHCount", "0".into()),
            ("TGIDCount", "1".into()),
            ("SubGroupCount", "0".into()),
            ("IndividualCount", "0".into()),
            ("SystemCreateType", "2".into()),
            ("BandTypeCode", new.band.clone()),
        ] {
            set(&mut v, &cols, col, val);
        }
        let params: Vec<Option<&str>> = v.iter().map(|x| Some(x.as_str())).collect();
        p.db.execute(&format!("INSERT INTO tabP25TrunkingSecurity VALUES({})", vec!["?"; v.len()].join(",")), &params)?;

        // The template's "All" talkgroup stays unless the caller listed it.
        let mut edit = edit.clone();
        if !edit.talkgroups.iter().any(|t| t.group_id == ALL_GROUP_ID) {
            let sys = Connection::open(&path)?;
            if let Some(all) = load_table(&sys, "tabGroup_New").into_iter().find(|r| r.s("GroupID") == ALL_GROUP_ID) {
                edit.talkgroups.insert(0, TalkgroupEdit { group_id: ALL_GROUP_ID.into(), tgid: all.i("P25TAddr"), name: all.s("GroupName").into(), note: all.s("Note").into(), ..Default::default() });
            }
        }
        let mut ch = apply_system_edit(p, templates, &edit, now)?;
        ch.created = true;
        Ok(ch)
    })();
    if r.is_err() {
        let _ = p.db.execute("DELETE FROM tabP25TrunkingSecurity WHERE P25TrunkingID = ?", &[Some(id)]);
        let _ = fs::remove_file(&path);
    }
    r
}

/// Add a copy of another profile's system to a working copy, as PPS's `addBySelect`
/// (`View/View.P25TSystem/P25TSyetemTotal.cs:628-668`) does: copy the system DB file under
/// a new ID, point its `ID`, `SystemSource` (`resource_<old ID>`) and `Name` at the copy,
/// and add the source's index row with the same three changes. Talkgroup GroupIDs stay
/// as they are, as in PPS. Then `apply_system_edit` applies the caller's state, so an
/// untouched copy is rewritten verbatim and edits made before writing apply as usual.
/// The system's band must be one the pager has.
pub fn copy_system(p: &Profile, templates: &Templates, copy: &SystemCopy, now: &str) -> Result<SystemChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let edit = &copy.system;
    let id = edit.system_id.as_str();
    if !is_guid(id) {
        return Err(Error::Invalid(format!("new system IDs must be lowercase GUIDs, got {id}")));
    }
    let index = load_table(&p.db, "tabP25TrunkingSecurity");
    if index.len() >= MAX_SYSTEMS {
        return Err(Error::Invalid(format!("a profile holds at most {MAX_SYSTEMS} systems")));
    }
    let path = p.folder.join(format!("{id}.db"));
    if index.iter().any(|r| r.s("P25TrunkingID") == id) || path.exists() {
        return Err(Error::Invalid(format!("system {id} already exists")));
    }
    check_name("System", &edit.name)?;
    let source = Profile::from_backup(&copy.backup)?;
    let src_index = load_table(&source.db, "tabP25TrunkingSecurity");
    let row = src_index
        .iter()
        .find(|r| r.s("P25TrunkingID") == copy.source_id)
        .ok_or_else(|| Error::Invalid(format!("system {} is not in that backup", copy.source_id)))?;
    let band = row.s("BandTypeCode");
    let model = load_table(&p.db, "tabModel").first().map(|r| r.s("Model").to_string()).unwrap_or_default();
    let bands = model_band_codes(&model);
    if band.is_empty() || band.len() % 2 != 0 || band.as_bytes().chunks(2).any(|c| !bands.iter().any(|b| b.as_bytes() == c)) {
        return Err(Error::Invalid(format!("{} is a {} system; this pager's bands are {}", row.s("SystemName"), if band.is_empty() { "no-band" } else { band }, bands.join(", "))));
    }
    let src_path = source.folder.join(format!("{}.db", copy.source_id));
    if !src_path.is_file() {
        return Err(Error::Invalid(format!("that backup has no system DB for {}", row.s("SystemName"))));
    }
    let origin = format!("resource_{}", copy.source_id);

    fs::create_dir_all(&p.folder)?;
    fs::copy(&src_path, &path)?;
    let r = (|| -> Result<SystemChange, Error> {
        {
            let sys = Connection::open(&path)?;
            if load_table(&sys, "tabP25TrunkingSystem").len() != 1 {
                return Err(Error::Invalid("the source system DB must hold exactly one system row".into()));
            }
            sys.execute("UPDATE tabP25TrunkingSystem SET ID = ?, SystemSource = ?, Name = ?", &[Some(id), Some(&origin), Some(&edit.name)])?;
        }
        let cols = p.db.table_columns("tabP25TrunkingSecurity")?;
        let mut v: Vec<String> = cols.iter().map(|c| row.s(c).to_string()).collect();
        for (col, val) in [("P25TrunkingID", id), ("DBFileName", id), ("SystemSource", origin.as_str()), ("SystemName", edit.name.as_str())] {
            set(&mut v, &cols, col, val);
        }
        let params: Vec<Option<&str>> = v.iter().map(|x| Some(x.as_str())).collect();
        p.db.execute(&format!("INSERT INTO tabP25TrunkingSecurity VALUES({})", vec!["?"; v.len()].join(",")), &params)?;
        apply_system_edit(p, templates, edit, now)?;
        // Like a created system, the counts are its contents.
        Ok(SystemChange {
            system_id: id.into(),
            name: edit.name.clone(),
            created: true,
            sites_added: edit.sites.len(),
            ccs_added: edit.control_channels.len(),
            tgs_added: edit.talkgroups.iter().filter(|t| t.group_id != ALL_GROUP_ID).count(),
            ..Default::default()
        })
    })();
    if r.is_err() {
        let _ = p.db.execute("DELETE FROM tabP25TrunkingSecurity WHERE P25TrunkingID = ?", &[Some(id)]);
        let _ = fs::remove_file(&path);
    }
    r
}

/// Remove an unused system from a working copy: its index row and its DB file.
/// Refuses while any knob uses it (PPS deletes anyway and then can't program).
pub fn delete_system(p: &Profile, system_id: &str, now: &str) -> Result<SystemChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let index = load_table(&p.db, "tabP25TrunkingSecurity");
    let idx = index.iter().find(|r| r.s("P25TrunkingID") == system_id).ok_or_else(|| Error::Invalid(format!("system {system_id} is not in this profile")))?;
    let name = idx.s("SystemName").to_string();
    let zones = load_table(&p.db, "tabZone_New");
    let users: Vec<String> = sel(&load_table(&p.db, "tabP25TrunkingSystemKnob"), "P25TrunkingID", system_id)
        .iter()
        .map(|k| {
            let zone = zones.iter().find(|z| z.s("ID") == k.s("ZoneID")).map(|z| z.s("Name")).unwrap_or("?");
            format!("{zone} knob {}", k.s("KnobIndex"))
        })
        .collect();
    if !users.is_empty() {
        return Err(Error::Invalid(format!("{name} is used by {}; move those knobs to another system first", users.join(", "))));
    }
    let path = p.folder.join(format!("{system_id}.db"));
    p.db.execute_batch("BEGIN;")?;
    let r = (|| -> Result<(), Error> {
        p.db.execute("DELETE FROM tabP25TrunkingSecurity WHERE P25TrunkingID = ?", &[Some(system_id)])?;
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    })();
    match r {
        Ok(()) => p.db.execute_batch("COMMIT;")?,
        Err(e) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            return Err(e);
        }
    }
    Ok(SystemChange { system_id: system_id.into(), name, deleted: true, ..Default::default() })
}
