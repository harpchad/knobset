//! Copying a knob to an empty position, and moving or swapping knobs, per
//! `notes/knob-copy-move.md`. A copy clones the knob's rows with a new
//! ChannelID, as PPS's Paste does; a move re-keys them and keeps the
//! ChannelID, so scan lists, priority channels and everything else that
//! names the channel follow the knob. The radio numbers channels by position
//! (zone x 100 + knob), so a move renumbers the knob's channel like a zone
//! reorder does; `edit_set` checks that it changed nothing else.

use serde::{Deserialize, Serialize};

use crate::edit::refuse_sub_features;
use crate::knob_remove::{KnobKind, at, classify, zone_id};
use crate::row::{Row, load_table};
use crate::system_edit::{check_name, is_guid};
use crate::{Error, Profile};

/// Tables holding a knob by (ZoneID, KnobIndex), the ones PPS's `moveKnob` re-keys.
const KNOB_TABLES: &[&str] = &["tabSingleConvCHKnob", "tabP25TrunkingSystemKnob", "tabMultiChannelScan", "tabMultiCHScanList", "tabAllBandScanCHKnob", "tabAdvancedConvCHKnob"];
/// Older tables keyed the same way that PPS's knob operations leave alone.
/// Empty in every real profile; Knobset refuses a position with rows there.
const UNTOUCHED_TABLES: &[&str] = &["tabAllBandScan", "tabP25TKnobGroup", "tabReceivingMode", "tabSingleChannel"];
/// Per-channel tables PPS clones for a pasted knob, in its order.
const CHANNEL_TABLES: &[&str] = &[
    "tabChannelGroupList",
    "tabChannelGroup",
    "tabRecvSaList",
    "tabP25TSubGroupEnable",
    "tabP25TChannelSubGroup",
    "tabChannelRevertGroup",
    "tabP25TCallAlertEnable",
    "tabP25TCallAlertSUID",
    "tabP25TCallAlertSUIDGroup",
    "tabP25TChannelRevertSubGroup",
    "tabAllBandScanBandParam",
    "tabAllBandScanProtocolParam",
];
/// The zone ID PPS parks a knob under during a swap.
const PARKED: &str = "00000000000";

/// Copy the knob at (`zone`, `knob`) to the empty position (`to_zone`, `to_knob`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnobCopy {
    pub zone: String,
    pub knob: i64,
    pub to_zone: String,
    pub to_knob: i64,
    /// The copy's alias (PPS keeps the source's; the user may change it).
    pub name: String,
    /// New lowercase GUID for the copy's channel; unused for a scan knob.
    #[serde(default)]
    pub channel_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnobCopyChange {
    pub zone: String,
    pub knob: i64,
    pub to_zone: String,
    pub to_knob: i64,
    pub source_name: String,
    pub knob_name: String,
    pub kind: KnobKind,
}

/// Move the knob at (`zone`, `knob`) to (`to_zone`, `to_knob`): an empty
/// position, or with `swap` an occupied one whose knob takes the old place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnobMove {
    pub zone: String,
    pub knob: i64,
    pub to_zone: String,
    pub to_knob: i64,
    #[serde(default)]
    pub swap: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnobMoveChange {
    pub zone: String,
    pub knob: i64,
    pub to_zone: String,
    pub to_knob: i64,
    pub knob_name: String,
    pub kind: KnobKind,
    /// The knob that moved the other way, for a swap.
    pub swapped_with: Option<String>,
    /// Scan knobs listing a moved channel, as "zone knob name"; they follow it.
    pub scans: Vec<String>,
}

/// A knob position's zone ID and receive row.
fn position(p: &Profile, zone: &str, knob: i64) -> Result<(String, Row), Error> {
    if !(1..=8).contains(&knob) {
        return Err(Error::Invalid("knob positions are 1 to 8".into()));
    }
    let zid = zone_id(p, zone)?;
    let row = at(&load_table(&p.db, "tabReceivingMode_NewG4"), &zid, knob).cloned().ok_or_else(|| Error::Invalid(format!("{zone:?} has no knob position {knob}")))?;
    if let Some(t) = UNTOUCHED_TABLES.iter().find(|t| at(&load_table(&p.db, t), &zid, knob).is_some()) {
        return Err(Error::Unsupported(format!("{zone:?} knob {knob} has rows in {t}")));
    }
    Ok((zid, row))
}

fn check_empty(p: &Profile, zid: &str, zone: &str, knob: i64, row: &Row) -> Result<(), Error> {
    if row.i("CurrentMode") != 0 {
        return Err(Error::Invalid(format!("{zone:?} knob {knob} isn't an empty position")));
    }
    if let Some(t) = KNOB_TABLES.iter().find(|t| at(&load_table(&p.db, t), zid, knob).is_some()) {
        return Err(Error::Invalid(format!("{zone:?} knob {knob} is empty but has a leftover row in {t}")));
    }
    Ok(())
}

fn has_table(p: &Profile, t: &str) -> bool {
    p.db.table_columns(t).is_ok_and(|c| !c.is_empty())
}

/// Append a clone of `row` with some columns replaced.
fn insert_clone(p: &Profile, table: &str, cols: &[String], row: &Row, set: &[(&str, &str)]) -> Result<(), Error> {
    let vals: Vec<Option<&str>> = cols.iter().map(|c| set.iter().find(|(k, _)| k.eq_ignore_ascii_case(c)).map(|(_, v)| *v).or_else(|| row.get(c))).collect();
    p.db.execute(&format!("INSERT INTO {table}({}) VALUES({})", cols.join(","), vec!["?"; cols.len()].join(",")), &vals)?;
    Ok(())
}

/// Write `values` (a receive row's) into the non-key columns of (zid, knob), with optional overrides.
fn set_receive_row(p: &Profile, zid: &str, knob: i64, values: &Row, set: &[(&str, &str)]) -> Result<(), Error> {
    let cols: Vec<String> = p.db.table_columns("tabReceivingMode_NewG4")?.into_iter().filter(|c| !c.eq_ignore_ascii_case("ZoneID") && !c.eq_ignore_ascii_case("KnobIndex")).collect();
    let k = knob.to_string();
    let mut vals: Vec<Option<&str>> = cols.iter().map(|c| Some(set.iter().find(|(x, _)| x.eq_ignore_ascii_case(c)).map(|(_, v)| *v).unwrap_or_else(|| values.s(c)))).collect();
    vals.extend([Some(zid), Some(k.as_str())]);
    let assign: Vec<String> = cols.iter().map(|c| format!("{c} = ?")).collect();
    let n = p.db.execute(&format!("UPDATE tabReceivingMode_NewG4 SET {} WHERE ZoneID = ? AND KnobIndex = ?", assign.join(", ")), &vals)?;
    if n != 1 {
        return Err(Error::Invalid(format!("updating the receive row touched {n} rows, expected 1")));
    }
    Ok(())
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

/// Clone a knob onto an empty position the way PPS's `copyKnob` does: the
/// receive row's values (with the new alias), the detail rows with the new
/// position and ChannelID, a scan knob's own member list, and every
/// per-channel row with the new ChannelID. Frequency, protocol setting and
/// talkgroup list rows are shared, not cloned.
pub fn copy_knob(p: &Profile, c: &KnobCopy, now: &str) -> Result<KnobCopyChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    check_name("Knob", &c.name)?;
    let (src_zid, src) = position(p, &c.zone, c.knob)?;
    let (dst_zid, dst) = position(p, &c.to_zone, c.to_knob)?;
    if src_zid == dst_zid && c.knob == c.to_knob {
        return Err(Error::Invalid("a knob can't be copied onto itself".into()));
    }
    let (kind, channel) = classify(p, &src_zid, c.knob, src.i("CurrentMode"))?;
    check_empty(p, &dst_zid, &c.to_zone, c.to_knob, &dst)?;
    let channels = match &channel {
        Some(old) => {
            refuse_sub_features(p, old)?;
            let new = c.channel_id.as_deref().filter(|x| is_guid(x)).ok_or_else(|| Error::Invalid("the copy needs a new lowercase GUID channel ID".into()))?;
            let taken = ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob", "tabP25TrunkingSystemKnob", "tabAllBandScanCHKnob"].iter().any(|t| load_table(&p.db, t).iter().any(|k| k.s("ChannelID") == new));
            if taken {
                return Err(Error::Invalid(format!("channel {new} already exists")));
            }
            Some((old.clone(), new.to_string()))
        }
        None => None,
    };
    let (dz, dk) = (dst_zid.clone(), c.to_knob.to_string());
    in_tx(p, || {
        set_receive_row(p, &dst_zid, c.to_knob, &src, &[("ChannelName", &c.name)])?;
        for t in KNOB_TABLES.iter().filter(|t| has_table(p, t)) {
            let cols = p.db.table_columns(t)?;
            for r in load_table(&p.db, t).iter().filter(|r| r.s("ZoneID") == src_zid && r.i("KnobIndex") == c.knob) {
                // Only the knob's own ChannelID changes; a scan list's member IDs stay.
                let ch = match &channels {
                    Some((old, new)) if r.s("ChannelID") == old => new.as_str(),
                    _ => r.s("ChannelID"),
                };
                let mut set = vec![("ZoneID", dz.as_str()), ("KnobIndex", dk.as_str())];
                if r.has("ChannelID") {
                    set.push(("ChannelID", ch));
                }
                insert_clone(p, t, &cols, r, &set)?;
            }
        }
        if let Some((old, new)) = &channels {
            for t in CHANNEL_TABLES.iter().filter(|t| has_table(p, t)) {
                let cols = p.db.table_columns(t)?;
                if !cols.iter().any(|c| c == "ChannelID") {
                    continue;
                }
                for r in load_table(&p.db, t).iter().filter(|r| r.s("ChannelID") == old) {
                    insert_clone(p, t, &cols, r, &[("ChannelID", new)])?;
                }
            }
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(KnobCopyChange {
            zone: c.zone.clone(),
            knob: c.knob,
            to_zone: c.to_zone.clone(),
            to_knob: c.to_knob,
            source_name: src.s("ChannelName").into(),
            knob_name: c.name.clone(),
            kind,
        })
    })
}

/// Move a knob to an empty position, or swap it with the knob there. The
/// detail rows are re-keyed in place, as PPS's `moveKnob` does (so contact and
/// signaling IDs, which follow table order, don't shift); the two receive
/// rows swap their values instead of their keys, which keeps each zone's
/// receive rows in knob order.
pub fn move_knob(p: &Profile, m: &KnobMove, now: &str) -> Result<KnobMoveChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let (src_zid, src) = position(p, &m.zone, m.knob)?;
    let (dst_zid, dst) = position(p, &m.to_zone, m.to_knob)?;
    if src_zid == dst_zid && m.knob == m.to_knob {
        return Err(Error::Invalid("pick a different position to move the knob to".into()));
    }
    let (kind, channel) = classify(p, &src_zid, m.knob, src.i("CurrentMode"))?;
    let other = if dst.i("CurrentMode") == 0 {
        if m.swap {
            return Err(Error::Invalid(format!("{:?} knob {} is empty, so there's nothing to swap with", m.to_zone, m.to_knob)));
        }
        check_empty(p, &dst_zid, &m.to_zone, m.to_knob, &dst)?;
        None
    } else {
        if !m.swap {
            return Err(Error::Invalid(format!("{:?} knob {} isn't empty; swap the two knobs instead", m.to_zone, m.to_knob)));
        }
        Some(classify(p, &dst_zid, m.to_knob, dst.i("CurrentMode"))?.1)
    };
    if KNOB_TABLES.iter().any(|t| load_table(&p.db, t).iter().any(|r| r.s("ZoneID") == PARKED)) {
        return Err(Error::Invalid(format!("the profile has knob rows under the placeholder zone {PARKED}")));
    }

    let moved: Vec<&str> = channel.iter().chain(other.iter().flatten()).map(String::as_str).collect();
    let zones = load_table(&p.db, "tabZone_New");
    let recv = load_table(&p.db, "tabReceivingMode_NewG4");
    let lists = load_table(&p.db, "tabMultiCHScanList");
    let mut scans = Vec::new();
    for s in load_table(&p.db, "tabMultiChannelScan") {
        let (sz, sk) = (s.s("ZoneID"), s.i("KnobIndex"));
        if lists.iter().any(|l| l.s("ZoneID") == sz && l.i("KnobIndex") == sk && moved.contains(&l.s("ChannelID"))) {
            let zn = zones.iter().find(|z| z.s("ID") == sz).map(|z| z.s("Name")).unwrap_or("?");
            let kn = at(&recv, sz, sk).map(|r| r.s("ChannelName")).unwrap_or("");
            scans.push(format!("{zn} {sk} {kn}"));
        }
    }

    in_tx(p, || {
        set_receive_row(p, &src_zid, m.knob, &dst, &[])?;
        set_receive_row(p, &dst_zid, m.to_knob, &src, &[])?;
        let (sk, dk) = (m.knob.to_string(), m.to_knob.to_string());
        for t in KNOB_TABLES.iter().filter(|t| has_table(p, t)) {
            let rekey = |from: (&str, &str), to: (&str, &str)| {
                p.db.execute(&format!("UPDATE {t} SET ZoneID = ?, KnobIndex = ? WHERE ZoneID = ? AND KnobIndex = ?"), &[Some(to.0), Some(to.1), Some(from.0), Some(from.1)])
            };
            rekey((&dst_zid, &dk), (PARKED, &sk))?;
            rekey((&src_zid, &sk), (&dst_zid, &dk))?;
            rekey((PARKED, &sk), (&src_zid, &sk))?;
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(KnobMoveChange {
            zone: m.zone.clone(),
            knob: m.knob,
            to_zone: m.to_zone.clone(),
            to_knob: m.to_knob,
            knob_name: src.s("ChannelName").into(),
            kind,
            swapped_with: other.is_some().then(|| dst.s("ChannelName").to_string()),
            scans,
        })
    })
}
