import { ArrowDown, ArrowUp, Star, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { knobKey, useStore } from "@/lib/store";

const SELECT = "h-8 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50";
const MAX_NAME = 14;
const MAX_MEMBERS = 16;

/** Problems that block writing, mirroring the backend's checks. */
export function scanProblems(name: string | null, members: string[], mode: number, priority: string | null): string[] {
  const out: string[] = [];
  if (name !== null) {
    if (!name.trim()) out.push("Name can't be empty");
    else if (name.length > MAX_NAME) out.push(`Name: at most ${MAX_NAME} characters`);
  }
  if (members.length < 2) out.push("A scan needs at least 2 channels");
  if (mode === 36 && (!priority || !members.includes(priority))) out.push("Pick the priority channel");
  return out;
}

/** A scan knob being added, or an existing scan knob's member list being edited. */
export function ScanKnobPanel({ zone, position }: { zone: string; position: number }) {
  const edit = useStore((s) => s.scanKnobs[knobKey(zone, position)]);
  const update = useStore((s) => s.updateScanKnob);
  const clear = useStore((s) => s.clearScanKnob);
  useStore((s) => s.convKnobs);
  useStore((s) => s.renames);
  const candidates = useStore((s) => s.scanCandidates)();
  if (!edit) return null;
  const isNew = edit.name !== null;
  const set = (patch: Parameters<typeof update>[2]) => update(zone, position, patch);
  const byId = new Map(candidates.map((c) => [c.channel_id, c]));
  const setMembers = (members: string[]) => set({ members, priority: edit.priority && members.includes(edit.priority) ? edit.priority : null });
  const move = (i: number, d: -1 | 1) => {
    const m = [...edit.members];
    [m[i], m[i + d]] = [m[i + d], m[i]];
    setMembers(m);
  };
  const available = candidates.filter((c) => !edit.members.includes(c.channel_id));
  const problems = scanProblems(edit.name, edit.members, edit.mode, edit.priority);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>
          {zone} · position {position} · {isNew ? "new scan knob" : "editing scan list"}
        </span>
        <Button variant="ghost" size="xs" className="text-destructive hover:text-destructive" onClick={() => clear(zone, position)}>
          <Trash2 />
          {isNew ? "Remove" : "Undo"}
        </Button>
      </div>
      {isNew && (
        <>
          <div className="space-y-1.5">
            <Label htmlFor="scan-name">Name</Label>
            <Input id="scan-name" value={edit.name ?? ""} maxLength={MAX_NAME} className="font-mono" onChange={(e) => set({ name: e.target.value })} />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="scan-mode">Type</Label>
              <select id="scan-mode" className={SELECT} value={edit.mode} onChange={(e) => set({ mode: Number(e.target.value), priority: null, voice_prompt: 0 })}>
                <option value={4}>Normal Scan</option>
                <option value={36}>Priority Scan</option>
              </select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="scan-prompt">Voice prompt</Label>
              <select id="scan-prompt" className={SELECT} value={edit.voice_prompt} onChange={(e) => set({ voice_prompt: Number(e.target.value) })}>
                <option value={0}>Off</option>
                <option value={edit.mode === 36 ? 4 : 3}>Built in</option>
              </select>
            </div>
          </div>
        </>
      )}
      <div className="space-y-1.5">
        <Label>
          Channels <span className="font-normal text-muted-foreground">({edit.members.length} of {MAX_MEMBERS}, scanned in this order)</span>
        </Label>
        <ul className="divide-y rounded-md border">
          {edit.members.length === 0 && <li className="px-2 py-1.5 text-xs text-muted-foreground">Add at least two channels below.</li>}
          {edit.members.map((m, i) => {
            const c = byId.get(m);
            const prio = edit.mode === 36 && edit.priority === m;
            return (
              <li key={m} className="flex items-center gap-1.5 px-2 py-1 text-sm">
                {edit.mode === 36 && (
                  <button title={prio ? "Priority channel" : "Make this the priority channel"} onClick={() => set({ priority: m })}>
                    <Star className={cn("size-3.5", prio ? "fill-amber-400 text-amber-400" : "text-muted-foreground/40")} />
                  </button>
                )}
                <span className="min-w-0 flex-1 truncate">
                  <span className="font-mono">{c?.name ?? "?"}</span> <span className="text-xs text-muted-foreground">{c ? `${c.zone} ${c.knob} · ${c.target}` : "not a scannable channel"}</span>
                </span>
                <Button variant="ghost" size="icon-xs" disabled={i === 0} onClick={() => move(i, -1)} title="Move up">
                  <ArrowUp />
                </Button>
                <Button variant="ghost" size="icon-xs" disabled={i === edit.members.length - 1} onClick={() => move(i, 1)} title="Move down">
                  <ArrowDown />
                </Button>
                <Button variant="ghost" size="icon-xs" onClick={() => setMembers(edit.members.filter((x) => x !== m))} title="Remove from scan">
                  <X />
                </Button>
              </li>
            );
          })}
        </ul>
        {edit.mode === 36 && <p className="text-xs text-muted-foreground">The starred channel is the priority channel; the pager checks it while scanning the others.</p>}
      </div>
      <div className="space-y-1.5">
        <Label htmlFor="scan-add">Add a channel</Label>
        <select
          id="scan-add"
          className={SELECT}
          value=""
          disabled={edit.members.length >= MAX_MEMBERS || available.length === 0}
          onChange={(e) => e.target.value && setMembers([...edit.members, e.target.value])}
        >
          <option value="">{available.length ? "Choose an analog conventional knob…" : "No more analog conventional knobs"}</option>
          {available.map((c) => (
            <option key={c.channel_id} value={c.channel_id}>
              {c.zone} {c.knob} · {c.name} · {c.target}
            </option>
          ))}
        </select>
        <p className="text-xs text-muted-foreground">Scans can list analog conventional knobs from any zone; trunked and digital channels can't be scanned this way.</p>
      </div>
      {problems.length > 0 && (
        <ul className="space-y-0.5 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
          {problems.map((p) => (
            <li key={p}>{p}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
