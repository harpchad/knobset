//! Removing a knob, and editing an analog conventional knob in place, per
//! `notes/knob-editing.md` 8.2 and 8.3. A removal followed by a new knob on
//! the same position in one batch converts the knob's type (8.5).

use serde::{Deserialize, Serialize};

use crate::conv_knob::{BW_12_5, BW_25, MAX_FREQS, MONITOR_PROMPT, MODE_MONITOR, Squelch};
use crate::edit::refuse_sub_features;
use crate::row::{Row, load_table, sel};
use crate::system_edit::{is_guid, profile_bands};
use crate::{Error, Profile, Templates};

const CSQ: &str = "0000000000000000";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnobRemoval {
    pub zone: String,
    pub knob: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnobKind {
    Trunked,
    Conventional,
    /// DMR or P25 conventional.
    Digital,
    Scan,
}

/// What a removal did.
#[derive(Debug, Clone, Serialize)]
pub struct KnobRemovalChange {
    pub zone: String,
    pub knob: i64,
    pub knob_name: String,
    pub kind: KnobKind,
    /// Scan knobs that lost this channel, as "zone knob name".
    pub scans_changed: Vec<String>,
    /// Priority scans whose priority channel moved to their next member.
    pub priority_moved: Vec<String>,
}

pub(crate) fn zone_id(p: &Profile, zone: &str) -> Result<String, Error> {
    let zones = load_table(&p.db, "tabZone_New");
    let matches = sel(&zones, "Name", zone);
    let [z] = matches.as_slice() else {
        return Err(Error::Invalid(format!("expected exactly one zone named {zone:?}")));
    };
    Ok(z.s("ID").to_string())
}

pub(crate) fn at<'a>(rows: &'a [Row], zid: &str, knob: i64) -> Option<&'a Row> {
    rows.iter().find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == knob)
}

/// The knob's kind, its detail row and ChannelID, checked against its mode.
pub(crate) fn classify(p: &Profile, zid: &str, knob: i64, mode: i64) -> Result<(KnobKind, Option<String>), Error> {
    let conv = load_table(&p.db, "tabSingleConvCHKnob");
    let trunk = load_table(&p.db, "tabP25TrunkingSystemKnob");
    let scan = load_table(&p.db, "tabMultiChannelScan");
    let other = ["tabAdvancedConvCHKnob", "tabAllBandScanCHKnob"].iter().any(|t| at(&load_table(&p.db, t), zid, knob).is_some());
    let (c, t, s) = (at(&conv, zid, knob), at(&trunk, zid, knob), at(&scan, zid, knob));
    match (mode, c, t, s, other) {
        (1 | 33, Some(c), None, None, false) => {
            let kind = if c.s("ProtocolSettingID") == CSQ { KnobKind::Conventional } else { KnobKind::Digital };
            Ok((kind, Some(c.s("ChannelID").to_string())))
        }
        (16 | 48, None, Some(t), None, false) => Ok((KnobKind::Trunked, Some(t.s("ChannelID").to_string()))),
        (4 | 36, None, None, Some(_), false) => Ok((KnobKind::Scan, None)),
        (0, ..) => Err(Error::Invalid("that knob position is already empty".into())),
        (1 | 33 | 16 | 48 | 4 | 36, ..) => Err(Error::Invalid("the knob's detail rows don't match its mode; fix it in PPS".into())),
        (m, ..) => Err(Error::Unsupported(format!("removing or editing a knob in mode {m}"))),
    }
}

/// Make an active knob position empty, cleaning every row that refers to it.
pub fn remove_knob(p: &Profile, r: &KnobRemoval, now: &str) -> Result<KnobRemovalChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let zid = zone_id(p, &r.zone)?;
    let recv = load_table(&p.db, "tabReceivingMode_NewG4");
    let row = at(&recv, &zid, r.knob).ok_or_else(|| Error::Invalid(format!("{:?} has no knob position {}", r.zone, r.knob)))?;
    if !(1..=8).contains(&r.knob) {
        return Err(Error::Invalid("knob positions are 1 to 8".into()));
    }
    let (kind, channel) = classify(p, &zid, r.knob, row.i("CurrentMode"))?;
    let name = row.s("ChannelName").to_string();
    let active = recv.iter().filter(|x| x.i("CurrentMode") != 0 && (1..=8).contains(&x.i("KnobIndex"))).count();
    if active <= 1 {
        return Err(Error::Invalid("the pager needs at least one active knob".into()));
    }
    if let Some(ch) = &channel {
        refuse_sub_features(p, ch)?;
        if load_table(&p.db, "tabEmergencySetting").iter().any(|e| e.s("ChannelID") == ch) {
            return Err(Error::Invalid(format!("{name:?} is the emergency channel; change that in PPS first")));
        }
    }

    // Scans that list this channel: each must keep 2 members; a lost priority moves to the next member.
    let zones = load_table(&p.db, "tabZone_New");
    let label = |z: &str, k: i64| {
        let zn = zones.iter().find(|x| x.s("ID") == z).map(|x| x.s("Name")).unwrap_or("?");
        let kn = recv.iter().find(|x| x.s("ZoneID") == z && x.i("KnobIndex") == k).map(|x| x.s("ChannelName")).unwrap_or("");
        format!("{zn} {k} {kn}")
    };
    let lists = load_table(&p.db, "tabMultiCHScanList");
    let scans = load_table(&p.db, "tabMultiChannelScan");
    let mut scans_changed = Vec::new();
    let mut priority_moves: Vec<(String, i64, String)> = Vec::new();
    // A second priority channel (PPS can set one; Knobset doesn't) is cleared.
    let mut second_cleared: Vec<(String, i64)> = Vec::new();
    if let Some(ch) = &channel {
        for s in &scans {
            let (sz, sk) = (s.s("ZoneID"), s.i("KnobIndex"));
            if sz == zid && sk == r.knob {
                continue;
            }
            let members: Vec<&str> = lists.iter().filter(|m| m.s("ZoneID") == sz && m.i("KnobIndex") == sk).map(|m| m.s("ChannelID")).collect();
            if !members.contains(&ch.as_str()) {
                continue;
            }
            let rest: Vec<&str> = members.iter().copied().filter(|m| m != ch).collect();
            if rest.len() < 2 {
                return Err(Error::Invalid(format!("removing {name:?} would leave scan {} with fewer than 2 channels; edit or remove that scan first", label(sz, sk))));
            }
            scans_changed.push(label(sz, sk));
            if s.s("FirstPriorityCH") == ch {
                priority_moves.push((sz.to_string(), sk, rest[0].to_string()));
            }
            if s.s("SecondPriorityCH") == ch {
                second_cleared.push((sz.to_string(), sk));
            }
        }
    }

    p.db.execute_batch("BEGIN;")?;
    let res = (|| -> Result<(), Error> {
        let k = r.knob.to_string();
        p.db.execute("UPDATE tabReceivingMode_NewG4 SET CurrentMode = '0', AlertToneID = '0', ChannelName = '' WHERE ZoneID = ? AND KnobIndex = ?", &[Some(&zid), Some(&k)])?;
        let freq = at(&load_table(&p.db, "tabSingleConvCHKnob"), &zid, r.knob).map(|c| c.s("ConvFreqID").to_string());
        for t in ["tabSingleConvCHKnob", "tabP25TrunkingSystemKnob", "tabMultiChannelScan", "tabMultiCHScanList", "tabAllBandScanCHKnob", "tabAdvancedConvCHKnob"] {
            p.db.execute(&format!("DELETE FROM {t} WHERE ZoneID = ? AND KnobIndex = ?"), &[Some(&zid), Some(&k)])?;
        }
        if let Some(ch) = &channel {
            for t in [
                "tabChannelGroup",
                "tabChannelGroupList",
                "tabRecvSaList",
                "tabP25TSubGroupEnable",
                "tabP25TChannelSubGroup",
                "tabChannelRevertGroup",
                "tabP25TCallAlertEnable",
                "tabP25TCallAlertSUID",
                "tabP25TCallAlertSUIDGroup",
                "tabP25TChannelRevertSubGroup",
                "tabMultiCHScanList",
                "tabAllBandScanBandParam",
                "tabAllBandScanProtocolParam",
            ] {
                if p.db.table_columns(t).map(|c| c.iter().any(|x| x == "ChannelID")).unwrap_or(false) {
                    p.db.execute(&format!("DELETE FROM {t} WHERE ChannelID = ?"), &[Some(ch)])?;
                }
            }
        }
        for (sz, sk) in &second_cleared {
            p.db.execute("UPDATE tabMultiChannelScan SET SecondPriorityCH = '' WHERE ZoneID = ? AND KnobIndex = ?", &[Some(sz), Some(&sk.to_string())])?;
        }
        for (sz, sk, next) in &priority_moves {
            p.db.execute("UPDATE tabMultiChannelScan SET FirstPriorityCH = ? WHERE ZoneID = ? AND KnobIndex = ?", &[Some(next), Some(sz), Some(&sk.to_string())])?;
        }
        // The knob's own frequency row goes too, unless another knob uses it or it's the last one.
        if let Some(f) = freq {
            let used = ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob"].iter().any(|t| load_table(&p.db, t).iter().any(|c| c.s("ConvFreqID") == f));
            if !used && load_table(&p.db, "tabConvFreqList").len() > 1 {
                p.db.execute("DELETE FROM tabConvFreqList WHERE FreqID = ?", &[Some(&f)])?;
            }
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(())
    })();
    match res {
        Ok(()) => {
            p.db.execute_batch("COMMIT;")?;
            let priority_moved = priority_moves.iter().map(|(z, k, _)| label(z, *k)).collect();
            Ok(KnobRemovalChange { zone: r.zone.clone(), knob: r.knob, knob_name: name, kind, scans_changed, priority_moved })
        }
        Err(e) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}

/// Changes to an existing analog conventional Monitor knob; `None` keeps a value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConvKnobUpdate {
    pub zone: String,
    pub knob: i64,
    pub rx_hz: Option<i64>,
    pub bandwidth: Option<i64>,
    pub squelch: Option<Squelch>,
    /// 1 tone, 2 vibrate, 3 silent, 4 tone and vibrate.
    pub alert_mode: Option<i64>,
    /// Voice storage: 0 off, 1 ID match, 2 all voice.
    pub record_flag: Option<i64>,
    pub voice_prompt: Option<i64>,
    /// New lowercase GUID, used only if the knob's frequency row is shared and a new one is needed.
    pub freq_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConvKnobUpdateChange {
    pub zone: String,
    pub knob: i64,
    pub knob_name: String,
    /// Human-readable list of what changed.
    pub changed: Vec<String>,
}

pub fn update_conv_knob(p: &Profile, templates: &Templates, u: &ConvKnobUpdate, now: &str) -> Result<ConvKnobUpdateChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let zid = zone_id(p, &u.zone)?;
    let recv = load_table(&p.db, "tabReceivingMode_NewG4");
    let row = at(&recv, &zid, u.knob).ok_or_else(|| Error::Invalid(format!("{:?} has no knob position {}", u.zone, u.knob)))?;
    if row.i("CurrentMode") != MODE_MONITOR {
        return Err(Error::Unsupported("editing conventional knobs other than Monitor mode".into()));
    }
    let (kind, channel) = classify(p, &zid, u.knob, MODE_MONITOR)?;
    let (KnobKind::Conventional, Some(ch)) = (kind, channel) else {
        return Err(Error::Unsupported("editing a digital conventional knob in place; remove it and add it again".into()));
    };
    let conv = load_table(&p.db, "tabSingleConvCHKnob");
    let knob_row = at(&conv, &zid, u.knob).unwrap();
    let freqs = load_table(&p.db, "tabConvFreqList");
    let fid = knob_row.s("ConvFreqID").to_string();
    let cur = freqs.iter().find(|f| f.s("FreqID") == fid);
    let mut changed = Vec::new();

    let new_rx = u.rx_hz.filter(|&rx| Some(rx) != cur.map(|f| f.i("RxFreq")));
    let new_bw = u.bandwidth.filter(|&bw| Some(bw) != cur.map(|f| f.i("RxBandWidth")));
    if let Some(bw) = new_bw
        && ![BW_12_5, BW_25].contains(&bw)
    {
        return Err(Error::Invalid(format!("bandwidth code {bw} isn't 12.5 or 25 kHz")));
    }
    if let Some(rx) = new_rx {
        if rx % 10 != 0 {
            return Err(Error::Invalid("frequencies are stored in 10 Hz steps".into()));
        }
        let bands = profile_bands(p, templates)?;
        if !bands.iter().any(|b| b.ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&rx))) {
            return Err(Error::Invalid(format!("{:.5} MHz is outside this pager's bands", rx as f64 / 1e6)));
        }
    }
    let tones: Vec<Row> = load_table(&p.db, "tabRecvSaList").into_iter().filter(|r| r.s("ChannelID") == ch).collect();
    if let Some(Squelch::Ctcss { tenths }) = u.squelch
        && !(670..=2550).contains(&tenths)
    {
        return Err(Error::Invalid("CTCSS tones are 67.0 to 255.0 Hz".into()));
    }
    if u.squelch.is_some() && tones.len() > 1 {
        return Err(Error::Unsupported("changing squelch on a knob with several tones".into()));
    }
    for (v, ok, what) in [
        (u.alert_mode, 1..=4, "alert mode"),
        (u.record_flag, 0..=2, "voice storage"),
    ] {
        if let Some(v) = v
            && !ok.contains(&v)
        {
            return Err(Error::Invalid(format!("{what} {v} is out of range")));
        }
    }
    if let Some(vp) = u.voice_prompt
        && ![0, MONITOR_PROMPT].contains(&vp)
    {
        return Err(Error::Invalid(format!("voice prompt {vp} isn't available for a monitor knob")));
    }
    let shared = ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob"].iter().map(|t| load_table(&p.db, t).iter().filter(|c| c.s("ConvFreqID") == fid).count()).sum::<usize>() != 1;
    if (new_rx.is_some() || new_bw.is_some()) && cur.is_none() && (u.rx_hz.is_none() || u.bandwidth.is_none()) {
        return Err(Error::Invalid("the knob's frequency row is missing; give both frequency and bandwidth".into()));
    }
    let need_new_freq = (new_rx.is_some() || new_bw.is_some()) && (shared || cur.is_none());
    if need_new_freq {
        if !u.freq_id.as_deref().is_some_and(|f| is_guid(f) && !freqs.iter().any(|x| x.s("FreqID") == f)) {
            return Err(Error::Invalid("this knob shares its frequency with another; a new lowercase GUID frequency ID is needed".into()));
        }
        if freqs.len() >= MAX_FREQS {
            return Err(Error::Invalid(format!("the profile already has {MAX_FREQS} conventional frequencies")));
        }
    }

    p.db.execute_batch("BEGIN;")?;
    let res = (|| -> Result<(), Error> {
        let k = u.knob.to_string();
        if new_rx.is_some() || new_bw.is_some() {
            let rx = new_rx.or(cur.map(|f| f.i("RxFreq"))).unwrap().to_string();
            let bw = new_bw.or(cur.map(|f| f.i("RxBandWidth"))).unwrap_or(BW_12_5).to_string();
            if need_new_freq {
                let f = u.freq_id.as_deref().unwrap();
                let names: std::collections::HashSet<&str> = freqs.iter().map(|f| f.s("Name")).collect();
                let name = row.s("ChannelName");
                let name = if names.contains(name) { (1..).map(|n| format!("Frequency {n}")).find(|n| !names.contains(n.as_str())).unwrap() } else { name.to_string() };
                p.db.execute(
                    "INSERT INTO tabConvFreqList(FreqID,Name,TransmitMode,RxFreq,TxFreq,RxBandWidth,TxBandWidth) VALUES(?,?,'1',?,?,?,?)",
                    &[Some(f), Some(&name), Some(&rx), Some(&rx), Some(&bw), Some(&bw)],
                )?;
                p.db.execute("UPDATE tabSingleConvCHKnob SET ConvFreqID = ? WHERE ZoneID = ? AND KnobIndex = ?", &[Some(f), Some(&zid), Some(&k)])?;
            } else {
                p.db.execute("UPDATE tabConvFreqList SET RxFreq = ?, TxFreq = ?, RxBandWidth = ?, TxBandWidth = ? WHERE FreqID = ?", &[Some(&rx), Some(&rx), Some(&bw), Some(&bw), Some(&fid)])?;
            }
            if new_rx.is_some() {
                changed.push(format!("frequency {:.5} MHz", rx.parse::<f64>().unwrap() / 1e6));
            }
            if new_bw.is_some() {
                changed.push(format!("bandwidth {}", if bw == "2" { "25 kHz" } else { "12.5 kHz" }));
            }
        }
        if let Some(sq) = u.squelch {
            p.db.execute("DELETE FROM tabRecvSaList WHERE ChannelID = ?", &[Some(&ch)])?;
            let tone = match sq {
                Squelch::Carrier => None,
                Squelch::Ctcss { tenths } => Some(("1", tenths.to_string(), "0".to_string(), "0")),
                Squelch::Dcs { code, inverted } => Some(("2", "0".into(), u16::from_str_radix(&String::from(code), 8).unwrap().to_string(), if inverted { "1" } else { "0" })),
            };
            if let Some((sa, ct, cd, sense)) = tone {
                p.db.execute(
                    "INSERT INTO tabRecvSaList(ChannelID,SigType,SaType,ctFreq,ctSte,cdCodeWord,cdSte,cdModSense) VALUES(?,'0',?,?,'1',?,'1',?)",
                    &[Some(&ch), Some(sa), Some(&ct), Some(&cd), Some(sense)],
                )?;
            }
            changed.push(format!("squelch {}", sq.label()));
        }
        for (v, col, what) in [(u.alert_mode, "AlertMode", "alert mode"), (u.record_flag, "RecordFlag", "voice storage")] {
            if let Some(v) = v
                && knob_row.i(col) != v
            {
                p.db.execute(&format!("UPDATE tabSingleConvCHKnob SET {col} = ? WHERE ZoneID = ? AND KnobIndex = ?"), &[Some(&v.to_string()), Some(&zid), Some(&k)])?;
                changed.push(what.into());
            }
        }
        if let Some(vp) = u.voice_prompt
            && row.s("AlertToneID") != vp.to_string()
        {
            p.db.execute("UPDATE tabReceivingMode_NewG4 SET AlertToneID = ? WHERE ZoneID = ? AND KnobIndex = ?", &[Some(&vp.to_string()), Some(&zid), Some(&k)])?;
            changed.push("voice prompt".into());
        }
        if changed.is_empty() {
            return Err(Error::Invalid(format!("{:?} knob {} is unchanged", u.zone, u.knob)));
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(())
    })();
    match res {
        Ok(()) => {
            p.db.execute_batch("COMMIT;")?;
            Ok(ConvKnobUpdateChange { zone: u.zone.clone(), knob: u.knob, knob_name: row.s("ChannelName").into(), changed })
        }
        Err(e) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}
