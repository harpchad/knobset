//! Scan knobs over conventional channels, per `notes/zones-and-scan.md` 9.6
//! and 9.7: a Normal Scan (4) or Priority Scan (36) on an empty position, or a
//! new member list for an existing one. Members are analog conventional knobs
//! from any zone.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::row::{load_table, sel};
use crate::system_edit::check_name;
use crate::{Error, Profile};

pub const MODE_NORMAL_SCAN: i64 = 4;
pub const MODE_PRIORITY_SCAN: i64 = 36;
/// PPS's built-in knob voice prompts ("Normal Scan", "Priority Scan"); Knobset defaults to none.
pub const fn builtin_prompt(mode: i64) -> i64 {
    if mode == MODE_PRIORITY_SCAN { 4 } else { 3 }
}
pub const MIN_MEMBERS: usize = 2;
pub const MAX_MEMBERS: usize = 16;
const CSQ: &str = "0000000000000000";

/// A scan knob's members, by the member knobs' `ChannelID`s, in scan order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanKnobEdit {
    pub zone: String,
    pub knob: i64,
    /// `MODE_NORMAL_SCAN` or `MODE_PRIORITY_SCAN`. Must match an existing scan knob's mode.
    pub mode: i64,
    pub members: Vec<String>,
    /// The priority channel (one of `members`), for a priority scan.
    #[serde(default)]
    pub priority: Option<String>,
    /// Alias for a new scan knob (required there).
    #[serde(default)]
    pub name: Option<String>,
    /// Voice prompt for a new scan knob: 0 or the mode's built-in prompt.
    #[serde(default)]
    pub voice_prompt: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanKnobChange {
    pub zone: String,
    pub knob: i64,
    pub knob_name: String,
    pub new_knob: bool,
    pub mode: i64,
    pub members: Vec<String>,
    pub priority: Option<String>,
}

/// The `tabMultiChannelScan` row PPS saves for a new scan knob with untouched controls.
fn default_scan_row(cols: &[String], zone_id: &str, knob: i64, mode: i64, priority: &str) -> Result<Vec<String>, Error> {
    let knob = knob.to_string();
    let prio_type = if mode == MODE_PRIORITY_SCAN { "1" } else { "0" };
    cols.iter()
        .map(|c| {
            Ok(match c.as_str() {
                "ZoneID" => zone_id,
                "KnobIndex" => &knob,
                "StayTime" => "200",
                "PriorityScanType" => prio_type,
                "FirstPriorityCH" => priority,
                "SecondPriorityCH" => "",
                "DwellTime" => "6",
                "DelayNTime" | "CarrierOffDelayTime" => "30",
                "OpenSpeaker" | "AlertMode" | "RecordFlag" | "AlwaysUnreadReminderAlert" => "1",
                "SamplingFlag" | "PushToListen" | "Privacy" | "ResetMode" | "EnableCarrierOffTime" | "DutyStatus" | "MatchCTCD" | "MatchColorCode" | "MatchNAC"
                | "AlwaysUnreadVoiceMessage" => "0",
                other => return Err(Error::Unsupported(format!("unknown scan knob column {other}"))),
            }
            .to_string())
        })
        .collect()
}

/// Check members: 2..16 distinct active analog conventional knobs on knobs 1..8.
fn check_members(p: &Profile, members: &[String], priority: Option<&str>, mode: i64) -> Result<(), Error> {
    if !(MIN_MEMBERS..=MAX_MEMBERS).contains(&members.len()) {
        return Err(Error::Invalid(format!("a scan knob needs {MIN_MEMBERS} to {MAX_MEMBERS} channels")));
    }
    if members.iter().collect::<HashSet<_>>().len() != members.len() {
        return Err(Error::Invalid("a channel is listed twice in the scan list".into()));
    }
    let conv = load_table(&p.db, "tabSingleConvCHKnob");
    let recv = load_table(&p.db, "tabReceivingMode_NewG4");
    let sub = load_table(&p.db, "tabP25TSubGroupEnable");
    for m in members {
        let found = sel(&conv, "ChannelID", m);
        let [k] = found.as_slice() else {
            return Err(Error::Invalid(format!("scan member {m} isn't a conventional knob")));
        };
        let mode_of = recv.iter().find(|r| r.s("ZoneID") == k.s("ZoneID") && r.s("KnobIndex") == k.s("KnobIndex")).map(|r| r.i("CurrentMode"));
        if !matches!(mode_of, Some(1) | Some(33)) || !(1..=8).contains(&k.i("KnobIndex")) {
            return Err(Error::Invalid(format!("scan member {m} isn't an active conventional knob")));
        }
        if k.s("ProtocolSettingID") != CSQ {
            return Err(Error::Unsupported("scanning digital conventional channels".into()));
        }
        if sub.iter().any(|r| r.s("ChannelID") == m) {
            return Err(Error::Invalid(format!("scan member {m} has sub-group settings")));
        }
    }
    match (mode, priority) {
        (MODE_PRIORITY_SCAN, Some(pr)) if members.iter().any(|m| m == pr) => Ok(()),
        (MODE_PRIORITY_SCAN, _) => Err(Error::Invalid("a priority scan needs its priority channel in the list".into())),
        (_, Some(_)) => Err(Error::Invalid("only a priority scan has a priority channel".into())),
        _ => Ok(()),
    }
}

/// Make an empty position a scan knob, or give an existing scan knob new members.
pub fn set_scan_knob(p: &Profile, e: &ScanKnobEdit, now: &str) -> Result<ScanKnobChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    if ![MODE_NORMAL_SCAN, MODE_PRIORITY_SCAN].contains(&e.mode) {
        return Err(Error::Invalid(format!("mode {} isn't a normal or priority scan", e.mode)));
    }
    let zones = load_table(&p.db, "tabZone_New");
    let matches = sel(&zones, "Name", &e.zone);
    let [z] = matches.as_slice() else {
        return Err(Error::Invalid(format!("expected exactly one zone named {:?}", e.zone)));
    };
    let zid = z.s("ID").to_string();
    let recv = load_table(&p.db, "tabReceivingMode_NewG4");
    let r = recv
        .iter()
        .find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == e.knob)
        .ok_or_else(|| Error::Invalid(format!("{:?} has no knob position {}", e.zone, e.knob)))?;
    if !(1..=8).contains(&e.knob) {
        return Err(Error::Invalid("knob positions are 1 to 8".into()));
    }
    let scans = load_table(&p.db, "tabMultiChannelScan");
    let existing = scans.iter().find(|s| s.s("ZoneID") == zid && s.i("KnobIndex") == e.knob);
    let current = r.i("CurrentMode");
    // Members first (they may be conventional knobs added earlier in this batch).
    let mut members = e.members.clone();
    if e.mode == MODE_PRIORITY_SCAN
        && let Some(pr) = &e.priority
        && let Some(i) = members.iter().position(|m| m == pr)
    {
        // PPS keeps the priority channel first in the list.
        let pr = members.remove(i);
        members.insert(0, pr);
    }
    check_members(p, &members, e.priority.as_deref(), e.mode)?;

    let (new_knob, knob_name) = match (current, existing) {
        (0, None) => {
            for t in ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob", "tabP25TrunkingSystemKnob", "tabAllBandScanCHKnob", "tabMultiCHScanList"] {
                if load_table(&p.db, t).iter().any(|k| k.s("ZoneID") == zid && k.i("KnobIndex") == e.knob) {
                    return Err(Error::Invalid(format!("{:?} knob {} is empty but has a leftover row in {t}", e.zone, e.knob)));
                }
            }
            let name = e.name.as_deref().ok_or_else(|| Error::Invalid("a new scan knob needs a name".into()))?;
            check_name("Knob", name)?;
            if ![0, builtin_prompt(e.mode)].contains(&e.voice_prompt) {
                return Err(Error::Invalid(format!("voice prompt {} isn't available for this scan mode", e.voice_prompt)));
            }
            (true, name.to_string())
        }
        (m, Some(_)) if m == e.mode => (false, r.s("ChannelName").to_string()),
        (m, Some(_)) if m == MODE_NORMAL_SCAN || m == MODE_PRIORITY_SCAN => {
            return Err(Error::Unsupported("switching a scan knob between normal and priority scan".into()));
        }
        (0, Some(_)) => return Err(Error::Invalid(format!("{:?} knob {} is empty but has a leftover scan row", e.zone, e.knob))),
        _ => return Err(Error::Unsupported("turning a non-scan knob into a scan knob".into())),
    };
    let priority = if e.mode == MODE_PRIORITY_SCAN { e.priority.clone().unwrap_or_default() } else { String::new() };

    p.db.execute_batch("BEGIN;")?;
    let res = (|| -> Result<(), Error> {
        let ki = r.s("KnobIndex").to_string();
        if new_knob {
            p.db.execute(
                "UPDATE tabReceivingMode_NewG4 SET CurrentMode = ?, ChannelName = ?, AlertToneID = ?, AlertFileName = coalesce(AlertFileName, '') WHERE ZoneID = ? AND KnobIndex = ?",
                &[Some(&e.mode.to_string()), Some(&knob_name), Some(&e.voice_prompt.to_string()), Some(&zid), Some(&ki)],
            )?;
            let cols = p.db.table_columns("tabMultiChannelScan")?;
            let row = default_scan_row(&cols, &zid, e.knob, e.mode, &priority)?;
            let params: Vec<Option<&str>> = row.iter().map(|v| Some(v.as_str())).collect();
            p.db.execute(&format!("INSERT INTO tabMultiChannelScan({}) VALUES({})", cols.join(","), vec!["?"; row.len()].join(",")), &params)?;
        } else {
            // Knobset models one priority channel; a second one PPS may have set would point outside the new list.
            p.db.execute("UPDATE tabMultiChannelScan SET FirstPriorityCH = ?, SecondPriorityCH = '' WHERE ZoneID = ? AND KnobIndex = ?", &[Some(&priority), Some(&zid), Some(&ki)])?;
            p.db.execute("DELETE FROM tabMultiCHScanList WHERE ZoneID = ? AND KnobIndex = ?", &[Some(&zid), Some(&ki)])?;
        }
        for (i, m) in members.iter().enumerate() {
            p.db.execute(
                "INSERT INTO tabMultiCHScanList(ZoneID,KnobIndex,ScanOrder,ChannelID) VALUES(?,?,?,?)",
                &[Some(&zid), Some(&ki), Some(&(i + 1).to_string()), Some(m)],
            )?;
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(())
    })();
    match res {
        Ok(()) => {
            p.db.execute_batch("COMMIT;")?;
            Ok(ScanKnobChange { zone: e.zone.clone(), knob: e.knob, knob_name, new_knob, mode: e.mode, members, priority: (!priority.is_empty()).then_some(priority) })
        }
        Err(err) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(err)
        }
    }
}
