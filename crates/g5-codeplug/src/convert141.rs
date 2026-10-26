//! Profile → radio DB (`u3db.db`) for U3DB versions DBV1.41b through DBV1.41e.
//!
//! Implements the behavior of PPS V00-04-00-48's
//! `PPSU4ParamConverter_01P41b` through `_01P41e`, specified in
//! `notes/converter-1.41.md` and validated row-for-row against real pagers. The structure follows the
//! Python model used for that validation (`convertZone`, `convertContacts`, ...)
//! so the two can be compared side by side.
//!
//! Feature paths the real profiles never exercised are refused
//! (`Error::Unsupported`) instead of guessed, so we never upload a radio DB
//! whose generation we haven't verified.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::rc::Rc;

use sqlite2::Connection;
use tracing::debug;

use crate::row::{Row, load_table, parse_int, record_error, sel, take_errors};
use crate::source::{Profile, Templates};
use crate::Error;

/// Ordered by version, so `>= Variant::C` means "1.41c or later".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variant {
    B,
    C,
    /// 1.41c plus two-tone detector thresholds, which only matter for
    /// two-tone conventional channels (refused), so output equals C's.
    D,
    /// 1.41d with contact IDs moved out of the talkgroup range.
    E,
}

impl Variant {
    pub fn for_db_version(v: &str) -> Option<Self> {
        match v {
            "DBV1.41b" => Some(Self::B),
            "DBV1.41c" => Some(Self::C),
            "DBV1.41d" => Some(Self::D),
            "DBV1.41e" => Some(Self::E),
            _ => None,
        }
    }

    pub fn db_version(self) -> &'static str {
        match self {
            Self::B => "DBV1.41b",
            Self::C => "DBV1.41c",
            Self::D => "DBV1.41d",
            Self::E => "DBV1.41e",
        }
    }

    fn contact_ids(self) -> ContactIds {
        match self {
            Self::B | Self::C | Self::D => ContactIds { my: 99999, all: 65535, first_group: 10000, first_suid: 1, pager_takes_suid: true },
            Self::E => ContactIds { my: 9_999_999, all: 9_999_998, first_group: 1, first_suid: 9_000_000, pager_takes_suid: false },
        }
    }
}

/// Radio DB contact IDs (notes/converter-1.41.md 1).
#[derive(Debug, Clone, Copy)]
struct ContactIds {
    /// The pager itself.
    my: i64,
    /// All call.
    all: i64,
    /// Talkgroups count up from here, conventional groups first.
    first_group: i64,
    /// Members count up from here.
    first_suid: i64,
    /// Whether the pager's own member row uses up a member number.
    pager_takes_suid: bool,
}

/// What the conversion did.
#[derive(Debug, Default)]
pub struct Report {
    pub statements: usize,
    /// Features used by the profile that convert by the spec but were not
    /// exercised by the validation pagers.
    pub unverified: Vec<String>,
    /// Converted from PPS's rules but never checked on a pager or against a
    /// PPS-made radio DB (notes/digital-conventional.md 7). Writing is allowed.
    pub experimental: Vec<String>,
}

const CSQ: &str = "0000000000000000";

/// Generate `out` (a new radio DB) from `profile` using the template for `variant`.
pub fn convert(profile: &Profile, templates: &Templates, variant: Variant, out: &Path) -> Result<Report, Error> {
    let template = templates.radio_template(variant.db_version());
    if !template.is_file() {
        return Err(Error::Invalid(format!("missing template {}", template.display())));
    }
    let data = Connection::open(&templates.data_db())?;
    take_errors();

    let mut c = Conv::new(profile, &data, variant);
    c.run()?;
    let errors = take_errors();
    if !errors.is_empty() {
        return Err(Error::Conversion(errors));
    }

    if out.exists() {
        fs::remove_file(out)?;
    }
    fs::copy(&template, out)?;
    let db = Connection::open(out)?;
    db.execute_batch("BEGIN;")?;
    let mut failures = Vec::new();
    for s in &c.sql {
        if let Err(e) = db.execute(s, &[]) {
            failures.push(format!("{e}: {}", truncate(s, 200)));
        }
    }
    if !failures.is_empty() {
        let _ = db.execute_batch("ROLLBACK;");
        drop(db);
        let _ = fs::remove_file(out);
        return Err(Error::Conversion(failures));
    }
    db.execute_batch("COMMIT;")?;
    debug!("{} statements applied", c.sql.len());
    Ok(Report { statements: c.sql.len(), unverified: c.unverified, experimental: c.experimental })
}

fn truncate(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

/// SQL string literal body (PPS `insertStringReplace`).
fn q(v: &str) -> String {
    v.replace('\'', "''")
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn floor_div(a: i64, b: i64) -> i64 {
    a.div_euclid(b) - if a.rem_euclid(b) != 0 && b < 0 { 1 } else { 0 }
}

fn get_recv_mode(m: i64) -> i64 {
    match m {
        1 => 1,
        33 => 32,
        4 | 36 | 68 => 4,
        16 => 16,
        48 => 64,
        8 => 8,
        128 | 129 => 128,
        65 => 1,
        97 => 97,
        _ => 0,
    }
}

fn addr_col(pt: i64) -> Option<&'static str> {
    Some(match pt {
        4 => "DMRAddr",
        5 => "MDCAddr",
        7 => "UDCAddr",
        1 => "TwoToneAddr",
        2 => "FiveToneAddr",
        8 => "P25Addr",
        9 => "P25TAddr",
        _ => return None,
    })
}

fn tone_source(a: i64) -> i64 {
    if a >= 9999 {
        100
    } else if a < 100 {
        a + 100
    } else {
        a
    }
}

fn relay(n: i64) -> (i64, i64) {
    if n == 65535 {
        (1, 0)
    } else if 0 < n && n < 65535 {
        (1, n)
    } else {
        (0, 0)
    }
}

fn squelch_value(lvl: i64) -> (i64, i64) {
    let n = match lvl {
        0 => 20,
        1 => 40,
        2 => 60,
        3 => 100,
        _ => 20,
    };
    (255 * n / 100, 20 * n / 100)
}

fn access_code_type(p: i64) -> i64 {
    match p {
        5 | 7 => 3,
        2 => 4,
        4 => 5,
        8 => 6,
        _ => 0,
    }
}

fn tone_vol(t: i64) -> i64 {
    ((5 - t) as f64 * 9.6) as i64
}

fn new_vol(v: i64) -> i64 {
    if v == 0 { 48 } else { (4 - (v - 1)) * 11 }
}

#[derive(Default)]
struct Knob {
    name: String,
    recv_mode: i64,
    signaling_type: i64,
    reset_mode: i64,
    timeout_reset_timer: i64,
    tgid_stay_function: i64,
    tgid_stay_timer: i64,
    rec_delay_function: i64,
    rec_delay_timer: i64,
    ptl: i64,
    privacy: i64,
    monitor_mode_on: i64,
    voice_storage: i64,
    voice_buffering: i64,
    voice_buffering_prompt: i64,
    voice_buffering_force_drop: i64,
    alert_mode: i64,
    unread_msg_opt: i64,
    unread_msg_alert: i64,
    squelch_option: i64,
}

struct Sa {
    sa_type: i64,
    ctcd: i64,
    mode: i64,
    sig: String,
}

struct Chan {
    zk: i64,
    rx: i64,
    bw: i64,
    csq: i64,
    sinad: i64,
    ssl: i64,
    emph: i64,
    comp: i64,
    psave: i64,
    sa: Vec<Sa>,
}

/// Function-status catalog entry (a mutable copy of a `tabFunctionStatusBase` row).
struct FsItem {
    setting_item_id: String,
    default_order: String,
    name: String,
    item_type: String,
    has_child: String,
    belong: Option<String>,
}

struct Conv<'a> {
    prof: &'a Profile,
    data: &'a Connection,
    variant: Variant,
    ids: ContactIds,
    cache: HashMap<String, Rc<Vec<Row>>>,
    sys_dbs: HashMap<String, Connection>,
    sql: Vec<String>,
    unverified: Vec<String>,
    experimental: Vec<String>,

    zone_map: HashMap<String, i64>,
    channel_map: HashMap<String, i64>,
    recv_modes: HashMap<String, Row>,
    custom_alert_tones: Vec<(String, i64)>,
    custom_announcements: Vec<(String, i64)>,

    group_map: HashMap<String, i64>,
    group_rows: HashMap<String, Row>,
    sub_group_addr: HashMap<String, String>,
    member_map: HashMap<String, i64>,
    contacts_seen: HashSet<i64>,
    p25t_members_seen: HashSet<String>,
    pager_id: Option<String>,
    p25t_sig_id: HashMap<String, i64>,

    contact_of_chan_seen: HashSet<(i64, i64, i64)>,
    recv_group_list_seen: HashSet<(i64, i64, i64, i64)>,
    recv_behavior_seen: HashSet<(i64, i64, i64)>,
}

impl<'a> Conv<'a> {
    fn new(prof: &'a Profile, data: &'a Connection, variant: Variant) -> Self {
        Self {
            prof,
            data,
            variant,
            ids: variant.contact_ids(),
            cache: HashMap::new(),
            sys_dbs: HashMap::new(),
            sql: Vec::new(),
            unverified: Vec::new(),
            experimental: Vec::new(),
            zone_map: HashMap::new(),
            channel_map: HashMap::new(),
            recv_modes: HashMap::new(),
            custom_alert_tones: Vec::new(),
            custom_announcements: Vec::new(),
            group_map: HashMap::new(),
            group_rows: HashMap::new(),
            sub_group_addr: HashMap::new(),
            member_map: HashMap::new(),
            contacts_seen: HashSet::new(),
            p25t_members_seen: HashSet::new(),
            pager_id: None,
            p25t_sig_id: HashMap::new(),
            contact_of_chan_seen: HashSet::new(),
            recv_group_list_seen: HashSet::new(),
            recv_behavior_seen: HashSet::new(),
        }
    }

    /// Profile table (cached).
    fn p(&mut self, t: &str) -> Rc<Vec<Row>> {
        if let Some(r) = self.cache.get(t) {
            return r.clone();
        }
        let rows = Rc::new(load_table(&self.prof.db, t));
        self.cache.insert(t.to_string(), rows.clone());
        rows
    }

    fn one(&mut self, t: &str) -> Option<Row> {
        self.p(t).first().cloned()
    }

    fn data_table(&self, t: &str) -> Vec<Row> {
        load_table(self.data, t)
    }

    fn sys_table(&mut self, sid: &str, t: &str) -> Result<Vec<Row>, Error> {
        if !self.sys_dbs.contains_key(sid) {
            let db = self.prof.system_db(sid)?;
            self.sys_dbs.insert(sid.to_string(), db);
        }
        Ok(load_table(&self.sys_dbs[sid], t))
    }

    fn push(&mut self, s: impl Into<String>) {
        self.sql.push(s.into());
    }

    fn push_opt(&mut self, s: Option<String>) {
        if let Some(s) = s {
            self.sql.push(s);
        }
    }

    fn unverified(&mut self, what: &str) {
        if !self.unverified.iter().any(|u| u == what) {
            self.unverified.push(what.to_string());
        }
    }

    fn run(&mut self) -> Result<(), Error> {
        self.build_maps();
        self.convert_zone();
        self.convert_contacts()?;
        self.convert_selective_call()?;
        if !self.p("tabAdvancedConvCHKnob").is_empty() {
            return Err(Error::Unsupported("advanced conventional channels".into()));
        }
        self.convert_p25_trunking()?;
        self.convert_multi_ch_scan();
        if !self.p("tabAllBandScanCHKnob").is_empty() {
            return Err(Error::Unsupported("free scan (all-band scan) channels".into()));
        }
        self.convert_g4_function();
        self.convert_function();
        self.convert_detail_screen_item();
        self.convert_dispatch_center_manual_ack();
        self.convert_custom_tones();
        self.convert_encryption()?;
        let private = self.convert_private()?;
        self.sql.extend(private);
        Ok(())
    }

    // ---------------- maps ----------------

    fn build_maps(&mut self) {
        for r in self.p("tabZone_New").iter() {
            let id = r.s("ID").to_string();
            let n = self.zone_map.len() as i64 + 1;
            self.zone_map.entry(id).or_insert(n);
        }
        for t in ["tabSingleConvCHKnob", "tabAdvancedConvCHKnob", "tabP25TrunkingSystemKnob", "tabAllBandScanCHKnob"] {
            for r in self.p(t).iter() {
                if let Some(z) = self.zone_map.get(r.s("ZoneID")) {
                    let v = z * 100 + r.i("KnobIndex");
                    self.channel_map.entry(r.s("ChannelID").to_string()).or_insert(v);
                }
            }
        }
        for r in self.p("tabReceivingMode_NewG4").iter() {
            self.recv_modes.insert(format!("{}_{}", r.s("ZoneID"), r.s("KnobIndex")), r.clone());
        }
    }

    fn alert_tone_id(&mut self, s: &str) -> i64 {
        if s.is_empty() {
            return 0;
        }
        if is_digits(s) && s.len() < 6 {
            return parse_int(s, "AlertToneID");
        }
        if !self.prof.folder_files.contains(&format!("{s}.wav")) {
            return 0;
        }
        if let Some((_, v)) = self.custom_alert_tones.iter().find(|(k, _)| k == s) {
            return *v;
        }
        let v = 100 + self.custom_alert_tones.len() as i64;
        self.custom_alert_tones.push((s.to_string(), v));
        self.unverified("custom alert tones");
        v
    }

    fn channel_announcement_id(&mut self, s: &str) -> i64 {
        if s.is_empty() {
            return 0;
        }
        if is_digits(s) && s.len() < 6 {
            return parse_int(s, "AlertToneID");
        }
        if !self.prof.folder_files.contains(&format!("{s}.wav")) {
            return 0;
        }
        if let Some((_, v)) = self.custom_announcements.iter().find(|(k, _)| k == s) {
            return *v;
        }
        let v = 32 + self.custom_announcements.len() as i64;
        self.custom_announcements.push((s.to_string(), v));
        self.unverified("custom channel announcements");
        v
    }

    fn alert_tone_file_name_int(&self, x: i64) -> String {
        if x == 0 {
            return String::new();
        }
        if let Some((k, _)) = self.custom_alert_tones.iter().find(|(_, v)| *v == x) {
            return self.alert_tone_file_name_str(k);
        }
        self.alert_tone_file_name_str(&x.to_string())
    }

    fn alert_tone_file_name_str(&self, x: &str) -> String {
        if is_digits(x) && x.len() < 6 {
            return format!("AlertTone{x}.dat");
        }
        match self.custom_alert_tones.iter().find(|(k, _)| k == x) {
            Some((_, v)) => format!("G4-Tone-{v:03}"),
            None => String::new(),
        }
    }

    fn channel_name(&self, z: &str, k: &str) -> String {
        self.recv_modes.get(&format!("{z}_{k}")).map(|r| r.s("ChannelName").to_string()).unwrap_or_default()
    }

    fn receive_mode(&self, z: &str, k: &str) -> i64 {
        self.recv_modes.get(&format!("{z}_{k}")).map(|r| get_recv_mode(r.i("CurrentMode"))).unwrap_or(0)
    }

    // ---------------- zones ----------------

    fn convert_zone(&mut self) {
        let zones = self.p("tabZone_New");
        if zones.is_empty() {
            return;
        }
        self.push("UPDATE tabCurrentZone SET CurrentZoneID ='1' ;");
        let modes = self.p("tabReceivingMode_NewG4");
        for z in zones.iter() {
            let zid = z.s("ID");
            let Some(&zn) = self.zone_map.get(zid) else { continue };
            let mut flag = true;
            for kr in sel(&modes, "ZoneID", zid) {
                let ki = kr.s("KnobIndex");
                let zk = zn * 100 + kr.i("KnobIndex");
                let mode = kr.i("CurrentMode");
                let rmode = get_recv_mode(mode);
                if rmode != 0 && flag {
                    flag = false;
                }
                if mode != 0 {
                    if !matches!(mode, 4 | 36 | 68) {
                        self.push(format!("insert into tabZoneChannelList(ZoneID, KnobIndex, ChannelID, CreateFlag) values('{zn}','{ki}','{zk}','1');"));
                    }
                    self.push(format!(
                        "insert into tabReceivingMode(ZoneID,knobID,SupportedMode,CurrentMode,DualCHScanSettingID,MultiCHScanSettingID,FreeScanSettingID,CreateFlag,Name,CtCdRequest,ColorCodeRequest,NACRequest) values('{zn}','{ki}','{rmode}','{rmode}','0','{zk}','{zk}','1','{}','1','1','1');",
                        q(kr.s("ChannelName"))
                    ));
                    let ann = self.channel_announcement_id(kr.s("AlertToneID"));
                    self.push(format!("insert into tabChannelAnnouncement(ZoneID, ChannelID, FileName) values('{zn}','{ki}','G4-ChVoice-{ann:03}.dat');"));
                }
            }
            self.push(format!(
                "insert into tabZone(ID, Name, ModifyByDevice, priorityToneID) values('{zn}','{}','{}','0');",
                q(z.s("Name")),
                if flag { 1 } else { 0 }
            ));
        }
    }

    // ---------------- contacts ----------------

    fn convert_address(addr: &str, p: i64) -> String {
        if matches!(p, 4 | 8 | 9) {
            (parse_int(addr, "address") + 2_147_483_648).to_string()
        } else {
            addr.to_string()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn group_contact_sql(&mut self, cid: i64, st: i64, addr: &str, alias: &str, tone: i64, vs: i64, rc: i64, rot: i64, bg: i64) {
        if self.contacts_seen.insert(cid) {
            self.push(format!("INSERT INTO tabRadioGroup VALUES('{}','{cid}');", self.ids.my));
            self.push(format!("insert into tabContacts(id, name, idType, role, isUnication, note) values('{cid}','{}','2','2','0','');", q(alias)));
            self.push(format!("insert into tabRecvVoiceCallRecordByContactID(DestContactID,AllowRecord) values('{cid}','{vs}');"));
            self.push(format!("insert into tabContactOfRelayCtl(ContactID,RelayID,RelayCtrl,RelayOnTime) values('{cid}','1','{rc}','{rot}');"));
            self.push(format!("insert into tabContactOfBackground(ContactID,BackgroundColor) values('{cid}','{bg}');"));
            let (tone_id, text) = match tone {
                9999 => (37, "Default Tone".to_string()),
                10000 => (39, "Chirp Tone".to_string()),
                t if 0 < t && t <= 32 => (t, format!("Tone {t}")),
                t if (33..=200).contains(&t) => (t, format!("Custom Tone {}", t - 32)),
                t => (t, String::new()),
            };
            if tone > 0 {
                self.push(format!(
                    "insert into tabRecvToneForContacts(id, enable, toneID, alias, sourceType, sourceID, type, interval, maxTime) values('{cid}','1','{tone_id}','{text}','1','{}','{}','0','30000');",
                    tone_source(tone),
                    if tone == 10000 { 1 } else { 2 }
                ));
            }
        }
        self.push(format!("insert into tabContactAddr(ContactID, SignalingType, Address) values('{cid}','{st}','{addr}');"));
        if tone > 0 {
            let f = self.alert_tone_file_name_int(tone);
            self.push(format!(
                "insert into tabContactAddrAndAlertTone(ContactID, SignalingType, Address, ToneId, FileName) values('{cid}','{st}','{addr}','{}','{f}');",
                tone_source(tone)
            ));
        }
    }

    fn member_contact_sql(&mut self, cid: i64, st: i64, addr: &str, alias: &str, tone: i64, bg: i64) {
        if self.contacts_seen.insert(cid) {
            self.push(format!("insert into tabContacts(id, name, idType, role, isUnication, note) values('{cid}','{}','1','0','1','');", q(alias)));
            self.push(format!("insert into tabContactOfBackground(ContactID,BackgroundColor) values('{cid}','{bg}');"));
            if cid == self.ids.my {
                let (tone_id, text) = match tone {
                    9999 => (37, "Default Tone".to_string()),
                    10000 => (39, "Chirp Tone".to_string()),
                    t if 0 < t && t <= 32 => (t, format!("Tone {t}")),
                    t if (33..=200).contains(&t) => (t + 100, format!("Custom Tone {}", t - 32)),
                    _ => (0, "Alert Tone".to_string()),
                };
                self.push(format!(
                    "insert into tabRecvToneForContacts(id, enable, toneID, alias, sourceType, sourceID, type, interval, maxTime) values('{cid}','1','{tone_id}','{text}','1','{}','{}','0','30000');",
                    tone_source(tone),
                    if tone == 10000 { 1 } else { 2 }
                ));
            }
        }
        if st == 0 {
            return;
        }
        self.push(format!("insert into tabContactAddr(ContactID, SignalingType, Address) values('{cid}','{st}','{addr}');"));
        if cid == self.ids.my && tone > 0 {
            let f = self.alert_tone_file_name_int(tone);
            self.push(format!(
                "insert into tabContactAddrAndAlertTone(ContactID, SignalingType, Address, ToneId, FileName) values('{cid}','{st}','{addr}','{}','{f}');",
                tone_source(tone)
            ));
        }
    }

    fn convert_contacts(&mut self) -> Result<(), Error> {
        let groups = self.p("tabGroup_New");
        let mut gmap: HashMap<String, Row> = HashMap::new();
        for r in groups.iter() {
            gmap.entry(r.s("GroupID").to_string()).or_insert_with(|| r.clone());
        }
        for (k, v) in &gmap {
            self.group_rows.insert(k.clone(), v.clone());
        }
        let mut tg = self.ids.first_group;
        let mut seen_grouping = HashSet::new();
        let mut plist: Vec<i64> = Vec::new();
        // GroupID -> protocol types, both in first-seen order.
        let mut g2p: Vec<(String, Vec<i64>)> = Vec::new();
        let groupings = self.p("tabGroupGrouping");
        let grouping_of_group = self.p("tabGroupingOfGroup");
        for r in self.p("tabChannelGroupList").iter() {
            let gid = r.s("GroupingID");
            if seen_grouping.contains(gid) {
                continue;
            }
            let gg = sel(&groupings, "GroupingID", gid);
            let Some(first) = gg.first() else { continue };
            seen_grouping.insert(gid.to_string());
            let pt = first.i("Type");
            if !plist.contains(&pt) {
                plist.push(pt);
            }
            for gog in sel(&grouping_of_group, "GroupingID", gid) {
                let key = gog.s("GroupID");
                match g2p.iter_mut().find(|(k, _)| k == key) {
                    Some((_, pts)) => {
                        if !pts.contains(&pt) {
                            pts.push(pt);
                        }
                    }
                    None => g2p.push((key.to_string(), vec![pt])),
                }
            }
        }
        for (key, pts) in &g2p {
            let Some(g) = gmap.get(key) else { continue };
            self.group_map.insert(key.clone(), tg);
            let cid = tg;
            tg += 1;
            let (mut vs, mut rc, mut rot) = (0, 0, 0);
            if key != CSQ {
                vs = g.i("RecordFlag");
                (rc, rot) = relay(g.i("Relay"));
            }
            for &pt in pts {
                let col = addr_col(pt).ok_or_else(|| Error::Unsupported(format!("grouping protocol type {pt}")))?;
                let a = Self::convert_address(g.s(col), pt);
                if pt == 8 {
                    self.sub_group_addr.insert(key.clone(), a.clone());
                }
                let tone = self.alert_tone_id(g.s("AlertToneID"));
                self.group_contact_sql(cid, pt, &a, g.s("GroupName"), tone, vs, rc, rot, g.i("BackgroundColor"));
            }
        }
        if !self.p("tabP25TChannelSubGroup").is_empty() || !self.p("tabP25TChannelRevertSubGroup").is_empty() {
            return Err(Error::Unsupported("P25 sub-groups".into()));
        }

        let mut suid = self.ids.first_suid;
        for r in self.p("tabMember_New").iter() {
            let mid = r.s("MemberID").to_string();
            let is_pager = r.s("RadioNo") == "99999";
            if is_pager {
                self.member_map.insert(mid.clone(), self.ids.my);
                if self.ids.pager_takes_suid {
                    suid += 1;
                }
                self.pager_id = Some(mid.clone());
            } else {
                if self.member_map.contains_key(&mid) {
                    continue;
                }
                self.member_map.insert(mid.clone(), suid);
                suid += 1;
            }
            let cid = self.member_map[&mid];
            let bg = if r.s("BackgroundColor").is_empty() { 0 } else { r.i("BackgroundColor") };
            let tone = self.alert_tone_id(r.s("AlertToneID"));
            let _alert_mode = r.i("AlertMode") - 1;
            if is_pager {
                let ps = self.p("tabPagerSUID");
                let addr = ps.first().map(|p| p.s("P25TAddr")).filter(|a| !a.is_empty()).unwrap_or("268435455").to_string();
                self.member_contact_sql(cid, 9, &addr, r.s("MemberName"), tone, bg);
                for i in 1..9 {
                    if i == 3 || i == 6 {
                        continue;
                    }
                    let col = addr_col(i).unwrap();
                    let a2 = if !r.s(col).is_empty() { r.s(col).to_string() } else if i < 3 { String::new() } else { "0".into() };
                    self.member_contact_sql(cid, i, &a2, r.s("MemberName"), tone, bg);
                }
                continue;
            }
            for &pt in &plist {
                let mut a2 = String::new();
                if pt != 0 {
                    let col = addr_col(pt).ok_or_else(|| Error::Unsupported(format!("member protocol type {pt}")))?;
                    if r.s(col).is_empty() {
                        continue;
                    }
                    a2 = r.s(col).to_string();
                }
                self.member_contact_sql(cid, pt, &a2, r.s("MemberName"), tone, bg);
            }
        }

        // P25 trunking talkgroups and units, per knob.
        let mut sys_groups: HashMap<String, HashMap<String, Row>> = HashMap::new();
        let channel_group = self.p("tabChannelGroup");
        let channel_revert_group = self.p("tabChannelRevertGroup");
        for k in self.p("tabP25TrunkingSystemKnob").iter() {
            let ch = k.s("ChannelID");
            let sid = k.s("P25TrunkingID").to_string();
            if !sys_groups.contains_key(&sid) {
                let mut d = HashMap::new();
                for g in self.sys_table(&sid, "tabGroup_New")? {
                    d.insert(g.s("GroupID").to_string(), g);
                }
                for g in self.sys_table(&sid, "tabSubGroup")? {
                    d.insert(format!("Sub{}", g.s("GroupID")), g);
                }
                sys_groups.insert(sid.clone(), d);
                for m in self.sys_table(&sid, "tabMember_New")? {
                    if m.is_null("P25TAddr") || m.s("P25TAddr") == "0" {
                        continue;
                    }
                    let key = format!("{}_{}", m.s("MemberName"), m.s("P25TAddr"));
                    if !self.p25t_members_seen.insert(key) {
                        continue;
                    }
                    let member_key = format!("{sid}_{}", m.s("MemberID"));
                    if !self.member_map.contains_key(&member_key) {
                        self.member_map.insert(member_key.clone(), suid);
                        suid += 1;
                        self.member_contact_sql(self.member_map[&member_key], 9, m.s("P25TAddr"), m.s("MemberName"), 0, 0);
                    }
                }
            }
            tg = self.group_map.len() as i64 + self.ids.first_group;
            for table in [&channel_group, &channel_revert_group] {
                for cg in sel(table, "ChannelID", ch) {
                    let gid = cg.s("GroupID");
                    let Some(g) = sys_groups[&sid].get(gid).cloned() else { continue };
                    let key = format!("{sid}_{gid}");
                    if self.group_map.contains_key(&key) {
                        continue;
                    }
                    self.group_map.insert(key.clone(), tg);
                    tg += 1;
                    self.group_rows.entry(key.clone()).or_insert_with(|| g.clone());
                    let vs = g.i("RecordFlag");
                    let (rc, rot) = relay(g.i("Relay"));
                    let a = Self::convert_address(g.s("P25TAddr"), 9);
                    self.sub_group_addr.entry(key.clone()).or_insert_with(|| a.clone());
                    let tone = self.alert_tone_id(g.s("AlertToneID"));
                    self.group_contact_sql(self.group_map[&key], 9, &a, g.s("GroupName"), tone, vs, rc, rot, g.i("BackgroundColor"));
                }
            }
        }

        let all = self.ids.all;
        self.push(format!("INSERT INTO tabContactOfBackground VALUES({all},0);"));
        self.push(format!("INSERT INTO tabContacts VALUES({all},'All',0,2,0,'');"));
        self.push("INSERT INTO tabRecvToneForContacts VALUES(0,1,37,'Default Tone',1,100,2,0,30000);");
        self.push(format!("INSERT INTO tabRecvToneForContacts VALUES({all},1,37,'Default Tone',1,100,2,0,30000);"));
        for (st, a) in [(2, "AAAAA"), (4, "2164260863"), (5, "65535"), (7, "65535"), (8, "2147549183")] {
            self.push(format!("insert into tabContactAddr(ContactID, SignalingType, Address) values('{all}','{st}','{a}');"));
        }
        Ok(())
    }

    // ---------------- per-channel helpers ----------------

    fn contact_of_chan(&mut self, zk: i64, st: i64, cid: i64) -> Option<String> {
        self.contact_of_chan_seen
            .insert((zk, st, cid))
            .then(|| format!("INSERT INTO tabContactOfChan(ChannelID,ContactID,SignalingType)VALUES({zk},{cid},{st});"))
    }

    fn recv_group_list(&mut self, zk: i64, st: i64, sig: i64, addr: &str, cid: i64, seq: i64) -> Option<String> {
        self.recv_group_list_seen.insert((zk, st, sig, cid)).then(|| {
            format!("INSERT INTO tabRecvGroupList(ZoneKnobID,SigType,SigID,GroupAddr,GroupId,SeqNo)VALUES('{zk}','{st}','{sig}','{addr}','{cid}','{seq}');")
        })
    }

    fn recv_behavior(&mut self, zk: i64, st: i64, cid: i64, cg: Option<&Row>, g: Option<&Row>) -> Option<String> {
        if !self.recv_behavior_seen.insert((zk, st, cid)) {
            return None;
        }
        let (mut voice, mut text, mut image) = ("0".to_string(), "0".to_string(), "0".to_string());
        let (mut off, mut pri) = (0, 0);
        if let Some(cg) = cg {
            off = if cg.s("OffDuty").is_empty() { 0 } else { cg.i("OffDuty") };
            pri = if cg.s("Priority").is_empty() { 0 } else { cg.i("Priority") };
            let or0 = |v: &str| if v.is_empty() { "0".to_string() } else { v.to_string() };
            voice = or0(cg.s("VoiceRecvAck"));
            text = or0(cg.s("TextRecvAck"));
            image = or0(cg.s("ImageRecvAck"));
        }
        let (mut vs, mut am) = (2, 0);
        if let Some(g) = g {
            vs = if g.i("RecordFlag") > 0 { 1 } else { 2 };
            am = g.i("AlertMode") - 1;
        }
        let duty = if st == 9 { 0 } else if off <= 0 { 1 } else { 0 };
        Some(format!(
            "insert into tabRecvBehaviorByContactID(ChId,RadioID,Priority,Duty,AlertMode,VoiceStorage,SigType,VoiceRecvAck,VoiceReadAck,VoiceManualAck,TextRecvAck,TextReadAck,TextManualAck,ImageRecvAck,ImageReadAck,ImageManualAck) values('{zk}','{cid}','{pri}','{duty}','{am}','{vs}','{st}','{voice}','{voice}','{voice}','{text}','{text}','{text}','{image}','{image}','{image}');"
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn tab_channel_sql(&mut self, ch: &Chan, kn: &Knob, color: &str, nac: &str, defcall: &str, atgid: &str, tmode: &str, plist: &[i64]) {
        let zk = ch.zk;
        self.push(format!(
            "insert into tabChannel(ID,Name,isSupportRepeater,RXFrequency,TXFrequency,RxChanBandWidth,TxChanBandWidth) values('{zk}','{}','0','{}','{}','{}','{}');",
            q(&kn.name), ch.rx, ch.rx, ch.bw, ch.bw
        ));
        self.push(format!(
            "insert into tabChannelAdvInfo(ID,ChannelType,SignallingSquechLogic,AutoMute,SideTone,AudioSquelchType,CarrierSquelchSetting,SINADSquelchSetting,Emphasis,Compander,MsgRetries,RFActivityTimeout,UniLongDataEnable,ColorCode,NAC,DefaultCallID,ATGID,TaTxSaType,TaTxCtFreq,TaTxCtSte,TaTxCdCodeWord,TaTxCdSte,TaTxCdModSense,SquelchOption,PowerSavingRxOffDur) values('{zk}','0','{}','1','0','1','{}','{}','{}','{}','2','500','0','{color}','{nac}','{defcall}','{atgid}','0','670','1','19','1','0','{}','{}');",
            ch.ssl, ch.csq, ch.sinad, ch.emph, ch.comp, kn.squelch_option, ch.psave
        ));
        let mut vals: Vec<String> = vec![
            zk.to_string(), "0".into(), "0".into(), "0".into(), "0".into(), "0".into(), "0".into(),
            tmode.to_string(),
            kn.privacy.to_string(),
            kn.ptl.to_string(),
            (if kn.signaling_type == 9 { 1 } else { 0 }).to_string(),
            kn.voice_buffering.to_string(),
            (if kn.monitor_mode_on > 0 { 1 } else { 0 }).to_string(),
            kn.voice_buffering_prompt.to_string(),
            kn.unread_msg_opt.to_string(),
            kn.unread_msg_alert.to_string(),
            "0".into(),
        ];
        if self.variant >= Variant::C {
            vals.push(kn.voice_buffering_force_drop.to_string());
        }
        self.push(format!("insert into tabFunctionOnOffOfChan values({});", quote_list(&vals)));
        let p9 = kn.signaling_type == 9;
        self.push(format!(
            "insert into tabFunctionParamOfChan(ChannelID,VoiceCallSetID,TxPowerSettingID,rAndRSetID,RFPreformanceId,ResetModeSetID) values('{zk}','{zk}','3','{zk}','1','{}');",
            if p9 { 0 } else { zk }
        ));
        self.push(format!("insert into tabVoiceCallSettings(ID,DefaultCallID,HangTimeout,PenaltyTime) values('{zk}','0','0','0');"));
        self.push(format!(
            "insert into tabResetModeSetting(ID,ResetModeClass,DelayNTime,EnableCarrierOffTime,CarrierOffDelayTime,EnableTgidStayTime,TgidStayTime) values('{zk}','{}','{}','{}','{}','{}','{}');",
            kn.reset_mode,
            kn.timeout_reset_timer,
            if p9 { 0 } else { kn.rec_delay_function },
            if p9 { 30 } else { kn.rec_delay_timer },
            kn.tgid_stay_function,
            kn.tgid_stay_timer
        ));
        self.push(format!("insert into tabR2RModeSetting(SettingID,Mode) values('{zk}','0');"));
        self.push(format!(
            "insert into tabAlertModeSetting(id,ch_id,isOn,mode,time,TwoToneAlertType,TwoToneAlertDuration,FiveToneMdcAlertDuration,DigitalAlertDuration,DependOn) values('{zk}','{zk}','1','{}','2','0','2000','2000','2000','0');",
            kn.alert_mode
        ));
        self.push(format!(
            "insert into tabRecvVoiceCallRecordByChannel(ZoneID,KnobID,AllowRecord) values('{}','{}','{}');",
            zk / 100, zk % 100, kn.voice_storage
        ));
        for &p in plist {
            if p != 9 {
                self.push(format!(
                    "insert into tabRepeaterSettingOfChan values('{zk}','{p}','{}','{zk}','0','670','1','19','1','0','0','670','19','0');",
                    access_code_type(p)
                ));
            }
        }
        for sa in &ch.sa {
            for sig in sa.sig.split(';') {
                self.push(format!(
                    "INSERT INTO tabRecvSaList(ZoneKnobID,SigType,SaType,ctFreq,ctSte,cdCodeWord,cdSte,cdModSense) values('{zk}','{sig}','{}','{}','1','{}','1','{}');",
                    sa.sa_type,
                    if sa.sa_type == 1 { sa.ctcd } else { 670 },
                    if sa.sa_type == 2 { sa.ctcd } else { 19 },
                    sa.mode
                ));
            }
        }
    }

    fn knob_common(&self, k: &Row) -> Knob {
        let (z, ki) = (k.s("ZoneID"), k.s("KnobIndex"));
        Knob { name: self.channel_name(z, ki), recv_mode: self.receive_mode(z, ki), tgid_stay_timer: 30, ..Default::default() }
    }

    fn force_drop(&self, k: &Row, vb: i64) -> i64 {
        if self.variant >= Variant::C && vb > 0 { k.i("VoiceBufferingForceDrop") } else { 0 }
    }

    fn sub_group_enabled(&mut self, ch_id: &str) -> bool {
        let e = self.p("tabP25TSubGroupEnable");
        sel(&e, "ChannelID", ch_id).first().is_some_and(|r| r.i("Enable2TSubGroup") > 0)
    }

    // ---------------- conventional ----------------

    fn convert_single_ch_knob(&mut self, k: &Row) -> Result<(), Error> {
        let ch_id = k.s("ChannelID");
        let zk = self.channel_map[ch_id];
        let mut kn = self.knob_common(k);
        let ps = k.s("ProtocolSettingID");
        let proto = if ps == CSQ {
            0
        } else {
            let a = self.p("tabProtocolSetting");
            let found = sel(&a, "ID", ps);
            let Some(p) = found.first() else { return Err(Error::Invalid(format!("protocol setting {ps} not found"))) };
            p.i("ProtocolType")
        };
        if ![0, 4, 8].contains(&proto) {
            return Err(Error::Unsupported(format!("conventional channels with protocol type {proto} (carrier squelch, DMR and P25 conventional only)")));
        }
        kn.signaling_type = proto;
        let freqs = self.p("tabConvFreqList");
        let f = sel(&freqs, "FreqID", k.s("ConvFreqID"));
        let Some(f) = f.first() else { return Err(Error::Invalid(format!("frequency {} not found", k.s("ConvFreqID")))) };
        if proto != 0 {
            let what = if proto == 4 { "DMR conventional channel" } else { "P25 conventional channel" };
            if !self.experimental.iter().any(|x| x == what) {
                self.experimental.push(what.into());
            }
            // PPS would emit tone rows under the digital signaling type, and fails D1 on wide channels.
            if !sel(&self.p("tabRecvSaList"), "ChannelID", ch_id).is_empty() {
                return Err(Error::Unsupported("CTCSS/DCS tones on a digital conventional channel".into()));
            }
            if f.i("RxBandWidth") != 1 {
                return Err(Error::Invalid("DMR and P25 conventional channels must be 12.5 kHz".into()));
            }
            if proto == 8 {
                // Monitor mode turns "selective" and "don't care NAC" into 3; Selective Call turns 3 into 0.
                let opt = k.i("SquelchOption");
                kn.squelch_option = match kn.recv_mode {
                    32 if opt == 0 || opt == 2 => 3,
                    1 if opt == 3 => 0,
                    _ => opt,
                };
            }
        }
        let (csq, sinad) = squelch_value(k.i("AudioSquelch"));
        let mut ch = Chan {
            zk,
            rx: f.i("RxFreq"),
            bw: f.i("RxBandWidth"),
            csq,
            sinad,
            ssl: 0,
            emph: k.i("Emphasis"),
            comp: k.i("Compander"),
            psave: 0,
            sa: Vec::new(),
        };
        if kn.recv_mode == 32 {
            ch.ssl = 1;
        }
        ch.psave = match proto {
            2 | 4 | 8 | 9 => 0,
            _ => 100,
        };
        let sa_all = self.p("tabRecvSaList");
        let salist = sel(&sa_all, "ChannelID", ch_id);
        for r in salist.iter().take(5) {
            let t = r.i("SaType");
            let (ctcd, mode) = match t {
                1 => (r.i("ctFreq"), 0),
                2 => (r.i("cdCodeWord"), r.i("cdModSense")),
                _ => (0, 0),
            };
            ch.sa.push(Sa { sa_type: t, ctcd, mode, sig: (if t > 0 { proto } else { 255 }).to_string() });
        }
        kn.reset_mode = k.i("ResetMode");
        kn.timeout_reset_timer = k.i("DelayNTime");
        if k.has("EnableTgidStayTime") {
            kn.tgid_stay_function = k.i("EnableTgidStayTime");
        }
        if k.has("TgidStayTime") {
            kn.tgid_stay_timer = k.i("TgidStayTime");
        }
        kn.rec_delay_function = k.i("EnableCarrierOffTime");
        kn.rec_delay_timer = k.i("CarrierOffDelayTime");
        kn.ptl = k.i("PushToListen");
        kn.privacy = k.i("Privacy");
        kn.monitor_mode_on = if kn.recv_mode == 32 { if salist.is_empty() { 2 } else { 1 } } else { 0 };
        kn.voice_storage = k.i("RecordFlag");
        kn.voice_buffering = if kn.ptl <= 0 { k.i("VoiceBuffering") } else { 0 };
        kn.voice_buffering_prompt = if kn.voice_buffering > 0 { k.i("VoiceBufferPrompt") } else { 0 };
        kn.voice_buffering_force_drop = self.force_drop(k, kn.voice_buffering);
        kn.alert_mode = k.i("AlertMode") - 1;
        kn.unread_msg_opt = k.i("AlwaysUnreadVoiceMessage");
        kn.unread_msg_alert = k.i("AlwaysUnreadReminderAlert");
        let defcall = if proto == 0 || proto == 1 { self.ids.all.to_string() } else { "0".into() };
        self.tab_channel_sql(
            &ch,
            &kn,
            k.get("RxColorCode").unwrap_or(""),
            k.get("RxNAC").unwrap_or(""),
            &defcall,
            "0",
            k.get("TransmitMode").unwrap_or(""),
            &[proto],
        );
        if proto != 0 {
            self.convert_conv_signaling(k, zk, proto)?;
        }
        Ok(())
    }

    /// The per-knob signaling rows of a DMR (4) or P25 conventional (8) knob,
    /// keyed by the knob's ZK (notes/digital-conventional.md 5.1, 5.2).
    fn convert_conv_signaling(&mut self, k: &Row, zk: i64, proto: i64) -> Result<(), Error> {
        let ps = k.s("ProtocolSettingID");
        let or = |r: &Row, c: &str| r.s(c).to_string();
        if proto == 4 {
            let (st, rt, vt) = (self.p("tabDMRSignaling"), self.p("tabDMRRepeaterSetting"), self.p("tabVoiceCallVocoderSetting"));
            let sig = sel(&st, "ID", ps).first().map(|r| (*r).clone()).ok_or_else(|| Error::Invalid("DMR protocol setting without signaling".into()))?;
            let rep = sel(&rt, "ID", ps).first().map(|r| (*r).clone()).ok_or_else(|| Error::Invalid("DMR protocol setting without repeater setting".into()))?;
            let voc = vt.first().cloned().ok_or_else(|| Error::Invalid("no DMR vocoder setting".into()))?;
            let cc = k.s("RxColorCode");
            // PPS inserts these values unquoted.
            self.push(format!(
                "insert into tabDMRSignaling(SigID,TXPreFrames,RxColorCode,RCRxEnable,NoSigActTimeout,ACKWaitTimeout,MFID,SAP4NRS,SAP4RS,DMRCompatibility,DMRVocoder) values({zk},{},{cc},{},{},{},{},{},{},{},{});",
                or(&sig, "TXPreFrames"),
                or(&sig, "RCRxEnable"),
                or(&sig, "NoSigActTimeout"),
                or(&sig, "ACKWaitTimeout"),
                or(&sig, "MFID"),
                or(&sig, "SAP4NRS"),
                or(&sig, "SAP4RS"),
                or(&voc, "DMRCompatibility"),
                or(&voc, "DMRVocoder"),
            ));
            self.push(format!(
                "insert into tabDMRRepeaterSetting(SACID,TimeSlot,TimingAlign,RxColorCode,TxColorCode) values('{zk}','{}','{}','{cc}','{}');",
                or(&rep, "TimeSlot"),
                or(&rep, "TimingAlign"),
                q(k.s("TxColorCode"))
            ));
        } else {
            let st = self.p("tabP25Signaling");
            let sig = sel(&st, "ID", ps).first().map(|r| (*r).clone()).ok_or_else(|| Error::Invalid("P25 conventional protocol setting without signaling".into()))?;
            let nac = k.s("RxNAC");
            self.push(format!(
                "insert into tabP25Signaling(SigID,TXPreTime,NoSigActTimeout,ACKWaitTimeout,RxNAC,MFID,AlgId,MinShortDigitalToneDuration,MinLongDigitalToneDuration,MinToneAddrGap,EncryptedCallFlag,ShortDigitalToneATxDuration,ShortDigitalToneBTxDuration,LongDigitalToneBTxDuration,TwoDigitalToneAddrTxGap) values('{zk}','{}','{}','{}','{nac}','{}','{}','{}','{}','{}','{}','{}','{}','{}','{}');",
                or(&sig, "TXPreTime"),
                or(&sig, "NoSigActTimeout"),
                or(&sig, "ACKWaitTimeout"),
                or(&sig, "MFID"),
                or(&sig, "AlgId"),
                or(&sig, "MinShortDigitalToneDuration"),
                or(&sig, "MinLongDigitalToneDuration"),
                or(&sig, "MinToneAddrGap"),
                k.s("EncryptedCallFlag"),
                or(&sig, "ShortDigitalToneATxDuration"),
                or(&sig, "ShortDigitalToneBTxDuration"),
                or(&sig, "LongDigitalToneTxDuration"),
                or(&sig, "TxToneAddrGap"),
            ));
            self.push(format!("insert into tabP25RepeaterSetting(SACID,RxNAC,TxNAC) values('{zk}','{nac}','{}');", q(k.s("TxNAC"))));
        }
        Ok(())
    }

    fn convert_recv_group_list(&mut self, k: &Row, sys_id: &str) -> Result<(), Error> {
        let ch_id = k.s("ChannelID");
        let zk = self.channel_map[ch_id];
        if k.is_null("ProtocolSettingID") {
            return Ok(());
        }
        let ps = k.s("ProtocolSettingID");
        if ps == CSQ {
            let all = self.ids.all;
            let a = self.contact_of_chan(zk, 0, all);
            self.push_opt(a);
            let b = self.recv_group_list(zk, 0, zk, "", all, 0);
            self.push_opt(b);
            let c = self.recv_behavior(zk, 0, all, None, None);
            self.push_opt(c);
            return Ok(());
        }
        let settings = self.p("tabProtocolSetting");
        let found = sel(&settings, "ID", ps);
        let Some(p) = found.first() else { return Err(Error::Invalid(format!("protocol setting {ps} not found"))) };
        let proto = p.i("ProtocolType");
        if proto == 4 || proto == 8 {
            return self.convert_conv_recv_list(k, zk, proto);
        }
        if proto != 9 {
            return Err(Error::Unsupported(format!("receive groups for protocol type {proto}")));
        }
        let sig = *self
            .p25t_sig_id
            .get(&format!("{}_{}", k.s("P25TrunkingID"), ps))
            .ok_or_else(|| Error::Invalid("P25 knob without signaling".into()))?;
        if self.sub_group_enabled(ch_id) {
            let sg = self.p("tabP25TChannelSubGroup");
            let rsg = self.p("tabP25TChannelRevertSubGroup");
            if !sel(&sg, "ChannelID", ch_id).is_empty() || !sel(&rsg, "ChannelID", ch_id).is_empty() {
                return Err(Error::Unsupported("P25 sub-groups on a channel".into()));
            }
        }
        let mut next_seq = 1;
        let cgs = self.p("tabChannelGroup");
        for cg in sel(&cgs, "ChannelID", ch_id) {
            let key = format!("{sys_id}_{}", cg.s("GroupID"));
            let Some(&cid) = self.group_map.get(&key) else { continue };
            let a = self.contact_of_chan(zk, 9, cid);
            self.push_opt(a);
            let grow = self.group_rows.get(&key).cloned();
            let b = self.recv_behavior(zk, 9, cid, Some(cg), grow.as_ref());
            self.push_opt(b);
            let seq = cg.i("SeqNo");
            let addr = self.sub_group_addr.get(&key).cloned().unwrap_or_default();
            let c = self.recv_group_list(zk, 9, sig, &addr, cid, if seq > 0 { next_seq } else { 0 });
            self.push_opt(c);
            if seq > 0 {
                next_seq += 1;
            }
        }
        let rgs = self.p("tabChannelRevertGroup");
        if !sel(&rgs, "ChannelID", ch_id).is_empty() {
            return Err(Error::Unsupported("revert talkgroups".into()));
        }
        Ok(())
    }

    /// Receive list of a DMR or P25 conventional knob: every member of its
    /// grouping, in grouping order. No all-call row (PPS drops it).
    fn convert_conv_recv_list(&mut self, k: &Row, zk: i64, proto: i64) -> Result<(), Error> {
        let ch_id = k.s("ChannelID");
        let cgl = self.p("tabChannelGroupList");
        let lists: Vec<Row> = sel(&cgl, "ChannelID", ch_id).into_iter().filter(|r| r.i("isRevert") == 0).cloned().collect();
        if sel(&cgl, "ChannelID", ch_id).iter().any(|r| r.i("isRevert") != 0) {
            return Err(Error::Unsupported("revert lists on a digital conventional channel".into()));
        }
        if proto == 8 && self.sub_group_enabled(ch_id) {
            return Err(Error::Unsupported("sub-groups on a P25 conventional channel".into()));
        }
        let groupings = self.p("tabGroupGrouping");
        let grouping = match lists.as_slice() {
            [] => return Err(Error::Invalid("a digital conventional knob needs a talkgroup list".into())),
            [one] => one.s("GroupingID").to_string(),
            many => many
                .iter()
                .find(|r| sel(&groupings, "GroupingID", r.s("GroupingID")).first().is_some_and(|g| g.i("Type") == proto))
                .map(|r| r.s("GroupingID").to_string())
                .ok_or_else(|| Error::Invalid("no talkgroup list of the knob's protocol".into()))?,
        };
        let gog = self.p("tabGroupingOfGroup");
        let members: Vec<String> = sel(&gog, "GroupingID", &grouping).iter().map(|m| m.s("GroupID").to_string()).collect();
        if members.is_empty() {
            return Err(Error::Invalid("the knob's talkgroup list is empty".into()));
        }
        let cgs = self.p("tabChannelGroup");
        let col = addr_col(proto).unwrap();
        let mut seq = 1;
        for gid in &members {
            let cid = *self.group_map.get(gid).ok_or_else(|| Error::Invalid(format!("talkgroup {gid} has no contact")))?;
            let g = self.group_rows.get(gid).cloned();
            let addr = Self::convert_address(g.as_ref().map(|g| g.s(col)).unwrap_or(""), proto);
            let cg = sel(&cgs, "ChannelID", ch_id).into_iter().find(|r| r.s("GroupID") == gid).cloned();
            let a = self.contact_of_chan(zk, proto, cid);
            self.push_opt(a);
            // P25C numbers the members with a SeqNo; DMR leaves 0.
            let n = if proto == 8 && cg.as_ref().is_some_and(|c| c.i("SeqNo") > 0) {
                seq += 1;
                seq - 1
            } else {
                0
            };
            let b = self.recv_group_list(zk, proto, zk, &addr, cid, n);
            self.push_opt(b);
            let c = self.recv_behavior(zk, proto, cid, cg.as_ref(), g.as_ref());
            self.push_opt(c);
        }
        Ok(())
    }

    fn convert_selective_call(&mut self) -> Result<(), Error> {
        for k in self.p("tabSingleConvCHKnob").iter() {
            self.convert_single_ch_knob(k)?;
            self.convert_recv_group_list(k, "")?;
        }
        Ok(())
    }

    // ---------------- P25 trunking ----------------

    fn first_band_range(&mut self) -> Result<(i64, i64), Error> {
        let mp = self.one("tabMfgProductCode").ok_or_else(|| Error::Invalid("profile has no tabMfgProductCode".into()))?;
        let bt = mp.s("RFFirstBandTypeID").to_string();
        let ranges = self.data_table("tabRFBandRanges");
        let r = ranges
            .iter()
            .find(|r| r.s("BandTypeID") == bt && r.s("RxTx") == "0")
            .ok_or_else(|| Error::Invalid(format!("no Rx band range for band type {bt}")))?;
        Ok((floor_div(r.i("MinFreq"), 1_000_000) * 1_000_000, floor_div(r.i("MaxFreq"), 1_000_000) * 1_000_000))
    }

    fn convert_p25t_signaling(&mut self, k: &Row) -> Result<(), Error> {
        let (sid, ps) = (k.s("P25TrunkingID").to_string(), k.s("ProtocolSettingID").to_string());
        let sigs = self.p("tabP25TSignaling");
        let a = sel(&sigs, "ID", &ps).first().map(|r| (*r).clone()).ok_or_else(|| Error::Invalid(format!("no tabP25TSignaling {ps}")))?;
        let key = format!("{sid}_{ps}");
        if self.p25t_sig_id.contains_key(&key) {
            return Ok(());
        }
        let n = self.p25t_sig_id.len() as i64 + 1;
        self.p25t_sig_id.insert(key, n);
        let s = self.sys_table(&sid, "tabP25TrunkingSystem")?.into_iter().next().ok_or_else(|| Error::Invalid(format!("system {sid} has no tabP25TrunkingSystem row")))?;
        let (lo, hi) = self.first_band_range()?;
        let fss = s.i("FullSpectrumScanFlag");
        let ranges = if fss != 0 { s.i("FullSpectrumScanRanges") } else { 0 };
        let rng = |idx: i64| -> (i64, i64, i64) {
            let st = s.s(&format!("StartFreqOfRange{idx}"));
            let en = s.s(&format!("EndFreqOfRange{idx}"));
            let sp = s.s(&format!("StepSizeOfRange{idx}"));
            let st = if ranges < idx || st.is_empty() { lo } else { parse_int(st, "StartFreq") };
            let en = if ranges < idx || en.is_empty() { hi } else { parse_int(en, "EndFreq") };
            let sp = if sp.is_empty() { 12500 } else { parse_int(sp, "StepSize") };
            (st, en, sp)
        };
        let (r1, r2, r3) = (rng(1), rng(2), rng(3));
        self.push(format!(
            "INSERT INTO tabP25TrunkingSignaling(SigID,ACKWaitTimeout,RfssRspWaitTimeout,MFID,AlgId,WacnID,SystemID,WacnSystemIDCheckSkipFlag,FullSpectrumScanFlag,FullSpectrumScanTimer,FullSpectrumScanRanges,StartFreqOfRange1,EndFreqOfRange1,StepSizeOfRange1,StartFreqOfRange2,EndFreqOfRange2,StepSizeOfRange2,StartFreqOfRange3,EndFreqOfRange3,StepSizeOfRange3,SiteTrunkingFlag,OutOfRangeErrorRate)VALUES('{n}','{}','{}','{}','{}','{}','{}','0','{fss}','0','{ranges}','{}','{}','{}','{}','{}','{}','{}','{}','{}','{}','10');",
            a.s("ACKWaitTimeout"), a.s("RfssRspWaitTimeout"), a.s("MFID"), a.s("AlgId"), s.s("WacnID"), s.s("SystemID"),
            r1.0, r1.1, r1.2, r2.0, r2.1, r2.2, r3.0, r3.1, r3.2, s.s("SiteTrunkingFlag")
        ));
        self.push(format!("INSERT INTO tabSystemAlias(SigType,SigID,SystemAlias)Values('9','{n}','{}');", q(s.s("Name"))));
        let mut seen = HashSet::new();
        let mut c = 0;
        for cc in self.sys_table(&sid, "tabP25TrunkingControlCH")? {
            if !seen.insert(cc.s("RxFreq").to_string()) {
                continue;
            }
            self.push(format!(
                "INSERT INTO tabP25TrunkingControlChannels(SigID,SeqNo,RxChFreq,RfssID,SiteID)VALUES({n},{},{},0,0);",
                256 - c,
                cc.s("RxFreq")
            ));
            c += 1;
        }
        let mut seen = HashSet::new();
        let mut c = 0;
        for st in self.sys_table(&sid, "tabP25TrunkingSite")? {
            let rf = st.s("RFSSID");
            let t3 = if is_digits(rf) {
                rf.to_string()
            } else if is_hex(rf) {
                i64::from_str_radix(rf, 16).map(|v| v.to_string()).unwrap_or_else(|_| "0".into())
            } else {
                "0".into()
            };
            if !seen.insert(format!("{t3}_{}", st.s("SiteID"))) {
                continue;
            }
            self.push(format!(
                "INSERT INTO tabP25TrunkingSites(SigID,RfssID,SiteID,SiteName,SeqNo) VALUES('{n}','{t3}','{}','{}','{}');",
                st.s("SiteID"),
                q(st.s("Name")),
                256 - c
            ));
            c += 1;
        }
        Ok(())
    }

    fn convert_p25t_knob(&mut self, k: &Row) -> Result<(), Error> {
        let ch_id = k.s("ChannelID");
        let zk = self.channel_map[ch_id];
        let mut kn = self.knob_common(k);
        kn.signaling_type = 9;
        let ps = k.s("ProtocolSettingID");
        let flag = self.sub_group_enabled(ch_id);
        kn.reset_mode = k.i("ResetMode");
        kn.timeout_reset_timer = k.i("DelayNTime");
        kn.tgid_stay_function = k.i("EnableTgidStayTime");
        kn.tgid_stay_timer = k.i("TgidStayTime");
        // DelayNFeature / DelayNTimeEx are read by PPS but not written to the 1.41 radio DB.
        let _ = (k.i("DelayNFeature"), k.i("DelayNTimeEx"));
        kn.rec_delay_function = k.i("EnableCarrierOffTime");
        kn.rec_delay_timer = if k.i("EnableCarrierOffTime") != 0 { k.i("CarrierOffDelayTime") } else { 0 };
        kn.ptl = k.i("PushToListen");
        kn.privacy = k.i("Privacy");
        kn.voice_storage = k.i("RecordFlag");
        kn.voice_buffering = if kn.ptl <= 0 { k.i("VoiceBuffering") } else { 0 };
        kn.voice_buffering_prompt = if kn.voice_buffering > 0 { k.i("VoiceBufferPrompt") } else { 0 };
        kn.voice_buffering_force_drop = self.force_drop(k, kn.voice_buffering);
        kn.alert_mode = k.i("AlertMode") - 1;
        kn.unread_msg_opt = k.i("AlwaysUnreadVoiceMessage");
        kn.unread_msg_alert = k.i("AlwaysUnreadReminderAlert");
        let (csq, sinad) = squelch_value(k.i("AudioSquelch"));
        let ch = Chan { zk, rx: 0, bw: 1, csq, sinad, ssl: 0, emph: 0, comp: 0, psave: 0, sa: Vec::new() };
        let atgid = k.i("ATGID").to_string();
        self.tab_channel_sql(&ch, &kn, "0", "0", "0", &atgid, "0", &[9]);
        let sigs = self.p("tabP25TSignaling");
        let a = sel(&sigs, "ID", ps).first().map(|r| (*r).clone()).ok_or_else(|| Error::Invalid(format!("no tabP25TSignaling {ps}")))?;
        self.push(format!(
            "insert into tabP25TSetting(ChannelID,MSELCallSamplingFlag,MaxMSELSamplingTime,TwoToneSubGroupOn,MinShortDigitalToneDuration,MinLongDigitalToneDuration,MinToneAddrGap,OTAToneFlag,ShortDigitalToneATxDuration,ShortDigitalToneBTxDuration,LongDigitalToneBTxDuration,TwoDigitalToneAddrTxGap,EncryptedCallFlag) values('{zk}','{}','{}','{}','{}','{}','{}','1','{}','{}','{}','{}','{}');",
            k.s("MSELCallSamplingFlag"), k.s("MaxMSELSamplingTime"), if flag { 1 } else { 0 },
            a.s("MinShortDigitalToneDuration"), a.s("MinLongDigitalToneDuration"), a.s("MinToneAddrGap"),
            a.s("ShortDigitalToneATxDuration"), a.s("ShortDigitalToneBTxDuration"), a.s("LongDigitalToneTxDuration"), a.s("TxToneAddrGap"),
            k.s("EncryptedCallFlag")
        ));
        Ok(())
    }

    fn convert_p25_trunking(&mut self) -> Result<(), Error> {
        for k in self.p("tabP25TrunkingSystemKnob").iter() {
            self.convert_p25t_signaling(k)?;
            self.convert_p25t_knob(k)?;
            self.convert_recv_group_list(k, k.s("P25TrunkingID"))?;
            let ca = self.p("tabP25TCallAlertEnable");
            if sel(&ca, "ChannelID", k.s("ChannelID")).first().is_some_and(|e| e.i("EnableCallAlert") > 0) {
                return Err(Error::Unsupported("P25 call alert".into()));
            }
        }
        Ok(())
    }

    // ---------------- scan lists ----------------

    fn convert_multi_ch_scan(&mut self) {
        let lists = self.p("tabMultiCHScanList");
        for r in self.p("tabMultiChannelScan").iter() {
            let pri = r.s("PriorityScanType") == "1";
            let mut text = String::new();
            if pri {
                for c in ["FirstPriorityCH", "SecondPriorityCH"] {
                    if let Some(v) = self.channel_map.get(r.s(c)) {
                        text += &format!("{v},");
                    }
                }
            }
            let text = text.trim_matches(',').to_string();
            let mut lst = Vec::new();
            for d in lists.iter().filter(|d| d.s("ZoneID") == r.s("ZoneID") && d.s("KnobIndex") == r.s("KnobIndex")) {
                let cid = d.s("ChannelID");
                if pri && (cid == r.s("FirstPriorityCH") || cid == r.s("SecondPriorityCH")) {
                    continue;
                }
                if let Some(v) = self.channel_map.get(cid) {
                    lst.push(v.to_string());
                }
            }
            let Some(&zn) = self.zone_map.get(r.s("ZoneID")) else {
                record_error(format!("scan list references unknown zone {}", r.s("ZoneID")));
                continue;
            };
            let n = zn * 100 + r.i("KnobIndex");
            self.push(format!(
                "insert into tabScanListSettings(ScanListSettingID,StayTime,RecvAllFlag,PriorityScanType,SamplingFlag,PriorityZoneKnobID,ScanList,CallBackTime,RxPauseBetweenScanPaths,PriorityIntervalTime) values('{n}','{}','{}','{}','{}','{text}','{}',0,0,'{}');",
                r.s("StayTime"), r.s("OpenSpeaker"), r.s("PriorityScanType"), r.s("SamplingFlag"), lst.join(","), r.s("DwellTime")
            ));
            self.push(format!("insert into tabFunctionParamOfChan(ChannelID,VoiceCallSetID,TxPowerSettingID,rAndRSetID,RFPreformanceId,ResetModeSetID) values('{n}','0','0','0','1','{n}');"));
            let mut vals: Vec<String> = vec![
                n.to_string(), "0".into(), "0".into(), "0".into(), "0".into(), "0".into(), "0".into(), "1".into(),
                r.s("Privacy").into(), r.s("PushToListen").into(), "0".into(), "0".into(), "0".into(), "0".into(),
                r.s("AlwaysUnreadVoiceMessage").into(), r.s("AlwaysUnreadReminderAlert").into(), "0".into(),
            ];
            if self.variant >= Variant::C {
                vals.push("0".into());
            }
            self.push(format!("insert into tabFunctionOnOffOfChan values({});", quote_list(&vals)));
            self.push(format!(
                "insert into tabResetModeSetting(ID,ResetModeClass,DelayNTime,EnableCarrierOffTime,CarrierOffDelayTime,EnableTgidStayTime,TgidStayTime) values('{n}','{}','{}','{}','{}','0','30');",
                r.s("ResetMode"), r.s("DelayNTime"), r.s("EnableCarrierOffTime"), r.s("CarrierOffDelayTime")
            ));
            self.push(format!(
                "insert into tabAlertModeSetting(id,ch_id,isOn,mode,time,TwoToneAlertType,TwoToneAlertDuration,FiveToneMdcAlertDuration,DigitalAlertDuration,DependOn) values('{n}','{n}','1','{}','2','0','2000','2000','2000','0');",
                r.i("AlertMode") - 1
            ));
            self.push(format!("insert into tabRecvVoiceCallRecordByChannel(ZoneID,KnobID,AllowRecord) values('{}','{}','{}');", n / 100, n % 100, r.s("RecordFlag")));
            self.push(format!("update tabReceivingMode set CtCdRequest='{}' where ZoneID='{}' and knobID='{}';", r.s("MatchCTCD"), n / 100, n % 100));
        }
    }

    fn convert_g4_function(&mut self) {
        for t in ["tabSingleConvCHKnob", "tabP25TrunkingSystemKnob"] {
            for r in self.p(t).iter() {
                if r.s("DispatchCHFlag") == "0"
                    && let Some(v) = self.channel_map.get(r.s("ChannelID")).copied() {
                        self.push(format!("INSERT INTO tabDispatchChannel(DispatchChannelID)Values('{v}');"));
                    }
            }
        }
    }

    // ---------------- device settings ----------------

    fn convert_function(&mut self) {
        if let Some(o) = self.one("tabOrganization_New") {
            self.push(format!("insert into tabRadioBaseSetting(UserCompany,UserName,Password,RadioNo) values('{}','U3','000000','');", q(o.s("OrgName"))));
        }
        if let Some(v) = self.one("tabVoiceCallSetting") {
            let limit_opt = v.i("CallTimeLimitOption");
            let default_limit = match limit_opt {
                0 => 4,
                1..=3 => limit_opt,
                _ => 0,
            };
            let opt3 = if limit_opt == 3 { v.s("UserDefineCallTimeLimit").to_string() } else { "90".into() };
            self.push(format!("update tabCallTimeLimit set option3={opt3}, defaultCallTimeLimit={default_limit}, liveTime={};", v.s("CallTimeLimitChangeTime")));
        }
        for (i, r) in self.p("tabConnedMessage").iter().enumerate() {
            self.push(format!("insert into tabCannedMessageMap(MessageID,MessageContent) values({},'{}');", i + 1, q(r.s("Content"))));
        }
        if let Some(t) = self.one("tabTextMessage") {
            self.push(format!("update tabTextMessageSetting set MaxTextNoteLen = {};", t.s("MaxLen")));
            self.push(format!("update tabFunctionsOnOff set showMsgContentOn = {};", t.s("IsAutoShow")));
        }
        self.push("insert into tabAlertModeSetting(id,ch_id,isOn,mode,time,TwoToneAlertType,TwoToneAlertDuration,FiveToneMdcAlertDuration,DigitalAlertDuration,DependOn) values('0', '-1', '1', '0', '2', '0', '2000', '2000', '2000', '0');");
        self.convert_other_function_setting();
        self.convert_device_parameters();
        if let Some(g) = self.one("tabGPRSAckSettingPPS") {
            self.push(format!("update tabGPRSAckSetting set FuncOption='{}',IP='{}',Port='{}';", g.s("FuncOption"), g.s("IP"), g.s("Port")));
        }
        if let Some(g) = self.one("tabGSMBaseSettingPPS") {
            self.push(format!("update tabGSMBaseSetting set IMEI='{}',IMSI='{}',PagerID='{}';", g.s("IMEI"), g.s("IMSI"), g.s("PagerID")));
        }
        let e = self.one("tabDSPEngineerSetting").map(|r| r.i("Enable")).unwrap_or(0);
        if e != 0 {
            self.unverified("engineer DSP parameters");
        }
        let params = self.p("tabDSPEngineerParam");
        let mut bs = String::new();
        for i in 1..33 {
            if let Some(a) = sel(&params, "ID", &i.to_string()).first() {
                let x = parse_int(a.s("Value"), "DSP Value");
                bs += &format!("{:02X}{:02X}", x & 0xFF, (x >> 8) & 0xFF);
            }
        }
        self.push(format!("INSERT INTO tabDSPEngineerSettings VALUES('{e}','{bs}');"));
        self.push("DELETE FROM tabAccessPassword;");
        let ap = self.p("tabAccessPassword");
        if ap.is_empty() {
            self.push("INSERT INTO tabAccessPassword VALUES (1,'0','');");
        }
        for r in ap.iter() {
            self.push(format!(
                "INSERT INTO tabAccessPassword VALUES ('{}','{}','{}');",
                parse_int(r.s("ID"), "AccessPassword ID"),
                if parse_int(r.s("Enable"), "AccessPassword Enable") != 0 { 1 } else { 0 },
                r.s("AccessPassword")
            ));
        }
        let pin = self.p("tabPINPasswordSetting");
        if !pin.is_empty() {
            self.push("DELETE FROM tabPINPasswordSetting;");
        }
        for r in pin.iter() {
            self.push(format!(
                "INSERT INTO tabPINPasswordSetting(ID,BootingLockEnable,KeypadLockEnable,AdminPINPwd,DefaultPINPwd,UserPINPwd,MaxErrorCount,ErrorDisableTime) VALUES('{}','{}','{}','{}','{}','{}','{}','{}');",
                r.s("ID"), r.s("BootingLockEnable"), r.s("KeypadLockEnable"), r.s("AdminPINPwd"), r.s("UserPINPwd"), r.s("UserPINPwd"), r.s("MaxErrorCount"), r.s("ErrorDisableTime")
            ));
        }
    }

    fn convert_other_function_setting(&mut self) {
        if let Some(m) = self.one("tabMenuUIType") {
            self.push(format!("update tabMenuUIType set Type  ='{}';", m.s("Type")));
        }
        if let Some(t) = self.one("tabTodaySettings") {
            self.push(format!("update tabTodaySettings set isAlertToneOn  ='{}',AlertToneInterval ='{}';", t.s("isAlertToneOn"), t.s("AlertToneInterval")));
            self.push(format!("UPDATE tabFunctionsOnOff SET ReminderAlertOn='{}';", t.s("isAlertToneOn")));
            self.push(format!("UPDATE tabFunctionsOnOff SET UnreadAlertOn='{}';", t.s("isAlertToneOn")));
            self.push(format!("UPDATE tabUnreadAlertSettings SET AlertType='0', AlertToneInterval='{}';", t.s("AlertToneInterval")));
        }
        let hs = self.one("tabHomeSettings");
        let hss = self.one("tabHomeScreenSettings");
        const HOME_TAIL: &str = "P25TSystemShowOption='0',P25TSystemName='MPSCS'";
        const HOME_END: &str = "NumberFormat='1',ScnningTime='460',PromptShowTime='470',PromptHideTime='480',ScanFrqTime='450',IsRfssSiteVisible='1';";
        match self.variant {
            Variant::B => {
                if let Some(hss) = &hss {
                    let fv = hs.as_ref().map(|h| h.i("isFrequencyVisible")).unwrap_or_else(|| {
                        record_error("tabHomeSettings missing".into());
                        0
                    });
                    self.push(format!("UPDATE tabHomeSettings SET isFrequencyVisible='{fv}',{HOME_TAIL},HomeStyle='{}',{HOME_END}", hss.i("HomeScreenStyle")));
                }
            }
            Variant::C | Variant::D | Variant::E => {
                let fv = hs.as_ref().map(|h| h.i("isFrequencyVisible")).unwrap_or(0);
                let rssi = hs.as_ref().map(|h| h.i("StatusBarRssiShowType")).unwrap_or(0);
                let style = hss.as_ref().map(|h| h.i("HomeScreenStyle")).unwrap_or(0);
                self.push(format!("UPDATE tabHomeSettings SET isFrequencyVisible='{fv}',{HOME_TAIL},HomeStyle='{style}',{HOME_END}"));
                self.push(format!("UPDATE tabFunctionsOnOff SET SignalLevel='{rssi}';"));
            }
        }
        if let Some(u) = self.one("tabUiNumberFormat") {
            self.push(format!("update tabUiNumberFormat set CallIdShowType='{}',SysIdShowType='{}',FrequencyUnit='0';", u.s("CallIdShowType"), u.s("SysIdShowType")));
        }
        if let Some(d) = self.one("tabDateTimeSetting") {
            self.push(format!("update tabDateTimeSetting set date_format='{}',hour_format='{}';", d.s("date_format"), d.s("hour_format")));
        }
        if let Some(o) = self.one("tabOutRangeSetting") {
            let rep = o.i("AlertRepeatedly");
            let iv = if rep != 0 { o.i("AlertInterval") } else { 0 };
            let times = if rep != 0 { if iv != 0 { floor_div(o.i("AlertDuration"), iv) + 1 } else { 0 } } else { 0 };
            let dur = if rep != 0 { o.i("AlertDuration") } else { 0 };
            self.push(format!(
                "UPDATE tabOutRangeSetting SET AlertTimes={times},AlertInterval={iv},outRangeCheckTimer={},inRangeCheckTime={},InOutRangeAlert={},AlertTimeout={dur};",
                o.s("outRangeCheckTimer"), o.s("inRangeCheckTime"), o.i("InOutRangeAlert")
            ));
        }
        if let Some(r) = self.one("tabRingModeSettingPPS") {
            self.push(format!(
                "UPDATE tabRingModeSetting SET id='1',isOn='{}',mode='{}',time='{}',IntervalTime='{}',IsVibInterval='{}',VibIntervalOnTime='{}',VibIntervalOffTime='{}',isVoiceStartStopRing='1';",
                r.i("isOn"), r.i("mode"), r.i("time"), r.i("IntervalTime"), r.i("IsVibInterval"), r.i("VibIntervalOnTime"), r.i("VibIntervalOffTime")
            ));
            self.push(format!(
                "UPDATE tabAlertModeSetting SET time='{}',TwoToneAlertType='{}',TwoToneAlertDuration='{}',FiveToneMdcAlertDuration='{}',DigitalAlertDuration='{}',DependOn='0';",
                r.i("time"), r.i("TwoToneAlertType"), r.i("TwoToneAlertDuration"), r.i("FiveToneMdcAlertDuration"), r.i("DigitalAlertDuration")
            ));
            self.push("UPDATE tabFunctionsOnOff SET AlertModeByCH='1';");
        }
        if let Some(r) = self.one("tabReminderAlertSettingPPS") {
            self.push(format!(
                "UPDATE tabReminderAlertSetting SET option='{}',ReminderAlertInterval='{}',ReminderAlertTimeout='{}',ReminderAlertDuration='{}';",
                r.i("option"), r.i("ReminderAlertInterval"), r.i("ReminderAlertTimeout"), r.i("ReminderAlertDuration")
            ));
        }
        if let Some(r) = self.one("tabVoiceMemoSetting") {
            self.push(format!("UPDATE tabMemory SET maxMem='{}' WHERE funId='19';", r.i("MaxNumber")));
            self.push(format!("UPDATE tabAudioRecordTimer SET second='{}' WHERE function='310' AND attribute='1';", r.i("RecordTime")));
            self.push(format!("UPDATE tabVoiceMemoSettings SET AlertToneDur='{}',PlayOption='{}';", r.i("AlertToneDur"), r.i("PlayOption")));
        }
        if let Some(r) = self.one("tabDutyStatusSetting") {
            self.push(format!(
                "UPDATE tabDutySetting SET DutyStatus='{}',OffDutyRecord='{}',DutyCycle='{}',StartTimeHr='{}',StartTimeMin='{}',EndTimeHr='{}',EndTimeMin='{}';",
                r.i("DefaultDutyStatus"), r.i("OffDutyRecordFlag"), r.i("DutyCycle"), r.i("StartTimeHr"), r.i("StartTimeMin"), r.i("EndTimeHr"), r.i("EndTimeMin")
            ));
        }
        if let Some(r) = self.one("tabVoiceStorageSetting") {
            self.push(format!("UPDATE tabAudioRecordTimer SET second='{}' WHERE function='201';", r.i("MaxRecordTime")));
        }
        let fat = self.p("tabFunctionAlertTone");
        if let Some(fa) = fat.iter().find(|x| x.s("FuncID") == "1") {
            let s = fa.s("AlertToneID");
            let n = if s.is_empty() { -1 } else { self.alert_tone_id(s) };
            let tone_id = if n != 9999 && n != 10000 { n + 100 } else { 100 };
            match n {
                9999 => self.push("UPDATE tabZone SET priorityToneID='37';"),
                10000 => self.push("UPDATE tabZone SET priorityToneID='39';"),
                n if 0 < n && n <= 32 => self.push(format!("UPDATE tabZone SET priorityToneID='{n}';")),
                n if n >= 33 => self.push(format!("UPDATE tabZone SET priorityToneID='{tone_id}';")),
                _ => {}
            }
        }
        if let Some(r) = self.one("tabKeypadLockSetting") {
            self.push(format!("UPDATE tabKeypadLockSetting SET AutoKeypadLock='{}';", r.i("AutoKeypadLock")));
        }
        if let Some(r) = self.one("tabBacklightSettings") {
            self.push(format!("UPDATE tabBacklightSettings SET DefaultTime='{}';", r.i("DefaultTime")));
        }
        if let Some(r) = self.one("tabImageMessage") {
            self.push(format!("UPDATE TabImageBoxSetting SET HdphotoPercentForShow='{}';", r.i("HdphotoPercentForShow")));
        }
        for r in self.p("tabSpecialKeyActionTime").iter() {
            self.push(format!("UPDATE tabSpecialKeyActionTime SET ActionTime='{}' WHERE KeyCode='{}';", r.i("ActionTime"), r.i("KeyCode")));
        }
        if let Some(r) = self.one("tabRecvBackgroundColorSetting") {
            self.push(format!(
                "Update tabRecvBackgroundColorSetting SET FlashOnTime='{}',FlashOffTime='{}',FlashDuration='{}',IsVoiceFlash='{}',IsTextFlash='{}',IsImageFlash='{}';",
                r.s("FlashOnTime"), r.s("FlashOffTime"), r.s("FlashDuration"), r.s("IsVoiceFlash"), r.s("IsTextFlash"), r.s("IsImageFlash")
            ));
        }
        if let Some(r) = self.one("tabTowTruckSetting") {
            self.push(format!("UPDATE tabFunctionsOnOff SET CradleType='{}',PowerControlMode='{}';", r.s("CradleType"), r.s("PowerControlMode")));
        }
        let bt = self.one("tabBtHeadsetSetting").map(|r| r.i("BTHeadsetNotification")).unwrap_or(1);
        self.push(format!("UPDATE tabFunctionsOnOff SET BTHeadsetNotification='{bt}';"));
        let (a, b, c) = self.one("tabGPRSAckSettingPPS").map(|r| (r.i("DutyStatusACK"), r.i("InOutRangeACK"), r.i("PowerOffACK"))).unwrap_or((0, 0, 0));
        self.push(format!("UPDATE tabFunctionsOnOff SET DutyStatusACK='{a}',OORStatusACK='{b}',PowerOffACK='{c}';"));
        let (h, to, ua, rv) = self
            .one("tabTGIDHoldAndRemoveSetting")
            .map(|r| (r.i("TGHoldFunc"), r.i("TGHoldTimeout"), r.i("UnholdAlert"), r.i("TGRemvFunc")))
            .unwrap_or((0, 0, 0, 0));
        self.push(format!("UPDATE tabFunctionsOnOff SET TGHoldFunc='{h}',TGRemvFunc='{rv}',UnholdAlert='{ua}';"));
        self.push(format!("UPDATE tabFunctionsParam SET TGHoldTimeout='{to}';"));
    }

    fn convert_device_parameters(&mut self) {
        let dev = self.p("tabDeviceOnOff");
        if let Some(d) = dev.iter().find(|x| x.s("DeviceID") == "11") {
            self.push(format!("update tabFunctionsOnOff set KeyToneOn = '{}';", d.i("OnOff")));
        }
        if let Some(b) = self.one("tabBacklightModeSettings") {
            self.push(format!("update tabBacklightModeSettings set id  ='{}',Mode ='{}',Brightness ='{}';", b.s("ID"), b.s("Mode"), b.s("Brightness")));
            self.push(format!("update tabFunctionsOnOff set backlighton = '{}';", parse_int(b.s("Mode"), "backlight Mode")));
            self.push(format!("update tabBackLightModeSetting set Mode='{}',BrightnessLevel='{}';", b.s("Mode"), b.s("BrightLevel")));
        }
        for r in self.p("tabAudioToneSetting").iter() {
            let tt = r.s("ToneType");
            if tt != "0" && tt != "1" {
                continue;
            }
            let v = tone_vol(r.i("ToneVolume"));
            if tt == "0" {
                self.push(format!("update tabAlertToneSettings set ToneVolume = '{v}' where SceneID = 9 or (SceneID >= 100 and SceneID < 200);"));
            } else {
                self.push(format!("update tabAlertToneSettings set ToneVolume = '{v}' where SceneID <> 9 and (SceneID < 100 or SceneID >= 200);"));
            }
        }
        for r in self.p("tabFuncClassToneVolume").iter() {
            self.push(format!("update tabFuncClassToneVolume set type='{}', volume='{}' where funcClass='{}';", r.s("type"), new_vol(r.i("volume")), r.s("funcClass")));
        }
        for r in self.p("tabFuncLEDSettings").iter() {
            if r.s("funID") != "3" {
                continue;
            }
            self.push(format!(
                "update tabFuncLEDSettings set period = {}, percent = {}, cycle = {}, interval = {}, count = {} where funId = {};",
                r.s("period"), r.s("percent"), r.s("cycle"), r.s("interval"), r.s("count"), r.s("funId")
            ));
        }
    }

    fn convert_detail_screen_item(&mut self) {
        for r in self.p("tabDetailScreenItem").iter() {
            let (rm, it) = (r.i("RecvMode"), r.i("Item"));
            if it == 5 && (3..=6).contains(&rm) {
                continue;
            }
            let vals: Vec<String> = ["Style", "RecvMode", "Item", "ItemIndex", "ContentColor", "FontSize", "BackgroundColor", "LineHeight", "Alignment"]
                .iter()
                .map(|c| r.s(c).to_string())
                .collect();
            self.push(format!(
                "INSERT INTO tabDetailScreenItem(Style, RecvMode, Item, ItemIndex, ContentColor, FontSize, BackgroundColor, LineHeight, Alignment)VALUES({});",
                quote_list(&vals)
            ));
        }
    }

    fn convert_dispatch_center_manual_ack(&mut self) {
        let rows = self.p("tabDispathCenterManualAck");
        for (i, r) in rows.iter().enumerate() {
            self.push(format!("insert into tabDispathCenterManualAck(id,text) values('{}','{}');", i + 1, q(r.s("text"))));
        }
        if let Some(a) = self.one("tabDispathCenterACK")
            && !rows.is_empty() {
                self.push("delete from tabDispathCenterAck;");
                self.push(format!("insert into tabDispathCenterAck(manualAckKeeptime) values ({});", parse_int(a.s("manualAckKeeptime"), "manualAckKeeptime")));
            }
    }

    fn convert_custom_tones(&mut self) {
        let tones = self.custom_alert_tones.clone();
        for (key, v) in tones {
            let f = self.alert_tone_file_name_str(&key);
            let name = format!("/defpara/Applications/chvoicerecord/{f}.dat");
            self.push(format!("insert into tabTone(id, alias, type, sourceID) values('{v}','{f}','2','{v}');"));
            self.push(format!("insert into tabToneFileDetail(sourceID,type,file) values('{v}','1','{}');", q(&name)));
        }
    }

    // ---------------- encryption ----------------

    fn convert_encryption(&mut self) -> Result<(), Error> {
        let mut flag = false;
        if let Some(es) = self.one("tabEncryptionSetting") {
            flag = es.i("EncryptionType") == 3;
            let cols = ["PeriodicKeyfailAlert", "ClearAlert", "ChkSecureSwitchWhenStrap", "DisplayOnSecureSwitch", "DisplayOnPTT", "DisplayOnChannelChange"];
            let rest: Vec<&str> = cols.iter().map(|c| es.s(c)).collect();
            self.push(format!(
                "insert into tabEncryptionSetting values('1','{}','{}');",
                if flag { 3 } else { 1 },
                rest.join("','")
            ));
        }
        if let Some(kvl) = self.one("tabKVLUSBConnectionSetting") {
            self.push("delete from tabKVLUSBConnect;");
            self.push(format!(
                "insert into tabKVLUSBConnect values('1','{}','{}','{}','{}');",
                kvl.s("SubscriberIPAddr"), kvl.s("SubscriberUDPPort"), kvl.s("KeyLoaderIPAddr"), kvl.s("KeyLoaderUDPPort")
            ));
        }
        let otar = self.p("tabOTARFeatureControlSetting");
        let conn = self.p("tabOTARConnectionSetting");
        let mut kmf: Vec<(String, i64)> = Vec::new();
        const OTAR_COLS: [&str; 9] = [
            "P25OTAREnable", "P25OTARRadioID", "P25OTARRxSecurityLevel", "P25OTARTxSecurityLevel", "ResponseKind",
            "TimeBetweenAttempts", "UserSelectableRekeyRequest", "OTARInactivityTime", "RadioInhibitP25OTAR",
        ];
        for r in conn.iter() {
            if kmf.iter().any(|(k, _)| k == r.s("ID")) {
                continue;
            }
            let n = kmf.len() as i64 + 1;
            kmf.push((r.s("ID").to_string(), n));
            let base = format!(
                "'{n}','{}','{}','{}','{}','{}'",
                q(r.s("KMFName")), r.s("KMFIPAddr"), r.s("KMFUDPPort"), r.s("SubscriberIPAddr"), r.s("SubscriberOTARPort")
            );
            match self.variant {
                Variant::B => self.push(format!("insert into tabKMFSystem values({base});")),
                Variant::C | Variant::D | Variant::E => {
                    let a = sel(&otar, "ID", r.s("ID")).first().map(|x| (*x).clone()).ok_or_else(|| Error::Invalid("OTAR feature row missing".into()))?;
                    self.push(format!("insert into tabKMFSystem values({base},'{}');", a.s("OTARBtMac")));
                    let mut vals = vec![n.to_string()];
                    vals.extend(OTAR_COLS.iter().map(|c| a.s(c).to_string()));
                    vals.extend(["UserSelectableKeyset", "UserSelectableKey", "NetworkAddrType"].iter().map(|c| a.s(c).to_string()));
                    vals.push(r.s("SubscriberIPAddr").into());
                    vals.push(r.s("SubscriberOTARPort").into());
                    vals.extend(["PacketDataRegVersion", "IPHeaderCompression", "DAC"].iter().map(|c| a.s(c).to_string()));
                    self.push(format!("insert into tabP25OTARFeatureControl values({});", quote_list(&vals)));
                }
            }
        }
        if !flag {
            return Ok(());
        }
        let keys = self.p("tabEncryptionKeyList");
        if self.p("tabProtocolSetting").is_empty() || keys.is_empty() {
            return Ok(());
        }
        for (i, k) in keys.iter().enumerate() {
            self.push(format!("insert into tabEncryptionKeyList values('{}','{}','{}','{}');", i + 1, k.s("KeyID"), q(k.s("KeyAlias")), k.s("CKR")));
        }
        // Per-channel key assignment is only implemented for the paths PPS takes
        // with a key per channel; anything else is refused.
        let mut had_otar = false;
        let mut knobs: Vec<Row> = self.p("tabSingleConvCHKnob").iter().cloned().collect();
        let adv: Vec<Row> = self.p("tabAdvancedConvCHKnob").iter().filter(|r| !r.s("DigitalProtocolSettingID").is_empty()).cloned().collect();
        if !adv.is_empty() && !self.p("tabChannelGroup").is_empty() {
            knobs.extend(adv);
        }
        knobs.extend(self.p("tabP25TrunkingSystemKnob").iter().cloned());
        let settings = self.p("tabProtocolSetting");
        for row in &knobs {
            let key = row.s("ChannelID");
            let Some(&ch_no) = self.channel_map.get(key) else { continue };
            if row.i("EncryptionBy") == 2 {
                continue;
            }
            let pcol = if row.has("DigitalProtocolSettingID") { "DigitalProtocolSettingID" } else { "ProtocolSettingID" };
            let Some(a) = sel(&settings, "ID", row.s(pcol)).first().map(|x| (*x).clone()) else { continue };
            let proto = a.i("ProtocolType");
            if !matches!(proto, 9 | 8 | 4 | 7) {
                continue;
            }
            if matches!(proto, 4 | 7) && row.has("EncryptedCallFlag") && row.i("EncryptedCallFlag") == 0 {
                continue;
            }
            let mut t = row.s("KMFSystemID").to_string();
            match kmf.iter().find(|(k, _)| *k == t) {
                Some((_, v)) => self.push(format!("insert into tabKMFSystemByChannel values('{ch_no}','{v}');")),
                None => {
                    self.push(format!("insert into tabKMFSystemByChannel values('{ch_no}','1');"));
                    t = "000000000000".into();
                }
            }
            if !otar.is_empty()
                && let Some(a2) = sel(&otar, "ID", &t).first() {
                    had_otar = true;
                    match self.variant {
                        Variant::B => {
                            let mut vals = vec![ch_no.to_string()];
                            vals.extend(OTAR_COLS.iter().map(|c| a2.s(c).to_string()));
                            vals.push(row.s("UserSelectableKeyset").into());
                            vals.push(row.s("UserSelectableKey").into());
                            self.push(format!("insert into tabP25OTARFeatureControl values({});", quote_list(&vals)));
                            self.push(format!("insert into tabP25OTARFeatureControlByChannel values('{ch_no}','{ch_no}');"));
                        }
                        Variant::C | Variant::D | Variant::E => {
                            let v = kmf.iter().find(|(k, _)| *k == t).map(|(_, v)| *v).unwrap_or(1);
                            self.push(format!("insert into tabP25OTARFeatureControlByChannel values('{ch_no}','{v}');"));
                        }
                    }
                }
            if row.i("EncryptionBy") == 0 || proto == 4 {
                let kid = row.s("EncryptionKeyID");
                let cipher_key = sel(&keys, "ID", kid).first().or(sel(&keys, "ID", "000000000000").first()).map(|x| (*x).clone());
                if let Some(cipher_key) = cipher_key {
                    self.push(format!(
                        "insert into tabCipherKeyAssignment values('{ch_no}','{}','{}','{}','1','{}','2','0');",
                        q(cipher_key.s("KeyAlias")), cipher_key.s("CKR"), cipher_key.s("KeyID"), (ch_no / 100 - 1) * 16 + (ch_no % 100 - 1)
                    ));
                }
            } else {
                return Err(Error::Unsupported("per-talkgroup encryption keys".into()));
            }
        }
        if self.variant == Variant::B && !had_otar && !otar.is_empty() {
            for r6 in otar.iter() {
                if let Some((_, v)) = kmf.iter().find(|(k, _)| k == r6.s("ID")) {
                    let mut vals = vec![v.to_string()];
                    vals.extend(OTAR_COLS.iter().map(|c| r6.s(c).to_string()));
                    vals.push(r6.s("UserSelectableKeyset").into());
                    vals.push(r6.s("UserSelectableKey").into());
                    self.push(format!("insert into tabP25OTARFeatureControl values({});", quote_list(&vals)));
                }
            }
        }
        Ok(())
    }

    // ---------------- private (menus, pager identity) ----------------

    fn convert_private(&mut self) -> Result<Vec<String>, Error> {
        let mut out = Vec::new();
        let mid = self.pager_id.clone().ok_or_else(|| Error::Invalid("profile has no pager member (RadioNo 99999)".into()))?;
        let my_id = self.member_map[&mid];
        out.push(format!("insert into tabMyContact(ID) values('{my_id}');"));
        let members = self.p("tabMember_New");
        let mut am = 0;
        if let Some(mr) = sel(&members, "MemberID", &mid).first() {
            am = mr.i("AlertMode") - 1;
            out.push(format!("update tabRadioBaseSetting set RadioNo = '{}';", q(mr.s("UserDeviceID"))));
        }
        let mfl = self.p("tabMemberFunctionList");
        let std_on: HashSet<String> = self.data_table("tabU4Function").iter().filter(|r| r.s("FunctionOnOff") == "1").map(|r| r.s("FunctionID").to_string()).collect();
        let mut fl: Vec<(String, String)> = mfl.iter().map(|r| (r.s("FunctionID").to_string(), r.s("Enable").to_string())).collect();
        fl.extend(mfl.iter().filter(|r| std_on.contains(r.s("FunctionID"))).map(|r| (r.s("FunctionID").to_string(), r.s("Enable").to_string())));
        let mpc = self.one("tabMfgProductCode");
        let series = mpc.as_ref().map(|m| format!("{}{}", m.s("ProductSeriesCode"), m.s("ProductSeriesNo"))).ok_or_else(|| Error::Invalid("profile has no tabMfgProductCode".into()))?;

        let en = enable_list(&sorted_by_order(self.data_table("tabMenuBase")).into_iter().filter(|r| model_ok(r, &series, true)).collect::<Vec<_>>(), mpc.as_ref(), &fl);
        let mut page = -1;
        for (i, r) in en.iter().enumerate() {
            let pos = i % 9;
            if pos == 0 {
                page += 1;
            }
            out.push(format!("insert into tabMenuLayout(PageNo,Position,ItemKey) values('{page}','{pos}','{}');", r.s("ItemKey")));
            out.push(format!("insert into tabMenuListLayout(Position,ItemKey) values('{i}','{}');", r.s("ItemKey")));
        }

        let fsb = self.data_table("tabFunctionStatusBase");
        let top: Vec<Row> = sorted_by_order(fsb.iter().filter(|r| r.is_null("BelongSettingItem")).cloned().collect())
            .into_iter()
            .filter(|r| model_ok(r, &series, true))
            .collect();
        let mut items: Vec<FsItem> = enable_list(&top, mpc.as_ref(), &fl)
            .into_iter()
            .enumerate()
            .map(|(i, r)| FsItem {
                setting_item_id: r.s("SettingItemID").into(),
                default_order: (i + 1).to_string(),
                name: r.s("Name").into(),
                item_type: r.s("ItemType").into(),
                has_child: r.s("HasChild").into(),
                belong: r.get("BelongSettingItem").map(String::from),
            })
            .collect();
        // Parents sorted by DefaultOrder as strings (DataTable.Select order).
        let mut parents: Vec<(String, String)> = items.iter().filter(|r| r.has_child == "1").map(|r| (r.default_order.clone(), r.setting_item_id.clone())).collect();
        parents.sort_by(|a, b| a.0.cmp(&b.0));
        for (order, pid) in parents {
            let kids: Vec<Row> = sorted_by_order(fsb.iter().filter(|r| r.s("BelongSettingItem") == pid).cloned().collect())
                .into_iter()
                .filter(|r| model_ok(r, &series, true))
                .collect();
            let mut next_order = if parse_int(&order, "DefaultOrder") % 2 != 0 { 1 } else { 2 };
            for kr in kids {
                items.push(FsItem {
                    setting_item_id: kr.s("SettingItemID").into(),
                    default_order: next_order.to_string(),
                    name: kr.s("Name").into(),
                    item_type: kr.s("ItemType").into(),
                    has_child: kr.s("HasChild").into(),
                    belong: kr.get("BelongSettingItem").map(String::from),
                });
                next_order += 2;
            }
        }
        for r in &items {
            if r.setting_item_id == "54" {
                continue;
            }
            // PPS does not escape quotes in Name here.
            out.push(format!(
                "insert into tabFunctionStatus values('{}','{}','{}','{}','{}','{}','0');",
                r.setting_item_id, r.default_order, r.name, r.item_type, r.has_child, r.belong.as_deref().unwrap_or("")
            ));
        }

        let base: Vec<Row> = sorted_by_order(self.data_table("tabFunctionSettingBase")).into_iter().filter(|r| model_ok(r, &series, true)).collect();
        let mut seen: Vec<String> = Vec::new();
        for r in enable_list(&base, mpc.as_ref(), &fl) {
            let sid = r.s("SettingID").to_string();
            if seen.contains(&sid) {
                continue;
            }
            seen.push(sid.clone());
            out.push(format!("INSERT INTO tabFunctionSettingList(SettingItemSerial,FuncSettingId,Enable) values('{}','{sid}','1');", seen.len()));
        }

        let base: Vec<Row> = sorted_by_order(self.data_table("tabOtaMenuBase")).into_iter().filter(|r| model_ok(r, &series, false)).collect();
        for (i, r) in enable_list(&base, mpc.as_ref(), &fl).iter().enumerate() {
            out.push(format!("insert into tabOtaMenuList(OTAMenuSerial,OTASettingId,Enable) values('{}','{}','1');", i + 1, r.s("SettingID")));
        }
        let base = sorted_by_order(self.data_table("tabSpecialMenuBase"));
        for (i, r) in enable_list(&base, mpc.as_ref(), &fl).iter().enumerate() {
            out.push(format!("insert into tabSpecialMenuLayout(ID,ItemKey) values('{}','{}');", i + 1, r.s("ItemKey")));
        }
        for r in enable_list(&self.data_table("tabEnablePassiveWorkStationFunctionsBase"), mpc.as_ref(), &fl) {
            out.push(format!("insert into tabEnablePassiveWorkStationFunctions(FunctionID, Note) values('{}','');", r.s("PassiveFuncID")));
        }
        for r in self.p("tabDebugEntry").iter() {
            if !r.s("enable").is_empty() && r.i("enable") > 0 {
                return Err(Error::Unsupported("engineer debug entries".into()));
            }
        }
        let fen = |fid: &str| fl.iter().find(|(f, _)| f == fid).is_some_and(|(_, e)| parse_int(e, "Enable") > 0);
        if !fen("141") {
            out.push("DELETE FROM tabZone WHERE ID='1025';".into());
        }
        if !fen("161") {
            out.push("DELETE FROM tabZone WHERE ID='1100';".into());
        }
        out.push(format!("insert into tabRecvBehaviorByContactID(ChId,RadioID,Priority,Duty,AlertMode,VoiceStorage,SigType) values('0','{my_id}','0','0','{am}','1','-1');"));
        out.push("Delete from tabBleParamSetting;".into());
        match self.one("tabBleParamSetting") {
            Some(b) => out.push(format!(
                "Insert into tabBleParamSetting values ({},{},'{}','{}',{});",
                b.i("BlePair"), b.i("UserSettingAdvertiseStatus"), b.s("BleFwVer"), q(b.s("BleName")), b.i("BleAdvertiseTime")
            )),
            None => out.push("Insert into tabBleParamSetting values (0,0,'','',0);".into()),
        }
        Ok(out)
    }
}

fn quote_list(vals: &[String]) -> String {
    vals.iter().map(|v| format!("'{v}'")).collect::<Vec<_>>().join(",")
}

/// Stable sort by DefaultOrder: numbers (ascending) before non-numbers (lexical).
fn sorted_by_order(mut rows: Vec<Row>) -> Vec<Row> {
    fn key(r: &Row) -> (u8, f64, String) {
        let s = r.s("DefaultOrder");
        match s.trim().parse::<f64>() {
            Ok(v) if !r.is_null("DefaultOrder") => (0, v, String::new()),
            _ => (1, 0.0, s.to_string()),
        }
    }
    rows.sort_by(|a, b| {
        let (ka, kb) = (key(a), key(b));
        ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1)).then_with(|| ka.2.cmp(&kb.2))
    });
    rows
}

fn model_ok(r: &Row, series: &str, allow_empty: bool) -> bool {
    let m = r.s("Model");
    (allow_empty && m.is_empty()) || m.to_uppercase().contains(&series.to_uppercase())
}

/// Catalog rows enabled for this model's hardware and the member's functions.
fn enable_list(base: &[Row], mpc: Option<&Row>, fl: &[(String, String)]) -> Vec<Row> {
    let hw = mpc.map(|m| m.s("ProductHardwareCode").to_uppercase());
    let on: HashSet<&str> = fl.iter().filter(|(_, e)| e == "1").map(|(f, _)| f.as_str()).collect();
    let mut out = Vec::new();
    for r in base {
        let (blk, fid) = (r.s("HWBlock"), r.s("FunctionID"));
        if !blk.is_empty() {
            match &hw {
                Some(hw) if hw.contains(&blk.to_uppercase()) => {}
                _ => continue,
            }
        }
        if fid.is_empty() {
            out.push(r.clone());
            continue;
        }
        if fl.is_empty() {
            continue;
        }
        if fid.split(',').any(|grp| grp.split(';').all(|x| on.contains(x))) {
            out.push(r.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_helpers_match_pps() {
        assert_eq!(get_recv_mode(33), 32);
        assert_eq!(get_recv_mode(48), 64);
        assert_eq!(get_recv_mode(2), 0);
        assert_eq!(tone_source(9999), 100);
        assert_eq!(tone_source(5), 105);
        assert_eq!(relay(65535), (1, 0));
        assert_eq!(squelch_value(1), (102, 8));
        assert_eq!(tone_vol(2), 28);
        assert_eq!(new_vol(0), 48);
        assert_eq!(new_vol(2), 33);
        assert_eq!(floor_div(-3, 2), -2);
        assert_eq!(Conv::convert_address("100", 9), "2147483748");
        assert_eq!(Conv::convert_address("100", 1), "100");
    }
}
