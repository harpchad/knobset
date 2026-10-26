//! Digital conventional channels: DMR (color code, timeslot) and P25
//! conventional (NAC) on an empty knob position, in Monitor mode, per
//! `notes/digital-conventional.md` section 9. Unverified on a pager: the
//! converter reports them as experimental.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::conv_knob::{BW_12_5, MAX_FREQS, MODE_MONITOR, MONITOR_PROMPT, default_knob_row};
use crate::row::{Row, load_table, sel};
use crate::system_edit::{check_name, is_guid, profile_bands};
use crate::{Error, Profile, Templates};

/// `tabProtocolSetting.ProtocolType` for DMR conventional.
pub const TYPE_DMR: i64 = 4;
/// `tabProtocolSetting.ProtocolType` for P25 conventional.
pub const TYPE_P25C: i64 = 8;
pub const MAX_PROTOCOL_SETTINGS: usize = 16;
/// Talkgroups per digital knob (PPS's DMR paste helper caps at 15, P25C at 20).
pub const MAX_DMR_TALKGROUPS: usize = 15;
pub const MAX_P25C_TALKGROUPS: usize = 20;
/// The all-call address used when a Monitor knob lists no talkgroups.
pub const DMR_ALL: i64 = 16_777_215;
pub const P25C_ALL: i64 = 65_535;
/// "Receive any NAC" (PPS help); 0xF7F is not allowed.
pub const NAC_ANY: i64 = 0xF7E;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DigitalProtocol {
    /// DMR: timeslot 1 or 2, color code 0..15.
    Dmr { slot: i64, color_code: i64 },
    /// P25 conventional: NAC 0..0xFFF. With `filter_nac` the knob opens only
    /// on that NAC (squelch option Normal); otherwise Monitor ignores it.
    P25c { nac: i64, #[serde(default)] filter_nac: bool },
}

impl DigitalProtocol {
    fn ptype(&self) -> i64 {
        match self {
            DigitalProtocol::Dmr { .. } => TYPE_DMR,
            DigitalProtocol::P25c { .. } => TYPE_P25C,
        }
    }
    pub fn label(&self) -> String {
        match self {
            DigitalProtocol::Dmr { slot, color_code } => format!("DMR slot {slot}, color code {color_code}"),
            DigitalProtocol::P25c { nac, filter_nac } => format!("P25 NAC {nac:03X}{}", if *filter_nac { " only" } else { "" }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigitalTalkgroup {
    /// Decimal talkgroup ID.
    pub id: i64,
    pub name: String,
}

/// A DMR or P25 conventional Monitor knob on an empty position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigitalKnobEdit {
    pub zone: String,
    pub knob: i64,
    pub name: String,
    pub rx_hz: i64,
    pub protocol: DigitalProtocol,
    /// Talkgroups to list; empty means the all-call placeholder.
    #[serde(default)]
    pub talkgroups: Vec<DigitalTalkgroup>,
    /// Repeater mode instead of talkaround (`TransmitMode 0`).
    #[serde(default)]
    pub repeater_mode: bool,
    #[serde(default)]
    pub voice_prompt: i64,
    /// New lowercase GUIDs chosen by the caller. Further IDs (protocol
    /// setting, groups, grouping) are derived from `channel_id`.
    pub channel_id: String,
    pub freq_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DigitalKnobChange {
    pub zone: String,
    pub knob: i64,
    pub knob_name: String,
    pub rx_hz: i64,
    pub protocol: String,
    pub dmr: bool,
    pub talkgroups: usize,
    /// New talkgroups added to the profile (renumbers the radio's contacts).
    pub new_groups: usize,
}

/// IDs derived from the caller's GUID: same first 24 characters, a counter
/// in the last 12, skipping any already used.
struct Ids {
    prefix: String,
    n: u64,
    taken: HashSet<String>,
}

impl Ids {
    fn next(&mut self) -> String {
        loop {
            self.n += 1;
            let id = format!("{}{:012x}", self.prefix, self.n);
            if self.taken.insert(id.clone()) {
                return id;
            }
        }
    }
}

/// Smallest `"{base} N"` not in `names` (PPS's GetNextName).
fn next_name(base: &str, names: &HashSet<String>) -> String {
    (1..).map(|n| format!("{base} {n}")).find(|n| !names.contains(n)).unwrap()
}

fn next_no(rows: &[Row], col: &str) -> i64 {
    let used: HashSet<i64> = rows.iter().filter_map(|r| r.s(col).trim().parse().ok()).collect();
    (1..).find(|n| !used.contains(n)).unwrap()
}

pub fn add_digital_knob(p: &Profile, templates: &Templates, e: &DigitalKnobEdit, now: &str) -> Result<DigitalKnobChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    check_name("Knob", &e.name)?;
    let ptype = e.protocol.ptype();
    let mode = load_table(&p.db, "tabProtocolMode").into_iter().next();
    let supported = mode.as_ref().is_some_and(|m| m.s(if ptype == TYPE_DMR { "DMRC" } else { "P25C" }) == "1");
    if !supported {
        return Err(Error::Unsupported(format!("{} on this pager model", if ptype == TYPE_DMR { "DMR" } else { "P25 conventional" })));
    }
    match e.protocol {
        DigitalProtocol::Dmr { slot, color_code } => {
            if ![1, 2].contains(&slot) {
                return Err(Error::Invalid("DMR timeslot is 1 or 2".into()));
            }
            if !(0..=15).contains(&color_code) {
                return Err(Error::Invalid("DMR color code is 0 to 15".into()));
            }
            if load_table(&p.db, "tabVoiceCallVocoderSetting").is_empty() {
                return Err(Error::Invalid("the profile has no DMR vocoder setting".into()));
            }
        }
        DigitalProtocol::P25c { nac, .. } => {
            if !(0..=0xFFF).contains(&nac) || nac == 0xF7F {
                return Err(Error::Invalid("NAC is 000 to FFF, and F7F isn't allowed".into()));
            }
        }
    }
    if ![0, MONITOR_PROMPT].contains(&e.voice_prompt) {
        return Err(Error::Invalid(format!("voice prompt {} isn't available for a monitor knob", e.voice_prompt)));
    }
    if e.rx_hz % 10 != 0 {
        return Err(Error::Invalid("frequencies are stored in 10 Hz steps".into()));
    }
    let bands = profile_bands(p, templates)?;
    if !bands.iter().any(|b| b.ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&e.rx_hz))) {
        return Err(Error::Invalid(format!("{:.5} MHz is outside this pager's bands", e.rx_hz as f64 / 1e6)));
    }
    let (max_tg, tg_range): (usize, &dyn Fn(i64) -> bool) = if ptype == TYPE_DMR {
        (MAX_DMR_TALKGROUPS, &|v| (1..=16_776_415).contains(&v) || (16_777_200..=16_777_215).contains(&v))
    } else {
        (MAX_P25C_TALKGROUPS, &|v| (1..=65_535).contains(&v))
    };
    if e.talkgroups.len() > max_tg {
        return Err(Error::Invalid(format!("at most {max_tg} talkgroups on this kind of knob")));
    }
    let mut seen = HashSet::new();
    for t in &e.talkgroups {
        check_name("Talkgroup", &t.name)?;
        if !tg_range(t.id) {
            return Err(Error::Invalid(format!("talkgroup {} is out of range", t.id)));
        }
        if !seen.insert(t.id) {
            return Err(Error::Invalid(format!("talkgroup {} is listed twice", t.id)));
        }
    }

    // The position, as for an analog knob.
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
    if !(1..=8).contains(&e.knob) || r.i("CurrentMode") != 0 {
        return Err(Error::Invalid(format!("{:?} knob {} isn't an empty position", e.zone, e.knob)));
    }
    for t in ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob", "tabP25TrunkingSystemKnob", "tabMultiChannelScan", "tabAllBandScanCHKnob"] {
        if load_table(&p.db, t).iter().any(|k| k.s("ZoneID") == zid && k.i("KnobIndex") == e.knob) {
            return Err(Error::Invalid(format!("{:?} knob {} is empty but has a leftover row in {t}", e.zone, e.knob)));
        }
    }
    for (what, id) in [("channel", &e.channel_id), ("frequency", &e.freq_id)] {
        if !is_guid(id) {
            return Err(Error::Invalid(format!("the new {what} ID must be a lowercase GUID")));
        }
    }
    let channel_taken = ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob", "tabP25TrunkingSystemKnob"]
        .iter()
        .any(|t| load_table(&p.db, t).iter().any(|k| k.s("ChannelID") == e.channel_id));
    if channel_taken {
        return Err(Error::Invalid(format!("channel {} already exists", e.channel_id)));
    }
    let freqs = load_table(&p.db, "tabConvFreqList");
    if freqs.len() >= MAX_FREQS || freqs.iter().any(|f| f.s("FreqID") == e.freq_id) {
        return Err(Error::Invalid("can't add the frequency (full, or the ID exists)".into()));
    }

    let settings = load_table(&p.db, "tabProtocolSetting");
    let dmr_sig = load_table(&p.db, "tabDMRSignaling");
    let dmr_rep = load_table(&p.db, "tabDMRRepeaterSetting");
    let p25_sig = load_table(&p.db, "tabP25Signaling");
    let groups = load_table(&p.db, "tabGroup_New");
    let groupings = load_table(&p.db, "tabGroupGrouping");
    let gog = load_table(&p.db, "tabGroupingOfGroup");
    let mut ids = Ids {
        prefix: e.channel_id[..24].to_string(),
        n: 0,
        taken: [&settings[..], &groups[..], &groupings[..]]
            .iter()
            .flat_map(|t| t.iter().flat_map(|r| ["ID", "GroupID", "GroupingID"].into_iter().map(|c| r.s(c).to_string())))
            .chain([e.channel_id.clone(), e.freq_id.clone()])
            .collect(),
    };

    // Reuse a protocol setting of the same type with PPS's default parameters (and, for DMR, the same slot).
    let dmr_default = |id: &str| {
        sel(&dmr_sig, "ID", id).first().is_some_and(|s| {
            [("TXPreFrames", "4"), ("RCRxEnable", "0"), ("NoSigActTimeout", "600"), ("ACKWaitTimeout", "5000"), ("MFID", "0"), ("SAP4NRS", "4"), ("SAP4RS", "5")]
                .iter()
                .all(|(c, v)| s.s(c) == *v)
        })
    };
    let p25_default = |id: &str| {
        sel(&p25_sig, "ID", id).first().is_some_and(|s| [("TXPreTime", "20"), ("MFID", "0"), ("AlgId", "133")].iter().all(|(c, v)| s.s(c) == *v))
    };
    let reuse = settings.iter().find(|s| {
        s.i("ProtocolType") == ptype
            && match e.protocol {
                DigitalProtocol::Dmr { slot, .. } => dmr_default(s.s("ID")) && sel(&dmr_rep, "ID", s.s("ID")).first().is_some_and(|r| r.i("TimeSlot") == slot - 1),
                DigitalProtocol::P25c { .. } => p25_default(s.s("ID")),
            }
    });
    let new_setting = reuse.is_none();
    if new_setting && settings.len() >= MAX_PROTOCOL_SETTINGS {
        return Err(Error::Invalid(format!("the profile already has {MAX_PROTOCOL_SETTINGS} protocol settings")));
    }
    let setting_id = reuse.map(|s| s.s("ID").to_string()).unwrap_or_else(|| ids.next());

    // Talkgroups: reuse a group with the same address and name.
    let addr_col = if ptype == TYPE_DMR { "DMRAddr" } else { "P25Addr" };
    let list: Vec<DigitalTalkgroup> = if e.talkgroups.is_empty() {
        vec![DigitalTalkgroup { id: if ptype == TYPE_DMR { DMR_ALL } else { P25C_ALL }, name: "All".into() }]
    } else {
        e.talkgroups.clone()
    };
    let mut member_ids = Vec::new();
    let mut new_groups: Vec<(String, &DigitalTalkgroup)> = Vec::new();
    for t in &list {
        match groups.iter().find(|g| g.s(addr_col) == t.id.to_string() && g.s("GroupName") == t.name) {
            Some(g) => member_ids.push(g.s("GroupID").to_string()),
            None => {
                let id = ids.next();
                member_ids.push(id.clone());
                new_groups.push((id, t));
            }
        }
    }
    // Reuse a grouping of the same type with exactly these members.
    let reuse_grouping = groupings.iter().find(|g| {
        g.i("Type") == ptype && sel(&gog, "GroupingID", g.s("GroupingID")).iter().map(|m| m.s("GroupID").to_string()).collect::<Vec<_>>() == member_ids
    });
    let grouping_id = reuse_grouping.map(|g| g.s("GroupingID").to_string()).unwrap_or_else(|| ids.next());
    if groups.len() + new_groups.len() > 8192 || (reuse_grouping.is_none() && groupings.len() >= 1024) {
        return Err(Error::Invalid("the profile's talkgroup or talkgroup-list limit is reached".into()));
    }

    p.db.execute_batch("BEGIN;")?;
    let res = (|| -> Result<(), Error> {
        if new_setting {
            let names: HashSet<String> = settings.iter().map(|s| s.s("ProtocolName").to_string()).collect();
            let base = match e.protocol {
                DigitalProtocol::Dmr { slot, .. } => format!("DMR TS{slot}"),
                DigitalProtocol::P25c { .. } => "P25 Conv".into(),
            };
            let name = if names.contains(&base) { next_name("Protocol", &names) } else { base };
            let no = next_no(&settings, "ProtocolNo").to_string();
            p.db.execute(
                "INSERT INTO tabProtocolSetting(ID,ProtocolNo,ProtocolName,ProtocolType) VALUES(?,?,?,?)",
                &[Some(&setting_id), Some(&no), Some(&name), Some(&ptype.to_string())],
            )?;
            match e.protocol {
                DigitalProtocol::Dmr { slot, .. } => {
                    p.db.execute(
                        "INSERT INTO tabDMRSignaling(ID,TXPreFrames,RCRxEnable,NoSigActTimeout,ACKWaitTimeout,MFID,SAP4NRS,SAP4RS,DMRCompatibility) VALUES(?,'4','0','600','5000','0','4','5','2')",
                        &[Some(&setting_id)],
                    )?;
                    p.db.execute("INSERT INTO tabDMRRepeaterSetting(ID,TimeSlot,TimingAlign) VALUES(?,?,'0')", &[Some(&setting_id), Some(&(slot - 1).to_string())])?;
                }
                DigitalProtocol::P25c { .. } => {
                    p.db.execute(
                        "INSERT INTO tabP25Signaling(ID,TXPreTime,NoSigActTimeout,ACKWaitTimeout,MFID,AlgId,MinShortDigitalToneDuration,MinLongDigitalToneDuration,MinToneAddrGap,OTAToneFlag,ShortDigitalToneATxDuration,ShortDigitalToneBTxDuration,LongDigitalToneTxDuration,TxToneAddrGap) VALUES(?,'20','600','5000','0','133','200','2500','1000','1','1000','2000','5000','1500')",
                        &[Some(&setting_id)],
                    )?;
                }
            }
        }
        let key = load_table(&p.db, "tabEncryptionKeyList").first().map(|k| k.s("ID").to_string()).unwrap_or_else(|| "000000000000".into());
        // Each new group takes the smallest number still free.
        let mut used: HashSet<i64> = groups.iter().filter_map(|g| g.s("GroupNo").trim().parse().ok()).collect();
        for (id, t) in &new_groups {
            let no = (1..).find(|n| !used.contains(n)).unwrap();
            used.insert(no);
            let (dmr, p25) = if ptype == TYPE_DMR { (t.id.to_string(), String::new()) } else { (String::new(), t.id.to_string()) };
            p.db.execute(
                "INSERT INTO tabGroup_New(GroupID,GroupName,GroupNo,Note,RecordFlag,Type,Relay,Priority,DutyStatus,AlertMode,AlertToneID,BackgroundColor,EncryptionKeyID,KeyStrapOption,TwoToneAddr,FiveToneAddr,MDCAddr,UDCAddr,DMRAddr,P25Addr,P25Sub1Addr,P25Sub2Addr,P25Sub3Addr) VALUES(?,?,?,'','1','0','0','0','0','1','0','0',?,'2','','','','',?,?,'','','')",
                &[Some(id), Some(&t.name), Some(&no.to_string()), Some(&key), Some(&dmr), Some(&p25)],
            )?;
        }
        if reuse_grouping.is_none() {
            let names: HashSet<String> = groupings.iter().map(|g| g.s("GroupingAlias").to_string()).collect();
            p.db.execute(
                "INSERT INTO tabGroupGrouping(GroupingID,GroupingAlias,Type,Note) VALUES(?,?,?,'')",
                &[Some(&grouping_id), Some(&next_name("TGID List", &names)), Some(&ptype.to_string())],
            )?;
            for m in &member_ids {
                p.db.execute("INSERT INTO tabGroupingOfGroup(GroupingID,GroupID) VALUES(?,?)", &[Some(&grouping_id), Some(m)])?;
            }
        }
        // Frequency (12.5 kHz: DMR and P25C must be narrow) and the knob, as for analog.
        let names: HashSet<String> = freqs.iter().map(|f| f.s("Name").to_string()).collect();
        let freq_name = if names.contains(&e.name) { next_name("Frequency", &names) } else { e.name.clone() };
        let (rx, bw) = (e.rx_hz.to_string(), BW_12_5.to_string());
        p.db.execute(
            "INSERT INTO tabConvFreqList(FreqID,Name,TransmitMode,RxFreq,TxFreq,RxBandWidth,TxBandWidth) VALUES(?,?,'1',?,?,?,?)",
            &[Some(&e.freq_id), Some(&freq_name), Some(&rx), Some(&rx), Some(&bw), Some(&bw)],
        )?;
        p.db.execute(
            "UPDATE tabReceivingMode_NewG4 SET CurrentMode = ?, ChannelName = ?, AlertToneID = ?, AlertFileName = coalesce(AlertFileName, '') WHERE ZoneID = ? AND KnobIndex = ?",
            &[Some(&MODE_MONITOR.to_string()), Some(&e.name), Some(&e.voice_prompt.to_string()), Some(&zid), Some(r.s("KnobIndex"))],
        )?;
        let cols = p.db.table_columns("tabSingleConvCHKnob")?;
        let mut row = default_knob_row(&cols, &zid, e.knob, &e.channel_id, &e.freq_id)?;
        let (cc, nac, sq) = match e.protocol {
            DigitalProtocol::Dmr { color_code, .. } => (color_code, 0, 3),
            DigitalProtocol::P25c { nac, filter_nac } => (0, nac, if filter_nac { 1 } else { 3 }),
        };
        for (c, v) in [
            ("ProtocolSettingID", setting_id.clone()),
            ("RxColorCode", cc.to_string()),
            ("RxNAC", nac.to_string()),
            ("SquelchOption", sq.to_string()),
            ("TransmitMode", if e.repeater_mode { "0" } else { "1" }.into()),
        ] {
            let i = cols.iter().position(|x| x == c).ok_or_else(|| Error::Unsupported(format!("knob table has no {c} column")))?;
            row[i] = v;
        }
        let params: Vec<Option<&str>> = row.iter().map(|v| Some(v.as_str())).collect();
        p.db.execute(&format!("INSERT INTO tabSingleConvCHKnob VALUES({})", vec!["?"; row.len()].join(",")), &params)?;
        p.db.execute("INSERT INTO tabChannelGroupList(ChannelID,GroupingID,isRevert) VALUES(?,?,'0')", &[Some(&e.channel_id), Some(&grouping_id)])?;
        // PPS appends the knob's member rows last member first, SeqNo = list position.
        for (i, m) in member_ids.iter().enumerate().rev() {
            p.db.execute(
                "INSERT INTO tabChannelGroup(ChannelID,GroupID,SigType,SeqNo,OffDuty,Priority,OutOfRange,OTA,VoiceRecvAck,VoiceReadAck,VoiceManualAck,TextRecvAck,TextReadAck,TextManualAck,ImageRecvAck,ImageReadAck,ImageManualAck) VALUES(?,?,?,?,'0','0','0','0','0','0','0','0','0','0','0','0','0')",
                &[Some(&e.channel_id), Some(m), Some(&ptype.to_string()), Some(&(i + 1).to_string())],
            )?;
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(())
    })();
    match res {
        Ok(()) => {
            p.db.execute_batch("COMMIT;")?;
            Ok(DigitalKnobChange {
                zone: e.zone.clone(),
                knob: e.knob,
                knob_name: e.name.clone(),
                rx_hz: e.rx_hz,
                protocol: e.protocol.label(),
                dmr: ptype == TYPE_DMR,
                talkgroups: list.len(),
                new_groups: new_groups.len(),
            })
        }
        Err(err) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(err)
        }
    }
}
