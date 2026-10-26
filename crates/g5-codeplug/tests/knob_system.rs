//! Assigning systems to knobs against private real-pager fixtures (skipped without them).
//! Allowed radio-DB changes per notes/system-creation.md 7.4.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::edit::{KNOB_CHANNEL_TABLES, SYSTEM_EDIT_TABLES, TALKGROUP_EDIT_TABLES};
use g5_codeplug::knob_system::{KnobSystemEdit, MODE_TG_MONITOR, MODE_TG_SCAN, assign_knob_system};
use g5_codeplug::row::load_table;
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::summary::Summary;
use g5_codeplug::system_edit::{ControlChannelEdit, NewSystem, SiteEdit, SystemEdit, TalkgroupEdit, create_system, profile_bands};
use g5_codeplug::{Profile, Templates};

const NOW: &str = "2026-10-18 12:00:00";
const CH: &str = "5d4c3b2a-1908-4e7f-8a6b-5c4d3e2f1a0b";

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
        .find(|p| version(p).as_deref().and_then(Variant::for_db_version).is_some())?;
    let variant = Variant::for_db_version(&version(&dir)?)?;
    Some((dir, templates, variant))
}

fn changed(profile: &Profile, templates: &Templates, variant: Variant, dir: &Path) -> Vec<String> {
    let out = tempfile::NamedTempFile::new().unwrap();
    let report = convert(profile, templates, variant, out.path()).unwrap();
    assert!(report.unverified.is_empty(), "unverified: {:?}", report.unverified);
    let ours = sqlite2::Connection::open(out.path()).unwrap();
    let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
    compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect()
}

fn assert_only(changed: &[String], allowed: &[&[&str]]) {
    for t in changed {
        assert!(allowed.iter().any(|a| a.contains(&t.as_str())), "unexpected change in {t} (changed: {changed:?})");
    }
}

/// (zone name, knob index) of an empty knob position.
fn empty_knob(p: &Profile) -> (String, i64) {
    let zones = load_table(&p.db, "tabZone_New");
    let r = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == 0).expect("an empty knob");
    (zones.iter().find(|z| z.s("ID") == r.s("ZoneID")).unwrap().s("Name").to_string(), r.i("KnobIndex"))
}

fn knob_of(p: &Profile, zone: &str, knob: i64) -> Option<g5_codeplug::summary::Knob> {
    Summary::read(p).unwrap().zones.into_iter().find(|z| z.name == zone)?.knobs.into_iter().find(|k| k.index == knob)
}

fn new_knob(zone: &str, knob: i64, system: &str, mode: i64, tgs: Vec<String>) -> KnobSystemEdit {
    KnobSystemEdit { zone: zone.into(), knob, system_id: system.into(), mode, channel_id: Some(CH.into()), name: Some("Knobset Knob".into()), voice_prompt: 0, priority: vec![], normal: tgs }
}

#[test]
fn empty_knob_becomes_tg_scan_with_pps_defaults() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let sys = s.systems.iter().find(|s| s.in_use).unwrap();
    let tgs: Vec<String> = sys.talkgroup_list.iter().skip(1).take(3).map(|t| t.group_id.clone()).collect();
    let (zone, knob) = empty_knob(&p);
    let real = load_table(&p.db, "tabP25TrunkingSystemKnob").remove(0);

    let mut e = new_knob(&zone, knob, &sys.id, MODE_TG_SCAN, tgs[1..].to_vec());
    e.priority = vec![tgs[0].clone()];
    let ch = assign_knob_system(&p, &e, NOW).unwrap();
    assert!(ch.new_knob);
    assert_eq!(ch.talkgroups, 3);

    // Every non-identity column matches what PPS saved for the real knobs.
    let row = load_table(&p.db, "tabP25TrunkingSystemKnob").into_iter().find(|r| r.s("ChannelID") == CH).unwrap();
    for c in p.db.table_columns("tabP25TrunkingSystemKnob").unwrap() {
        if !["ZoneID", "KnobIndex", "ChannelID", "P25TrunkingID", "ProtocolSettingID"].contains(&c.as_str()) {
            assert_eq!(row.s(&c), real.s(&c), "{c}");
        }
    }
    assert_eq!(row.s("ProtocolSettingID"), real.s("ProtocolSettingID"));
    let k = knob_of(&p, &zone, knob).expect("knob is active");
    assert_eq!((k.mode, k.name.as_str(), k.system_id.as_deref()), (MODE_TG_SCAN, "Knobset Knob", Some(sys.id.as_str())));
    assert_eq!(k.talkgroup_list.iter().filter(|t| t.priority).count(), 1);
    assert!(load_table(&p.db, "tabP25TSubGroupEnable").iter().all(|r| r.s("ChannelID") != CH), "TG scan writes no sub-group row");

    let c = changed(&p, &tpl, v, &dir);
    assert_only(&c, &[KNOB_CHANNEL_TABLES, TALKGROUP_EDIT_TABLES, SYSTEM_EDIT_TABLES]);
    for t in ["tabChannel", "tabReceivingMode", "tabRecvGroupList"] {
        assert!(c.iter().any(|x| x == t), "the new knob must reach {t} (changed: {c:?})");
    }
}

#[test]
fn new_system_on_an_empty_knob_as_tg_monitor() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let band = profile_bands(&p, &tpl).unwrap().remove(0);
    let sid = "8f3e4d5c-6b7a-4091-9e8f-7d6c5b4a3e2f";
    let tg = "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d";
    create_system(
        &p,
        &tpl,
        &NewSystem {
            band: band.code,
            system: SystemEdit {
                system_id: sid.into(),
                name: "Monitor Sys".into(),
                wacn: 0xBEE00,
                sysid: 0x123,
                sites: vec![SiteEdit { id: None, name: "Hill".into(), rfss: 1, site: 2 }],
                control_channels: vec![ControlChannelEdit { id: None, rx_hz: band.ranges[0].0 + 12_500, note: String::new() }],
                talkgroups: vec![TalkgroupEdit { group_id: tg.into(), tgid: 4242, name: "Ops".into(), note: String::new(), ..Default::default() }],
            },
        },
        NOW,
    )
    .unwrap();
    let (zone, knob) = empty_knob(&p);
    let mut e = new_knob(&zone, knob, sid, MODE_TG_MONITOR, vec![tg.into()]);
    e.voice_prompt = 11;
    assign_knob_system(&p, &e, NOW).unwrap();
    let sub = load_table(&p.db, "tabP25TSubGroupEnable").into_iter().find(|r| r.s("ChannelID") == CH).expect("monitor writes a sub-group row");
    assert_eq!((sub.s("GroupID"), sub.s("Enable2TSubGroup")), ("", "1"));
    let r = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == MODE_TG_MONITOR && r.s("ChannelName") == "Knobset Knob").unwrap();
    assert_eq!(r.s("AlertToneID"), "11");
    assert_only(&changed(&p, &tpl, v, &dir), &[KNOB_CHANNEL_TABLES, TALKGROUP_EDIT_TABLES, SYSTEM_EDIT_TABLES]);
}

#[test]
fn trunked_knob_moves_to_another_system() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let (zone, k) = s.zones.iter().flat_map(|z| z.knobs.iter().map(move |k| (z.name.clone(), k.clone()))).find(|(_, k)| k.mode == MODE_TG_SCAN).unwrap();
    let old = k.system_id.clone().unwrap();
    let other = s.systems.iter().find(|x| x.id != old && x.talkgroup_list.len() > 2).unwrap();
    let zid = load_table(&p.db, "tabZone_New").into_iter().find(|z| z.s("Name") == zone).unwrap().s("ID").to_string();
    let before = load_table(&p.db, "tabP25TrunkingSystemKnob").into_iter().find(|r| r.s("ZoneID") == zid && r.i("KnobIndex") == k.index).unwrap();
    assert_eq!(before.s("P25TrunkingID"), old);
    let e = KnobSystemEdit {
        zone: zone.clone(),
        knob: k.index,
        system_id: other.id.clone(),
        mode: MODE_TG_SCAN,
        channel_id: None,
        name: None,
        voice_prompt: 0,
        priority: vec![],
        normal: other.talkgroup_list.iter().skip(1).take(2).map(|t| t.group_id.clone()).collect(),
    };
    let ch = assign_knob_system(&p, &e, NOW).unwrap();
    assert!(!ch.new_knob);
    assert_eq!(ch.new_system, other.name);
    let after = knob_of(&p, &zone, k.index).unwrap();
    assert_eq!((after.name.as_str(), after.system_id.as_deref(), after.talkgroups), (k.name.as_str(), Some(other.id.as_str()), 2));
    let row = load_table(&p.db, "tabP25TrunkingSystemKnob").into_iter().find(|r| r.s("ChannelID") == before.s("ChannelID")).unwrap();
    for c in p.db.table_columns("tabP25TrunkingSystemKnob").unwrap() {
        if c != "P25TrunkingID" {
            assert_eq!(row.s(&c), before.s(&c), "{c} must be kept");
        }
    }
    let c = changed(&p, &tpl, v, &dir);
    assert_only(&c, &[SYSTEM_EDIT_TABLES, TALKGROUP_EDIT_TABLES]);
    assert!(c.iter().any(|x| x == "tabRecvGroupList"), "the new list must reach the radio (changed: {c:?})");
}

#[test]
fn refuses_unsupported_or_invalid_assignments() {
    let Some((dir, _, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let sys = s.systems.iter().find(|s| s.in_use).unwrap();
    let other = s.systems.iter().find(|x| x.id != sys.id && x.talkgroup_list.len() > 1).unwrap();
    let tg = sys.talkgroup_list[1].group_id.clone();
    let (zone, knob) = empty_knob(&p);
    let base = new_knob(&zone, knob, &sys.id, MODE_TG_SCAN, vec![tg.clone()]);
    let try_edit = |f: &dyn Fn(&mut KnobSystemEdit)| {
        let mut e = base.clone();
        f(&mut e);
        assign_knob_system(&p, &e, NOW)
    };
    assert!(try_edit(&|e| e.normal.clear()).is_err(), "no talkgroups");
    assert!(try_edit(&|e| e.normal = vec![other.talkgroup_list[1].group_id.clone()]).is_err(), "talkgroup from another system");
    assert!(try_edit(&|e| e.name = None).is_err(), "new knob without a name");
    assert!(try_edit(&|e| e.channel_id = Some("NOT-A-GUID".into())).is_err(), "bad channel id");
    assert!(try_edit(&|e| e.voice_prompt = 11).is_err(), "monitor prompt on a scan knob");
    assert!(try_edit(&|e| e.mode = 33).is_err(), "not a trunked mode");
    let (cz, ck) = s.zones.iter().flat_map(|z| z.knobs.iter().map(move |k| (z.name.clone(), k.index, k.mode))).find(|(_, _, m)| *m == 33).map(|(z, k, _)| (z, k)).unwrap();
    assert!(try_edit(&|e| {
        e.zone = cz.clone();
        e.knob = ck;
    })
    .is_err(), "conventional knob");
    // Switching TG Scan and TG Monitor is supported now; see tests/knob_remove.rs.
    assert!(knob_of(&p, &zone, knob).is_none(), "refused edits leave the knob empty");
}
