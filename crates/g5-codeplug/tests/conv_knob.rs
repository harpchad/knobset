//! Analog conventional knobs against private real-pager fixtures (skipped
//! without them). Rows and allowed radio changes per notes/conventional-knobs.md 8.

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::conv_knob::{BW_12_5, BW_25, ConvKnobEdit, DcsCode, MODE_MONITOR, Squelch, add_conv_knob};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES;
use g5_codeplug::row::load_table;
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::summary::Summary;
use g5_codeplug::{Profile, Templates};

const NOW: &str = "2026-10-18 12:00:00";
const CH: &str = "6a5b4c3d-2e1f-4a0b-9c8d-7e6f5a4b3c2d";
const FQ: &str = "7b6c5d4e-3f2a-4b1c-8d9e-0f1a2b3c4d5e";

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

fn assert_only(changed: &[String]) {
    for t in changed {
        assert!(CONV_KNOB_CHANNEL_TABLES.contains(&t.as_str()), "unexpected change in {t} (changed: {changed:?})");
    }
}

/// (zone name, knob index) of an empty position on the 8-position grid.
fn empty_knob(p: &Profile) -> (String, i64) {
    let zones = load_table(&p.db, "tabZone_New");
    let r = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == 0 && (1..=8).contains(&r.i("KnobIndex"))).expect("an empty knob");
    (zones.iter().find(|z| z.s("ID") == r.s("ZoneID")).unwrap().s("Name").to_string(), r.i("KnobIndex"))
}

/// An analog frequency inside the pager's bands: NOAA weather 162.425 MHz (VHF).
const WX: i64 = 162_425_000;

fn edit(zone: &str, knob: i64, squelch: Squelch) -> ConvKnobEdit {
    ConvKnobEdit { zone: zone.into(), knob, name: "Knobset Conv".into(), rx_hz: WX, bandwidth: BW_12_5, squelch, voice_prompt: 0, channel_id: CH.into(), freq_id: FQ.into() }
}

#[test]
fn carrier_knob_matches_pps_defaults() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    // A real knob nobody edited (Tone alerts, ID Match recording).
    let real = load_table(&p.db, "tabSingleConvCHKnob").into_iter().find(|r| r.s("AlertMode") == "1" && r.s("RecordFlag") == "1").expect("an unedited conventional knob");
    let (zone, knob) = empty_knob(&p);
    let ch = add_conv_knob(&p, &tpl, &edit(&zone, knob, Squelch::Carrier), NOW).unwrap();
    assert_eq!(ch.squelch, "carrier");

    let row = load_table(&p.db, "tabSingleConvCHKnob").into_iter().find(|r| r.s("ChannelID") == CH).unwrap();
    for c in p.db.table_columns("tabSingleConvCHKnob").unwrap() {
        if !["ZoneID", "KnobIndex", "ChannelID", "ConvFreqID"].contains(&c.as_str()) {
            assert_eq!(row.s(&c), real.s(&c), "{c}");
        }
    }
    assert_eq!(row.s("ConvFreqID"), FQ);
    let f = load_table(&p.db, "tabConvFreqList").into_iter().find(|f| f.s("FreqID") == FQ).unwrap();
    assert_eq!(
        (f.s("Name"), f.s("TransmitMode"), f.s("RxFreq"), f.s("TxFreq"), f.s("RxBandWidth"), f.s("TxBandWidth")),
        ("Knobset Conv", "1", "162425000", "162425000", "1", "1")
    );
    assert!(load_table(&p.db, "tabRecvSaList").iter().all(|r| r.s("ChannelID") != CH), "carrier squelch writes no tone row");
    let r = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.s("ChannelName") == "Knobset Conv").unwrap();
    assert_eq!((r.i("CurrentMode"), r.s("AlertToneID")), (MODE_MONITOR, "0"));
    let k = Summary::read(&p).unwrap().zones.into_iter().find(|z| z.name == zone).unwrap().knobs.into_iter().find(|k| k.index == knob).unwrap();
    assert_eq!((k.mode, k.target.as_deref()), (MODE_MONITOR, Some("162.42500 MHz")));

    let c = changed(&p, &tpl, v, &dir);
    assert_only(&c);
    for t in ["tabChannel", "tabReceivingMode", "tabChannelAdvInfo"] {
        assert!(c.iter().any(|x| x == t), "the new knob must reach {t} (changed: {c:?})");
    }
    assert!(!c.iter().any(|x| x == "tabRecvSaList"), "no tone, no tabRecvSaList change");
}

#[test]
fn ctcss_matches_pps_rows_and_reaches_the_radio() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let real = load_table(&p.db, "tabRecvSaList").into_iter().find(|r| r.s("SaType") == "1").expect("a real CTCSS row");
    let (zone, knob) = empty_knob(&p);
    add_conv_knob(&p, &tpl, &edit(&zone, knob, Squelch::Ctcss { tenths: real.i("ctFreq") }), NOW).unwrap();
    let ours = load_table(&p.db, "tabRecvSaList").into_iter().find(|r| r.s("ChannelID") == CH).unwrap();
    for c in ["SigType", "SaType", "ctFreq", "ctSte", "cdCodeWord", "cdSte", "cdModSense"] {
        assert_eq!(ours.s(c), real.s(c), "{c}");
    }
    let c = changed(&p, &tpl, v, &dir);
    assert_only(&c);
    assert!(c.iter().any(|x| x == "tabRecvSaList"), "the tone must reach the radio (changed: {c:?})");
}

#[test]
fn dcs_is_stored_as_the_octal_value() {
    let Some((dir, tpl, v)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, knob) = empty_knob(&p);
    let sq = Squelch::Dcs { code: DcsCode::try_from("065".to_string()).unwrap(), inverted: true };
    let mut e = edit(&zone, knob, sq);
    e.bandwidth = BW_25;
    let ch = add_conv_knob(&p, &tpl, &e, NOW).unwrap();
    assert_eq!(ch.squelch, "DCS 065 inverted");
    let r = load_table(&p.db, "tabRecvSaList").into_iter().find(|r| r.s("ChannelID") == CH).unwrap();
    assert_eq!((r.s("SaType"), r.s("ctFreq"), r.s("cdCodeWord"), r.s("cdModSense")), ("2", "0", "53", "1"));
    assert_only(&changed(&p, &tpl, v, &dir));
}

#[test]
fn refusals() {
    let Some((dir, tpl, _)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let (zone, knob) = empty_knob(&p);
    let ok = edit(&zone, knob, Squelch::Carrier);
    let err = |e: ConvKnobEdit| add_conv_knob(&p, &tpl, &e, NOW).unwrap_err().to_string();

    let busy = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") != 0).unwrap();
    let busy_zone = load_table(&p.db, "tabZone_New").into_iter().find(|z| z.s("ID") == busy.s("ZoneID")).unwrap().s("Name").to_string();
    assert!(err(ConvKnobEdit { zone: busy_zone, knob: busy.i("KnobIndex"), ..ok.clone() }).contains("isn't an empty position"));
    assert!(err(ConvKnobEdit { rx_hz: 450_000_000, ..ok.clone() }).contains("outside this pager's bands"));
    assert!(err(ConvKnobEdit { rx_hz: 162_425_005, ..ok.clone() }).contains("10 Hz"));
    assert!(err(ConvKnobEdit { bandwidth: 3, ..ok.clone() }).contains("bandwidth"));
    assert!(err(ConvKnobEdit { squelch: Squelch::Ctcss { tenths: 600 }, ..ok.clone() }).contains("67.0"));
    assert!(err(ConvKnobEdit { name: "Fifteen chars!!".into(), ..ok.clone() }).contains("longer than 14"));
    assert!(err(ConvKnobEdit { voice_prompt: 6, ..ok.clone() }).contains("voice prompt"));
    assert!(err(ConvKnobEdit { channel_id: "NOT-A-GUID".into(), ..ok.clone() }).contains("GUID"));
    assert!(DcsCode::try_from("024".to_string()).is_err(), "024 isn't a DCS code");
    assert!(DcsCode::try_from("23".to_string()).is_ok(), "leading zero is optional");
    let existing = load_table(&p.db, "tabConvFreqList").into_iter().find(|f| f.s("FreqID").contains('-')).unwrap();
    assert!(err(ConvKnobEdit { freq_id: existing.s("FreqID").into(), ..ok.clone() }).contains("already exists"));
    // Nothing above changed the profile.
    assert!(load_table(&p.db, "tabSingleConvCHKnob").iter().all(|r| r.s("ChannelID") != CH));

    // A knob named like an existing frequency gets a numbered frequency name.
    let taken = existing.s("Name").to_string();
    add_conv_knob(&p, &tpl, &ConvKnobEdit { name: taken, ..ok }, NOW).unwrap();
    let f = load_table(&p.db, "tabConvFreqList").into_iter().find(|f| f.s("FreqID") == FQ).unwrap();
    assert!(f.s("Name").starts_with("Frequency "), "{}", f.s("Name"));
}
