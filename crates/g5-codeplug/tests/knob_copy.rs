//! Copying, moving and swapping knobs against private real-pager fixtures
//! (skipped without them), per notes/knob-copy-move.md: the profile rows PPS
//! writes, and exactly which radio tables change.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use g5_codeplug::compare::{TableDiff, TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::digital_knob::{DigitalKnobEdit, DigitalProtocol, add_digital_knob};
use g5_codeplug::edit::{DIGITAL_RENUMBER_TABLES, ZONE_RENUMBER_TABLES};
use g5_codeplug::knob_copy::{KnobCopy, KnobMove, copy_knob, move_knob};
use g5_codeplug::knob_remove::KnobKind;
use g5_codeplug::row::{Row, load_table};
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::zone_edit::{NewZone, create_zone};
use g5_codeplug::{Profile, Templates};

const NOW: &str = "2026-10-20 12:00:00";
const CH: &str = "5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d";

/// A knob position: zone name and knob.
type Pos<'a> = (&'a str, i64);

/// What every new channel adds on the radio, whatever its kind.
const CHANNEL: &[&str] = &[
    "tabAlertModeSetting",
    "tabChannel",
    "tabChannelAdvInfo",
    "tabChannelAnnouncement",
    "tabContactOfChan",
    "tabDispatchChannel",
    "tabFunctionOnOffOfChan",
    "tabFunctionParamOfChan",
    "tabR2RModeSetting",
    "tabReceivingMode",
    "tabRecvBehaviorByContactID",
    "tabRecvGroupList",
    "tabRecvVoiceCallRecordByChannel",
    "tabResetModeSetting",
    "tabVoiceCallSettings",
    "tabZoneChannelList",
];
const TRUNKED_EXTRA: &[&str] = &["tabCipherKeyAssignment", "tabKMFSystemByChannel", "tabP25OTARFeatureControlByChannel", "tabP25TSetting"];
const SCAN: &[&str] = &[
    "tabAlertModeSetting",
    "tabChannelAnnouncement",
    "tabFunctionOnOffOfChan",
    "tabFunctionParamOfChan",
    "tabReceivingMode",
    "tabRecvVoiceCallRecordByChannel",
    "tabResetModeSetting",
    "tabScanListSettings",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The fixtures are real pager backups, so tests use stand-ins for zone and
/// knob names that would identify the owner. `fixture-names.txt` in the
/// backups folder (gitignored, one `stand-in=real name` per line) maps them
/// back; a name with no entry is used as is.
fn real(stand_in: &'static str) -> &'static str {
    static NAMES: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        fs::read_to_string(env_or("G5_BACKUPS", repo().join("backups")).join("fixture-names.txt"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), &*Box::leak(v.trim().to_string().into_boxed_str())))
            .collect()
    });
    names.get(stand_in).copied().unwrap_or(stand_in)
}

fn env_or(var: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).unwrap_or(default)
}

fn version(p: &Path) -> Option<String> {
    fs::read_to_string(p.join("MANIFEST.txt")).ok()?.lines().find_map(|l| l.strip_prefix("Radio DB version:").map(|v| v.trim().to_string()))
}

/// The newest backup (by folder name) that `want` accepts.
fn newest(want: impl Fn(&Path, &str) -> bool) -> Option<(PathBuf, Templates, Variant)> {
    let tpl = Templates::new(env_or("G5_TEMPLATES", repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"))).ok()?;
    let mut dirs: Vec<PathBuf> = fs::read_dir(env_or("G5_BACKUPS", repo().join("backups"))).ok()?.filter_map(|e| e.ok().map(|e| e.path())).collect();
    dirs.sort();
    let dir = dirs.into_iter().rev().find(|d| version(d).is_some_and(|v| Variant::for_db_version(&v).is_some() && want(d, &v)))?;
    let variant = Variant::for_db_version(&version(&dir)?)?;
    Some((dir, tpl, variant))
}

/// Unit B's newest backup: trunked, conventional, scan and a DMR knob.
fn unit_b() -> Option<(PathBuf, Templates, Variant)> {
    newest(|d, _| {
        Profile::from_backup(d).is_ok_and(|p| {
            let dmr: Vec<String> = load_table(&p.db, "tabProtocolSetting").iter().filter(|s| s.i("ProtocolType") == 4).map(|s| s.s("ID").to_string()).collect();
            load_table(&p.db, "tabSingleConvCHKnob").iter().any(|k| dmr.iter().any(|d| d == k.s("ProtocolSettingID")))
        })
    })
}

/// Unit A (DBV1.41b).
fn unit_a() -> Option<(PathBuf, Templates, Variant)> {
    newest(|_, v| v == "DBV1.41b")
}

fn convert_to(profile: &Profile, templates: &Templates, variant: Variant) -> tempfile::NamedTempFile {
    let out = tempfile::NamedTempFile::new().unwrap();
    let report = convert(profile, templates, variant, out.path()).unwrap();
    assert!(report.unverified.is_empty(), "unverified: {:?}", report.unverified);
    out
}

fn diff(ours: &Path, theirs: &Path) -> Vec<TableDiff> {
    let a = sqlite2::Connection::open(ours).unwrap();
    let b = sqlite2::Connection::open(theirs).unwrap();
    compare(&a, &b).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).collect()
}

fn names(d: &[TableDiff]) -> Vec<String> {
    d.iter().map(|t| t.table.clone()).collect()
}

fn expect(lists: &[&[&str]]) -> Vec<String> {
    let mut v: Vec<String> = lists.iter().flat_map(|l| l.iter().map(|s| s.to_string())).collect();
    v.sort();
    v
}

fn rows(db: &Path, sql: &str) -> Vec<Vec<String>> {
    let c = sqlite2::Connection::open(db).unwrap();
    c.query(sql, &[]).unwrap().rows.into_iter().map(|r| r.into_iter().map(|v| v.unwrap_or_default()).collect()).collect()
}

fn zone_id(p: &Profile, zone: &str) -> String {
    load_table(&p.db, "tabZone_New").into_iter().find(|z| z.s("Name") == zone).unwrap().s("ID").to_string()
}

/// Radio channel ID of a position (zone position x 100 + knob).
fn zk(p: &Profile, zone: &str, knob: i64) -> i64 {
    (load_table(&p.db, "tabZone_New").iter().position(|z| z.s("Name") == zone).unwrap() as i64 + 1) * 100 + knob
}

fn at(p: &Profile, table: &str, zone: &str, knob: i64) -> Vec<Row> {
    let zid = zone_id(p, zone);
    load_table(&p.db, table).into_iter().filter(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == knob).collect()
}

fn values(r: &Row, cols: &[String], skip: &[&str]) -> Vec<Option<String>> {
    cols.iter().filter(|c| !skip.contains(&c.as_str())).map(|c| r.get(c).map(str::to_string)).collect()
}

/// The knob's own ChannelID, if it has one.
fn channel(p: &Profile, zone: &str, knob: i64) -> Option<String> {
    ["tabSingleConvCHKnob", "tabP25TrunkingSystemKnob"].iter().find_map(|t| at(p, t, zone, knob).first().map(|r| r.s("ChannelID").to_string()))
}

/// A copy is the source's rows with the new position, ChannelID and name, as PPS's copyKnob writes them.
fn assert_copied(p: &Profile, from: (&str, i64), to: (&str, i64), name: &str) {
    let recv_cols = p.db.table_columns("tabReceivingMode_NewG4").unwrap();
    let (src, dst) = (&at(p, "tabReceivingMode_NewG4", from.0, from.1)[0], &at(p, "tabReceivingMode_NewG4", to.0, to.1)[0]);
    assert_eq!(dst.s("ChannelName"), name);
    assert_eq!(values(src, &recv_cols, &["ZoneID", "KnobIndex", "ChannelName"]), values(dst, &recv_cols, &["ZoneID", "KnobIndex", "ChannelName"]), "receive row");
    for t in ["tabSingleConvCHKnob", "tabP25TrunkingSystemKnob", "tabMultiChannelScan", "tabMultiCHScanList"] {
        let cols = p.db.table_columns(t).unwrap();
        let skip = ["ZoneID", "KnobIndex", "ChannelID"];
        let (a, b) = (at(p, t, from.0, from.1), at(p, t, to.0, to.1));
        assert_eq!(a.len(), b.len(), "{t} rows");
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(values(x, &cols, &skip), values(y, &cols, &skip), "{t}");
            if t == "tabMultiCHScanList" {
                assert_eq!(x.s("ChannelID"), y.s("ChannelID"), "a copied scan lists the same members");
            } else if x.has("ChannelID") {
                assert_eq!(y.s("ChannelID"), CH);
            }
        }
    }
    let Some(old) = channel(p, from.0, from.1) else { return };
    for t in ["tabChannelGroupList", "tabChannelGroup", "tabRecvSaList", "tabP25TSubGroupEnable"] {
        let cols = p.db.table_columns(t).unwrap();
        let all = load_table(&p.db, t);
        let (a, b): (Vec<&Row>, Vec<&Row>) = (all.iter().filter(|r| r.s("ChannelID") == old).collect(), all.iter().filter(|r| r.s("ChannelID") == CH).collect());
        assert_eq!(a.len(), b.len(), "{t} rows");
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(values(x, &cols, &["ChannelID"]), values(y, &cols, &["ChannelID"]), "{t}");
        }
    }
}

/// The copy's radio rows equal the source's, channel ID aside.
fn assert_same_radio(db: &Path, from: i64, to: i64) {
    for (t, col) in [("tabChannelAdvInfo", "ID"), ("tabScanListSettings", "ScanListSettingID"), ("tabResetModeSetting", "ID")] {
        let get = |zk: i64| rows(db, &format!("SELECT * FROM {t} WHERE {col} = '{zk}'")).into_iter().map(|r| r[1..].to_vec()).collect::<Vec<_>>();
        assert_eq!(get(from), get(to), "{t}");
    }
    let lists = |zk: i64| rows(db, &format!("SELECT SigType,GroupAddr,SeqNo FROM tabRecvGroupList WHERE ZoneKnobID = '{zk}'"));
    assert_eq!(lists(from), lists(to), "tabRecvGroupList");
}

fn copy(zone: &str, knob: i64, to_zone: &str, to_knob: i64, name: &str) -> KnobCopy {
    KnobCopy { zone: zone.into(), knob, to_zone: to_zone.into(), to_knob, name: name.into(), channel_id: Some(CH.into()) }
}

fn mv(zone: &str, knob: i64, to_zone: &str, to_knob: i64, swap: bool) -> KnobMove {
    KnobMove { zone: zone.into(), knob, to_zone: to_zone.into(), to_knob, swap }
}

/// Every table a move changes is keyed by channel ID and keeps its row count.
fn assert_renumbering(d: &[TableDiff]) {
    for t in d {
        assert!(ZONE_RENUMBER_TABLES.contains(&t.table.as_str()) || DIGITAL_RENUMBER_TABLES.contains(&t.table.as_str()), "{} isn't keyed by channel", t.table);
        if t.table != "tabZone" {
            assert_eq!(t.left_rows, t.right_rows, "{}", t.table);
        }
    }
}

#[test]
fn copies_each_kind_within_and_across_zones() {
    let Some((dir, tpl, v)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let radio = dir.join(BACKUP_RADIO_DB);
    let freqs = |p: &Profile| load_table(&p.db, "tabConvFreqList").len();
    // (source, target, name, kind, radio tables)
    let cases: [(Pos, Pos, &str, KnobKind, Vec<String>); 7] = [
        ((real("Metro"), 1), ("Test", 4), real("MTFD 2"), KnobKind::Trunked, expect(&[CHANNEL, TRUNKED_EXTRA])),
        ((real("Metro"), 2), ("Test", 5), real("STATEWIDE 2"), KnobKind::Trunked, expect(&[CHANNEL, TRUNKED_EXTRA])),
        (("Test", 1), ("Test", 4), "TEST 2", KnobKind::Trunked, expect(&[CHANNEL, TRUNKED_EXTRA])),
        (("Lake", 3), ("Test", 4), real("Elmwood Fire"), KnobKind::Conventional, expect(&[CHANNEL, &["tabRepeaterSettingOfChan", "tabRecvSaList"]])),
        (("Test", 2), ("Test", 6), "DMR Test 2", KnobKind::Digital, expect(&[CHANNEL, &["tabRepeaterSettingOfChan", "tabDMRSignaling", "tabDMRRepeaterSetting"]])),
        (("NOAA", 1), ("Test", 4), "NOAA Scan 2", KnobKind::Scan, expect(&[SCAN])),
        (("Lake", 2), ("Test", 8), real("Elmwood Scan"), KnobKind::Scan, expect(&[SCAN])),
    ];
    for (from, to, name, kind, tables) in cases {
        let p = Profile::from_backup(&dir).unwrap();
        let before = freqs(&p);
        let ch = copy_knob(&p, &copy(from.0, from.1, to.0, to.1, name), NOW).unwrap();
        assert_eq!((ch.kind, ch.knob_name.as_str()), (kind, name));
        assert_copied(&p, from, to, name);
        assert_eq!(freqs(&p), before, "the copy shares the source's frequency row, as PPS's paste does");
        assert_eq!(load_table(&p.db, "tabOrganization_New")[0].s("FinishDate"), NOW);
        let out = convert_to(&p, &tpl, v);
        let d = diff(out.path(), &radio);
        assert_eq!(names(&d), tables, "{from:?} -> {to:?}");
        for t in &d {
            assert!(t.left_rows > t.right_rows, "{}: a copy only adds rows", t.table);
        }
        assert_same_radio(out.path(), zk(&p, from.0, from.1), zk(&p, to.0, to.1));
    }
}

#[test]
fn copies_on_unit_a() {
    let Some((dir, tpl, v)) = unit_a() else { return eprintln!("skipping: fixtures not available") };
    let radio = dir.join(BACKUP_RADIO_DB);
    // 1.41b has one more per-channel P25 table than 1.41c.
    let trunked = expect(&[CHANNEL, TRUNKED_EXTRA, &["tabP25OTARFeatureControl"]]);
    let cases: [(Pos, Pos, Vec<String>); 4] = [
        ((real("Metro"), 1), (real("Metro"), 6), trunked),
        (("Lake", 3), ("Lake", 7), expect(&[CHANNEL, &["tabRepeaterSettingOfChan", "tabRecvSaList"]])),
        (("NOAA", 2), (real("Metro"), 7), expect(&[CHANNEL, &["tabRepeaterSettingOfChan"]])),
        (("NOAA", 1), (real("Metro"), 8), expect(&[SCAN])),
    ];
    for (from, to, tables) in cases {
        let p = Profile::from_backup(&dir).unwrap();
        copy_knob(&p, &copy(from.0, from.1, to.0, to.1, "Copy"), NOW).unwrap();
        assert_copied(&p, from, to, "Copy");
        let out = convert_to(&p, &tpl, v);
        assert_eq!(names(&diff(out.path(), &radio)), tables, "{from:?} -> {to:?}");
        assert_same_radio(out.path(), zk(&p, from.0, from.1), zk(&p, to.0, to.1));
    }
}

#[test]
fn copying_a_p25_conventional_knob() {
    let Some((dir, tpl, v)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let e = DigitalKnobEdit {
        zone: "Test".into(),
        knob: 4,
        name: "P25 Conv".into(),
        rx_hz: 155_310_000,
        protocol: DigitalProtocol::P25c { nac: 0x293, filter_nac: true },
        talkgroups: vec![],
        repeater_mode: false,
        voice_prompt: 0,
        channel_id: "0c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f".into(),
        freq_id: "1d2e3f4a-5b6c-4d7e-8f9a-0b1c2d3e4f5a".into(),
    };
    add_digital_knob(&p, &tpl, &e, NOW).unwrap();
    let base = convert_to(&p, &tpl, v);
    copy_knob(&p, &copy("Test", 4, "Test", 5, "P25 Conv 2"), NOW).unwrap();
    assert_copied(&p, ("Test", 4), ("Test", 5), "P25 Conv 2");
    let out = convert_to(&p, &tpl, v);
    // The copy reuses the protocol setting and talkgroup list (no new contacts) and gets the per-channel encryption rows.
    assert_eq!(names(&diff(out.path(), base.path())), expect(&[CHANNEL, &["tabRepeaterSettingOfChan", "tabP25Signaling", "tabP25RepeaterSetting"], &TRUNKED_EXTRA[..3]]));
    assert_same_radio(out.path(), 304, 305);
}

#[test]
fn a_scan_can_list_a_copy() {
    let Some((dir, tpl, v)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    copy_knob(&p, &copy("NOAA", 2, "Test", 4, "WX1 copy"), NOW).unwrap();
    let scan = at(&p, "tabReceivingMode_NewG4", "NOAA", 1)[0].s("ChannelName").to_string();
    let mut members: Vec<String> = at(&p, "tabMultiCHScanList", "NOAA", 1).iter().map(|r| r.s("ChannelID").to_string()).collect();
    members.push(CH.into());
    let e = g5_codeplug::scan_knob::ScanKnobEdit { zone: "NOAA".into(), knob: 1, mode: 4, members, priority: None, name: None, voice_prompt: 0 };
    let ch = g5_codeplug::scan_knob::set_scan_knob(&p, &e, NOW).unwrap();
    assert_eq!(ch.knob_name, scan);
    let out = convert_to(&p, &tpl, v);
    let list = rows(out.path(), &format!("SELECT ScanList FROM tabScanListSettings WHERE ScanListSettingID = '{}'", zk(&p, "NOAA", 1)));
    assert!(list[0][0].ends_with(&format!(",{}", zk(&p, "Test", 4))), "{list:?}");
}

#[test]
fn moving_a_scan_member_renumbers_it_and_the_scan_follows() {
    let Some((dir, tpl, v)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let radio = dir.join(BACKUP_RADIO_DB);
    let p = Profile::from_backup(&dir).unwrap();
    // Lake 3 is the priority channel of the priority scan on Lake 2.
    let (old, new) = (zk(&p, "Lake", 3), zk(&p, "Test", 4));
    let scan = zk(&p, "Lake", 2);
    let prio = |db: &Path| rows(db, &format!("SELECT PriorityZoneKnobID FROM tabScanListSettings WHERE ScanListSettingID = '{scan}'"))[0][0].clone();
    assert_eq!(prio(&radio), old.to_string());
    let ch = channel(&p, "Lake", 3).unwrap();
    let recv = at(&p, "tabReceivingMode_NewG4", "Lake", 3)[0].clone();
    let lists = load_table(&p.db, "tabMultiCHScanList");

    let m = move_knob(&p, &mv("Lake", 3, "Test", 4, false), NOW).unwrap();
    assert_eq!((m.kind, m.knob_name.as_str(), m.swapped_with.as_deref()), (KnobKind::Conventional, real("Elmwood Fire"), None));
    assert_eq!(m.scans, [real("Lake 2 ElmwoodCo Fire")]);
    assert_eq!(channel(&p, "Test", 4).as_deref(), Some(ch.as_str()), "a move keeps the ChannelID");
    assert!(channel(&p, "Lake", 3).is_none());
    assert_eq!(at(&p, "tabReceivingMode_NewG4", "Test", 4)[0].s("ChannelName"), recv.s("ChannelName"));
    assert_eq!(at(&p, "tabReceivingMode_NewG4", "Lake", 3)[0].s("CurrentMode"), "0");
    let after: Vec<(String, String)> = load_table(&p.db, "tabMultiCHScanList").iter().map(|r| (r.s("ZoneID").to_string(), r.s("ChannelID").to_string())).collect();
    assert_eq!(after, lists.iter().map(|r| (r.s("ZoneID").to_string(), r.s("ChannelID").to_string())).collect::<Vec<_>>(), "member lists are untouched");
    // Receive rows keep their places, so each zone's rows stay in knob order.
    let order = |p: &Profile| load_table(&p.db, "tabReceivingMode_NewG4").iter().map(|r| (r.s("ZoneID").to_string(), r.i("KnobIndex"))).collect::<Vec<_>>();
    let base = Profile::from_backup(&dir).unwrap();
    assert_eq!(order(&p), order(&base));

    let out = convert_to(&p, &tpl, v);
    assert_eq!(prio(out.path()), new.to_string(), "the priority scan follows the channel");
    let d = diff(out.path(), &radio);
    assert!(names(&d).contains(&"tabScanListSettings".to_string()) && names(&d).contains(&"tabChannel".to_string()), "{:?}", names(&d));
    assert_renumbering(&d);
    assert_eq!(rows(out.path(), &format!("SELECT count(*) FROM tabChannelAdvInfo WHERE ID = '{old}'"))[0][0], "0");

    // Moving it back restores the radio DB exactly.
    move_knob(&p, &mv("Test", 4, "Lake", 3, false), NOW).unwrap();
    assert!(diff(convert_to(&p, &tpl, v).path(), &radio).is_empty());
    assert_eq!(order(&p), order(&base));
}

#[test]
fn moves_within_a_zone_into_a_new_zone_and_of_scan_knobs() {
    let Some((dir, tpl, v)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let radio = dir.join(BACKUP_RADIO_DB);
    for (from, to) in [(("Test", 1), ("Test", 6)), (("Test", 2), ("Test", 5)), (("NOAA", 1), ("Test", 4)), (("Lake", 2), ("Test", 7))] {
        let p = Profile::from_backup(&dir).unwrap();
        let scan = at(&p, "tabMultiChannelScan", from.0, from.1);
        let members = at(&p, "tabMultiCHScanList", from.0, from.1).iter().map(|r| r.s("ChannelID").to_string()).collect::<Vec<_>>();
        move_knob(&p, &mv(from.0, from.1, to.0, to.1, false), NOW).unwrap();
        assert_eq!(at(&p, "tabMultiChannelScan", to.0, to.1).len(), scan.len(), "a scan knob's own rows move with it");
        assert_eq!(at(&p, "tabMultiCHScanList", to.0, to.1).iter().map(|r| r.s("ChannelID").to_string()).collect::<Vec<_>>(), members);
        assert!(at(&p, "tabMultiCHScanList", from.0, from.1).is_empty());
        let out = convert_to(&p, &tpl, v);
        let d = diff(out.path(), &radio);
        assert!(!d.is_empty());
        assert_renumbering(&d);
        if !scan.is_empty() {
            let id = zk(&p, to.0, to.1);
            assert_eq!(rows(out.path(), &format!("SELECT count(*) FROM tabScanListSettings WHERE ScanListSettingID = '{id}'"))[0][0], "1");
        }
        move_knob(&p, &mv(to.0, to.1, from.0, from.1, false), NOW).unwrap();
        assert!(diff(convert_to(&p, &tpl, v).path(), &radio).is_empty(), "{from:?} and back");
    }

    // Into a new, empty zone: its tabZone row flips ModifyByDevice, nothing else gains or loses rows.
    let p = Profile::from_backup(&dir).unwrap();
    create_zone(&p, &NewZone { zone_id: "9e8d7c6b-5a49-4382-a1b0-c9d8e7f6a5b4".into(), name: "Moved".into() }, NOW).unwrap();
    let base = convert_to(&p, &tpl, v);
    move_knob(&p, &mv(real("Metro"), 4, "Moved", 1, false), NOW).unwrap();
    let out = convert_to(&p, &tpl, v);
    let d = diff(out.path(), base.path());
    assert!(names(&d).contains(&"tabZone".to_string()), "{:?}", names(&d));
    assert_renumbering(&d);
    assert_eq!(rows(out.path(), "SELECT ModifyByDevice FROM tabZone WHERE ID = '5'"), [["0"]]);
}

#[test]
fn swaps() {
    let Some((dir, tpl, v)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let radio = dir.join(BACKUP_RADIO_DB);
    for (a, b) in [(("Lake", 3), ("Lake", 4)), ((real("Metro"), 1), ("Test", 1)), (("Lake", 2), ("NOAA", 1)), (("Test", 2), ("NOAA", 8))] {
        let p = Profile::from_backup(&dir).unwrap();
        let (ca, cb) = (channel(&p, a.0, a.1), channel(&p, b.0, b.1));
        let (na, nb) = (at(&p, "tabReceivingMode_NewG4", a.0, a.1)[0].s("ChannelName").to_string(), at(&p, "tabReceivingMode_NewG4", b.0, b.1)[0].s("ChannelName").to_string());
        let m = move_knob(&p, &mv(a.0, a.1, b.0, b.1, true), NOW).unwrap();
        assert_eq!(m.swapped_with.as_deref(), Some(nb.as_str()));
        assert_eq!((channel(&p, b.0, b.1), channel(&p, a.0, a.1)), (ca, cb));
        assert_eq!(at(&p, "tabReceivingMode_NewG4", b.0, b.1)[0].s("ChannelName"), na);
        assert_eq!(at(&p, "tabReceivingMode_NewG4", a.0, a.1)[0].s("ChannelName"), nb);
        let d = diff(convert_to(&p, &tpl, v).path(), &radio);
        assert!(!d.is_empty());
        assert_renumbering(&d);
        move_knob(&p, &mv(b.0, b.1, a.0, a.1, true), NOW).unwrap();
        assert!(diff(convert_to(&p, &tpl, v).path(), &radio).is_empty(), "{a:?} <-> {b:?} twice");
    }
    // Swapping a priority scan's priority channel with another member: the radio's priority follows.
    let p = Profile::from_backup(&dir).unwrap();
    move_knob(&p, &mv("Lake", 3, "Lake", 4, true), NOW).unwrap();
    let out = convert_to(&p, &tpl, v);
    let s = rows(out.path(), &format!("SELECT PriorityZoneKnobID, ScanList FROM tabScanListSettings WHERE ScanListSettingID = '{}'", zk(&p, "Lake", 2)));
    assert_eq!(s[0][0], zk(&p, "Lake", 4).to_string());
    assert!(s[0][1].split(',').any(|x| x == zk(&p, "Lake", 3).to_string()), "{s:?}");
}

#[test]
fn refusals() {
    let Some((dir, _, _)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let err = |r: Result<(), g5_codeplug::Error>| r.unwrap_err().to_string();
    let c = |k: KnobCopy| copy_knob(&p, &k, NOW).map(|_| ());
    let m = |k: KnobMove| move_knob(&p, &k, NOW).map(|_| ());
    assert!(err(c(copy("Lake", 3, "Lake", 4, "X"))).contains("isn't an empty position"));
    assert!(err(c(copy("Lake", 3, "Lake", 3, "X"))).contains("onto itself"));
    assert!(err(c(copy("Test", 4, "Test", 5, "X"))).contains("already empty"));
    assert!(err(c(KnobCopy { channel_id: None, ..copy("Lake", 3, "Test", 4, "X") })).contains("GUID"));
    let taken = channel(&p, "Lake", 4).unwrap();
    assert!(err(c(KnobCopy { channel_id: Some(taken), ..copy("Lake", 3, "Test", 4, "X") })).contains("already exists"));
    assert!(err(c(copy("Lake", 3, "Test", 4, "A name too long"))).contains("longer than 14"));
    assert!(err(c(copy("Lake", 3, "Test", 9, "X"))).contains("1 to 8"));
    assert!(err(c(copy("Lake", 3, "Nowhere", 4, "X"))).contains("zone named"));
    assert!(err(m(mv("Lake", 3, "Lake", 4, false))).contains("swap the two knobs"));
    assert!(err(m(mv("Lake", 3, "Test", 4, true))).contains("nothing to swap"));
    assert!(err(m(mv("Lake", 3, "Lake", 3, false))).contains("different position"));
    assert!(err(m(mv("Test", 4, "Test", 5, false))).contains("already empty"));
    // Nothing was written.
    let fresh = Profile::from_backup(&dir).unwrap();
    for t in ["tabReceivingMode_NewG4", "tabSingleConvCHKnob", "tabP25TrunkingSystemKnob", "tabMultiCHScanList"] {
        assert_eq!(format!("{:?}", load_table(&p.db, t)), format!("{:?}", load_table(&fresh.db, t)), "{t}");
    }
}
