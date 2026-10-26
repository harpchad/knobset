export const hex = (n: number, width: number) => n.toString(16).toUpperCase().padStart(width, "0");

export const mhz = (hz: number) => (hz / 1e6).toFixed(5);

/** A backup's `taken_at` instant in the device's local time, with its zone: `Oct 17, 2026, 11:59 AM CDT`. */
export function formatTaken(b: { taken: string; taken_at: string }): string {
  const d = new Date(b.taken_at);
  if (!b.taken_at || isNaN(d.getTime())) return b.taken;
  // dateStyle/timeStyle can't be combined with timeZoneName (toLocaleString throws), so spell the fields out.
  return d.toLocaleString(undefined, { year: "numeric", month: "short", day: "numeric", hour: "numeric", minute: "2-digit", timeZoneName: "short" });
}

export type ModeFamily = "trunked" | "conventional" | "scan" | "search" | "none";

export const MODES: Record<number, { label: string; family: ModeFamily }> = {
  0: { label: "Empty", family: "none" },
  1: { label: "Selective Call", family: "conventional" },
  4: { label: "Scan", family: "scan" },
  8: { label: "Free Scan", family: "search" },
  16: { label: "TG Scan", family: "trunked" },
  33: { label: "Monitor", family: "conventional" },
  36: { label: "Priority Scan", family: "scan" },
  48: { label: "TG Monitor", family: "trunked" },
  65: { label: "Multi Selective Call", family: "conventional" },
  68: { label: "Silent Scan", family: "scan" },
  97: { label: "Multi Monitor", family: "conventional" },
  128: { label: "Radio Compatible", family: "conventional" },
  129: { label: "Radio Compatible (Trunked)", family: "trunked" },
};

export const mode = (m: number) => MODES[m] ?? { label: `Mode ${m}`, family: "none" as ModeFamily };

/** One-line explanation shown on hover. */
export const MODE_HELP: Record<number, string> = {
  16: "Trunked: plays only the talkgroups on this knob's list",
  48: "Trunked: plays every talkgroup; listed ones get their alert settings",
  33: "Conventional: plays anything matching the channel's squelch",
  1: "Conventional: plays only the listed IDs",
  4: "Scans a list of conventional knobs",
  36: "Scans a list of knobs, sampling a priority channel more often",
  8: "Sweeps a frequency range and decodes what it finds",
};
