//! Zones: create, rename, delete (empty zones only) and reorder, per
//! `notes/zones-and-scan.md` section 9.
//!
//! The radio numbers channels by zone position (zone x 100 + knob), so
//! deleting a zone that isn't last, or reordering, renumbers every later
//! channel. That is expected; `edit_set` checks that it changed nothing else.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::row::load_table;
use crate::system_edit::{check_name, is_guid};
use crate::{Error, Profile};

/// A profile holds at most this many zones (PPS disables Add at 64).
pub const MAX_ZONES: usize = 64;
/// Knob positions in a zone.
pub const POSITIONS: i64 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewZone {
    /// New lowercase GUID chosen by the caller.
    pub zone_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneRename {
    pub zone_id: String,
    pub name: String,
}

/// What a zone operation did.
#[derive(Debug, Clone, Serialize)]
pub struct ZoneChange {
    pub zone_id: String,
    pub name: String,
    pub created: bool,
    pub deleted: bool,
    /// Old name, for a rename.
    pub renamed_from: Option<String>,
    /// The zone order changed (a reorder, or a delete that wasn't last).
    pub renumbers: bool,
}

/// Trimmed, valid, and not the name of another zone (compared without case,
/// since knob edits find zones by name).
fn check_zone_name(p: &Profile, name: &str, except: Option<&str>) -> Result<String, Error> {
    let name = name.trim().to_string();
    check_name("Zone", &name)?;
    let clash = load_table(&p.db, "tabZone_New").into_iter().any(|z| Some(z.s("ID")) != except && z.s("Name").trim().eq_ignore_ascii_case(&name));
    if clash {
        return Err(Error::Invalid(format!("another zone is already called {name:?}")));
    }
    Ok(name)
}

fn in_tx<T>(p: &Profile, f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    p.db.execute_batch("BEGIN;")?;
    match f() {
        Ok(v) => {
            p.db.execute_batch("COMMIT;")?;
            Ok(v)
        }
        Err(e) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

/// Append a zone with 8 empty knob positions, as PPS's Add does.
pub fn create_zone(p: &Profile, z: &NewZone, now: &str) -> Result<ZoneChange, Error> {
    let zones = load_table(&p.db, "tabZone_New");
    if zones.len() >= MAX_ZONES {
        return Err(Error::Invalid(format!("a profile holds at most {MAX_ZONES} zones")));
    }
    if !is_guid(&z.zone_id) || zones.iter().any(|r| r.s("ID") == z.zone_id) {
        return Err(Error::Invalid("the new zone ID must be a new lowercase GUID".into()));
    }
    let name = check_zone_name(p, &z.name, None)?;
    in_tx(p, || {
        p.db.execute("INSERT INTO tabZone_New(ID,ZoneNo,Name,Note,ModifyByDevice) VALUES(?,'',?,'','')", &[Some(&z.zone_id), Some(&name)])?;
        for k in 1..=POSITIONS {
            p.db.execute(
                "INSERT INTO tabReceivingMode_NewG4(ZoneID,KnobIndex,CurrentMode,ChannelName,AlertToneID,AlertFileName) VALUES(?,?,'0','','0','')",
                &[Some(&z.zone_id), Some(&k.to_string())],
            )?;
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(ZoneChange { zone_id: z.zone_id.clone(), name, created: true, deleted: false, renamed_from: None, renumbers: false })
    })
}

pub fn rename_zone(p: &Profile, r: &ZoneRename, now: &str) -> Result<ZoneChange, Error> {
    let zones = load_table(&p.db, "tabZone_New");
    let old = zones.iter().find(|z| z.s("ID") == r.zone_id).ok_or_else(|| Error::Invalid(format!("no zone {}", r.zone_id)))?.s("Name").to_string();
    let name = check_zone_name(p, &r.name, Some(&r.zone_id))?;
    if name == old {
        return Err(Error::Invalid(format!("zone {old:?} already has that name")));
    }
    in_tx(p, || {
        p.db.execute("UPDATE tabZone_New SET Name = ? WHERE ID = ?", &[Some(&name), Some(&r.zone_id)])?;
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(ZoneChange { zone_id: r.zone_id.clone(), name, created: false, deleted: false, renamed_from: Some(old), renumbers: false })
    })
}

/// Tables that hold a knob's detail by `ZoneID`; a zone with rows in any of
/// them isn't empty, whatever its receive rows say.
const KNOB_TABLES: &[&str] = &["tabSingleConvCHKnob", "tabAdvancedConvCHKnob", "tabP25TrunkingSystemKnob", "tabMultiChannelScan", "tabAllBandScanCHKnob", "tabMultiCHScanList"];

/// Delete a zone whose knob positions are all empty (and not the only zone).
pub fn delete_zone(p: &Profile, zone_id: &str, now: &str) -> Result<ZoneChange, Error> {
    let zones = load_table(&p.db, "tabZone_New");
    let pos = zones.iter().position(|z| z.s("ID") == zone_id).ok_or_else(|| Error::Invalid(format!("no zone {zone_id}")))?;
    let name = zones[pos].s("Name").to_string();
    if zones.len() == 1 {
        return Err(Error::Invalid("a profile needs at least one zone".into()));
    }
    let used: Vec<String> = load_table(&p.db, "tabReceivingMode_NewG4")
        .into_iter()
        .filter(|r| r.s("ZoneID") == zone_id && r.i("CurrentMode") != 0)
        .map(|r| format!("{} ({})", r.s("KnobIndex"), r.s("ChannelName")))
        .collect();
    if !used.is_empty() {
        return Err(Error::Invalid(format!("zone {name:?} still has knobs {}; remove them first", used.join(", "))));
    }
    if let Some(t) = KNOB_TABLES.iter().find(|t| load_table(&p.db, t).iter().any(|r| r.s("ZoneID") == zone_id)) {
        return Err(Error::Invalid(format!("zone {name:?} has leftover knob rows in {t}")));
    }
    in_tx(p, || {
        p.db.execute("DELETE FROM tabZone_New WHERE ID = ?", &[Some(zone_id)])?;
        p.db.execute("DELETE FROM tabReceivingMode_NewG4 WHERE ZoneID = ?", &[Some(zone_id)])?;
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(ZoneChange { zone_id: zone_id.into(), name, created: false, deleted: true, renamed_from: None, renumbers: pos + 1 != zones.len() })
    })
}

/// Put the zones in `order` (every zone ID exactly once). The first zone is
/// the one the pager shows after programming.
pub fn reorder_zones(p: &Profile, order: &[String], now: &str) -> Result<ZoneChange, Error> {
    let zones = load_table(&p.db, "tabZone_New");
    let ids: Vec<&str> = zones.iter().map(|z| z.s("ID")).collect();
    let wanted: HashSet<&str> = order.iter().map(String::as_str).collect();
    if order.len() != ids.len() || wanted.len() != ids.len() || ids.iter().any(|id| !wanted.contains(id)) {
        return Err(Error::Invalid("the new zone order must list every zone exactly once".into()));
    }
    if ids.iter().zip(order).all(|(a, b)| *a == b) {
        return Err(Error::Invalid("the zone order is unchanged".into()));
    }
    let cols = p.db.table_columns("tabZone_New")?;
    in_tx(p, || {
        // Row order is the only order PPS and the converter use: rewrite the
        // rows, values unchanged, so rowids ascend in the new order.
        p.db.execute("DELETE FROM tabZone_New", &[])?;
        for id in order {
            let z = zones.iter().find(|z| z.s("ID") == id).unwrap();
            let vals: Vec<Option<&str>> = cols.iter().map(|c| z.get(c)).collect();
            p.db.execute(&format!("INSERT INTO tabZone_New({}) VALUES({})", cols.join(","), vec!["?"; cols.len()].join(",")), &vals)?;
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        let first = zones.iter().find(|z| z.s("ID") == order[0]).unwrap();
        Ok(ZoneChange { zone_id: order[0].clone(), name: first.s("Name").into(), created: false, deleted: false, renamed_from: None, renumbers: true })
    })
}
