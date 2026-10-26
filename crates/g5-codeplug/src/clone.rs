//! Programming one pager's profile into another pager of the same model
//! (`notes/cloning.md`). PPS sends the same profile to every pager it's
//! used with, identity included; Knobset keeps the target's own identity.

use std::collections::HashSet;

use serde::Serialize;
use sqlite2::Connection;

use crate::compare::{TableStatus, compare};
use crate::row::load_table;
use crate::{Error, Profile};

/// Profile tables that describe the pager rather than the profile. The target keeps its own rows.
pub const IDENTITY_TABLES: &[&str] = &["tabPagerSUID", "tabAccessPassword", "tabPINPasswordSetting", "tabGSMBaseSettingPPS", "tabDeviceMember"];

/// Columns of the pager's own member row (`RadioNo` 99999) the target keeps: its alias, radio
/// number and unit addresses (PPS's Pager ID page, `View/View.GroupSetting/PagerID.cs:236-251`).
pub const PAGER_MEMBER_COLUMNS: &[&str] = &["MemberName", "UserDeviceID", "TwoToneAddr", "FiveToneAddr", "MDCAddr", "UDCAddr", "DMRAddr", "P25Addr"];

/// Radio DB tables the identity fields reach (converter-1.41.md 2.5, 2.6): the only tables
/// a clone's radio DB may differ in from the source's.
pub const IDENTITY_RADIO_TABLES: &[&str] = &["tabRadioBaseSetting", "tabContacts", "tabContactAddr", "tabContactAddrAndAlertTone", "tabAccessPassword", "tabPINPasswordSetting", "tabGSMBaseSetting"];

const PAGER_RADIO_NO: &str = "99999";

/// The identity fields of a profile, for display. Password hashes are never included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Identity {
    /// `UserDeviceID` of the pager's member row (the handshake's radio number).
    pub radio_number: String,
    /// The pager's alias in its own contact list.
    pub alias: String,
    /// P25 trunking unit ID (`tabPagerSUID`), empty when unset.
    pub pager_suid: String,
    /// Conventional unit addresses that are set, as (protocol, address).
    pub addresses: Vec<(String, String)>,
    pub read_password: bool,
    pub write_password: bool,
    pub pin_lock: bool,
}

/// One identity field and the value the target keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeptField {
    pub label: String,
    pub value: String,
    /// The source has a different value.
    pub differs: bool,
}

fn pager_row(db: &Connection) -> Result<crate::row::Row, Error> {
    let rows: Vec<_> = load_table(db, "tabMember_New").into_iter().filter(|r| r.s("RadioNo") == PAGER_RADIO_NO).collect();
    match rows.len() {
        1 => Ok(rows.into_iter().next().unwrap()),
        0 => Err(Error::Invalid("profile has no pager member row (RadioNo 99999)".into())),
        n => Err(Error::Invalid(format!("profile has {n} pager member rows (RadioNo 99999)"))),
    }
}

pub fn identity(p: &Profile) -> Result<Identity, Error> {
    let me = pager_row(&p.db)?;
    let enabled = |id: &str| load_table(&p.db, "tabAccessPassword").iter().any(|r| r.s("ID") == id && r.s("Enable") != "0" && !r.s("Enable").is_empty());
    let pin = load_table(&p.db, "tabPINPasswordSetting").first().is_some_and(|r| r.s("BootingLockEnable") == "1" || r.s("KeypadLockEnable") == "1");
    let addresses = [("2-tone", "TwoToneAddr"), ("5/6-tone", "FiveToneAddr"), ("MDC1200", "MDCAddr"), ("DVOA", "UDCAddr"), ("DMR", "DMRAddr"), ("P25 conventional", "P25Addr")]
        .into_iter()
        .filter(|(_, c)| !matches!(me.s(c), "" | "0"))
        .map(|(l, c)| (l.to_string(), me.s(c).to_string()))
        .collect();
    Ok(Identity {
        radio_number: me.s("UserDeviceID").into(),
        alias: me.s("MemberName").into(),
        pager_suid: load_table(&p.db, "tabPagerSUID").first().map(|r| r.s("P25TAddr").to_string()).unwrap_or_default(),
        addresses,
        read_password: enabled("1"),
        write_password: enabled("2"),
        pin_lock: pin,
    })
}

/// The identity fields the target keeps, compared with the source's.
pub fn kept_fields(source: &Identity, target: &Identity) -> Vec<KeptField> {
    let on = |b: bool| if b { "on" } else { "off" }.to_string();
    let addr = |i: &Identity| i.addresses.iter().map(|(l, a)| format!("{l} {a}")).collect::<Vec<_>>().join(", ");
    let none = |s: String| if s.is_empty() { "none".to_string() } else { s };
    [
        ("Radio number", target.radio_number.clone(), source.radio_number != target.radio_number),
        ("Pager alias", target.alias.clone(), source.alias != target.alias),
        ("P25 trunking unit ID", none(target.pager_suid.clone()), source.pager_suid != target.pager_suid),
        ("Conventional unit IDs", none(addr(target)), addr(source) != addr(target)),
        ("Read password", on(target.read_password), source.read_password != target.read_password),
        ("Write password", on(target.write_password), source.write_password != target.write_password),
        ("PIN lock", on(target.pin_lock), source.pin_lock != target.pin_lock),
    ]
    .into_iter()
    .map(|(label, value, differs)| KeptField { label: label.into(), value, differs })
    .collect()
}

type RawRows = (Vec<String>, Vec<Vec<Option<String>>>);

fn raw_rows(db: &Connection, table: &str) -> Result<RawRows, Error> {
    let cols = db.table_columns(table)?;
    let rows = db.query(&format!("SELECT * FROM {table} ORDER BY rowid"), &[])?.rows;
    Ok((cols, rows))
}

/// Write `target`'s identity into `p`, a working copy of the source profile: its rows of
/// [`IDENTITY_TABLES`] verbatim and the [`PAGER_MEMBER_COLUMNS`] of its pager member row.
pub fn keep_identity(p: &Profile, target: &Profile) -> Result<(), Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let (ours, theirs) = (p.db.table_names()?, target.db.table_names()?);
    let me = pager_row(&p.db)?;
    let them = pager_row(&target.db)?;
    p.db.execute_batch("BEGIN;")?;
    let r = (|| -> Result<(), Error> {
        for t in IDENTITY_TABLES {
            match (ours.iter().any(|x| x == t), theirs.iter().any(|x| x == t)) {
                (false, false) => continue,
                (true, true) => {}
                _ => return Err(Error::Invalid(format!("only one of the two profiles has {t}; their schemas differ"))),
            }
            let (cols, rows) = raw_rows(&target.db, t)?;
            if cols != p.db.table_columns(t)? {
                return Err(Error::Invalid(format!("{t} has different columns in the two profiles")));
            }
            p.db.execute(&format!("DELETE FROM {t}"), &[])?;
            let ph = vec!["?"; cols.len()].join(",");
            for row in &rows {
                let params: Vec<Option<&str>> = row.iter().map(|v| v.as_deref()).collect();
                p.db.execute(&format!("INSERT INTO {t} VALUES({ph})"), &params)?;
            }
        }
        for c in PAGER_MEMBER_COLUMNS {
            if !me.has(c) || !them.has(c) {
                return Err(Error::Invalid(format!("tabMember_New has no {c} column")));
            }
            p.db.execute(&format!("UPDATE tabMember_New SET {c} = ? WHERE RadioNo = '{PAGER_RADIO_NO}'"), &[them.get(c)])?;
        }
        Ok(())
    })();
    match r {
        Ok(()) => Ok(p.db.execute_batch("COMMIT;")?),
        Err(e) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

/// The clone's guard: `result` must equal `source` table for table, except that
/// [`IDENTITY_TABLES`] equal `target`'s and the pager member row carries the target's
/// [`PAGER_MEMBER_COLUMNS`]. Returns the identity tables that differ from the source.
pub fn check_clone(result: &Connection, source: &Connection, target: &Connection) -> Result<Vec<String>, Error> {
    let vs_target: Vec<_> = compare(result, target)?;
    let same_as_target = |t: &str| vs_target.iter().any(|d| d.table == t && d.status == TableStatus::Same);
    let target_tables = target.table_names()?;
    let mut changed = Vec::new();
    for d in compare(result, source)? {
        if d.status == TableStatus::Same {
            if IDENTITY_TABLES.contains(&d.table.as_str()) && !same_as_target(&d.table) && target_tables.contains(&d.table) {
                return Err(Error::Invalid(format!("{} doesn't carry the target's identity", d.table)));
            }
            continue;
        }
        if IDENTITY_TABLES.contains(&d.table.as_str()) {
            if !same_as_target(&d.table) {
                return Err(Error::Invalid(format!("{} matches neither the source nor the target", d.table)));
            }
            changed.push(d.table);
            continue;
        }
        if d.table != "tabMember_New" {
            return Err(Error::Invalid(format!("the clone changed {} relative to the source", d.table)));
        }
    }
    // tabMember_New: the source's rows, with the target's identity columns on the pager row.
    let (cols, res) = raw_rows(result, "tabMember_New")?;
    let (scols, src) = raw_rows(source, "tabMember_New")?;
    if cols != scols || res.len() != src.len() {
        return Err(Error::Invalid("the clone changed the rows of tabMember_New".into()));
    }
    let them = pager_row(target)?;
    let radio_no = cols.iter().position(|c| c == "RadioNo").ok_or_else(|| Error::Invalid("tabMember_New has no RadioNo".into()))?;
    let keep: HashSet<usize> = cols.iter().enumerate().filter(|(_, c)| PAGER_MEMBER_COLUMNS.contains(&c.as_str())).map(|(i, _)| i).collect();
    let mut member_changed = false;
    for (a, b) in res.iter().zip(&src) {
        let pager = b[radio_no].as_deref() == Some(PAGER_RADIO_NO);
        for (i, c) in cols.iter().enumerate() {
            let want = if pager && keep.contains(&i) { them.get(c).map(str::to_string) } else { b[i].clone() };
            if a[i] != want {
                return Err(Error::Invalid(format!("tabMember_New.{c} isn't what the clone should write")));
            }
            member_changed |= a[i] != b[i];
        }
    }
    if member_changed {
        changed.push("tabMember_New".into());
    }
    Ok(changed)
}

/// Protocols the source profile enables (`tabProtocolMode`) that the target's doesn't.
pub fn missing_protocols(source: &Profile, target: &Profile) -> Vec<String> {
    let (Some(s), Some(t)) = (load_table(&source.db, "tabProtocolMode").into_iter().next(), load_table(&target.db, "tabProtocolMode").into_iter().next()) else {
        return vec!["tabProtocolMode".into()];
    };
    let label = |c: &str| match c {
        "DMRC" => "DMR".to_string(),
        "P25C" => "P25 conventional".into(),
        "P25T" => "P25 trunking".into(),
        "CtcssCdcss" => "CTCSS/DCS".into(),
        c => c.into(),
    };
    s.cols().iter().filter(|c| *c != "CommunicationMode" && s.s(c) == "1" && t.s(c) != "1").map(|c| label(c)).collect()
}
