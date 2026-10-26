import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { ArrowDown, ArrowUp, Copy, Download, Plus, RotateCcw, Trash2, Undo2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { hex, mhz } from "@/lib/format";
import { knobKey, useStore } from "@/lib/store";
import type { KnobSystemEdit, P25System, SystemEdit, TalkgroupEdit, Zone } from "@/lib/types";
import { HexInput, MhzInput } from "@/components/hex-input";
import { TalkgroupTable, applyBulk, type TgRow } from "@/components/talkgroup-table";
import { TalkgroupDialog } from "@/components/talkgroup-dialog";
import { RrImportDialog } from "@/components/rr-import-dialog";
import { CopySystemDialog } from "@/components/copy-system-dialog";
import { EDITOR_DEFAULTS, sameSettings } from "@/lib/tg-settings";

const MAX_NAME = 14;

function nameError(what: string, n: string): string | null {
  if (!n.trim()) return `${what} name can't be blank`;
  if (n.length > MAX_NAME) return `${what} name is at most ${MAX_NAME} characters`;
  if (!/^[\x20-\x7E]+$/.test(n)) return `${what} name: printable characters only`;
  return null;
}

/** Client-side checks mirroring the backend; the backend has the final say. */
function problems(d: SystemEdit, otherNames: string[], bands: [number, number][]): string[] {
  const out: string[] = [];
  const e = nameError("System", d.name);
  if (e) out.push(e);
  if (otherNames.includes(d.name.trim())) out.push(`Another system is already called ${d.name}`);
  if (d.sites.length === 0) out.push("A system needs at least one site");
  const names = new Set<string>();
  const pairs = new Set<string>();
  for (const s of d.sites) {
    const ne = nameError("Site", s.name);
    if (ne) out.push(ne);
    if (names.has(s.name)) out.push(`Two sites are called ${s.name}`);
    names.add(s.name);
    const p = `${s.rfss}/${s.site}`;
    if (pairs.has(p)) out.push(`Two sites use RFSS ${hex(s.rfss, 2)} / Site ${hex(s.site, 2)}`);
    pairs.add(p);
  }
  if (d.control_channels.length === 0) out.push("A system needs at least one control channel");
  const fs = new Set<number>();
  for (const c of d.control_channels) {
    if (bands.length && !bands.some(([lo, hi]) => c.rx_hz >= lo && c.rx_hz <= hi)) out.push(`${mhz(c.rx_hz)} MHz is outside this pager's bands`);
    if (fs.has(c.rx_hz)) out.push(`${mhz(c.rx_hz)} MHz is listed twice`);
    fs.add(c.rx_hz);
  }
  return [...new Set(out)];
}

function SystemEditor({ systemId, onDeleted }: { systemId: string; onDeleted: () => void }) {
  const { profile, systems, bands, profileBands, knobSystems, removedKnobs, loadSystemDraft, updateSystem, revertSystem, deleteTalkgroup, systemEdited, deleteSystem } = useStore();
  const entry = systems[systemId];
  const [dialog, setDialog] = useState<{ tg: TalkgroupEdit; isNew: boolean } | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  useEffect(() => {
    loadSystemDraft(systemId);
  }, [systemId, loadSystemDraft]);

  const used = useMemo(() => {
    const m = new Map<string, string[]>();
    for (const z of profile?.summary.zones ?? [])
      for (const k of z.knobs) if (k.system_id === systemId) for (const t of k.talkgroup_list) m.set(t.group_id, [...(m.get(t.group_id) ?? []), `${z.name} ${k.index}`]);
    return m;
  }, [profile, systemId]);

  const rows: TgRow[] = useMemo(() => {
    if (!entry) return [];
    const orig = new Map(entry.original.talkgroups.map((t) => [t.group_id, t]));
    return entry.draft.talkgroups.map((t) => {
      const o = orig.get(t.group_id);
      const status = !o ? "new" : o.tgid !== t.tgid || o.name !== t.name || o.note !== t.note || !sameSettings(o, t) ? "changed" : undefined;
      return { ...t, knobs: used.get(t.group_id) ?? [], status };
    });
  }, [entry, used]);

  if (!profile) return null;
  if (!entry) return <div className="p-6 text-sm text-muted-foreground">Loading…</div>;
  const d = entry.draft;
  const sysBands = bands[systemId] ?? [];
  const others = [
    ...profile.summary.systems.filter((s) => s.id !== systemId && !systems[s.id]).map((s) => s.name.trim()),
    ...Object.values(systems).filter((e) => e.draft.system_id !== systemId).map((e) => e.draft.name.trim()),
  ];
  const isNew = !!entry.band;
  const band = profileBands.find((b) => b.code === entry.band);
  const usedBy = knobsUsing(profile.summary.zones, knobSystems, removedKnobs, systemId);
  const issues = problems(d, others, sysBands);
  const edited = systemEdited(systemId);
  const set = (f: (d: SystemEdit) => SystemEdit) => updateSystem(systemId, f);

  const usedPairs = new Set(d.sites.map((s) => `${s.rfss}/${s.site}`));
  const addSite = () =>
    set((x) => {
      let n = 1;
      while (x.sites.some((s) => s.name === `Site ${n}`)) n++;
      let rfss = 1;
      let site = 1;
      outer: for (rfss = 1; rfss < 256; rfss++) for (site = 1; site < 256; site++) if (!usedPairs.has(`${rfss}/${site}`)) break outer;
      return { ...x, sites: [...x.sites, { id: null, name: `Site ${n}`, rfss, site }] };
    });
  const moveSite = (i: number, dir: -1 | 1) =>
    set((x) => {
      const s = [...x.sites];
      [s[i], s[i + dir]] = [s[i + dir], s[i]];
      return { ...x, sites: s };
    });
  const addCc = () =>
    set((x) => {
      const base = sysBands[0]?.[0] ?? 851_000_000;
      let f = base;
      while (x.control_channels.some((c) => c.rx_hz === f)) f += 12_500;
      return { ...x, control_channels: [...x.control_channels, { id: null, rx_hz: f, note: "" }] };
    });

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-5 overflow-auto p-6">
      <div className="flex flex-wrap items-end gap-4">
        <div className="space-y-1.5">
          <Label htmlFor="sys-name">System name</Label>
          <Input id="sys-name" value={d.name} maxLength={MAX_NAME} className="w-48 font-mono" onChange={(e) => set((x) => ({ ...x, name: e.target.value }))} />
        </div>
        <div className="space-y-1.5">
          <Label>WACN (hex)</Label>
          <HexInput aria-label="WACN" value={d.wacn} max={0xfffff} digits={5} className="w-24" onChange={(v) => set((x) => ({ ...x, wacn: v }))} />
        </div>
        <div className="space-y-1.5">
          <Label>System ID (hex)</Label>
          <HexInput aria-label="System ID" value={d.sysid} max={0xfff} digits={3} className="w-20" onChange={(v) => set((x) => ({ ...x, sysid: v }))} />
        </div>
        <div className="ml-auto flex items-center gap-1">
          {edited && !isNew && (
            <Button variant="ghost" size="sm" onClick={() => revertSystem(systemId)}>
              <RotateCcw />
              Revert this system
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            className="text-destructive hover:text-destructive"
            disabled={usedBy.length > 0}
            title={usedBy.length > 0 ? `Used by ${usedBy.join(", ")}. Move those knobs to another system first.` : undefined}
            onClick={() => {
              deleteSystem(systemId);
              onDeleted();
            }}
          >
            <Trash2 />
            {isNew ? "Discard new system" : "Delete system"}
          </Button>
        </div>
      </div>
      {isNew && entry.copyFrom && (
        <p className="-mt-2 text-xs text-muted-foreground">
          Copied from backup {entry.copyFrom.backup} with every site, control channel and talkgroup setting. It's added to the profile on the next write, and reaches the radio once a knob
          uses it.
        </p>
      )}
      {isNew && !entry.copyFrom && (
        <p className="-mt-2 text-xs text-muted-foreground">
          New {band ? `${band.name} ${band.comment}` : ""} system. Enter the WACN, System ID, sites and control channels from RadioReference. It's added to the profile on the next
          write, and reaches the radio once a knob uses it.
        </p>
      )}
      {issues.length > 0 && (
        <ul className="space-y-0.5 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
          {issues.map((i) => (
            <li key={i}>{i}</li>
          ))}
        </ul>
      )}

      <div className="grid grid-cols-1 gap-5 2xl:grid-cols-2">
        <section>
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-xs font-medium text-muted-foreground">
              Sites ({d.sites.length}) · in the order the pager tries them · enter IDs in hex (RadioReference's Hex column)
            </h2>
            <Button variant="ghost" size="xs" onClick={addSite} disabled={d.sites.length >= 256}>
              <Plus />
              Add site
            </Button>
          </div>
          <div className="divide-y rounded-lg border">
            <div className="grid grid-cols-[minmax(9rem,1fr)_6rem_6rem_5.5rem] gap-2 px-2 py-1.5 text-[11px] font-medium text-muted-foreground">
              <span>Name</span>
              <span title="Enter hex; the decimal value is shown in the field">RFSS (hex)</span>
              <span title="Enter hex; the decimal value is shown in the field">Site (hex)</span>
              <span />
            </div>
            {d.sites.map((s, i) => (
              <div key={s.id ?? `new-${i}`} className={cn("grid grid-cols-[minmax(9rem,1fr)_6rem_6rem_5.5rem] items-center gap-2 px-2 py-1", !s.id && "bg-amber-500/10")}>
                <Input aria-label="Site name" value={s.name} maxLength={MAX_NAME} className="h-8 font-mono" onChange={(e) => set((x) => ({ ...x, sites: x.sites.map((y, j) => (j === i ? { ...y, name: e.target.value } : y)) }))} />
                <HexInput aria-label="RFSS ID" showDecimal value={s.rfss} max={255} digits={2} className="h-8" onChange={(v) => set((x) => ({ ...x, sites: x.sites.map((y, j) => (j === i ? { ...y, rfss: v } : y)) }))} />
                <HexInput aria-label="Site ID" showDecimal value={s.site} max={255} digits={2} className="h-8" onChange={(v) => set((x) => ({ ...x, sites: x.sites.map((y, j) => (j === i ? { ...y, site: v } : y)) }))} />
                <div className="flex justify-end">
                  <Button variant="ghost" size="icon-xs" disabled={i === 0} onClick={() => moveSite(i, -1)} title="Move up">
                    <ArrowUp />
                  </Button>
                  <Button variant="ghost" size="icon-xs" disabled={i === d.sites.length - 1} onClick={() => moveSite(i, 1)} title="Move down">
                    <ArrowDown />
                  </Button>
                  <Button variant="ghost" size="icon-xs" disabled={d.sites.length === 1} onClick={() => set((x) => ({ ...x, sites: x.sites.filter((_, j) => j !== i) }))} title="Remove site">
                    <Trash2 />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section>
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-xs font-medium text-muted-foreground">
              Control channels ({d.control_channels.length})
              {sysBands.length > 0 && ` · allowed ${sysBands.map(([lo, hi]) => `${lo / 1e6}–${hi / 1e6}`).join(", ")} MHz`}
            </h2>
            <Button variant="ghost" size="xs" onClick={addCc} disabled={d.control_channels.length >= 256}>
              <Plus />
              Add channel
            </Button>
          </div>
          <div className="divide-y rounded-lg border">
            {d.control_channels.map((c, i) => {
              const bad = sysBands.length > 0 && !sysBands.some(([lo, hi]) => c.rx_hz >= lo && c.rx_hz <= hi);
              return (
                <div key={c.id ?? `new-${i}`} className={cn("grid grid-cols-[8.5rem_1fr_2rem] items-center gap-2 px-2 py-1", c.id === null && "bg-amber-500/10")}>
                  <MhzInput aria-label="Control channel MHz" hz={c.rx_hz} invalid={bad} className="h-8" onChange={(hz) => set((x) => ({ ...x, control_channels: x.control_channels.map((y, j) => (j === i ? { ...y, rx_hz: hz } : y)) }))} />
                  <Input value={c.note} maxLength={64} placeholder="Note" className="h-8" onChange={(e) => set((x) => ({ ...x, control_channels: x.control_channels.map((y, j) => (j === i ? { ...y, note: e.target.value } : y)) }))} />
                  <Button variant="ghost" size="icon-xs" onClick={() => set((x) => ({ ...x, control_channels: x.control_channels.filter((_, j) => j !== i) }))} title="Remove channel">
                    <Trash2 />
                  </Button>
                </div>
              );
            })}
          </div>
        </section>
      </div>

      <section className="flex min-h-96 flex-1 flex-col">
        <h2 className="mb-2 text-xs font-medium text-muted-foreground">
          Talkgroups · click one to edit or delete it · tick several (shift-click for a range, or search and tick the header) to set alerts, tone, storage or colour at once
        </h2>
        <TalkgroupTable
          key={systemId}
          data={rows}
          onRowClick={({ knobs: _knobs, status: _status, ...tg }) => setDialog({ tg, isNew: false })}
          selected={selected}
          onSelectedChange={setSelected}
          onBulkApply={(patch) => {
            let kept = 0;
            set((x) => {
              const r = applyBulk(x.talkgroups, selected, patch);
              kept = r.keptCustom;
              return { ...x, talkgroups: r.talkgroups };
            });
            if (kept > 0) toast(`Kept the custom tone on ${kept} talkgroup${kept === 1 ? "" : "s"}`, { description: "Knobset doesn't change custom tones; set them in PPS." });
          }}
          actions={
            <Button size="sm" variant="outline" onClick={() => setDialog({ tg: { group_id: crypto.randomUUID().toLowerCase(), tgid: NaN, name: "", note: "", ...EDITOR_DEFAULTS }, isNew: true })}>
              <Plus />
              Add talkgroup
            </Button>
          }
        />
      </section>

      {dialog && (
        <TalkgroupDialog
          open
          onOpenChange={(o) => !o && setDialog(null)}
          initial={dialog.tg}
          isNew={dialog.isNew}
          all={d.talkgroups}
          usedOn={used.get(dialog.tg.group_id) ?? []}
          onSave={(t) =>
            set((x) => ({ ...x, talkgroups: dialog.isNew ? [...x.talkgroups, t] : x.talkgroups.map((y) => (y.group_id === t.group_id ? t : y)) }))
          }
          onDelete={
            dialog.isNew
              ? undefined
              : () => {
                  deleteTalkgroup(systemId, dialog.tg.group_id);
                  setSelected((s) => new Set([...s].filter((g) => g !== dialog.tg.group_id)));
                }
          }
        />
      )}
    </div>
  );
}

/** Knobs on `systemId`, counting unsaved moves and new knobs. */
function knobsUsing(zones: Zone[], knobSystems: Record<string, KnobSystemEdit>, removed: Record<string, unknown>, systemId: string): string[] {
  const out: string[] = [];
  for (const z of zones) {
    for (const k of z.knobs) {
      const key = knobKey(z.name, k.index);
      // A knob pending removal frees its system; the write removes knobs before deleting systems.
      const system = knobSystems[key]?.system_id ?? (key in removed ? undefined : k.system_id);
      if (system === systemId) out.push(`${z.name} knob ${k.index}`);
    }
    for (const i of z.empty_knobs) if (knobSystems[knobKey(z.name, i)]?.system_id === systemId) out.push(`${z.name} knob ${i} (new)`);
  }
  return out;
}

interface ListItem {
  id: string;
  name: string;
  wacn: number;
  sysid: number;
  sites: number;
  tgs: number;
  inUse: boolean;
  isNew: boolean;
  deleted: boolean;
  edited: boolean;
}

function SystemList({ items, selected, onSelect }: { items: ListItem[]; selected: string | undefined; onSelect: (id: string) => void }) {
  const profileBands = useStore((s) => s.profileBands);
  const addSystem = useStore((s) => s.addSystem);
  const [importing, setImporting] = useState(false);
  const [copying, setCopying] = useState(false);
  return (
    <nav className="flex w-64 shrink-0 flex-col border-r">
      <div className="flex-1 space-y-1 overflow-auto p-3">
        {items.map((s) => (
          <button key={s.id} onClick={() => onSelect(s.id)} className={cn("w-full rounded-lg px-3 py-2 text-left transition-colors hover:bg-muted", s.id === selected && "bg-muted")}>
            <div className={cn("flex items-center gap-1.5 truncate text-sm font-medium", !s.inUse && "text-muted-foreground", s.deleted && "line-through")}>
              <span className="truncate">{s.name}</span>
              {s.isNew && <span className="shrink-0 rounded bg-emerald-500/15 px-1 text-[10px] font-medium text-emerald-700 no-underline dark:text-emerald-300">NEW</span>}
              {s.edited && !s.isNew && <span className="size-2 shrink-0 rounded-full bg-amber-500" title="Unsaved changes" />}
            </div>
            <div className={cn("font-mono text-[11px] text-muted-foreground", s.deleted && "line-through")}>
              {hex(s.wacn, 5)}.{hex(s.sysid, 3)} · {s.sites} site{s.sites !== 1 && "s"} · {s.tgs} TGs
            </div>
            {s.deleted ? (
              <div className="text-[11px] text-destructive italic">Deleted on the next write</div>
            ) : (
              !s.inUse && <div className="text-[11px] text-muted-foreground italic">Not on any knob · changes don't reach the radio</div>
            )}
          </button>
        ))}
      </div>
      <div className="space-y-1 border-t p-3">
        {profileBands.map((b) => (
          <Button
            key={b.code}
            variant="outline"
            size="sm"
            className="w-full justify-start"
            title={`${b.name} ${b.comment}`}
            disabled={items.length >= 1024}
            onClick={() => onSelect(addSystem(b.code))}
          >
            <Plus />
            New {profileBands.length > 1 ? `${b.name} ` : ""}system
          </Button>
        ))}
        <Button variant="outline" size="sm" className="w-full justify-start" disabled={profileBands.length === 0} onClick={() => setImporting(true)}>
          <Download />
          Import from RadioReference…
        </Button>
        <RrImportDialog open={importing} onOpenChange={setImporting} onImported={onSelect} />
        <Button variant="outline" size="sm" className="w-full justify-start" disabled={profileBands.length === 0 || items.length >= 1024} onClick={() => setCopying(true)}>
          <Copy />
          Copy from a backup…
        </Button>
        <CopySystemDialog open={copying} onOpenChange={setCopying} onCopied={onSelect} />
      </div>
    </nav>
  );
}

function DeletedPanel({ system }: { system: P25System }) {
  const restoreSystem = useStore((s) => s.restoreSystem);
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center text-sm text-muted-foreground">
      <p>
        <span className="font-medium text-foreground">{system.name}</span> will be removed from the profile on the next write.
      </p>
      <Button variant="outline" size="sm" onClick={() => restoreSystem(system.id)}>
        <Undo2 />
        Keep this system
      </Button>
    </div>
  );
}

export default function SystemsPage() {
  const profile = useStore((s) => s.profile);
  const drafts = useStore((s) => s.systems);
  const deleted = useStore((s) => s.deletedSystems);
  const systemEdited = useStore((s) => s.systemEdited);
  const systems = profile?.summary.systems ?? [];
  const items: ListItem[] = [
    ...systems.map((s) => {
      const d = drafts[s.id]?.draft;
      return {
        id: s.id,
        name: d?.name ?? s.name,
        wacn: d?.wacn ?? s.wacn,
        sysid: d?.sysid ?? s.system_id,
        sites: d?.sites.length ?? s.sites.length,
        tgs: d?.talkgroups.length ?? s.talkgroups,
        inUse: s.in_use,
        isNew: false,
        deleted: deleted.includes(s.id),
        edited: systemEdited(s.id),
      };
    }),
    ...Object.values(drafts)
      .filter((e) => e.band)
      .map((e) => ({
        id: e.draft.system_id,
        name: e.draft.name,
        wacn: e.draft.wacn,
        sysid: e.draft.sysid,
        sites: e.draft.sites.length,
        tgs: e.draft.talkgroups.length,
        inUse: false,
        isNew: true,
        deleted: false,
        edited: true,
      })),
  ];
  const [id, setId] = useState<string | null>(items[0]?.id ?? null);
  const sel = items.find((s) => s.id === id) ?? items[0];
  if (!profile) return null;

  return (
    <div className="flex h-full">
      <SystemList items={items} selected={sel?.id} onSelect={setId} />
      {!sel ? (
        <div className="p-8 text-sm text-muted-foreground">This profile has no P25 trunking systems yet. Add one with New system.</div>
      ) : sel.deleted ? (
        <DeletedPanel system={systems.find((s) => s.id === sel.id)!} />
      ) : (
        <SystemEditor key={sel.id} systemId={sel.id} onDeleted={() => setId(sel.isNew ? (items.find((s) => s.id !== sel.id)?.id ?? null) : sel.id)} />
      )}
    </div>
  );
}
