import { useState } from "react";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { HexInput, MhzInput } from "@/components/hex-input";
import { cn } from "@/lib/utils";
import { mhz } from "@/lib/format";
import { CTCSS_TONES, DCS_CODES, toneLabel } from "@/lib/squelch";
import { knobKey, useStore } from "@/lib/store";
import type { DigitalSettings, Squelch } from "@/lib/types";

const SELECT = "h-8 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50";
const MAX_NAME = 14;

/** Problems that block writing, mirroring the backend's checks. */
export function convProblems(name: string, rx: number, ranges: [number, number][]): string[] {
  const out: string[] = [];
  if (!name.trim()) out.push("Name can't be empty");
  else if (name.length > MAX_NAME) out.push(`Name: at most ${MAX_NAME} characters`);
  else if (!/^[\x20-\x7E]+$/.test(name)) out.push("Name: letters, digits, spaces and basic punctuation only");
  if (!Number.isFinite(rx) || rx % 10 !== 0) out.push("Enter the frequency in MHz, up to 5 decimals");
  else if (ranges.length && !ranges.some(([lo, hi]) => rx >= lo && rx <= hi)) out.push(`${mhz(rx)} MHz is outside this pager's bands`);
  return out;
}

/** Problems with a digital knob's talkgroup list. */
function digitalProblems(d: DigitalSettings): string[] {
  const out: string[] = [];
  const dmr = d.protocol.type === "dmr";
  const max = dmr ? 15 : 20;
  if (d.talkgroups.length > max) out.push(`At most ${max} talkgroups`);
  const ok = (v: number) => (dmr ? (v >= 1 && v <= 16776415) || (v >= 16777200 && v <= 16777215) : v >= 1 && v <= 65535);
  const seen = new Set<number>();
  for (const t of d.talkgroups) {
    if (!Number.isInteger(t.id) || !ok(t.id)) out.push(`Talkgroup ${Number.isNaN(t.id) ? "?" : t.id} is out of range`);
    if (seen.has(t.id)) out.push(`Talkgroup ${t.id} is listed twice`);
    seen.add(t.id);
    if (!t.name.trim() || t.name.length > MAX_NAME) out.push(`Talkgroup ${t.id}: name 1 to ${MAX_NAME} characters`);
  }
  if (d.protocol.type === "p25c" && d.protocol.nac === 0xf7f) out.push("NAC F7F isn't allowed");
  return out;
}

const DEFAULT_DMR: DigitalSettings = { protocol: { type: "dmr", slot: 1, color_code: 1 }, talkgroups: [], repeater_mode: false };
const DEFAULT_P25C: DigitalSettings = { protocol: { type: "p25c", nac: 0xf7e, filter_nac: false }, talkgroups: [], repeater_mode: false };

/** DMR / P25 conventional fields: protocol parameters and an optional talkgroup list. */
function DigitalFields({ d, set }: { d: DigitalSettings; set: (d: DigitalSettings) => void }) {
  const p = d.protocol;
  const [tgText, setTgText] = useState("");
  const [tgName, setTgName] = useState("");
  const tgId = /^\d+$/.test(tgText) ? parseInt(tgText, 10) : NaN;
  return (
    <div className="space-y-3 rounded-md border p-3">
      {p.type === "dmr" ? (
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1.5">
            <Label htmlFor="dmr-slot">Timeslot</Label>
            <select id="dmr-slot" className={SELECT} value={p.slot} onChange={(e) => set({ ...d, protocol: { ...p, slot: Number(e.target.value) } })}>
              <option value={1}>Slot 1</option>
              <option value={2}>Slot 2</option>
            </select>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="dmr-cc">Color code</Label>
            <select id="dmr-cc" className={SELECT} value={p.color_code} onChange={(e) => set({ ...d, protocol: { ...p, color_code: Number(e.target.value) } })}>
              {Array.from({ length: 16 }, (_, i) => (
                <option key={i} value={i}>
                  CC {i}
                </option>
              ))}
            </select>
          </div>
        </div>
      ) : (
        <div className="space-y-1.5">
          <Label htmlFor="p25c-nac">NAC (hex)</Label>
          <div className="flex items-center gap-3">
            <HexInput aria-label="NAC" value={p.nac} max={0xfff} digits={3} className="w-20" onChange={(nac) => set({ ...d, protocol: { ...p, nac } })} />
            <label className="flex items-center gap-1.5 text-sm">
              <input type="checkbox" className="accent-primary" checked={p.filter_nac} onChange={(e) => set({ ...d, protocol: { ...p, filter_nac: e.target.checked } })} />
              Only this NAC
            </label>
          </div>
          <p className="text-xs text-muted-foreground">Monitor plays every NAC unless "Only this NAC" is on. F7E means any NAC.</p>
        </div>
      )}
      <div className="space-y-1">
        <label className="flex items-center gap-1.5 text-sm">
          <input type="checkbox" className="accent-primary" checked={d.repeater_mode} onChange={(e) => set({ ...d, repeater_mode: e.target.checked })} />
          Repeater mode
        </label>
        <p className="text-xs text-muted-foreground">Off is PPS's default and should still hear a repeater. Turn it on if the knob stays silent.</p>
      </div>
      <div className="space-y-1.5">
        <Label>Talkgroups</Label>
        {d.talkgroups.length === 0 && <p className="text-xs text-muted-foreground">None listed: the knob uses an "All" placeholder ({p.type === "dmr" ? "16777215" : "65535"}).</p>}
        <ul className="divide-y rounded-md border empty:hidden">
          {d.talkgroups.map((t) => (
            <li key={t.id} className="flex items-center gap-2 px-2 py-1 text-sm">
              <span className="w-20 font-mono text-xs tabular-nums">{t.id}</span>
              <span className="flex-1 truncate font-mono">{t.name}</span>
              <Button variant="ghost" size="icon-xs" title="Remove" onClick={() => set({ ...d, talkgroups: d.talkgroups.filter((x) => x.id !== t.id) })}>
                <Trash2 />
              </Button>
            </li>
          ))}
        </ul>
        <div className="flex gap-2">
          <Input aria-label="Talkgroup ID" placeholder="ID" inputMode="numeric" className="w-24 font-mono" value={tgText} onChange={(e) => setTgText(e.target.value)} />
          <Input aria-label="Talkgroup name" placeholder="Name" maxLength={MAX_NAME} className="font-mono" value={tgName} onChange={(e) => setTgName(e.target.value)} />
          <Button
            size="sm"
            variant="outline"
            disabled={Number.isNaN(tgId) || !tgName.trim()}
            onClick={() => {
              set({ ...d, talkgroups: [...d.talkgroups, { id: tgId, name: tgName.trim() }] });
              setTgText("");
              setTgName("");
            }}
          >
            Add
          </Button>
        </div>
      </div>
    </div>
  );
}

/** A new conventional channel on an empty position (analog, P25 or DMR), editable until it's written. */
export function ConvKnobPanel({ zone, position }: { zone: string; position: number }) {
  const edit = useStore((s) => s.convKnobs[knobKey(zone, position)]);
  const profileBands = useStore((s) => s.profileBands);
  const supports = { dmr: useStore((s) => !!s.profile?.summary.dmr), p25c: useStore((s) => !!s.profile?.summary.p25c) };
  const update = useStore((s) => s.updateConvKnob);
  const clear = useStore((s) => s.clearConvKnob);
  if (!edit) return null;
  const set = (patch: Parameters<typeof update>[2]) => update(zone, position, patch);
  const setSquelch = (squelch: Squelch) => set({ squelch });
  const ranges = profileBands.flatMap((b) => b.ranges);
  const problems = [...convProblems(edit.name, edit.rx_hz, ranges), ...(edit.digital ? digitalProblems(edit.digital) : [])];
  const kind = edit.digital?.protocol.type ?? "analog";
  const offGrid = Number.isFinite(edit.rx_hz) && edit.rx_hz % 2500 !== 0;
  const sq = edit.squelch;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>
          {zone} · position {position} · new conventional knob
        </span>
        <Button variant="ghost" size="xs" className="text-destructive hover:text-destructive" onClick={() => clear(zone, position)}>
          <Trash2 />
          Remove
        </Button>
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="conv-name">Name</Label>
        <Input id="conv-name" value={edit.name} maxLength={MAX_NAME} className="font-mono" onChange={(e) => set({ name: e.target.value })} />
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="conv-kind">Type</Label>
        <select
          id="conv-kind"
          className={SELECT}
          value={kind}
          onChange={(e) =>
            set(
              e.target.value === "dmr"
                ? { digital: DEFAULT_DMR, bandwidth: 1, squelch: { type: "carrier" } }
                : e.target.value === "p25c"
                  ? { digital: DEFAULT_P25C, bandwidth: 1, squelch: { type: "carrier" } }
                  : { digital: undefined },
            )
          }
        >
          <option value="analog">Analog FM</option>
          {supports.p25c && <option value="p25c">P25 conventional (experimental)</option>}
          {supports.dmr && <option value="dmr">DMR (experimental)</option>}
        </select>
        {edit.digital && (
          <p className="text-xs text-amber-700 dark:text-amber-400">
            Built from Unication's rules but not yet tested on a pager. Back up first (Knobset always does) and listen to check it.
          </p>
        )}
      </div>
      <div className="grid grid-cols-[1fr_6.5rem] gap-3">
        <div className="space-y-1.5">
          <Label htmlFor="conv-freq">Frequency (MHz)</Label>
          <MhzInput id="conv-freq" aria-label="Frequency" hz={edit.rx_hz} onChange={(rx_hz) => set({ rx_hz })} />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="conv-bw">Bandwidth</Label>
          <select id="conv-bw" className={SELECT} value={edit.bandwidth} disabled={!!edit.digital} onChange={(e) => set({ bandwidth: Number(e.target.value) })}>
            <option value={1}>12.5 kHz</option>
            <option value={2}>25 kHz</option>
          </select>
        </div>
      </div>
      <p className="-mt-2 text-xs text-muted-foreground">
        {profileBands.map((b) => `${b.name} ${b.comment}`).join(" · ")}. Most public-safety analog is 12.5 kHz; NOAA weather and older systems use 25 kHz.
      </p>
      {edit.digital && <DigitalFields d={edit.digital} set={(digital) => set({ digital })} />}
      <div className={cn("space-y-1.5", edit.digital && "hidden")}>
        <Label htmlFor="conv-sq">Squelch</Label>
        <select
          id="conv-sq"
          className={SELECT}
          value={sq.type}
          onChange={(e) => setSquelch(e.target.value === "ctcss" ? { type: "ctcss", tenths: 1000 } : e.target.value === "dcs" ? { type: "dcs", code: "023", inverted: false } : { type: "carrier" })}
        >
          <option value="carrier">Carrier (open on any signal)</option>
          <option value="ctcss">CTCSS tone (PL)</option>
          <option value="dcs">DCS code (DPL)</option>
        </select>
        {sq.type === "ctcss" && (
          <select aria-label="CTCSS tone" className={SELECT} value={sq.tenths} onChange={(e) => setSquelch({ type: "ctcss", tenths: Number(e.target.value) })}>
            {CTCSS_TONES.map((t) => (
              <option key={t} value={t}>
                {toneLabel(t)} Hz
              </option>
            ))}
          </select>
        )}
        {sq.type === "dcs" && (
          <div className="flex items-center gap-3">
            <select aria-label="DCS code" className={SELECT} value={sq.code} onChange={(e) => setSquelch({ ...sq, code: e.target.value })}>
              {DCS_CODES.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
            <label className="flex shrink-0 items-center gap-1.5 text-sm">
              <input type="checkbox" className="accent-primary" checked={sq.inverted} onChange={(e) => setSquelch({ ...sq, inverted: e.target.checked })} />
              Inverted
            </label>
          </div>
        )}
        <p className="text-xs text-muted-foreground">RadioReference lists the tone in the frequency's Tone column (for example 127.3 PL or 023 DPL).</p>
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="conv-prompt">Voice prompt</Label>
        <select id="conv-prompt" className={SELECT} value={edit.voice_prompt} onChange={(e) => set({ voice_prompt: Number(e.target.value) })}>
          <option value={0}>Off</option>
          <option value={2}>"Monitor Mode" (built in)</option>
        </select>
      </div>
      {offGrid && problems.length === 0 && <p className="text-xs text-amber-700 dark:text-amber-400">{mhz(edit.rx_hz)} MHz isn't on the usual 2.5 kHz channel grid. Double-check it.</p>}
      {problems.length > 0 && (
        <ul className="space-y-0.5 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
          {problems.map((p) => (
            <li key={p}>{p}</li>
          ))}
        </ul>
      )}
      <p className="text-xs text-muted-foreground">Monitor mode plays anything on the frequency that passes the squelch. Nothing reaches the pager until you write.</p>
    </div>
  );
}

/** Settings of an existing analog conventional knob, edited in place. */
export function ConvEditor({ zone, knob }: { zone: string; knob: import("@/lib/types").Knob }) {
  const update = useStore((s) => s.convUpdates[knobKey(zone, knob.index)]);
  const setConvUpdate = useStore((s) => s.setConvUpdate);
  const profileBands = useStore((s) => s.profileBands);
  const cur = knob.conv;
  if (!cur) return null;
  const v = {
    rx_hz: update?.rx_hz ?? cur.rx_hz,
    bandwidth: update?.bandwidth ?? cur.bandwidth,
    squelch: update?.squelch ?? cur.squelch,
    alert_mode: update?.alert_mode ?? cur.alert_mode,
    record_flag: update?.record_flag ?? cur.record_flag,
    voice_prompt: update?.voice_prompt ?? cur.voice_prompt,
  };
  const set = (patch: Omit<import("@/lib/types").ConvKnobUpdate, "zone" | "knob">) => setConvUpdate(zone, knob.index, patch);
  const ranges = profileBands.flatMap((b) => b.ranges);
  const problems = convProblems("ok", v.rx_hz, ranges);
  const sq = v.squelch;
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-[1fr_6.5rem] gap-3">
        <div className="space-y-1.5">
          <Label htmlFor="ce-freq">Frequency (MHz)</Label>
          <MhzInput id="ce-freq" aria-label="Frequency" hz={v.rx_hz} onChange={(rx_hz) => set({ rx_hz })} />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="ce-bw">Bandwidth</Label>
          <select id="ce-bw" className={SELECT} value={v.bandwidth} onChange={(e) => set({ bandwidth: Number(e.target.value) })}>
            <option value={1}>12.5 kHz</option>
            <option value={2}>25 kHz</option>
          </select>
        </div>
      </div>
      {sq === null ? (
        <p className="text-xs text-muted-foreground">This knob has several squelch tones; edit them in Unication PPS.</p>
      ) : (
        <div className="space-y-1.5">
          <Label htmlFor="ce-sq">Squelch</Label>
          <select
            id="ce-sq"
            className={SELECT}
            value={sq.type}
            onChange={(e) => set({ squelch: e.target.value === "ctcss" ? { type: "ctcss", tenths: 1000 } : e.target.value === "dcs" ? { type: "dcs", code: "023", inverted: false } : { type: "carrier" } })}
          >
            <option value="carrier">Carrier (open on any signal)</option>
            <option value="ctcss">CTCSS tone (PL)</option>
            <option value="dcs">DCS code (DPL)</option>
          </select>
          {sq.type === "ctcss" && (
            <select aria-label="CTCSS tone" className={SELECT} value={sq.tenths} onChange={(e) => set({ squelch: { type: "ctcss", tenths: Number(e.target.value) } })}>
              {(CTCSS_TONES.includes(sq.tenths) ? CTCSS_TONES : [sq.tenths, ...CTCSS_TONES]).map((t) => (
                <option key={t} value={t}>
                  {toneLabel(t)} Hz
                </option>
              ))}
            </select>
          )}
          {sq.type === "dcs" && (
            <div className="flex items-center gap-3">
              <select aria-label="DCS code" className={SELECT} value={sq.code} onChange={(e) => set({ squelch: { ...sq, code: e.target.value } })}>
                {DCS_CODES.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
              <label className="flex shrink-0 items-center gap-1.5 text-sm">
                <input type="checkbox" className="accent-primary" checked={sq.inverted} onChange={(e) => set({ squelch: { ...sq, inverted: e.target.checked } })} />
                Inverted
              </label>
            </div>
          )}
        </div>
      )}
      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-1.5">
          <Label htmlFor="ce-alert">Alert</Label>
          <select id="ce-alert" className={SELECT} value={v.alert_mode} onChange={(e) => set({ alert_mode: Number(e.target.value) })}>
            <option value={1}>Tone</option>
            <option value={2}>Vibrate</option>
            <option value={3}>Silent</option>
            <option value={4}>Tone and vibrate</option>
          </select>
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="ce-rec">Voice storage</Label>
          <select id="ce-rec" className={SELECT} value={v.record_flag} onChange={(e) => set({ record_flag: Number(e.target.value) })}>
            <option value={0}>Off</option>
            <option value={1}>ID match</option>
            <option value={2}>All voice</option>
          </select>
        </div>
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="ce-prompt">Voice prompt</Label>
        <select id="ce-prompt" className={SELECT} value={v.voice_prompt === 2 ? 2 : 0} onChange={(e) => set({ voice_prompt: Number(e.target.value) })}>
          <option value={0}>Off</option>
          <option value={2}>"Monitor Mode" (built in)</option>
        </select>
      </div>
      {problems.length > 0 && (
        <ul className="space-y-0.5 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
          {problems.map((p) => (
            <li key={p}>{p}</li>
          ))}
        </ul>
      )}
      {update && <p className="text-xs text-amber-700 dark:text-amber-400">Changed; written on the next write.</p>}
    </div>
  );
}
