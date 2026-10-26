//! Pager settings (notes/pager-settings.md) against private real-pager
//! fixtures (skipped without them): every key, on the DBV1.41b backup and the
//! newest DBV1.41c backup, changes its profile column, exactly its radio
//! tables, and the converted radio value.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::convert141::{Variant, convert};
use g5_codeplug::row::load_table;
use g5_codeplug::settings::{apply, radio_tables, read};
use g5_codeplug::source::BACKUP_RADIO_DB;
use g5_codeplug::{Profile, Templates};

const NOW: &str = "2026-10-20 12:00:00";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn env_or(var: &str, default: PathBuf) -> PathBuf {
    std::env::var_os(var).map(PathBuf::from).unwrap_or(default)
}

fn db_version(p: &Path) -> Option<String> {
    fs::read_to_string(p.join("MANIFEST.txt")).ok()?.lines().find_map(|l| l.strip_prefix("Radio DB version:").map(|v| v.trim().to_string()))
}

/// The newest backup (by folder name) of each supported radio DB version.
fn fixtures() -> Vec<(PathBuf, Templates, Variant)> {
    let Ok(templates) = Templates::new(env_or("G5_TEMPLATES", repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"))) else { return Vec::new() };
    let Ok(rd) = fs::read_dir(env_or("G5_BACKUPS", repo().join("backups"))) else { return Vec::new() };
    let mut dirs: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    dirs.sort();
    let mut out = Vec::new();
    for v in [Variant::B, Variant::C] {
        match dirs.iter().rev().find(|d| db_version(d).as_deref() == Some(v.db_version())) {
            Some(d) => out.push((d.clone(), templates.clone(), v)),
            None => eprintln!("skipping {}: no backup", v.db_version()),
        }
    }
    out
}

/// Radio tables that differ from the backup's radio DB, and the radio DB itself.
fn convert_and_diff(p: &Profile, t: &Templates, v: Variant, dir: &Path) -> (Vec<String>, tempfile::NamedTempFile) {
    let out = tempfile::NamedTempFile::new().unwrap();
    let report = convert(p, t, v, out.path()).unwrap();
    assert!(report.unverified.is_empty(), "unverified: {:?}", report.unverified);
    let ours = sqlite2::Connection::open(out.path()).unwrap();
    let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
    let changed = compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).map(|d| d.table).collect();
    (changed, out)
}

fn radio_value(db: &Path, table: &str, column: &str, row: Option<(&str, &str)>) -> String {
    let c = sqlite2::Connection::open(db).unwrap();
    let rows = load_table(&c, table);
    let r = match row {
        Some((k, v)) => rows.iter().find(|r| r.s(k) == v).unwrap_or_else(|| panic!("{table}: no row {k} = {v}")),
        None => rows.first().unwrap_or_else(|| panic!("{table} is empty")),
    };
    r.s(column).to_string()
}

/// `(table, column, row filter, value)` the radio must hold.
type RadioValue = (&'static str, &'static str, Option<(&'static str, &'static str)>, &'static str);

struct Case {
    /// Only DBV1.41c and later carry it.
    only_c: bool,
    /// Every key set in one apply (later ones depend on earlier ones).
    set: &'static [(&'static str, i64)],
    /// Radio tables expected to differ, exactly.
    tables: &'static [&'static str],
    radio: &'static [RadioValue],
}

const CASES: &[Case] = &[
    Case { only_c: false, set: &[("hold.enabled", 1)], tables: &["tabFunctionsOnOff"], radio: &[("tabFunctionsOnOff", "TGHoldFunc", None, "1")] },
    Case { only_c: false, set: &[("hold.timeout", 15)], tables: &["tabFunctionsParam"], radio: &[("tabFunctionsParam", "TGHoldTimeout", None, "15")] },
    Case { only_c: false, set: &[("hold.unhold_alert", 1)], tables: &["tabFunctionsOnOff"], radio: &[("tabFunctionsOnOff", "UnholdAlert", None, "1")] },
    Case { only_c: false, set: &[("hold.remove", 1)], tables: &["tabFunctionsOnOff"], radio: &[("tabFunctionsOnOff", "TGRemvFunc", None, "1")] },
    Case { only_c: false, set: &[("display.system_id", 1)], tables: &["tabUiNumberFormat"], radio: &[("tabUiNumberFormat", "SysIdShowType", None, "1")] },
    Case { only_c: false, set: &[("display.call_id", 1)], tables: &["tabUiNumberFormat"], radio: &[("tabUiNumberFormat", "CallIdShowType", None, "1")] },
    Case { only_c: false, set: &[("display.date", 2)], tables: &["tabDateTimeSetting"], radio: &[("tabDateTimeSetting", "date_format", None, "2")] },
    Case { only_c: false, set: &[("display.hour", 1)], tables: &["tabDateTimeSetting"], radio: &[("tabDateTimeSetting", "hour_format", None, "1")] },
    Case { only_c: true, set: &[("display.rssi", 1)], tables: &["tabFunctionsOnOff"], radio: &[("tabFunctionsOnOff", "SignalLevel", None, "1")] },
    Case { only_c: false, set: &[("alerts.receive_type", 1)], tables: &["tabFuncClassToneVolume"], radio: &[("tabFuncClassToneVolume", "type", RECEIVE, "1")] },
    Case { only_c: false, set: &[("alerts.receive_volume", 2)], tables: &["tabFuncClassToneVolume"], radio: &[("tabFuncClassToneVolume", "volume", RECEIVE, "33")] },
    Case { only_c: false, set: &[("alerts.receive_volume", 0)], tables: &["tabFuncClassToneVolume"], radio: &[("tabFuncClassToneVolume", "volume", RECEIVE, "48")] },
    Case { only_c: false, set: &[("alerts.notify_type", 2)], tables: &["tabFuncClassToneVolume"], radio: &[("tabFuncClassToneVolume", "type", NOTIFY, "2")] },
    Case { only_c: false, set: &[("alerts.notify_volume", 5)], tables: &["tabFuncClassToneVolume"], radio: &[("tabFuncClassToneVolume", "volume", NOTIFY, "0")] },
    Case { only_c: false, set: &[("alerts.oor_alert", 0)], tables: &["tabOutRangeSetting"], radio: &[("tabOutRangeSetting", "InOutRangeAlert", None, "0")] },
    Case { only_c: false, set: &[("alerts.oor_timer", 60)], tables: &["tabOutRangeSetting"], radio: &[("tabOutRangeSetting", "outRangeCheckTimer", None, "60")] },
    Case { only_c: false, set: &[("alerts.in_range_timer", 10)], tables: &["tabOutRangeSetting"], radio: &[("tabOutRangeSetting", "inRangeCheckTime", None, "10")] },
    Case {
        only_c: false,
        set: &[("alerts.oor_repeat", 1)],
        tables: &["tabOutRangeSetting"],
        radio: &[("tabOutRangeSetting", "AlertInterval", None, "180"), ("tabOutRangeSetting", "AlertTimes", None, "6"), ("tabOutRangeSetting", "AlertTimeout", None, "900")],
    },
    Case {
        only_c: false,
        set: &[("alerts.oor_repeat", 1), ("alerts.oor_interval", 300), ("alerts.oor_timeout", 1800)],
        tables: &["tabOutRangeSetting"],
        radio: &[("tabOutRangeSetting", "AlertInterval", None, "300"), ("tabOutRangeSetting", "AlertTimes", None, "7"), ("tabOutRangeSetting", "AlertTimeout", None, "1800")],
    },
    // With Alert Repeatedly off the converter writes 0s: a profile-only change.
    Case { only_c: false, set: &[("alerts.oor_interval", 300)], tables: &[], radio: &[("tabOutRangeSetting", "AlertInterval", None, "0")] },
    Case {
        only_c: false,
        set: &[("alerts.unread", 0)],
        tables: &["tabTodaySettings", "tabFunctionsOnOff"],
        radio: &[("tabTodaySettings", "isAlertToneOn", None, "0"), ("tabFunctionsOnOff", "ReminderAlertOn", None, "0"), ("tabFunctionsOnOff", "UnreadAlertOn", None, "0")],
    },
    Case {
        only_c: false,
        set: &[("alerts.unread_interval", 600)],
        tables: &["tabTodaySettings", "tabUnreadAlertSettings"],
        radio: &[("tabTodaySettings", "AlertToneInterval", None, "600"), ("tabUnreadAlertSettings", "AlertToneInterval", None, "600")],
    },
    Case {
        only_c: false,
        set: &[("backlight.mode", 0)],
        tables: &["tabBacklightModeSettings", "tabBackLightModeSetting", "tabFunctionsOnOff"],
        radio: &[("tabBacklightModeSettings", "Mode", None, "0"), ("tabBackLightModeSetting", "Mode", None, "0"), ("tabFunctionsOnOff", "backlightOn", None, "0")],
    },
    Case { only_c: false, set: &[("backlight.level", 8)], tables: &["tabBackLightModeSetting"], radio: &[("tabBackLightModeSetting", "BrightnessLevel", None, "8")] },
    Case { only_c: false, set: &[("backlight.timeout", 30)], tables: &["tabBacklightSettings"], radio: &[("tabBacklightSettings", "DefaultTime", None, "30")] },
    Case { only_c: false, set: &[("backlight.timeout", 0)], tables: &["tabBacklightSettings"], radio: &[("tabBacklightSettings", "DefaultTime", None, "0")] },
    Case { only_c: false, set: &[("backlight.flash_on", 2)], tables: &["tabRecvBackgroundColorSetting"], radio: &[("tabRecvBackgroundColorSetting", "FlashOnTime", None, "2")] },
    Case { only_c: false, set: &[("backlight.flash_off", 3)], tables: &["tabRecvBackgroundColorSetting"], radio: &[("tabRecvBackgroundColorSetting", "FlashOffTime", None, "3")] },
    Case { only_c: false, set: &[("backlight.flash_duration", 30)], tables: &["tabRecvBackgroundColorSetting"], radio: &[("tabRecvBackgroundColorSetting", "FlashDuration", None, "30")] },
    Case { only_c: false, set: &[("backlight.flash_voice", 1)], tables: &["tabRecvBackgroundColorSetting"], radio: &[("tabRecvBackgroundColorSetting", "IsVoiceFlash", None, "1")] },
    Case { only_c: false, set: &[("backlight.flash_text", 1)], tables: &["tabRecvBackgroundColorSetting"], radio: &[("tabRecvBackgroundColorSetting", "IsTextFlash", None, "1")] },
    Case { only_c: false, set: &[("backlight.flash_image", 1)], tables: &["tabRecvBackgroundColorSetting"], radio: &[("tabRecvBackgroundColorSetting", "IsImageFlash", None, "1")] },
    Case { only_c: false, set: &[("voice.record_limit", 120)], tables: &["tabAudioRecordTimer"], radio: &[("tabAudioRecordTimer", "second", Some(("function", "201")), "120")] },
    Case { only_c: false, set: &[("menu.852", 1)], tables: &["tabFunctionSettingList"], radio: &[("tabFunctionSettingList", "Enable", Some(("FuncSettingId", "26")), "1")] },
    Case { only_c: false, set: &[("menu.814", 1)], tables: &["tabFunctionStatus"], radio: &[("tabFunctionStatus", "Name", Some(("SettingItemId", "23")), "In/Out Range Alert")] },
];

const RECEIVE: Option<(&str, &str)> = Some(("funcClass", "0"));
const NOTIFY: Option<(&str, &str)> = Some(("funcClass", "1"));

#[test]
fn every_setting_changes_its_column_and_only_its_radio_tables() {
    let fx = fixtures();
    if fx.is_empty() {
        return eprintln!("skipping: fixtures not available");
    }
    for (dir, t, v) in &fx {
        let label = v.db_version();
        let cases: Vec<&Case> = CASES.iter().filter(|c| !c.only_c || *v == Variant::C).collect();
        eprintln!("{label}: {} cases on {}", cases.len(), dir.file_name().unwrap().to_string_lossy().split('-').next().unwrap());
        for case in cases {
            let p = Profile::from_backup(dir).unwrap();
            let before = read(&p);
            let edits: BTreeMap<String, i64> = case.set.iter().map(|(k, n)| (k.to_string(), *n)).collect();
            for (k, n) in case.set {
                assert_ne!(before.get(*k), Some(n), "{label}: {k} already {n}; pick another test value");
            }
            let changes = apply(&p, &edits, *v, NOW).unwrap();
            assert_eq!(changes.len(), case.set.len(), "{label}: {:?}", case.set);
            let after = read(&p);
            for (k, n) in case.set {
                assert_eq!(after.get(*k), Some(n), "{label}: profile {k}");
            }
            for (k, old) in &before {
                if !case.set.iter().any(|(x, _)| x == k) {
                    assert_eq!(after.get(k), Some(old), "{label}: {k} changed as a side effect");
                }
            }
            assert_eq!(load_table(&p.db, "tabOrganization_New")[0].s("FinishDate"), NOW);

            let (mut changed, out) = convert_and_diff(&p, t, *v, dir);
            changed.sort();
            let mut want: Vec<String> = case.tables.iter().map(|s| s.to_string()).collect();
            want.sort();
            assert_eq!(changed, want, "{label}: {:?}", case.set);
            let mut guard: Vec<&str> = case.set.iter().flat_map(|(k, _)| radio_tables(k).iter().copied()).collect();
            guard.sort();
            guard.dedup();
            assert!(want.iter().all(|w| guard.contains(&w.as_str())), "{label}: write guard for {:?} lacks {want:?}", case.set);
            for (table, column, row, value) in case.radio {
                assert_eq!(radio_value(out.path(), table, column, *row), *value, "{label}: {table}.{column} after {:?}", case.set);
            }
        }
    }
}

#[test]
fn writing_current_values_is_a_no_op() {
    let fx = fixtures();
    if fx.is_empty() {
        return eprintln!("skipping: fixtures not available");
    }
    for (dir, t, v) in &fx {
        let p = Profile::from_backup(dir).unwrap();
        let mut current = read(&p);
        if *v == Variant::B {
            current.remove("display.rssi");
        }
        assert!(current.len() >= 4, "read found {current:?}");
        let bytes = fs::read(&p.db_path).unwrap();
        assert!(apply(&p, &current, *v, NOW).unwrap().is_empty());
        assert_eq!(fs::read(&p.db_path).unwrap(), bytes, "{}: profile changed", v.db_version());
        assert!(convert_and_diff(&p, t, *v, dir).0.is_empty());
    }
}

#[test]
fn out_of_range_and_unknown_settings_are_refused() {
    let fx = fixtures();
    if fx.is_empty() {
        return eprintln!("skipping: fixtures not available");
    }
    for (dir, _, v) in &fx {
        let p = Profile::from_backup(dir).unwrap();
        let bad = |k: &str, n: i64| apply(&p, &BTreeMap::from([(k.to_string(), n)]), *v, NOW).unwrap_err().to_string();
        assert!(bad("hold.timeout", 100).contains("outside"));
        assert!(bad("hold.enabled", 2).contains("outside"));
        assert!(bad("hold.timeout", -1).contains("outside"));
        assert!(bad("no.such", 1).contains("unknown"));
        assert!(bad("display.date", 3).contains("outside"));
        assert!(bad("alerts.receive_type", 0).contains("outside"));
        assert!(bad("alerts.receive_volume", 6).contains("outside"));
        assert!(bad("alerts.oor_interval", 20).contains("outside"));
        assert!(bad("alerts.oor_timeout", 28801).contains("outside"));
        assert!(bad("alerts.unread_interval", 2).contains("outside"));
        assert!(bad("backlight.level", 0).contains("outside"));
        assert!(bad("backlight.level", 11).contains("outside"));
        assert!(bad("backlight.timeout", 61).contains("outside"));
        assert!(bad("backlight.flash_duration", 2).contains("outside"));
        assert!(bad("backlight.flash_duration", 61).contains("outside"));
        assert!(bad("backlight.flash_on", 0).contains("outside"));
        assert!(bad("voice.record_limit", 90).contains("outside"));
        assert!(bad("voice.record_limit", 0).contains("outside"));
        assert!(bad("voice.record_limit", 660).contains("outside"));
        assert!(bad("menu.852", 2).contains("outside"));
        assert!(bad("menu.141", 0).contains("unknown"), "top-level functions aren't offered");
        assert!(bad("menu.112", 1).contains("only while function 111"), "PPS hides 112 while 111 is off");
        if *v == Variant::B {
            assert!(bad("display.rssi", 1).contains("no place"), "RSSI type has no 1.41b destination");
        }
        // A refused batch writes nothing, even its valid keys.
        let before = read(&p);
        assert!(apply(&p, &BTreeMap::from([("hold.enabled".to_string(), 1), ("hold.timeout".to_string(), 500)]), *v, NOW).is_err());
        assert_eq!(read(&p), before);
    }
}

/// Function Status items; the rest are Function Setting items (notes/pager-settings.md 2.6).
const STATUS_ITEMS: &[&str] = &["menu.810", "menu.811", "menu.812", "menu.814", "menu.815", "menu.816", "menu.817", "menu.820", "menu.102", "menu.103", "menu.112", "menu.522"];

#[test]
fn every_menu_item_adds_or_removes_its_menu_rows() {
    let fx = fixtures();
    if fx.is_empty() {
        return eprintln!("skipping: fixtures not available");
    }
    for (dir, t, v) in &fx {
        let label = v.db_version();
        let keys: Vec<String> = read(&Profile::from_backup(dir).unwrap()).into_keys().filter(|k| k.starts_with("menu.")).collect();
        assert!(keys.contains(&"menu.852".to_string()) && !keys.contains(&"menu.112".to_string()), "{label}: {keys:?}");
        eprintln!("{label}: {} menu items", keys.len());
        for key in keys {
            let p = Profile::from_backup(dir).unwrap();
            let cur = read(&p)[&key];
            apply(&p, &BTreeMap::from([(key.clone(), 1 - cur)]), *v, NOW).unwrap();
            let out = tempfile::NamedTempFile::new().unwrap();
            convert(&p, t, *v, out.path()).unwrap();
            let ours = sqlite2::Connection::open(out.path()).unwrap();
            let theirs = sqlite2::Connection::open(&dir.join(BACKUP_RADIO_DB)).unwrap();
            let diffs: Vec<_> = compare(&ours, &theirs).unwrap().into_iter().filter(|d| d.status != TableStatus::Same).collect();
            let want = if STATUS_ITEMS.contains(&key.as_str()) { "tabFunctionStatus" } else { "tabFunctionSettingList" };
            let [d] = diffs.as_slice() else { panic!("{label} {key}: {diffs:?}") };
            assert_eq!(d.table, want, "{label} {key}");
            assert_eq!(radio_tables(&key), [want], "{label} {key}: write guard");
            if cur == 0 {
                assert!(d.left_rows > d.right_rows, "{label} {key} on: {d:?}");
            } else {
                assert!(d.left_rows < d.right_rows, "{label} {key} off: {d:?}");
            }
        }
    }
}
