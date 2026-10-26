//! Putting an analog conventional channel on an empty knob position, per
//! `notes/conventional-knobs.md` section 8: Monitor mode (33), carrier
//! squelch or one Rx CTCSS/DCS tone, its own frequency row. P25 conventional
//! and DMR need protocol settings and groupings and come later.

use serde::{Deserialize, Serialize};

use crate::row::{load_table, sel};
use crate::system_edit::{check_name, is_guid, profile_bands};
use crate::{Error, Profile, Templates};

/// `tabReceivingMode_NewG4.CurrentMode` for a conventional monitor knob.
pub const MODE_MONITOR: i64 = 33;
/// PPS's built-in "Monitor Mode" knob voice prompt (Knobset defaults to none).
pub const MONITOR_PROMPT: i64 = 2;
/// `tabConvFreqList` holds at most this many frequencies.
pub const MAX_FREQS: usize = 1024;
/// Carrier squelch: the protocol setting ID PPS uses for "CSQ".
const CSQ: &str = "0000000000000000";
const ZERO_KEY: &str = "000000000000";

/// The standard EIA CTCSS tones, in tenths of Hz. PPS offers 42 of these and
/// accepts any value from 67.0 to 255.0 Hz.
pub const CTCSS_TONES: &[i64] = &[
    670, 693, 719, 744, 770, 797, 825, 854, 885, 915, 948, 974, 1000, 1035, 1072, 1109, 1148, 1188, 1230, 1273, 1318, 1365, 1413, 1462, 1514, 1567, 1598,
    1622, 1655, 1679, 1713, 1738, 1773, 1799, 1835, 1862, 1899, 1928, 1966, 1995, 2035, 2065, 2107, 2181, 2257, 2291, 2336, 2418, 2503, 2541,
];

/// The 104 DCS codes PPS offers, as written (octal).
pub const DCS_CODES: &[&str] = &[
    "023", "025", "026", "031", "032", "036", "043", "047", "051", "053", "054", "065", "071", "072", "073", "074", "114", "115", "116", "122", "125", "131",
    "132", "134", "143", "152", "155", "156", "162", "165", "172", "174", "205", "212", "223", "225", "226", "243", "244", "245", "246", "251", "252", "261",
    "263", "265", "266", "271", "306", "311", "315", "325", "331", "343", "346", "351", "364", "365", "371", "411", "412", "413", "423", "425", "431", "432",
    "445", "446", "452", "455", "464", "465", "466", "503", "506", "516", "521", "525", "532", "546", "552", "564", "565", "606", "612", "624", "627", "631",
    "632", "645", "652", "654", "662", "664", "703", "712", "723", "725", "726", "731", "732", "734", "743", "754",
];

/// How the channel opens. Tones are receive-only (the pager doesn't transmit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Squelch {
    /// Opens on any carrier.
    Carrier,
    /// Rx CTCSS tone in tenths of Hz (1273 = 127.3 Hz), 670..=2550.
    Ctcss { tenths: i64 },
    /// Rx DCS code as written ("023"), from `DCS_CODES`.
    Dcs {
        code: DcsCode,
        #[serde(default)]
        inverted: bool,
    },
}

/// A DCS code kept as its octal text so the wire format matches what users read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DcsCode(u16);

impl TryFrom<String> for DcsCode {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        let t = format!("{:0>3}", s.trim());
        if !DCS_CODES.contains(&t.as_str()) {
            return Err(format!("DCS code {s:?} isn't one the pager offers"));
        }
        Ok(DcsCode(u16::from_str_radix(&t, 8).map_err(|e| e.to_string())?))
    }
}

impl From<DcsCode> for String {
    fn from(c: DcsCode) -> String {
        format!("{:03o}", c.0)
    }
}

impl Squelch {
    pub fn label(&self) -> String {
        match self {
            Squelch::Carrier => "carrier".into(),
            Squelch::Ctcss { tenths } => format!("CTCSS {}.{} Hz", tenths / 10, tenths % 10),
            Squelch::Dcs { code, inverted } => format!("DCS {}{}", String::from(*code), if *inverted { " inverted" } else { "" }),
        }
    }
}

/// Bandwidth codes in `tabConvFreqList`.
pub const BW_12_5: i64 = 1;
pub const BW_25: i64 = 2;

/// Put an analog conventional channel on an empty knob position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvKnobEdit {
    pub zone: String,
    pub knob: i64,
    /// Knob alias, 1..14 printable ASCII.
    pub name: String,
    pub rx_hz: i64,
    /// `BW_12_5` or `BW_25`. (PPS also offers 20 kHz; untested on a pager.)
    pub bandwidth: i64,
    pub squelch: Squelch,
    /// 0 (none) or `MONITOR_PROMPT`.
    #[serde(default)]
    pub voice_prompt: i64,
    /// New lowercase GUIDs chosen by the caller.
    pub channel_id: String,
    pub freq_id: String,
}

/// What adding the knob did.
#[derive(Debug, Clone, Serialize)]
pub struct ConvKnobChange {
    pub zone: String,
    pub knob: i64,
    pub knob_name: String,
    pub rx_hz: i64,
    pub bandwidth: i64,
    pub squelch: String,
}

/// The `tabSingleConvCHKnob` row PPS saves for a new carrier-squelch Monitor
/// knob with untouched controls (conventional-knobs 1.2).
pub(crate) fn default_knob_row(cols: &[String], zone_id: &str, knob: i64, channel: &str, freq: &str) -> Result<Vec<String>, Error> {
    let knob = knob.to_string();
    cols.iter()
        .map(|c| {
            Ok(match c.as_str() {
                "ZoneID" => zone_id,
                "KnobIndex" => &knob,
                "ChannelID" => channel,
                "ConvFreqID" => freq,
                "ProtocolSettingID" => CSQ,
                // Sub-audio lives in tabRecvSaList; PPS's editor leaves these blank.
                "RxSaType" | "RxCTFreq" | "RxCDCodeWord" | "RxCDModSense" | "TxSaType" | "TxCTFreq" | "TxCDCodeWord" | "TxCDModSense" | "TxNAC" | "TxColorCode"
                | "Encryption" | "DefaultGroupID" => "",
                "Emphasis" | "TransmitMode" | "VoiceBuffering" | "AlertMode" | "RecordFlag" | "AlwaysUnreadReminderAlert" => "1",
                "TxPowerLevel" => "2",
                "SquelchOption" => "3",
                "HangTime" => "50",
                "DelayNTime" | "CarrierOffDelayTime" | "TgidStayTime" => "30",
                "DelayNTimeEx" => "5",
                "EncryptionKeyID" | "KMFSystemID" | "PrivateEncryptionKeyID" => ZERO_KEY,
                "RxNAC" | "RxColorCode" | "Compander" | "AudioSquelch" | "OpenSpeakerLogic" | "DispatchCHFlag" | "RFPerformance" | "PushToListen" | "Privacy"
                | "VoiceBufferPrompt" | "VoiceBufferingForceDrop" | "ResetMode" | "EnableCarrierOffTime" | "DutyStatus" | "AlwaysUnreadVoiceMessage"
                | "DelayNFeature" | "EnableTgidStayTime" | "EncryptionBy" | "KeyStrapOption" | "PrivateKeyStrapOption" | "UserSelectableKeyset" | "UserSelectableKey"
                | "EncryptedCallFlag" => "0",
                other => return Err(Error::Unsupported(format!("unknown conventional knob column {other}"))),
            }
            .to_string())
        })
        .collect()
}

/// Apply `e` to a working copy. `now` is the local time for `tabOrganization_New.FinishDate`.
pub fn add_conv_knob(p: &Profile, templates: &Templates, e: &ConvKnobEdit, now: &str) -> Result<ConvKnobChange, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    check_name("Knob", &e.name)?;
    if ![BW_12_5, BW_25].contains(&e.bandwidth) {
        return Err(Error::Invalid(format!("bandwidth code {} isn't 12.5 or 25 kHz", e.bandwidth)));
    }
    if ![0, MONITOR_PROMPT].contains(&e.voice_prompt) {
        return Err(Error::Invalid(format!("voice prompt {} isn't available for a monitor knob", e.voice_prompt)));
    }
    if let Squelch::Ctcss { tenths } = e.squelch
        && !(670..=2550).contains(&tenths)
    {
        return Err(Error::Invalid("CTCSS tones are 67.0 to 255.0 Hz".into()));
    }
    if e.rx_hz % 10 != 0 {
        return Err(Error::Invalid("frequencies are stored in 10 Hz steps".into()));
    }
    let bands = profile_bands(p, templates)?;
    if !bands.iter().any(|b| b.ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&e.rx_hz))) {
        return Err(Error::Invalid(format!("{:.5} MHz is outside this pager's bands", e.rx_hz as f64 / 1e6)));
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
    if !(1..=8).contains(&e.knob) || r.i("CurrentMode") != 0 {
        return Err(Error::Invalid(format!("{:?} knob {} isn't an empty position", e.zone, e.knob)));
    }
    // PPS would silently reuse a leftover row; Knobset refuses instead.
    for t in ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob", "tabP25TrunkingSystemKnob", "tabMultiChannelScan", "tabAllBandScanCHKnob"] {
        if load_table(&p.db, t).iter().any(|k| k.s("ZoneID") == zid && k.i("KnobIndex") == e.knob) {
            return Err(Error::Invalid(format!("{:?} knob {} is empty but has a leftover row in {t}", e.zone, e.knob)));
        }
    }
    let freqs = load_table(&p.db, "tabConvFreqList");
    if freqs.len() >= MAX_FREQS {
        return Err(Error::Invalid(format!("the profile already has {MAX_FREQS} conventional frequencies")));
    }
    for (what, id) in [("channel", &e.channel_id), ("frequency", &e.freq_id)] {
        if !is_guid(id) {
            return Err(Error::Invalid(format!("the new {what} ID must be a lowercase GUID")));
        }
    }
    if freqs.iter().any(|f| f.s("FreqID") == e.freq_id) {
        return Err(Error::Invalid(format!("frequency {} already exists", e.freq_id)));
    }
    let channel_taken = ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob", "tabP25TrunkingSystemKnob"]
        .iter()
        .any(|t| load_table(&p.db, t).iter().any(|k| k.s("ChannelID") == e.channel_id));
    if channel_taken {
        return Err(Error::Invalid(format!("channel {} already exists", e.channel_id)));
    }
    // The frequency takes the knob's name unless another frequency has it (PPS's GetNextName otherwise).
    let names: std::collections::HashSet<&str> = freqs.iter().map(|f| f.s("Name")).collect();
    let freq_name = if names.contains(e.name.as_str()) {
        (1..).map(|n| format!("Frequency {n}")).find(|n| !names.contains(n.as_str())).unwrap()
    } else {
        e.name.clone()
    };

    p.db.execute_batch("BEGIN;")?;
    let res = (|| -> Result<(), Error> {
        let (rx, bw) = (e.rx_hz.to_string(), e.bandwidth.to_string());
        // Tx = Rx, as every PPS creation path writes; the pager never transmits on it.
        p.db.execute(
            "INSERT INTO tabConvFreqList(FreqID,Name,TransmitMode,RxFreq,TxFreq,RxBandWidth,TxBandWidth) VALUES(?,?,'1',?,?,?,?)",
            &[Some(&e.freq_id), Some(&freq_name), Some(&rx), Some(&rx), Some(&bw), Some(&bw)],
        )?;
        p.db.execute(
            "UPDATE tabReceivingMode_NewG4 SET CurrentMode = ?, ChannelName = ?, AlertToneID = ?, AlertFileName = coalesce(AlertFileName, '') WHERE ZoneID = ? AND KnobIndex = ?",
            &[Some(&MODE_MONITOR.to_string()), Some(&e.name), Some(&e.voice_prompt.to_string()), Some(&zid), Some(r.s("KnobIndex"))],
        )?;
        let cols = p.db.table_columns("tabSingleConvCHKnob")?;
        let row = default_knob_row(&cols, &zid, e.knob, &e.channel_id, &e.freq_id)?;
        let params: Vec<Option<&str>> = row.iter().map(|v| Some(v.as_str())).collect();
        p.db.execute(&format!("INSERT INTO tabSingleConvCHKnob VALUES({})", vec!["?"; row.len()].join(",")), &params)?;
        let tone = match e.squelch {
            Squelch::Carrier => None,
            Squelch::Ctcss { tenths } => Some(("1", tenths.to_string(), "0".to_string(), "0")),
            Squelch::Dcs { code, inverted } => Some(("2", "0".into(), code.0.to_string(), if inverted { "1" } else { "0" })),
        };
        if let Some((sa, ct, cd, sense)) = tone {
            p.db.execute(
                "INSERT INTO tabRecvSaList(ChannelID,SigType,SaType,ctFreq,ctSte,cdCodeWord,cdSte,cdModSense) VALUES(?,'0',?,?,'1',?,'1',?)",
                &[Some(&e.channel_id), Some(sa), Some(&ct), Some(&cd), Some(sense)],
            )?;
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(())
    })();
    match res {
        Ok(()) => {
            p.db.execute_batch("COMMIT;")?;
            Ok(ConvKnobChange { zone: e.zone.clone(), knob: e.knob, knob_name: e.name.clone(), rx_hz: e.rx_hz, bandwidth: e.bandwidth, squelch: e.squelch.label() })
        }
        Err(err) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(err)
        }
    }
}
