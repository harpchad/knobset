// Per-talkgroup alert, tone, voice storage and colour (notes/talkgroup-settings.md).
import type { TalkgroupEdit } from "./types";

export const ALERT_MODES = [
  { value: 1, label: "Tone", short: "Tone" },
  { value: 2, label: "Vibrate", short: "Vibrate" },
  { value: 3, label: "Silent", short: "Silent" },
  { value: 4, label: "Tone and vibrate", short: "Tone + vib" },
];

/** The built-in tones PPS offers; custom tones (other IDs) are shown but not set. */
export const ALERT_TONES = [
  { value: "9999", label: "Default tone" },
  { value: "10000", label: "Chirp" },
  ...[1, 2, 3, 4, 5, 6, 7, 8].map((n) => ({ value: String(n), label: `Tone ${n}` })),
  { value: "0", label: "No tone" },
];

export const BACKGROUNDS = [
  { value: 0, label: "Off", swatch: "" },
  { value: 1, label: "Green", swatch: "#22c55e" },
  { value: 2, label: "Blue", swatch: "#3b82f6" },
  { value: 3, label: "Cyan", swatch: "#06b6d4" },
  { value: 4, label: "Red", swatch: "#ef4444" },
  { value: 5, label: "Yellow", swatch: "#eab308" },
  { value: 6, label: "Magenta", swatch: "#d946ef" },
  { value: 7, label: "White", swatch: "#f8fafc" },
];

export interface TgSettings {
  alert_mode: number;
  alert_tone: string;
  voice_storage: boolean;
  background: number;
}

/** PPS's talkgroup editor defaults (P25TGroupListControl.cs:201-217) and its RadioReference importer's (RadioReferencehandler.cs:553-562). */
export const EDITOR_DEFAULTS: TgSettings = { alert_mode: 1, alert_tone: "9999", voice_storage: true, background: 0 };
export const IMPORT_DEFAULTS: TgSettings = { alert_mode: 3, alert_tone: "9999", voice_storage: false, background: 0 };

/** What a talkgroup will be written with, filling unset fields with the defaults for a new row. */
export function settingsOf(t: TalkgroupEdit): TgSettings {
  const d = t.imported ? IMPORT_DEFAULTS : EDITOR_DEFAULTS;
  return { alert_mode: t.alert_mode ?? d.alert_mode, alert_tone: t.alert_tone ?? d.alert_tone, voice_storage: t.voice_storage ?? d.voice_storage, background: t.background ?? d.background };
}

export const isBuiltinTone = (tone: string) => ALERT_TONES.some((x) => x.value === tone);
/** The tone only sounds in these alert modes; PPS greys it out otherwise. */
export const toneAudible = (mode: number) => mode === 1 || mode === 4;

export const alertLabel = (m: number) => ALERT_MODES.find((x) => x.value === m)?.label ?? `Mode ${m}`;
export const alertShort = (m: number) => ALERT_MODES.find((x) => x.value === m)?.short ?? `Mode ${m}`;
export const toneLabel = (tone: string) => ALERT_TONES.find((x) => x.value === tone)?.label ?? "Custom tone";
export const background = (v: number) => BACKGROUNDS.find((x) => x.value === v) ?? { value: v, label: `Colour ${v}`, swatch: "" };

export const sameSettings = (a: TalkgroupEdit, b: TalkgroupEdit) => JSON.stringify(settingsOf(a)) === JSON.stringify(settingsOf(b));
