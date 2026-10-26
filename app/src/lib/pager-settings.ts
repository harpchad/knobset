// Pager settings page: one entry per setting key of g5_codeplug::settings.
// Values use the profile's own encoding (notes/pager-settings.md section 2).

export type Control =
  | { kind: "toggle" }
  | { kind: "select"; options: [number, string][] }
  | { kind: "number"; min: number; max: number; unit: string; zeroLabel?: string }
  /** 0 (`zeroLabel`), or min..max (`otherLabel`, starting at `start`). */
  | { kind: "zeroOr"; min: number; max: number; unit: string; zeroLabel: string; otherLabel: string; start: number };

export interface SettingDef {
  key: string;
  label: string;
  control: Control;
  /** Greyed out, as PPS does, unless this holds for the current values. */
  enabledIf?: (v: (key: string) => number | undefined) => boolean;
  hint?: string;
  /** Oldest radio DB version that carries the setting. */
  minDb?: string;
  /** Subheading the setting is listed under. */
  group?: string;
}

export interface SectionDef {
  id: string;
  title: string;
  /** What the settings do on the pager. */
  note: string;
  settings: SettingDef[];
}

const toggle: Control = { kind: "toggle" };
const toneType: Control = { kind: "select", options: [[1, "Follows the volume knob"], [2, "Fixed"]] };
const volume: Control = { kind: "select", options: [[0, "Mute"], [1, "1 (quietest)"], [2, "2"], [3, "3"], [4, "4"], [5, "5 (loudest)"]] };
const secs = (min: number, max: number): Control => ({ kind: "number", min, max, unit: "s" });

export const SECTIONS: SectionDef[] = [
  {
    id: "hold",
    title: "Talkgroup hold and lockout",
    note: "Soft keys on Trunking TG-Scan and TG-Monitor knobs. Hold keeps the pager on the talkgroup you're hearing until you press un-hold. Block skips the current talkgroup (up to 64) until you turn the knob away and back; nothing stays blocked after that, so take a talkgroup off the knob's list to drop it for good.",
    settings: [
      { key: "hold.enabled", label: "Hold soft key", control: toggle },
      {
        key: "hold.timeout",
        label: "Hold timeout",
        control: { kind: "number", min: 0, max: 99, unit: "min", zeroLabel: "never" },
        hint: "Releases a hold automatically after this long. To change it on the pager as well, turn on TGID Hold Timeout under Pager menu items.",
      },
      { key: "hold.unhold_alert", label: "Beep on un-hold", control: toggle },
      { key: "hold.remove", label: "Block soft key", control: toggle },
    ],
  },
  {
    id: "display",
    title: "Display format",
    note: "How the pager shows IDs, the date and the clock. Decimal talkgroup IDs match RadioReference.",
    settings: [
      { key: "display.call_id", label: "Talkgroup and unit IDs", control: { kind: "select", options: [[0, "Decimal"], [1, "Hexadecimal"]] }, hint: "Call IDs on P25, DMR and MDC channels." },
      { key: "display.system_id", label: "System IDs", control: { kind: "select", options: [[0, "Decimal"], [1, "Hexadecimal"]] }, hint: "WACN, System ID, RFSS, Site and NAC." },
      { key: "display.date", label: "Date format", control: { kind: "select", options: [[0, "MM/DD/YYYY"], [1, "DD/MM/YYYY"], [2, "YYYY/MM/DD"]] } },
      { key: "display.hour", label: "Clock", control: { kind: "select", options: [[0, "12 hour"], [1, "24 hour"]] } },
      { key: "display.rssi", label: "Signal strength in the status bar", control: { kind: "select", options: [[0, "Bars icon"], [1, "dBm"]] }, minDb: "DBV1.41c" },
    ],
  },
  {
    id: "alerts",
    title: "Alerts",
    note: "Out of the box every call beeps at full volume, the pager beeps each time it loses or regains the trunked system, and a reminder repeats until stored messages are played. On a scanner you'll usually want these quieter or off.",
    settings: [
      { key: "alerts.receive_type", label: "Call alert tone volume", control: toneType, hint: "The tone at the start of a call." },
      { key: "alerts.receive_volume", label: "Fixed call alert volume", control: volume, enabledIf: (v) => v("alerts.receive_type") === 2 },
      { key: "alerts.notify_type", label: "Notification tone volume", control: toneType, hint: "Other tones, such as reminders and range alerts." },
      { key: "alerts.notify_volume", label: "Fixed notification volume", control: volume, enabledIf: (v) => v("alerts.notify_type") === 2 },
      { key: "alerts.oor_alert", label: "Beep on losing or regaining the system", control: toggle, hint: "P25 trunking only." },
      { key: "alerts.oor_timer", label: "Out of range after", control: secs(0, 999), hint: "How long the system must be gone before the pager counts as out of range." },
      { key: "alerts.in_range_timer", label: "Back in range after", control: secs(0, 999) },
      { key: "alerts.oor_repeat", label: "Keep beeping while out of range", control: toggle },
      { key: "alerts.oor_interval", label: "Repeat every", control: secs(30, 3600), enabledIf: (v) => v("alerts.oor_repeat") === 1 },
      { key: "alerts.oor_timeout", label: "Stop repeating after", control: secs(30, 28800), enabledIf: (v) => v("alerts.oor_repeat") === 1 },
      { key: "alerts.unread", label: "Unread message reminder", control: toggle, hint: "Beeps while a stored message is unread." },
      { key: "alerts.unread_interval", label: "Remind every", control: secs(3, 3600), enabledIf: (v) => v("alerts.unread") === 1 },
    ],
  },
  {
    id: "backlight",
    title: "Backlight and screen",
    note: "Screen brightness, how long the screen stays lit, and the receive flash: the screen blinks a talkgroup's background colour while its call comes in. Talkgroups with no background colour have nothing to blink.",
    settings: [
      { key: "backlight.mode", label: "Brightness", control: { kind: "select", options: [[0, "Automatic"], [1, "Manual"]] } },
      {
        key: "backlight.level",
        label: "Manual brightness",
        control: { kind: "select", options: Array.from({ length: 10 }, (_, i) => [i + 1, `${(i + 1) * 10}%`] as [number, string]) },
        enabledIf: (v) => v("backlight.mode") === 1,
      },
      { key: "backlight.timeout", label: "Screen timeout", control: { kind: "number", min: 0, max: 60, unit: "s", zeroLabel: "always on" }, hint: "0 keeps the screen on." },
      { key: "backlight.flash_voice", label: "Flash on calls", control: toggle },
      { key: "backlight.flash_text", label: "Flash on text messages", control: toggle },
      { key: "backlight.flash_image", label: "Flash on image messages", control: toggle },
      { key: "backlight.flash_on", label: "Flash on time", control: secs(1, 5) },
      { key: "backlight.flash_off", label: "Flash off time", control: secs(1, 5) },
      {
        key: "backlight.flash_duration",
        label: "Flash for",
        control: { kind: "zeroOr", min: 3, max: 60, unit: "s", zeroLabel: "The whole call", otherLabel: "A set time", start: 60 },
        hint: "After a set time the screen stays on the colour.",
      },
    ],
  },
  {
    id: "voice",
    title: "Voice storage",
    note: "Caps how long one stored voice message can be. Which calls get stored is set per knob and per talkgroup.",
    settings: [
      {
        key: "voice.record_limit",
        label: "Longest stored message",
        control: { kind: "select", options: Array.from({ length: 10 }, (_, i) => [(i + 1) * 60, `${i + 1} minute${i ? "s" : ""}`] as [number, string]) },
      },
    ],
  },
];

const STATUS = "Function Status menu";
const SETTING = "Function Setting menu";
const menu = (id: number, label: string, group: string, hint?: string): SettingDef => ({ key: `menu.${id}`, label, control: toggle, group, hint });

SECTIONS.push({
  id: "menus",
  title: "Pager menu items",
  note: "Which items appear in the pager's Function Status and Function Setting menus. Turning one off only hides it from the menu; the setting itself keeps its value. Items whose parent feature is off in the profile aren't listed, as in Unication's software.",
  settings: [
    menu(810, "Select Zone", STATUS),
    menu(811, "Duty On/Off", STATUS),
    menu(814, "In/Out Range Alert", STATUS, "Turn the out-of-range beep on or off from the pager."),
    menu(815, "Alert Mode", STATUS, "Switch between tone, vibrate and silent."),
    menu(103, "Fixed Alert", STATUS, "Alert volume by the knob or fixed."),
    menu(102, "Voice Call Buffering", STATUS, "Voice buffering on digital channels."),
    menu(522, "Auto Keypad Lock", STATUS, "Locks the keys when the screen goes off."),
    menu(112, "Display Msg. Content", STATUS),
    menu(812, "Bluetooth Settings", STATUS, "Pair a Bluetooth earpiece."),
    menu(816, "Log Adv.", STATUS, "Unication's diagnostic logging."),
    menu(817, "Rev. Encrypt Msg", STATUS, "Receive encrypted messages or not."),
    menu(820, "TEK Keyset", STATUS, "Switch encryption keyset A or B."),
    menu(841, "Zone", SETTING),
    menu(852, "TGID Hold Timeout", SETTING, "Change the talkgroup hold timeout on the pager."),
    menu(850, "Unread Msg Alert", SETTING),
    menu(843, "LCD Brightness", SETTING),
    menu(844, "Screen Timeout", SETTING),
    menu(845, "Date & Time", SETTING),
    menu(848, "Date Format", SETTING),
    menu(849, "Hour Format", SETTING),
    menu(846, "Audio Squelch", SETTING, "Conventional channels only."),
    menu(502, "Voice Memo Setting", SETTING),
    menu(853, "Factory Setting", SETTING, "Wipes the pager's user data. Turn it off to hide it."),
  ],
});

export function settingDef(key: string): SettingDef | undefined {
  return SECTIONS.flatMap((s) => s.settings).find((d) => d.key === key);
}

/** How a value reads in the UI and the write dialog. */
export function formatValue(def: SettingDef | undefined, v: number | undefined): string {
  if (v === undefined) return "?";
  const c = def?.control;
  if (!c) return String(v);
  switch (c.kind) {
    case "toggle":
      return v ? "on" : "off";
    case "select":
      return c.options.find(([x]) => x === v)?.[1] ?? String(v);
    case "number":
      return v === 0 && c.zeroLabel ? c.zeroLabel : `${v} ${c.unit}`;
    case "zeroOr":
      return v === 0 ? c.zeroLabel.toLowerCase() : `${v} ${c.unit}`;
  }
}

/** Why a typed value can't be used, mirroring the backend's range check. */
export function valueProblem(def: SettingDef, v: number): string | null {
  const c = def.control;
  if (c.kind !== "number" && c.kind !== "zeroOr") return null;
  if (c.kind === "zeroOr" && v === 0) return null;
  if (!Number.isInteger(v)) return "Whole numbers only";
  if (v < c.min || v > c.max) return `${c.min} to ${c.max} ${c.unit}`;
  return null;
}
