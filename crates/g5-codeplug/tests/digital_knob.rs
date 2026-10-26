//! DMR and P25 conventional Monitor knobs against private real-pager fixtures
//! (skipped without them): the converted radio rows must be the ones
//! notes/digital-conventional.md 9.4 and 9.5 derive from PPS.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::digital_knob::{DigitalKnobEdit, DigitalProtocol, DigitalTalkgroup, add_digital_knob};
use g5_codeplug::edit::{CONV_KNOB_CHANNEL_TABLES, DMR_KNOB_TABLES, P25C_KNOB_TABLES, TALKGROUP_EDIT_TABLES};
use g5_codeplug::row::load_table;
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::{Profile, Templates};

const NOW: &str = "2026-10-18 12:00:00";
const CH: &str = "1f2e3d4c-5b6a-4978-8695-a4b3c2d1e0f9";
const FQ: &str = "2e3d4c5b-6a79-4887-96a5-b4c3d2e1f0a9";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn env_or(var: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).unwrap_or(default)
}

fn fixture() -> Option<(PathBuf, Templates, Variant)> {
    let templates = Templates::new(env_or("G5_TEMPLATES", repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"))).ok()?;
    let version = |p: &Path| {
        fs::read_to_string(p.join("MANIFEST.txt")).ok()?.lines().find_map(|l| l.strip_prefix("Radio DB version:").map(|v| v.trim().to_string()))
    };
    let dir = fs::read_dir(env_or("G5_BACKUPS", repo().join("backups")))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        // DMR needs a pager whose profile enables it (unit B).
        .find(|p| version(p).as_deref().and_then(Variant::for_db_version).is_some() && Profile::from_backup(p).is_ok_and(|pr| load_table(&pr.db, "tabProtocolMode").first().is_some_and(|m| m.s("DMRC") == "1")))?;
    let variant = Variant::for_db_version(&version(&dir)?)?;
    Some((dir, templates, variant))
}

/// Convert, returning the changed tables and the new radio DB.
fn convert_to(profile: &Profile, templates: &Templates, variant: Variant, dir: &Path) -> (Vec<String>, tempfile::NamedTempFile, Vec<String>) {
    let out = tempfile::NamedTempFile::new().unwrap();
    let report = convert(profile, templates, variant, out.path()).unwrap();
    assert!(report.unverified.is_empty(), "unverified: {:?}", report.unverified);
    let ours = sqlite2::Connection::open(out.path()).unwrap();
    let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
    let c = compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect();
    (c, out, report.experimental)
}

fn rows(db: &Path, sql: &str) -> Vec<Vec<String>> {
    let c = sqlite2::Connection::open(db).unwrap();
    c.query(sql, &[]).unwrap().rows.into_iter().map(|r| r.into_iter().map(|v| v.unwrap_or_default()).collect()).collect()
}

/// An empty position and the channel ID (zone position x 100 + knob) it gets on the radio.
fn empty_position(p: &Profile) -> (String, i64, i64) {
    let zones = load_table(&p.db, "tabZone_New");
    let r = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == 0 && (1..=8).contains(&r.i("KnobIndex"))).unwrap();
    let zi = zones.iter().position(|z| z.s("ID") == r.s("ZoneID")).unwrap() as i64 + 1;
    (zones[zi as usize - 1].s("Name").to_string(), r.i("KnobIndex"), zi * 100 + r.i("KnobIndex"))
}

fn edit(zone: &str, knob: i64, protocol: DigitalProtocol, talkgroups: Vec<DigitalTalkgroup>) -> DigitalKnobEdit {
    DigitalKnobEdit { zone: zone.into(), knob, name: "Digi Test".into(), rx_hz: 155_310_000, protocol, talkgroups, repeater_mode: false, voice_prompt: 0, channel_id: CH.into(), freq_id: FQ.into() }
}

#[test]
fn dmr_monitor_knob_converts_to_the_expected_rows() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, knob, zk) = empty_position(&p);
    let ch = add_digital_knob(&p, &tpl, &edit(&zone, knob, DigitalProtocol::Dmr { slot: 2, color_code: 7 }, vec![]), NOW).unwrap();
    assert!(ch.dmr);
    assert_eq!((ch.talkgroups, ch.new_groups), (1, 1), "the All placeholder");
    let k = load_table(&p.db, "tabSingleConvCHKnob").into_iter().find(|r| r.s("ChannelID") == CH).unwrap();
    assert_eq!((k.s("RxColorCode"), k.s("RxNAC"), k.s("SquelchOption"), k.s("TransmitMode")), ("7", "0", "3", "1"));
    let setting = load_table(&p.db, "tabProtocolSetting").into_iter().find(|s| s.s("ID") == k.s("ProtocolSettingID")).unwrap();
    assert_eq!((setting.s("ProtocolType"), setting.s("ProtocolName")), ("4", "DMR TS2"));

    let (c, out, exp) = convert_to(&p, &tpl, v, &dir);
    assert_eq!(exp, ["DMR conventional channel"]);
    let allowed: Vec<&str> = CONV_KNOB_CHANNEL_TABLES.iter().chain(DMR_KNOB_TABLES).chain(TALKGROUP_EDIT_TABLES).copied().collect();
    assert!(c.iter().all(|t| allowed.contains(&t.as_str())), "{c:?}");
    assert!(!c.iter().any(|t| t == "tabRecvSaList"), "{c:?}");
    let db = out.path();
    assert_eq!(rows(db, &format!("SELECT * FROM tabDMRSignaling WHERE SigID = {zk}")), [[zk.to_string(), "4".into(), "7".into(), "0".into(), "600".into(), "5000".into(), "0".into(), "4".into(), "5".into(), "1".into(), "1".into()]]);
    assert_eq!(rows(db, &format!("SELECT SACID,TimeSlot,TimingAlign,RxColorCode,TxColorCode FROM tabDMRRepeaterSetting WHERE SACID = '{zk}'")), [[zk.to_string(), "1".into(), "0".into(), "7".into(), "".into()]]);
    assert_eq!(rows(db, &format!("SELECT ColorCode,NAC,DefaultCallID,SquelchOption,PowerSavingRxOffDur FROM tabChannelAdvInfo WHERE ID = '{zk}'")), [["7", "0", "0", "0", "0"]]);
    assert_eq!(rows(db, &format!("SELECT SignalingType, AccessCodeType FROM tabRepeaterSettingOfChan WHERE ChannelID = '{zk}'")), [["4", "5"]]);
    let cof = rows(db, &format!("SELECT ContactID,SignalingType FROM tabContactOfChan WHERE ChannelID = {zk}"));
    assert_eq!(cof.len(), 1, "one member, no all-call row: {cof:?}");
    assert_eq!(cof[0][1], "4");
    let rg = rows(db, &format!("SELECT SigType,SigID,GroupAddr,GroupId,SeqNo FROM tabRecvGroupList WHERE ZoneKnobID = '{zk}'"));
    assert_eq!(rg, [["4".to_string(), zk.to_string(), "2164260863".into(), cof[0][0].clone(), "0".into()]]);
}

#[test]
fn p25c_monitor_knob_converts_to_the_expected_rows() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, knob, zk) = empty_position(&p);
    let tgs = vec![DigitalTalkgroup { id: 1201, name: "Fire Ops".into() }, DigitalTalkgroup { id: 1202, name: "EMS".into() }];
    let ch = add_digital_knob(&p, &tpl, &edit(&zone, knob, DigitalProtocol::P25c { nac: 0x293, filter_nac: true }, tgs), NOW).unwrap();
    assert!(!ch.dmr);
    assert_eq!(ch.new_groups, 2);
    let (c, out, exp) = convert_to(&p, &tpl, v, &dir);
    assert_eq!(exp, ["P25 conventional channel"]);
    let allowed: Vec<&str> = CONV_KNOB_CHANNEL_TABLES.iter().chain(P25C_KNOB_TABLES).chain(TALKGROUP_EDIT_TABLES).copied().collect();
    assert!(c.iter().all(|t| allowed.contains(&t.as_str())), "{c:?}");
    let db = out.path();
    let nac = 0x293.to_string();
    assert_eq!(
        rows(db, &format!("SELECT * FROM tabP25Signaling WHERE SigID = '{zk}'")),
        [[zk.to_string(), "20".into(), "600".into(), "5000".into(), nac.clone(), "0".into(), "133".into(), "200".into(), "2500".into(), "1000".into(), "0".into(), "1000".into(), "2000".into(), "5000".into(), "1500".into()]]
    );
    assert_eq!(rows(db, &format!("SELECT RxNAC,TxNAC FROM tabP25RepeaterSetting WHERE SACID = '{zk}'")), [[nac.clone(), "".into()]]);
    assert_eq!(rows(db, &format!("SELECT ColorCode,NAC,SquelchOption FROM tabChannelAdvInfo WHERE ID = '{zk}'")), [["0".to_string(), nac, "1".into()]]);
    let rg = rows(db, &format!("SELECT SigType,GroupAddr,SeqNo FROM tabRecvGroupList WHERE ZoneKnobID = '{zk}' ORDER BY SeqNo"));
    assert_eq!(rg, [["8", "2147484849", "1"], ["8", "2147484850", "2"]], "members in list order, numbered");
}

#[test]
fn settings_and_groups_are_reused_and_inputs_checked() {
    let Some((dir, tpl, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, knob, _) = empty_position(&p);
    let err = |e: DigitalKnobEdit| add_digital_knob(&p, &tpl, &e, NOW).unwrap_err().to_string();
    let base = edit(&zone, knob, DigitalProtocol::Dmr { slot: 1, color_code: 1 }, vec![]);
    assert!(err(DigitalKnobEdit { protocol: DigitalProtocol::Dmr { slot: 3, color_code: 1 }, ..base.clone() }).contains("timeslot"));
    assert!(err(DigitalKnobEdit { protocol: DigitalProtocol::Dmr { slot: 1, color_code: 16 }, ..base.clone() }).contains("color code"));
    assert!(err(DigitalKnobEdit { protocol: DigitalProtocol::P25c { nac: 0xF7F, filter_nac: false }, ..base.clone() }).contains("F7F"));
    assert!(err(DigitalKnobEdit { talkgroups: vec![DigitalTalkgroup { id: 16_776_500, name: "X".into() }], ..base.clone() }).contains("out of range"));
    let dup = vec![DigitalTalkgroup { id: 5, name: "A".into() }, DigitalTalkgroup { id: 5, name: "B".into() }];
    assert!(err(DigitalKnobEdit { talkgroups: dup, ..base.clone() }).contains("twice"));
    assert!(err(DigitalKnobEdit { rx_hz: 450_000_000, ..base.clone() }).contains("outside"));

    // A second knob on the same slot reuses the protocol setting, the All group and the grouping.
    add_digital_knob(&p, &tpl, &base, NOW).unwrap();
    let (s1, g1, gg1) = (load_table(&p.db, "tabProtocolSetting").len(), load_table(&p.db, "tabGroup_New").len(), load_table(&p.db, "tabGroupGrouping").len());
    let zones = load_table(&p.db, "tabZone_New");
    let r = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == 0 && (1..=8).contains(&r.i("KnobIndex"))).unwrap();
    let zone2 = zones.iter().find(|z| z.s("ID") == r.s("ZoneID")).unwrap().s("Name").to_string();
    let second = DigitalKnobEdit { zone: zone2, knob: r.i("KnobIndex"), channel_id: "3d4c5b6a-7988-4796-a5b4-c3d2e1f0a9b8".into(), freq_id: "4c5b6a79-8897-4a6b-b4c3-d2e1f0a9b8c7".into(), ..base };
    let ch = add_digital_knob(&p, &tpl, &second, NOW).unwrap();
    assert_eq!(ch.new_groups, 0);
    assert_eq!((load_table(&p.db, "tabProtocolSetting").len(), load_table(&p.db, "tabGroup_New").len(), load_table(&p.db, "tabGroupGrouping").len()), (s1, g1, gg1));
}
