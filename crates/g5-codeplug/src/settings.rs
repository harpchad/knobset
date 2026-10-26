//! Device-wide pager settings (notes/pager-settings.md): talkgroup hold and
//! remove, display format, alerts, backlight, voice storage and pager menu
//! items. Each setting is one integer in the profile's own encoding, keyed
//! like `hold.enabled`. The converter already carries every one of them to
//! the radio DB; a change may touch only the radio tables listed for its key.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::convert141::Variant;
use crate::row::load_table;
use crate::{Error, Profile};

#[derive(Debug, Clone, Copy)]
enum Allowed {
    Range(i64, i64),
    /// 0, or `lo..=hi`.
    ZeroOr(i64, i64),
    /// `lo, lo + step, ... hi`.
    Step(i64, i64, i64),
}

impl Allowed {
    fn contains(self, v: i64) -> bool {
        match self {
            Self::Range(lo, hi) => (lo..=hi).contains(&v),
            Self::ZeroOr(lo, hi) => v == 0 || (lo..=hi).contains(&v),
            Self::Step(lo, hi, step) => (lo..=hi).contains(&v) && (v - lo) % step == 0,
        }
    }

    fn describe(self) -> String {
        match self {
            Self::Range(lo, hi) => format!("{lo} to {hi}"),
            Self::ZeroOr(lo, hi) => format!("0, or {lo} to {hi}"),
            Self::Step(lo, hi, step) => format!("{lo} to {hi} in steps of {step}"),
        }
    }
}

const ON_OFF: Allowed = Allowed::Range(0, 1);

struct Setting {
    key: &'static str,
    table: &'static str,
    column: &'static str,
    /// `(column, value)` picking the row of a multi-row table.
    row: Option<(&'static str, &'static str)>,
    allowed: Allowed,
    /// Radio-DB tables a change may touch (measured in tests/settings.rs).
    radio: &'static [&'static str],
    /// The radio DB has no destination before DBV1.41c.
    needs_c: bool,
}

const fn single(key: &'static str, table: &'static str, column: &'static str, allowed: Allowed, radio: &'static [&'static str]) -> Setting {
    Setting { key, table, column, row: None, allowed, radio, needs_c: false }
}

const fn in_row(key: &'static str, table: &'static str, column: &'static str, row: (&'static str, &'static str), allowed: Allowed, radio: &'static [&'static str]) -> Setting {
    Setting { key, table, column, row: Some(row), allowed, radio, needs_c: false }
}

const HOLD: &str = "tabTGIDHoldAndRemoveSetting";
const UI_NUMBER: &str = "tabUiNumberFormat";
const DATE_TIME: &str = "tabDateTimeSetting";
const TONE_VOLUME: &str = "tabFuncClassToneVolume";
const RECEIVE_TONE: (&str, &str) = ("funcClass", "0");
const NOTIFY_TONE: (&str, &str) = ("funcClass", "1");
const OUT_OF_RANGE: &str = "tabOutRangeSetting";
const UNREAD: &str = "tabTodaySettings";
const BRIGHTNESS: &str = "tabBacklightModeSettings";
const FLASH: &str = "tabRecvBackgroundColorSetting";

const SETTINGS: &[Setting] = &[
    single("hold.enabled", HOLD, "TGHoldFunc", ON_OFF, &["tabFunctionsOnOff"]),
    single("hold.timeout", HOLD, "TGHoldTimeout", Allowed::Range(0, 99), &["tabFunctionsParam"]),
    single("hold.unhold_alert", HOLD, "UnholdAlert", ON_OFF, &["tabFunctionsOnOff"]),
    single("hold.remove", HOLD, "TGRemvFunc", ON_OFF, &["tabFunctionsOnOff"]),
    single("display.system_id", UI_NUMBER, "SysIdShowType", ON_OFF, &[UI_NUMBER]),
    single("display.call_id", UI_NUMBER, "CallIdShowType", ON_OFF, &[UI_NUMBER]),
    single("display.date", DATE_TIME, "date_format", Allowed::Range(0, 2), &[DATE_TIME]),
    single("display.hour", DATE_TIME, "hour_format", ON_OFF, &[DATE_TIME]),
    Setting { key: "display.rssi", table: "tabHomeSettings", column: "StatusBarRssiShowType", row: None, allowed: ON_OFF, radio: &["tabFunctionsOnOff"], needs_c: true },
    in_row("alerts.receive_type", TONE_VOLUME, "type", RECEIVE_TONE, Allowed::Range(1, 2), &[TONE_VOLUME]),
    in_row("alerts.receive_volume", TONE_VOLUME, "volume", RECEIVE_TONE, Allowed::Range(0, 5), &[TONE_VOLUME]),
    in_row("alerts.notify_type", TONE_VOLUME, "type", NOTIFY_TONE, Allowed::Range(1, 2), &[TONE_VOLUME]),
    in_row("alerts.notify_volume", TONE_VOLUME, "volume", NOTIFY_TONE, Allowed::Range(0, 5), &[TONE_VOLUME]),
    single("alerts.oor_alert", OUT_OF_RANGE, "InOutRangeAlert", ON_OFF, &[OUT_OF_RANGE]),
    single("alerts.oor_timer", OUT_OF_RANGE, "outRangeCheckTimer", Allowed::Range(0, 999), &[OUT_OF_RANGE]),
    single("alerts.in_range_timer", OUT_OF_RANGE, "inRangeCheckTime", Allowed::Range(0, 999), &[OUT_OF_RANGE]),
    single("alerts.oor_repeat", OUT_OF_RANGE, "AlertRepeatedly", ON_OFF, &[OUT_OF_RANGE]),
    single("alerts.oor_interval", OUT_OF_RANGE, "AlertInterval", Allowed::Range(30, 3600), &[OUT_OF_RANGE]),
    single("alerts.oor_timeout", OUT_OF_RANGE, "AlertDuration", Allowed::Range(30, 28800), &[OUT_OF_RANGE]),
    single("alerts.unread", UNREAD, "isAlertToneOn", ON_OFF, &[UNREAD, "tabFunctionsOnOff"]),
    single("alerts.unread_interval", UNREAD, "AlertToneInterval", Allowed::Range(3, 3600), &[UNREAD, "tabUnreadAlertSettings"]),
    single("backlight.mode", BRIGHTNESS, "Mode", ON_OFF, &[BRIGHTNESS, "tabBackLightModeSetting", "tabFunctionsOnOff"]),
    single("backlight.level", BRIGHTNESS, "BrightLevel", Allowed::Range(1, 10), &["tabBackLightModeSetting"]),
    single("backlight.timeout", "tabBacklightSettings", "DefaultTime", Allowed::Range(0, 60), &["tabBacklightSettings"]),
    single("backlight.flash_on", FLASH, "FlashOnTime", Allowed::Range(1, 5), &[FLASH]),
    single("backlight.flash_off", FLASH, "FlashOffTime", Allowed::Range(1, 5), &[FLASH]),
    single("backlight.flash_duration", FLASH, "FlashDuration", Allowed::ZeroOr(3, 60), &[FLASH]),
    single("backlight.flash_voice", FLASH, "IsVoiceFlash", ON_OFF, &[FLASH]),
    single("backlight.flash_text", FLASH, "IsTextFlash", ON_OFF, &[FLASH]),
    single("backlight.flash_image", FLASH, "IsImageFlash", ON_OFF, &[FLASH]),
    single("voice.record_limit", "tabVoiceStorageSetting", "MaxRecordTime", Allowed::Step(60, 600, 60), &["tabAudioRecordTimer"]),
];

const STATUS_MENU: &str = "tabFunctionStatus";
const SETTING_MENU: &str = "tabFunctionSettingList";

/// Device Function Setting items, keyed `menu.<FunctionID>` (notes/pager-settings.md
/// 2.6): leaf functions that each add one item to one pager menu. Function,
/// its parents (all must be enabled, as PPS shows it only then), menu table.
const MENU_ITEMS: &[(i64, &[i64], &str)] = &[
    (810, &[801], STATUS_MENU),
    (811, &[801], STATUS_MENU),
    (812, &[801], STATUS_MENU),
    (814, &[801], STATUS_MENU),
    (815, &[801], STATUS_MENU),
    (816, &[801], STATUS_MENU),
    (817, &[801], STATUS_MENU),
    (820, &[801], STATUS_MENU),
    (102, &[101], STATUS_MENU),
    (103, &[101], STATUS_MENU),
    (112, &[111], STATUS_MENU),
    (522, &[521], STATUS_MENU),
    (841, &[831], SETTING_MENU),
    (843, &[831], SETTING_MENU),
    (844, &[831], SETTING_MENU),
    (845, &[831], SETTING_MENU),
    (846, &[831], SETTING_MENU),
    (848, &[831], SETTING_MENU),
    (849, &[831], SETTING_MENU),
    (850, &[831], SETTING_MENU),
    (852, &[831], SETTING_MENU),
    (853, &[831], SETTING_MENU),
    (502, &[501], SETTING_MENU),
];

const FUNCTIONS: &str = "tabMemberFunctionList";

/// Where a key lives and what it may be.
struct Spec {
    table: &'static str,
    column: &'static str,
    row: Option<(&'static str, String)>,
    allowed: Allowed,
    radio: &'static [&'static str],
    needs_c: bool,
    /// Functions that must be enabled for PPS to offer the setting.
    parents: &'static [i64],
}

fn spec(key: &str) -> Option<Spec> {
    if let Some(id) = key.strip_prefix("menu.") {
        let (fid, parents, menu) = MENU_ITEMS.iter().find(|(f, _, _)| f.to_string() == id)?;
        let radio: &'static [&'static str] = if *menu == STATUS_MENU { &[STATUS_MENU] } else { &[SETTING_MENU] };
        return Some(Spec { table: FUNCTIONS, column: "Enable", row: Some(("FunctionID", fid.to_string())), allowed: ON_OFF, radio, needs_c: false, parents });
    }
    let s = SETTINGS.iter().find(|s| s.key == key)?;
    Some(Spec { table: s.table, column: s.column, row: s.row.map(|(c, v)| (c, v.to_string())), allowed: s.allowed, radio: s.radio, needs_c: s.needs_c, parents: &[] })
}

/// The first parent function that isn't enabled.
fn missing_parent(p: &Profile, s: &Spec) -> Option<i64> {
    let fl = if s.parents.is_empty() { Vec::new() } else { load_table(&p.db, FUNCTIONS) };
    s.parents.iter().copied().find(|id| !fl.iter().any(|r| r.s("FunctionID") == id.to_string() && r.s("Enable").trim() == "1"))
}

fn keys() -> impl Iterator<Item = String> {
    SETTINGS.iter().map(|s| s.key.to_string()).chain(MENU_ITEMS.iter().map(|(f, _, _)| format!("menu.{f}")))
}

fn current(p: &Profile, s: &Spec) -> Option<i64> {
    let rows = load_table(&p.db, s.table);
    let row = match &s.row {
        Some((c, v)) => rows.iter().find(|r| r.s(c) == v),
        None => rows.first(),
    }?;
    row.get(s.column)?.trim().parse().ok()
}

/// Every setting the profile holds, by key. Menu items PPS wouldn't offer are left out.
pub fn read(p: &Profile) -> BTreeMap<String, i64> {
    keys()
        .filter_map(|k| {
            let s = spec(&k)?;
            if missing_parent(p, &s).is_some() {
                return None;
            }
            Some((k, current(p, &s)?))
        })
        .collect()
}

/// Radio-DB tables a change of `key` may touch.
pub fn radio_tables(key: &str) -> &'static [&'static str] {
    spec(key).map(|s| s.radio).unwrap_or(&[])
}

/// One applied setting change.
#[derive(Debug, Clone, Serialize)]
pub struct SettingChange {
    pub key: String,
    pub old: i64,
    pub new: i64,
}

/// Set each key of `edits` the way PPS's settings pages save it. Keys whose
/// value is already the profile's are skipped; nothing is written then.
pub fn apply(p: &Profile, edits: &BTreeMap<String, i64>, variant: Variant, now: &str) -> Result<Vec<SettingChange>, Error> {
    if !p.is_working_copy() {
        return Err(Error::Invalid("refusing to edit a profile in place; load it from a zip".into()));
    }
    let mut todo = Vec::new();
    for (key, &new) in edits {
        let s = spec(key).ok_or_else(|| Error::Invalid(format!("unknown pager setting {key:?}")))?;
        if s.needs_c && variant == Variant::B {
            return Err(Error::Invalid(format!("{key}: this pager's radio DB ({}) has no place for this setting", variant.db_version())));
        }
        if let Some(f) = missing_parent(p, &s) {
            return Err(Error::Invalid(format!("{key}: PPS offers this menu item only while function {f} is enabled")));
        }
        if !s.allowed.contains(new) {
            return Err(Error::Invalid(format!("{key} = {new} is outside PPS's range ({})", s.allowed.describe())));
        }
        let old = current(p, &s).ok_or_else(|| Error::Invalid(format!("the profile has no {}.{} for {key}", s.table, s.column)))?;
        if old != new {
            todo.push((key, s, old, new));
        }
    }
    if todo.is_empty() {
        return Ok(Vec::new());
    }
    p.db.execute_batch("BEGIN;")?;
    let r = (|| {
        let mut out = Vec::new();
        for (key, s, old, new) in &todo {
            let v = new.to_string();
            let n = match &s.row {
                Some((c, rv)) => p.db.execute(&format!("UPDATE {} SET {} = ? WHERE {c} = ?", s.table, s.column), &[Some(&v), Some(rv)])?,
                None => p.db.execute(&format!("UPDATE {} SET {} = ?", s.table, s.column), &[Some(&v)])?,
            };
            if n != 1 {
                return Err(Error::Invalid(format!("{key} touched {n} rows of {}, expected 1", s.table)));
            }
            out.push(SettingChange { key: key.to_string(), old: *old, new: *new });
        }
        p.db.execute("UPDATE tabOrganization_New SET FinishDate = ?", &[Some(now)])?;
        Ok(out)
    })();
    match r {
        Ok(v) => {
            p.db.execute_batch("COMMIT;")?;
            Ok(v)
        }
        Err(e) => {
            let _ = p.db.execute_batch("ROLLBACK;");
            Err(e)
        }
    }
}
