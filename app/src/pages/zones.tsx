import { useMemo, useState } from "react";
import { ArrowDown, ArrowUpDown, ArrowUp, CircleAlert, Copy, ListChecks, MoveRight, Plus, Radio, RotateCcw, Star, Trash2, Undo2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { MODE_HELP, mode, type ModeFamily } from "@/lib/format";
import { MOVABLE_MODES, builtinPrompt, knobKey, splitList, useStore } from "@/lib/store";
import type { Knob, P25System, Zone } from "@/lib/types";
import { TalkgroupPicker } from "@/components/talkgroup-picker";
import { KnobTargetDialog } from "@/components/knob-target-dialog";
import { ConvEditor, ConvKnobPanel } from "@/components/conv-knob-panel";
import { ScanKnobPanel } from "@/components/scan-knob-panel";

const POSITIONS = 8;
const MAX_NAME = 14;

const FAMILY_STYLE: Record<ModeFamily, string> = {
  trunked: "bg-sky-500/10 text-sky-700 dark:text-sky-300 border-sky-500/20",
  conventional: "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 border-emerald-500/20",
  scan: "bg-amber-500/10 text-amber-700 dark:text-amber-300 border-amber-500/20",
  search: "bg-violet-500/10 text-violet-700 dark:text-violet-300 border-violet-500/20",
  none: "bg-muted text-muted-foreground",
};

export function validName(name: string): string | null {
  if (name.length === 0) return "Name can't be empty";
  if (name.length > MAX_NAME) return `At most ${MAX_NAME} characters (the pager's display width)`;
  if (!/^[\x20-\x7E]+$/.test(name)) return "Letters, digits, spaces and basic punctuation only";
  return null;
}

/** Re-render on anything that changes how a knob looks (pending assignment, system drafts). */
function useEffectiveKnob(zone: Zone, index: number | null): Knob | undefined {
  useStore((s) => (index === null ? undefined : s.knobSystems[knobKey(zone.name, index)]));
  useStore((s) => (index === null ? undefined : s.convKnobs[knobKey(zone.name, index)]));
  useStore((s) => (index === null ? undefined : s.scanKnobs[knobKey(zone.name, index)]));
  useStore((s) => (index === null ? undefined : s.removedKnobs[knobKey(zone.name, index)]));
  useStore((s) => (index === null ? undefined : s.convUpdates[knobKey(zone.name, index)]));
  useStore((s) => s.knobCopies);
  useStore((s) => s.knobMoves);
  useStore((s) => s.systems);
  const effectiveKnob = useStore((s) => s.effectiveKnob);
  return index === null ? undefined : effectiveKnob(zone, index);
}

/** Systems a knob can use: the profile's (minus deleted ones) and new ones. */
function useSystemChoices(): { id: string; name: string }[] {
  const profile = useStore((s) => s.profile);
  const drafts = useStore((s) => s.systems);
  const deleted = useStore((s) => s.deletedSystems);
  return useMemo(
    () => [
      ...(profile?.summary.systems ?? []).filter((s) => !deleted.includes(s.id)).map((s) => ({ id: s.id, name: drafts[s.id]?.draft.name ?? s.name })),
      ...Object.values(drafts)
        .filter((e) => e.band)
        .map((e) => ({ id: e.draft.system_id, name: `${e.draft.name} (new)` })),
    ],
    [profile, drafts, deleted],
  );
}

const SELECT = "h-8 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50";

function KnobCard({ zone, position, selected, onSelect }: { zone: Zone; position: number; selected: boolean; onSelect: () => void }) {
  const knob = useEffectiveKnob(zone, position);
  const pending = useStore((s) => (knob ? s.pendingFor(zone.name, knob.index) : undefined));
  // Select the stored edit itself (stable reference); building a new object here loops React.
  const tgEdited = useStore((s) => s.talkgroups[knobKey(zone.name, position)]);
  const assigned = useStore((s) => s.knobSystems[knobKey(zone.name, position)] ?? s.convKnobs[knobKey(zone.name, position)] ?? s.scanKnobs[knobKey(zone.name, position)]);
  const removed = useStore((s) => !!s.removedKnobs[knobKey(zone.name, position)]);
  const convEdited = useStore((s) => !!s.convUpdates[knobKey(zone.name, position)]);
  const move = useStore((s) => s.moveAt(zone.name, position));
  const copy = useStore((s) => s.knobCopies[knobKey(zone.name, position)]);
  const outbound = move && move.zone === zone.name && move.knob === position;
  if (!knob && move && outbound) {
    const was = zone.knobs.find((k) => k.index === position)?.name;
    return (
      <button
        onClick={onSelect}
        className={cn("flex h-32 flex-col rounded-xl border border-dashed border-amber-500/50 p-3 text-left text-muted-foreground transition hover:border-amber-500/80", selected && "ring-2 ring-ring")}
      >
        <span className="font-mono text-xs">{position}</span>
        <span className="mt-3 truncate font-mono text-[15px]">{was}</span>
        <span className="mt-auto text-xs text-amber-700 dark:text-amber-300">
          Moves to {move.to_zone} · {move.to_knob}
        </span>
      </button>
    );
  }
  if (!knob && removed) {
    const was = zone.knobs.find((k) => k.index === position)?.name;
    return (
      <button
        onClick={onSelect}
        className={cn("flex h-32 flex-col rounded-xl border border-dashed border-destructive/40 p-3 text-left text-muted-foreground transition hover:border-destructive/70", selected && "ring-2 ring-ring")}
      >
        <span className="font-mono text-xs">{position}</span>
        <span className="mt-3 truncate font-mono text-[15px] line-through decoration-destructive/60">{was}</span>
        <span className="mt-auto text-xs text-destructive">Removed on the next write</span>
      </button>
    );
  }
  if (!knob) {
    const assignable = zone.empty_knobs.includes(position);
    return (
      <button
        disabled={!assignable}
        onClick={onSelect}
        className={cn(
          "flex h-32 flex-col rounded-xl border border-dashed p-3 text-left text-muted-foreground/60 transition",
          assignable && "hover:border-foreground/30 hover:text-muted-foreground",
          selected && "ring-2 ring-ring",
        )}
      >
        <span className="font-mono text-xs">{position}</span>
        <span className="m-auto flex items-center gap-1 text-xs">
          {assignable && <Plus className="size-3.5" />}
          {assignable ? "Empty · add a knob" : "Empty"}
        </span>
      </button>
    );
  }
  const m = mode(knob.mode);
  const from = move ? (outbound ? `${move.to_zone} · ${move.to_knob}` : `${move.zone} · ${move.knob}`) : null;
  return (
    <button
      onClick={onSelect}
      className={cn(
        "flex h-32 flex-col rounded-xl border bg-card p-3 text-left shadow-xs transition hover:border-foreground/30",
        selected && "ring-2 ring-ring",
        (assigned || copy) && !zone.knobs.some((k) => k.index === position) && "border-dashed border-emerald-500/50",
        move && "border-dashed border-amber-500/50",
      )}
    >
      <div className="flex items-center gap-2">
        <span className="flex size-6 items-center justify-center rounded-full bg-muted font-mono text-xs">{position}</span>
        <span className={cn("rounded-md border px-1.5 py-0.5 text-[11px] font-medium", FAMILY_STYLE[m.family])}>{m.label}</span>
        {(pending !== undefined || tgEdited || assigned || convEdited || move || copy) && <span className="ml-auto size-2 rounded-full bg-amber-500" title="Unsaved changes" />}
      </div>
      <div className="mt-3 truncate font-mono text-[15px] font-medium tracking-tight">{pending ?? knob.name}</div>
      {(from || copy) && (
        <div className="truncate text-[11px] text-amber-700 dark:text-amber-300">{from ? `${move?.swap ? "Swapped" : "Moved"} from ${from}` : `Copy of ${copy?.zone} · ${copy?.knob}`}</div>
      )}
      <div className={cn("mt-auto truncate text-xs text-muted-foreground", knob.system_id && knob.talkgroups === 0 && "text-destructive")}>
        {knob.target ?? " "}
        {knob.system_id && ` · ${knob.talkgroups} TG${knob.talkgroups !== 1 ? "s" : ""}`}
      </div>
    </button>
  );
}

function EmptyKnobPanel({ zone, position }: { zone: Zone; position: number }) {
  const digital = useStore((s) => [s.profile?.summary.p25c && "P25 conventional", s.profile?.summary.dmr && "DMR"].filter(Boolean).join(" or "));
  const assignSystem = useStore((s) => s.assignSystem);
  const removed = useStore((s) => !!s.removedKnobs[knobKey(zone.name, position)]);
  const undoRemoveKnob = useStore((s) => s.undoRemoveKnob);
  const was = zone.knobs.find((k) => k.index === position)?.name;
  const addConvKnob = useStore((s) => s.addConvKnob);
  const startScanKnob = useStore((s) => s.startScanKnob);
  const choices = useSystemChoices();
  const [sid, setSid] = useState(choices[0]?.id ?? "");
  const removedName = useStore((s) => (s.removedKnobs[knobKey(zone.name, position)] ? zone.knobs.find((k) => k.index === position)?.name : undefined));
  // A replacement keeps the removed knob's name; a new knob takes the system's.
  const name = removedName ?? choices.find((c) => c.id === sid)?.name.replace(/ \(new\)$/, "").slice(0, MAX_NAME) ?? "";
  return (
    <div className="space-y-4">
      <div className="text-xs text-muted-foreground">
        {zone.name} · position {position}
      </div>
      {removed && (
        <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-2.5 text-xs">
          <span className="flex-1">
            <span className="font-mono">{was}</span> is removed on the next write. Add a knob below to replace it with a different kind.
          </span>
          <Button variant="ghost" size="xs" onClick={() => undoRemoveKnob(zone.name, position)}>
            <Undo2 />
            Undo
          </Button>
        </div>
      )}
      <p className="text-sm">This position is empty. Make it a P25 trunked knob, a conventional channel or a scan.</p>
      <h3 className="text-sm font-medium">P25 trunking system</h3>
      {choices.length === 0 ? (
        <p className="text-sm text-muted-foreground">Add a system on the P25 systems page first.</p>
      ) : (
        <>
          <div className="space-y-1.5">
            <Label htmlFor="new-knob-system">System</Label>
            <select id="new-knob-system" className={SELECT} value={sid} onChange={(e) => setSid(e.target.value)}>
              {choices.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          </div>
          <Button size="sm" disabled={!sid} onClick={() => assignSystem(zone, position, sid, { mode: 16, name })}>
            <Plus />
            Add P25 knob
          </Button>
        </>
      )}
      <h3 className="border-t pt-4 text-sm font-medium">Conventional channel</h3>
      <p className="text-sm text-muted-foreground">
        One frequency in Monitor mode. Analog with carrier, CTCSS or DCS squelch for fire dispatch, NOAA weather and the like
        {digital ? `; ${digital} (experimental) from the Type menu.` : "."}
      </p>
      <Button size="sm" variant="outline" onClick={() => addConvKnob(zone.name, position)}>
        <Radio />
        Add conventional knob
      </Button>
      {!removed && <h3 className="border-t pt-4 text-sm font-medium">Scan</h3>}
      {!removed && (
        <>
          <p className="text-sm text-muted-foreground">Scan 2 to 16 analog conventional knobs from any zone, optionally checking a priority channel.</p>
          <Button size="sm" variant="outline" onClick={() => startScanKnob(zone.name, position, 4)}>
            <ListChecks />
            Add scan knob
          </Button>
        </>
      )}
    </div>
  );
}

/** An existing scan knob's channels, with a way into the member editor. */
function ScanMembers({ zone, knob }: { zone: Zone; knob: Knob }) {
  const startScanKnob = useStore((s) => s.startScanKnob);
  const candidates = useStore((s) => s.scanCandidates)();
  const byId = new Map(candidates.map((c) => [c.channel_id, c]));
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between">
        <Label>Scan list</Label>
        <Button variant="ghost" size="xs" onClick={() => startScanKnob(zone.name, knob.index, knob.mode)}>
          <ListChecks />
          Edit scan list
        </Button>
      </div>
      <ul className="divide-y rounded-lg border">
        {(knob.scan_members ?? []).map((m) => {
          const c = byId.get(m);
          return (
            <li key={m} className="flex items-center gap-2 px-3 py-1.5 text-sm">
              <span className="truncate font-mono">{c?.name ?? "?"}</span>
              <span className="truncate text-xs text-muted-foreground">{c ? `${c.zone} ${c.knob}` : ""}</span>
              {knob.scan_priority === m && <Star className="ml-auto size-3.5 fill-amber-400 text-amber-400" aria-label="Priority" />}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** A knob copied to this empty position: its name, and a way to drop it. */
function CopyPanel({ zone, position }: { zone: Zone; position: number }) {
  const copy = useStore((s) => s.knobCopies[knobKey(zone.name, position)]);
  const knob = useEffectiveKnob(zone, position);
  const renameKnobCopy = useStore((s) => s.renameKnobCopy);
  const clearKnobCopy = useStore((s) => s.clearKnobCopy);
  const source = useStore((s) => (copy ? s.zonesView().find((z) => z.name === copy.zone)?.knobs.find((k) => k.index === copy.knob)?.name : undefined));
  const [draft, setDraft] = useState(copy?.name ?? "");
  if (!copy || !knob) return null;
  const error = validName(draft);
  const m = mode(knob.mode);
  return (
    <div className="space-y-5">
      <div>
        <div className="text-xs text-muted-foreground">
          {zone.name} · position {position}
        </div>
        <div className="mt-1 flex items-center gap-2">
          <span className={cn("rounded-md border px-1.5 py-0.5 text-xs font-medium", FAMILY_STYLE[m.family])}>{m.label}</span>
          {knob.target && <span className="text-sm">{knob.target}</span>}
        </div>
      </div>
      <div className="flex items-start gap-2 rounded-md border border-emerald-500/30 bg-emerald-500/5 p-2.5 text-xs">
        <span className="flex-1">
          Copy of <span className="font-mono">{source}</span> ({copy.zone} · {copy.knob}), added on the next write
          {knob.mode === 4 || knob.mode === 36 ? `. It scans the same ${knob.scan_members?.length ?? 0} channels.` : knob.system_id ? ` with the same ${knob.talkgroups} talkgroups.` : " with the same channel settings."}
        </span>
        <Button variant="ghost" size="xs" onClick={() => clearKnobCopy(zone.name, position)}>
          <Undo2 />
          Remove
        </Button>
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="copy-knob-name">Name on the pager display</Label>
        <Input
          id="copy-knob-name"
          value={draft}
          maxLength={MAX_NAME}
          className="font-mono"
          aria-invalid={!!error}
          onChange={(e) => {
            setDraft(e.target.value);
            if (!validName(e.target.value)) renameKnobCopy(zone.name, position, e.target.value);
          }}
        />
        <div className="flex justify-between text-xs">
          <span className={error ? "text-destructive" : "text-muted-foreground"}>{error ?? " "}</span>
          <span className="tabular-nums text-muted-foreground">
            {draft.length}/{MAX_NAME}
          </span>
        </div>
      </div>
      <p className="text-xs text-muted-foreground">To change the copy's talkgroups or settings, write it first; it's then a knob like any other.</p>
    </div>
  );
}

/** A position that's part of a pending move or swap. */
function MovePanel({ zone, position }: { zone: Zone; position: number }) {
  const move = useStore((s) => s.moveAt(zone.name, position));
  const knob = useEffectiveKnob(zone, position);
  const clearKnobMove = useStore((s) => s.clearKnobMove);
  if (!move) return null;
  const outbound = move.zone === zone.name && move.knob === position;
  const other = outbound ? `${move.to_zone} · ${move.to_knob}` : `${move.zone} · ${move.knob}`;
  const was = zone.knobs.find((k) => k.index === position)?.name;
  const m = knob ? mode(knob.mode) : null;
  return (
    <div className="space-y-5">
      <div>
        <div className="text-xs text-muted-foreground">
          {zone.name} · position {position}
        </div>
        {m && knob && (
          <div className="mt-1 flex items-center gap-2">
            <span className={cn("rounded-md border px-1.5 py-0.5 text-xs font-medium", FAMILY_STYLE[m.family])}>{m.label}</span>
            <span className="truncate font-mono text-sm">{knob.name}</span>
          </div>
        )}
      </div>
      <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/5 p-2.5 text-xs">
        <span className="flex-1">
          {knob ? (
            <>
              <span className="font-mono">{knob.name}</span> moves here from {other} on the next write
              {move.swap ? <>; <span className="font-mono">{was}</span> takes its old place.</> : "."}
            </>
          ) : (
            <>
              <span className="font-mono">{was}</span> moves to {other} on the next write, leaving this position empty.
            </>
          )}
        </span>
        <Button variant="ghost" size="xs" onClick={() => clearKnobMove(zone.name, position)}>
          <Undo2 />
          Undo
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        The pager numbers channels by position, so moved channels get new numbers. Scans that list them follow them. Write the move before changing these knobs.
      </p>
    </div>
  );
}

function KnobDetail({ zone, knob, onGoto }: { zone: Zone; knob: Knob; onGoto: (zone: string, position: number) => void }) {
  const { profile, renameKnob, pendingFor, talkgroupsFor, setKnobTalkgroups, assignSystem, updateKnobSystem, clearKnobSystem, removeKnob, setKnobMode, positionLock } = useStore();
  const knobCopies = useStore((s) => s.knobCopies);
  const outgoing = Object.values(knobCopies).filter((c) => c.zone === zone.name && c.knob === knob.index);
  const [target, setTarget] = useState<"copy" | "move" | null>(null);
  const assignment = useStore((s) => s.knobSystems[knobKey(zone.name, knob.index)]);
  const tgEdit = useStore((s) => s.talkgroups[knobKey(zone.name, knob.index)]);
  useStore((s) => s.systems); // re-render on system draft edits
  const systemView = useStore((s) => s.systemView);
  const choices = useSystemChoices();
  const [pickerOpen, setPickerOpen] = useState(false);
  const original = zone.knobs.find((k) => k.index === knob.index);
  const isNewKnob = !!assignment && !original;
  const current = talkgroupsFor(zone.name, knob);
  const pending = pendingFor(zone.name, knob.index);
  const [draft, setDraft] = useState(pending ?? knob.name);
  const error = validName(draft);
  // Unsaved system edits (new, renamed or deleted talkgroups) show up here too.
  const system: P25System | undefined = knob.system_id ? systemView(knob.system_id) : undefined;
  const tgs = useMemo(() => {
    const byId = new Map(system?.talkgroup_list.map((t) => [t.group_id, t]));
    return [...current.priority.map((g) => ({ group_id: g, priority: true })), ...current.normal.map((g) => ({ group_id: g, priority: false }))].map((k) => ({ ...k, tg: byId.get(k.group_id) }));
  }, [system, current.priority, current.normal]);
  const m = mode(knob.mode);
  const trunked = knob.mode === 16 || knob.mode === 48;
  const oldSystem = original?.system_id ? profile?.summary.systems.find((s) => s.id === original.system_id)?.name : undefined;

  return (
    <div className="space-y-5">
      <div>
        <div className="text-xs text-muted-foreground">
          {zone.name} · position {knob.index}
        </div>
        <div className="mt-1 flex items-center gap-2">
          <span className={cn("rounded-md border px-1.5 py-0.5 text-xs font-medium", FAMILY_STYLE[m.family])}>{m.label}</span>
          {knob.target && <span className="text-sm">{knob.target}</span>}
        </div>
        {MODE_HELP[knob.mode] && <p className="mt-2 text-xs text-muted-foreground">{MODE_HELP[knob.mode]}</p>}
      </div>

      {assignment && (
        <div className="flex items-start gap-2 rounded-md border border-emerald-500/30 bg-emerald-500/5 p-2.5 text-xs">
          <span className="flex-1">
            {isNewKnob
              ? "New knob, added on the next write."
              : original?.system_id === assignment.system_id
                ? `Switching to ${assignment.mode === 48 ? "TG Monitor" : "TG Scan"} on the next write.`
                : `Moving from ${oldSystem ?? "its old system"} to ${knob.target}. Talkgroups don't carry over between systems, so pick new ones.`}
          </span>
          <Button variant="ghost" size="xs" onClick={() => clearKnobSystem(zone.name, knob.index)}>
            <Undo2 />
            {isNewKnob ? "Remove" : "Undo"}
          </Button>
        </div>
      )}

      {outgoing.length > 0 && (
        <div className="rounded-md border border-emerald-500/30 bg-emerald-500/5 p-2.5 text-xs">
          Copied to {outgoing.map((c) => `${c.to_zone} · ${c.to_knob}`).join(", ")} on the next write. Write or remove the copy before changing this knob.
        </div>
      )}

      <fieldset disabled={outgoing.length > 0} className={cn("min-w-0 space-y-5", outgoing.length > 0 && "opacity-60")}>
      <div className="space-y-1.5">
        <Label htmlFor="knob-name">Name on the pager display</Label>
        <div className="flex gap-2">
          <Input
            id="knob-name"
            value={draft}
            maxLength={MAX_NAME}
            className="font-mono"
            aria-invalid={!!error}
            onChange={(e) => {
              setDraft(e.target.value);
              if (validName(e.target.value)) return;
              if (isNewKnob) updateKnobSystem(zone.name, knob.index, { name: e.target.value });
              else renameKnob(zone.name, knob.index, e.target.value);
            }}
          />
          {pending !== undefined && (
            <Button
              variant="ghost"
              size="icon"
              title={`Revert to "${knob.name}"`}
              onClick={() => {
                setDraft(knob.name);
                renameKnob(zone.name, knob.index, knob.name);
              }}
            >
              <RotateCcw />
            </Button>
          )}
        </div>
        <div className="flex justify-between text-xs">
          <span className={error ? "text-destructive" : "text-muted-foreground"}>{error ?? (pending !== undefined ? `Was "${knob.name}"` : " ")}</span>
          <span className="tabular-nums text-muted-foreground">
            {draft.length}/{MAX_NAME}
          </span>
        </div>
      </div>

      {trunked && (
        <div className="space-y-1.5">
          <Label htmlFor="knob-system">System</Label>
          <select id="knob-system" className={SELECT} value={knob.system_id ?? ""} onChange={(e) => assignSystem(zone, knob.index, e.target.value)}>
            {choices.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </div>
      )}

      {trunked && !isNewKnob && original && (
        <div className="space-y-1.5">
          <Label htmlFor="knob-switch">Mode</Label>
          <select id="knob-switch" className={SELECT} value={assignment?.mode ?? knob.mode} onChange={(e) => setKnobMode(zone, knob.index, Number(e.target.value))}>
            <option value={16}>TG Scan: only the talkgroups on this knob</option>
            <option value={48}>TG Monitor: every talkgroup on the system</option>
          </select>
        </div>
      )}

      {knob.conv && original && knob.mode === 33 && <ConvEditor zone={zone.name} knob={knob} />}

      {(knob.mode === 4 || knob.mode === 36) && <ScanMembers zone={zone} knob={knob} />}

      {isNewKnob && assignment && (
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1.5">
            <Label htmlFor="knob-mode">Mode</Label>
            <select
              id="knob-mode"
              className={SELECT}
              value={assignment.mode}
              onChange={(e) => {
                const mode = Number(e.target.value);
                updateKnobSystem(zone.name, knob.index, { mode, voice_prompt: assignment.voice_prompt ? builtinPrompt(mode) : 0 });
              }}
            >
              <option value={16}>TG Scan</option>
              <option value={48}>TG Monitor</option>
            </select>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="knob-prompt">Voice prompt</Label>
            <select
              id="knob-prompt"
              className={SELECT}
              value={assignment.voice_prompt ? "builtin" : "none"}
              title="Spoken when you turn to this knob. Unication's software turns its built-in prompt on for new knobs; your existing knobs have it off."
              onChange={(e) => updateKnobSystem(zone.name, knob.index, { voice_prompt: e.target.value === "builtin" ? builtinPrompt(assignment.mode) : 0 })}
            >
              <option value="none">None</option>
              <option value="builtin">Built-in</option>
            </select>
          </div>
        </div>
      )}

      {system && (
        <div className="flex items-center gap-2">
          <Button size="sm" variant={tgs.length === 0 ? "default" : "outline"} onClick={() => setPickerOpen(true)}>
            <ListChecks />
            {tgs.length === 0 ? "Pick talkgroups" : "Edit talkgroups"}
          </Button>
          {tgEdit && !assignment && (
            <Button size="sm" variant="ghost" onClick={() => setKnobTalkgroups(zone.name, knob, splitList(knob).priority, splitList(knob).normal)}>
              <RotateCcw />
              Revert
            </Button>
          )}
          {pickerOpen && <TalkgroupPicker open={pickerOpen} onOpenChange={setPickerOpen} zone={zone} knob={knob} system={system} />}
        </div>
      )}
      {system && tgs.length === 0 && (
        <p className="flex items-center gap-1.5 text-xs text-destructive">
          <CircleAlert className="size-3.5" />A trunked knob needs at least one talkgroup.
        </p>
      )}

      {tgs.length > 0 && (
        <div>
          <div className="mb-2 text-xs font-medium text-muted-foreground">
            Talkgroups on this knob ({tgs.length}){current.edited && <span className="text-amber-600 dark:text-amber-400"> · edited</span>}
          </div>
          <ul className="divide-y rounded-lg border">
            {tgs.map(({ group_id, priority, tg }) => (
              <li key={group_id} className="flex items-center gap-3 px-3 py-1.5 text-sm">
                <span className="w-14 font-mono text-xs tabular-nums text-muted-foreground">{tg?.tgid ?? "?"}</span>
                <span className="truncate">{tg?.name ?? group_id}</span>
                {priority && <Star className="ml-auto size-3.5 fill-amber-400 text-amber-400" aria-label="Priority" />}
              </li>
            ))}
          </ul>
        </div>
      )}
      {original && !assignment && [1, 4, 16, 33, 36, 48].includes(original.mode) && (
        <div className="border-t pt-4">
          <Button variant="ghost" size="sm" className="text-destructive hover:text-destructive" onClick={() => removeKnob(zone.name, knob.index)}>
            <Trash2 />
            Remove knob
          </Button>
          <p className="mt-1 text-xs text-muted-foreground">Empties the position. You can then put a different kind of knob there.</p>
        </div>
      )}
      </fieldset>
      {original && !assignment && MOVABLE_MODES.includes(original.mode) && (
        <div className="space-y-2 border-t pt-4">
          <div className="flex flex-wrap gap-2">
            {(() => {
              const lock = positionLock(zone.name, knob.index, true);
              return (
                <Button size="sm" variant="outline" disabled={!!lock} title={lock ? `This knob ${lock}` : "Copy this knob to an empty position"} onClick={() => setTarget("copy")}>
                  <Copy />
                  Copy to…
                </Button>
              );
            })()}
            {(() => {
              const lock = positionLock(zone.name, knob.index);
              return (
                <Button size="sm" variant="outline" disabled={!!lock} title={lock ? `This knob ${lock}` : "Move this knob, or swap it with another"} onClick={() => setTarget("move")}>
                  <MoveRight />
                  Move to…
                </Button>
              );
            })()}
          </div>
          <p className="text-xs text-muted-foreground">Copy puts a new knob like this one on an empty position, in this zone or another. Move puts this knob somewhere else, or swaps it with a knob there.</p>
          {target && <KnobTargetDialog action={target} zone={zone.name} knob={knob} open onOpenChange={(o) => !o && setTarget(null)} onDone={onGoto} />}
        </div>
      )}
    </div>
  );
}

/** Opens the zone order dialog. */
function ZoneOrderButton({ disabledReason }: { disabledReason?: string }) {
  const [open, setOpen] = useState(false);
  const moveZone = useStore((s) => s.moveZone);
  const zonesView = useStore((s) => s.zonesView);
  useStore((s) => s.zoneOrder);
  const zones = zonesView();
  return (
    <>
      <Button variant="ghost" size="sm" disabled={!!disabledReason || zones.length < 2} title={disabledReason} onClick={() => setOpen(true)}>
        <ArrowUpDown />
        Zone order…
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>Zone order</DialogTitle>
            <DialogDescription>
              The pager lists zones in this order and starts on the first one after a write. Changing it renumbers every channel on the pager, so it's written on its own.
            </DialogDescription>
          </DialogHeader>
          <ol className="divide-y rounded-md border">
            {zones.map((z, i) => (
              <li key={z.id} className="flex items-center gap-2 px-3 py-1.5 text-sm">
                <span className="w-5 font-mono text-xs text-muted-foreground">{i + 1}</span>
                <span className="flex-1 truncate font-mono">{z.name}</span>
                <Button variant="ghost" size="xs" disabled={i === 0} onClick={() => moveZone(z.id, -1)}>
                  <ArrowUp />
                  Up
                </Button>
                <Button variant="ghost" size="xs" disabled={i === zones.length - 1} onClick={() => moveZone(z.id, 1)}>
                  <ArrowDown />
                  Down
                </Button>
              </li>
            ))}
          </ol>
          <DialogFooter>
            <Button onClick={() => setOpen(false)}>Done</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

/** A pending zone reorder or delete, which must be written on its own. */
function ZoneLayoutBanner() {
  const order = useStore((s) => s.zoneOrder);
  const deleted = useStore((s) => s.deletedZones);
  const profile = useStore((s) => s.profile);
  const undo = useStore((s) => s.undoZoneLayout);
  if (!order && deleted.length === 0) return null;
  const names = deleted.map((id) => profile?.summary.zones.find((z) => z.id === id)?.name ?? id);
  return (
    <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/5 p-2.5 text-xs">
      <span className="flex-1">
        {[order && "Zone order changed", names.length > 0 && `Zone ${names.join(", ")} will be deleted`].filter(Boolean).join("; ")}. This renumbers the pager's channels, so it's
        written on its own: write it now, or undo it to make other changes.
      </span>
      <Button variant="ghost" size="xs" onClick={undo}>
        <Undo2 />
        Undo
      </Button>
    </div>
  );
}

/** Name, order and delete controls for the selected zone. */
function ZoneBar({ zone, index, count }: { zone: Zone; index: number; count: number }) {
  const renameZone = useStore((s) => s.renameZone);
  const deleteZone = useStore((s) => s.deleteZone);
  const zonesView = useStore((s) => s.zonesView);
  const isNew = useStore((s) => s.newZones.some((z) => z.zone_id === zone.id));
  const others = useStore((s) => s.hasNonZoneEdits());
  const pendingKnobs = useStore(
    (s) => [...Object.values(s.convKnobs), ...Object.values(s.knobSystems)].some((e) => e.zone === zone.name) || [...Object.values(s.knobCopies), ...Object.values(s.knobMoves)].some((e) => e.to_zone === zone.name),
  );
  const [draft, setDraft] = useState(zone.name);
  const trimmed = draft.trim();
  const clash = zonesView().some((z) => z.id !== zone.id && z.name.trim().toLowerCase() === trimmed.toLowerCase());
  const error = validName(trimmed) ?? (clash ? "Another zone has this name" : null);
  const empty = zone.knobs.length === 0 && !pendingKnobs;
  // The radio numbers channels by zone position, so moves and deletes are written on their own.
  const alone = others && !isNew ? "Write or undo your other changes first: zone moves and deletes are written on their own" : undefined;
  return (
    <div className="flex flex-wrap items-start gap-2">
      <div className="space-y-1">
        <Input
          aria-label="Zone name"
          value={draft}
          maxLength={MAX_NAME}
          aria-invalid={!!error}
          className="h-8 w-44 font-mono"
          onChange={(e) => setDraft(e.target.value)}
          onBlur={() => (error ? setDraft(zone.name) : renameZone(zone.id, trimmed))}
          onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        />
        {error && <p className="text-xs text-destructive">{error}</p>}
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="text-destructive hover:text-destructive"
        disabled={!empty || count === 1 || !!alone}
        title={!empty ? "Remove this zone's knobs first" : count === 1 ? "A profile needs at least one zone" : alone}
        onClick={() => deleteZone(zone.id)}
      >
        <Trash2 />
        {isNew ? "Discard zone" : "Delete zone"}
      </Button>
      {index === 0 && <span className="self-center text-xs text-muted-foreground">The pager starts on the first zone after a write.</span>}
    </div>
  );
}

export default function ZonesPage() {
  const profile = useStore((s) => s.profile);
  const [zoneId, setZoneId] = useState<string | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  // Re-render on zone edits (zonesView reads them).
  useStore((s) => s.newZones);
  useStore((s) => s.zoneRenames);
  useStore((s) => s.deletedZones);
  useStore((s) => s.zoneOrder);
  const zonesView = useStore((s) => s.zonesView);
  const addZone = useStore((s) => s.addZone);
  const zones = zonesView();
  const zoneIdx = Math.max(0, zones.findIndex((z) => z.id === zoneId));
  const zone = zones[zoneIdx];
  const knob = useEffectiveKnob(zone ?? { id: "", name: "", knobs: [], empty_knobs: [] }, zone ? selected : null);
  const convPending = useStore((s) => (zone && selected !== null ? !!s.convKnobs[knobKey(zone.name, selected)] : false));
  const scanPending = useStore((s) => (zone && selected !== null ? !!s.scanKnobs[knobKey(zone.name, selected)] : false));
  const removedPending = useStore((s) => (zone && selected !== null ? !!s.removedKnobs[knobKey(zone.name, selected)] : false));
  const movePending = useStore((s) => (zone && selected !== null ? !!s.moveAt(zone.name, selected) : false));
  const copyPending = useStore((s) => (zone && selected !== null ? !!s.knobCopies[knobKey(zone.name, selected)] : false));
  const goto = (name: string, position: number) => {
    const z = zonesView().find((x) => x.name === name);
    if (!z) return;
    setZoneId(z.id);
    setSelected(position);
  };
  const layoutPending = useStore((s) => !!s.zoneOrder || s.deletedZones.length > 0);
  const nonZoneEdits = useStore((s) => s.hasNonZoneEdits());
  if (!profile) return null;

  return (
    <div className="flex h-full">
      <div className="min-w-0 flex-1 space-y-5 overflow-auto p-6">
        <div className="flex flex-wrap gap-1.5">
          {zones.map((z, i) => (
            <Button key={z.id} size="sm" variant={i === zoneIdx ? "default" : "outline"} onClick={() => { setZoneId(z.id); setSelected(null); }}>
              {z.name}
              <Badge variant="secondary" className="ml-1">{z.knobs.length}</Badge>
            </Button>
          ))}
          <Button size="sm" variant="ghost" disabled={zones.length >= 64} onClick={() => { setZoneId(addZone()); setSelected(null); }}>
            <Plus />
            Zone
          </Button>
          <ZoneOrderButton disabledReason={nonZoneEdits ? "Write or undo your other changes first: zone order changes are written on their own" : undefined} />
        </div>
        {zone && <ZoneBar key={zone.id + zone.name} zone={zone} index={zoneIdx} count={zones.length} />}
        <ZoneLayoutBanner />
        {zone && (
          <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
            {Array.from({ length: POSITIONS }, (_, i) => i + 1).map((pos) => (
              <KnobCard key={pos} zone={zone} position={pos} selected={selected === pos} onSelect={() => setSelected(pos)} />
            ))}
          </div>
        )}
      </div>
      <aside className="w-80 shrink-0 overflow-auto border-l p-5">
        {layoutPending && selected !== null ? (
          <p className="text-sm text-muted-foreground">A zone order change or zone delete is pending. Write it or undo it before changing knobs.</p>
        ) : zone && selected !== null && movePending ? (
          <MovePanel key={`${zone.name}-${selected}`} zone={zone} position={selected} />
        ) : zone && selected !== null && copyPending ? (
          <CopyPanel key={`${zone.name}-${selected}`} zone={zone} position={selected} />
        ) : zone && selected !== null && scanPending ? (
          <ScanKnobPanel key={`${zone.name}-${selected}`} zone={zone.name} position={selected} />
        ) : zone && selected !== null && convPending ? (
          <ConvKnobPanel key={`${zone.name}-${selected}`} zone={zone.name} position={selected} />
        ) : zone && knob ? (
          <KnobDetail key={`${zone.name}-${knob.index}-${knob.system_id}`} zone={zone} knob={knob} onGoto={goto} />
        ) : zone && selected !== null && (zone.empty_knobs.includes(selected) || removedPending) ? (
          <EmptyKnobPanel key={`${zone.name}-${selected}`} zone={zone} position={selected} />
        ) : (
          <p className="text-sm text-muted-foreground">Select a knob position to see its talkgroups and rename it, or an empty one to add a knob.</p>
        )}
      </aside>
    </div>
  );
}
