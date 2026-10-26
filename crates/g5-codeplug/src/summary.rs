//! Read-only, typed view of the parts of a profile a scanner user cares about.

use serde::Serialize;

use crate::row::{load_table, sel};
use crate::{Error, Profile};

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub name: String,
    pub model: String,
    pub zones: Vec<Zone>,
    pub systems: Vec<System>,
    pub conventional_freqs: usize,
    /// The pager's model supports DMR / P25 conventional channels (`tabProtocolMode`).
    pub dmr: bool,
    pub p25c: bool,
    /// Device-wide settings by key (`settings::read`).
    pub settings: std::collections::BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Zone {
    /// `tabZone_New.ID`.
    pub id: String,
    pub name: String,
    pub knobs: Vec<Knob>,
    /// Knob positions with no channel (`CurrentMode` 0) that a system can be put on.
    pub empty_knobs: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Knob {
    pub index: i64,
    pub name: String,
    pub mode: i64,
    /// System name for trunked knobs, frequency for conventional ones.
    pub target: Option<String>,
    pub talkgroups: usize,
    /// P25 system ID for trunked knobs.
    pub system_id: Option<String>,
    /// Talkgroups on this knob's receive list, in list order.
    pub talkgroup_list: Vec<KnobTalkgroup>,
    /// `ChannelID` of a conventional or trunked knob (scan lists refer to it).
    pub channel_id: Option<String>,
    /// Conventional knobs: analog (carrier/CTCSS/DCS squelch), so it can be scanned.
    #[serde(default)]
    pub analog: bool,
    /// Scan knobs: member `ChannelID`s in scan order.
    pub scan_members: Vec<String>,
    /// Priority scans: the priority member.
    pub scan_priority: Option<String>,
    /// Analog conventional knobs: the settings Knobset can edit.
    pub conv: Option<ConvSettings>,
}

/// An analog conventional knob's editable settings.
#[derive(Debug, Clone, Serialize)]
pub struct ConvSettings {
    pub rx_hz: i64,
    pub bandwidth: i64,
    /// `None` when the knob has more than one tone (Knobset edits one).
    pub squelch: Option<crate::conv_knob::Squelch>,
    pub alert_mode: i64,
    pub record_flag: i64,
    pub voice_prompt: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnobTalkgroup {
    pub group_id: String,
    /// On the knob's priority list (PPS stores priority order as SeqNo 1..n, others as 0).
    pub priority: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Talkgroup {
    pub group_id: String,
    pub tgid: i64,
    pub name: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct System {
    pub id: String,
    pub name: String,
    pub wacn: i64,
    pub system_id: i64,
    pub sites: Vec<Site>,
    pub control_channels_hz: Vec<i64>,
    pub talkgroups: usize,
    pub talkgroup_list: Vec<Talkgroup>,
    /// Referenced by at least one knob (unreferenced systems are not sent to the pager).
    pub in_use: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Site {
    pub name: String,
    pub rfss: String,
    pub site_id: String,
}

/// Receive-mode labels as PPS shows them (`CurrentMode` in `tabReceivingMode_NewG4`).
pub fn mode_label(mode: i64) -> &'static str {
    match mode {
        0 => "None",
        1 => "Selective Call",
        4 => "Normal Scan",
        8 => "Free Scan",
        16 => "Trunking TG-Scan",
        33 => "Monitor",
        36 => "Priority Scan",
        48 => "Trunking TG-Monitor",
        65 => "Multi Selective Call",
        68 => "Silent Scan",
        97 => "Multi Monitor",
        128 => "Radio Compatible",
        129 => "Radio Compatible (Trunking)",
        _ => "Unknown",
    }
}

impl Summary {
    pub fn read(p: &Profile) -> Result<Self, Error> {
        let db = &p.db;
        let name = load_table(db, "tabOrganization_New").first().map(|r| r.s("OrgName").to_string()).unwrap_or_default();
        let model = load_table(db, "tabModel").first().map(|r| r.s("Model").to_string()).unwrap_or_default();

        let p25_knobs = load_table(db, "tabP25TrunkingSystemKnob");
        let conv_knobs = load_table(db, "tabSingleConvCHKnob");
        let channel_groups = load_table(db, "tabChannelGroup");
        let freqs = load_table(db, "tabConvFreqList");
        let scans = load_table(db, "tabMultiChannelScan");
        let tones = load_table(db, "tabRecvSaList");
        let scan_lists = load_table(db, "tabMultiCHScanList");

        let mut systems = Vec::new();
        for s in load_table(db, "tabP25TrunkingSecurity") {
            let id = s.s("P25TrunkingID").to_string();
            let Ok(sys_db) = p.system_db(&id) else { continue };
            let row = load_table(&sys_db, "tabP25TrunkingSystem").into_iter().next();
            systems.push(System {
                name: row.as_ref().map(|r| r.s("Name").to_string()).unwrap_or_else(|| s.s("SystemName").to_string()),
                wacn: row.as_ref().map(|r| r.i("WacnID")).unwrap_or(0),
                system_id: row.as_ref().map(|r| r.i("SystemID")).unwrap_or(0),
                sites: load_table(&sys_db, "tabP25TrunkingSite")
                    .iter()
                    .map(|r| Site { name: r.s("Name").into(), rfss: r.s("RFSSID").into(), site_id: r.s("SiteID").into() })
                    .collect(),
                control_channels_hz: load_table(&sys_db, "tabP25TrunkingControlCH").iter().map(|r| r.i("RxFreq")).collect(),
                talkgroups: load_table(&sys_db, "tabGroup_New").len(),
                talkgroup_list: load_table(&sys_db, "tabGroup_New")
                    .iter()
                    .map(|g| Talkgroup {
                        group_id: g.s("GroupID").into(),
                        tgid: g.get("P25TAddr").and_then(|v| v.trim().parse().ok()).unwrap_or(0),
                        name: g.s("GroupName").into(),
                        note: g.s("Note").into(),
                    })
                    .collect(),
                in_use: !sel(&p25_knobs, "P25TrunkingID", &id).is_empty(),
                id,
            });
        }

        let modes = load_table(db, "tabReceivingMode_NewG4");
        let mut zones = Vec::new();
        for z in load_table(db, "tabZone_New") {
            let zid = z.s("ID");
            let mut knobs = Vec::new();
            let mut empty_knobs = Vec::new();
            for k in sel(&modes, "ZoneID", zid) {
                let mode = k.i("CurrentMode");
                if mode == 0 {
                    empty_knobs.push(k.i("KnobIndex"));
                    continue;
                }
                let ki = k.s("KnobIndex");
                let at = |rows: &[crate::row::Row]| rows.iter().find(|r| r.s("ZoneID") == zid && r.s("KnobIndex") == ki).cloned();
                let mut system_id = None;
                let mut talkgroup_list = Vec::new();
                let mut channel_id = None;
                let mut analog = false;
                let mut scan_members = Vec::new();
                let mut scan_priority = None;
                let mut conv = None;
                let target = if let Some(pk) = at(&p25_knobs) {
                    let sid = pk.s("P25TrunkingID").to_string();
                    let sys = systems.iter().find(|s| s.id == sid).map(|s| s.name.clone());
                    let mut cgs: Vec<_> = sel(&channel_groups, "ChannelID", pk.s("ChannelID"));
                    // Priority list (SeqNo 1..n) first, then the rest in stored order.
                    cgs.sort_by_key(|r| match r.get("SeqNo").and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0) {
                        0 => i64::MAX,
                        n => n,
                    });
                    talkgroup_list = cgs
                        .iter()
                        .map(|r| KnobTalkgroup { group_id: r.s("GroupID").into(), priority: r.get("SeqNo").and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0) > 0 })
                        .collect();
                    system_id = Some(sid);
                    channel_id = Some(pk.s("ChannelID").to_string());
                    sys
                } else if let Some(ck) = at(&conv_knobs) {
                    channel_id = Some(ck.s("ChannelID").to_string());
                    analog = ck.s("ProtocolSettingID") == "0000000000000000";
                    let f = sel(&freqs, "FreqID", ck.s("ConvFreqID")).first().cloned();
                    if analog && let Some(f) = f {
                        let t: Vec<_> = sel(&tones, "ChannelID", ck.s("ChannelID"));
                        let squelch = match t.as_slice() {
                            [] => Some(crate::conv_knob::Squelch::Carrier),
                            [r] if r.s("SaType") == "1" => Some(crate::conv_knob::Squelch::Ctcss { tenths: r.i("ctFreq") }),
                            [r] if r.s("SaType") == "2" => crate::conv_knob::DcsCode::try_from(format!("{:o}", r.i("cdCodeWord")))
                                .ok()
                                .map(|code| crate::conv_knob::Squelch::Dcs { code, inverted: r.s("cdModSense") == "1" }),
                            _ => None,
                        };
                        conv = Some(ConvSettings {
                            rx_hz: f.i("RxFreq"),
                            bandwidth: f.i("RxBandWidth"),
                            squelch,
                            alert_mode: ck.i("AlertMode"),
                            record_flag: ck.i("RecordFlag"),
                            voice_prompt: k.s("AlertToneID").parse().unwrap_or(0),
                        });
                    }
                    sel(&freqs, "FreqID", ck.s("ConvFreqID")).first().map(|f| format!("{:.5} MHz", f.i("RxFreq") as f64 / 1e6))
                } else if let Some(sk) = at(&scans) {
                    // Member rows in table order, as PPS and the converter read them.
                    scan_members = scan_lists.iter().filter(|r| r.s("ZoneID") == zid && r.s("KnobIndex") == ki).map(|r| r.s("ChannelID").to_string()).collect();
                    scan_priority = Some(sk.s("FirstPriorityCH").to_string()).filter(|p| !p.is_empty());
                    Some(format!("{} channels", scan_members.len()))
                } else {
                    None
                };
                knobs.push(Knob {
                    index: k.i("KnobIndex"),
                    name: k.s("ChannelName").into(),
                    mode,
                    target,
                    talkgroups: talkgroup_list.len(),
                    system_id,
                    talkgroup_list,
                    channel_id,
                    analog,
                    scan_members,
                    scan_priority,
                    conv,
                });
            }
            empty_knobs.sort();
            zones.push(Zone { id: zid.to_string(), name: z.s("Name").into(), knobs, empty_knobs });
        }

        let pm = load_table(db, "tabProtocolMode").into_iter().next();
        let (dmr, p25c) = (pm.as_ref().is_some_and(|m| m.s("DMRC") == "1"), pm.as_ref().is_some_and(|m| m.s("P25C") == "1"));
        Ok(Self { name, model, zones, systems, conventional_freqs: freqs.len(), dmr, p25c, settings: crate::settings::read(p) })
    }
}
