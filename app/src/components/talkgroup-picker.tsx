import { useMemo, useRef, useState } from "react";
import { ArrowDown, ArrowUp, Plus, Search, Star, X } from "lucide-react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { useStore } from "@/lib/store";
import type { Knob, P25System, Talkgroup, Zone } from "@/lib/types";

export const MAX_KNOB_TALKGROUPS = 64;
const ROW_H = 30;

interface Props {
  open: boolean;
  onOpenChange: (o: boolean) => void;
  zone: Zone;
  knob: Knob;
  system: P25System;
}

/** Other knobs (zone + position) that list each talkgroup of `system`. */
function useOtherKnobs(system: P25System, zone: Zone, knob: Knob) {
  const profile = useStore((s) => s.profile);
  return useMemo(() => {
    const m = new Map<string, string[]>();
    for (const z of profile?.summary.zones ?? [])
      for (const k of z.knobs) {
        if (k.system_id !== system.id || (z.name === zone.name && k.index === knob.index)) continue;
        for (const t of k.talkgroup_list) m.set(t.group_id, [...(m.get(t.group_id) ?? []), `${z.name} ${k.index}`]);
      }
    return m;
  }, [profile, system, zone, knob]);
}

export function TalkgroupPicker({ open, onOpenChange, zone, knob, system }: Props) {
  const { talkgroupsFor, setKnobTalkgroups } = useStore();
  const initial = talkgroupsFor(zone.name, knob);
  const [priority, setPriority] = useState<string[]>(initial.priority);
  const [normal, setNormal] = useState<string[]>(initial.normal);
  const [query, setQuery] = useState("");
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const others = useOtherKnobs(system, zone, knob);

  const byId = useMemo(() => new Map(system.talkgroup_list.map((t) => [t.group_id, t])), [system]);
  const onKnob = useMemo(() => new Set([...priority, ...normal]), [priority, normal]);
  const total = priority.length + normal.length;
  const room = MAX_KNOB_TALKGROUPS - total;

  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    const list = [...system.talkgroup_list].sort((a, b) => a.tgid - b.tgid);
    if (!q) return list;
    return list.filter((t) => String(t.tgid).includes(q) || t.name.toLowerCase().includes(q) || t.note.toLowerCase().includes(q));
  }, [system, query]);
  // The "All" placeholder (65535) is never added in bulk; check it to add it.
  const addable = matches.filter((t) => !onKnob.has(t.group_id) && t.tgid !== 65535);
  // Without a search, only when everything left fits: adding the first 64 of a big system by accident helps nobody.
  const canAddAll = addable.length > 0 && room > 0 && (!!query.trim() || addable.length <= room);

  const scrollRef = useRef<HTMLDivElement>(null);
  const virtual = useVirtualizer({ count: matches.length, getScrollElement: () => scrollRef.current, estimateSize: () => ROW_H, overscan: 12 });

  // TGIDs must be unique on a knob; the pager can't tell two entries for the same ID apart.
  const tgidTaken = (t: Talkgroup) => [...onKnob].some((g) => byId.get(g)?.tgid === t.tgid);

  function add(ids: string[]) {
    const fresh = ids.filter((id) => !onKnob.has(id) && !tgidTaken(byId.get(id)!)).slice(0, room);
    setNormal([...normal, ...fresh]);
    setChecked(new Set());
  }

  const remove = (id: string) => {
    setPriority(priority.filter((x) => x !== id));
    setNormal(normal.filter((x) => x !== id));
  };
  const togglePriority = (id: string) => {
    if (priority.includes(id)) {
      setPriority(priority.filter((x) => x !== id));
      setNormal([id, ...normal]);
    } else {
      setNormal(normal.filter((x) => x !== id));
      setPriority([...priority, id]);
    }
  };
  const move = (id: string, dir: -1 | 1) => {
    const i = priority.indexOf(id);
    const j = i + dir;
    if (i < 0 || j < 0 || j >= priority.length) return;
    const next = [...priority];
    [next[i], next[j]] = [next[j], next[i]];
    setPriority(next);
  };

  function apply() {
    setKnobTalkgroups(zone.name, knob, priority, normal);
    onOpenChange(false);
  }

  const Row = ({ id, pri, index }: { id: string; pri: boolean; index: number }) => {
    const t = byId.get(id);
    return (
      <li className="group flex items-center gap-2 px-2 py-1 text-sm">
        <button onClick={() => togglePriority(id)} title={pri ? "Remove from priority list" : "Make priority"} className="shrink-0">
          <Star className={cn("size-3.5", pri ? "fill-amber-400 text-amber-400" : "text-muted-foreground/40 hover:text-amber-400")} />
        </button>
        <span className="w-12 shrink-0 font-mono text-xs tabular-nums text-muted-foreground">{t?.tgid}</span>
        <span className="truncate font-mono text-[13px]">{t?.name ?? id}</span>
        <span className="ml-auto flex shrink-0 items-center opacity-0 group-hover:opacity-100">
          {pri && (
            <>
              <Button variant="ghost" size="icon-xs" disabled={index === 0} onClick={() => move(id, -1)} title="Move up">
                <ArrowUp />
              </Button>
              <Button variant="ghost" size="icon-xs" disabled={index === priority.length - 1} onClick={() => move(id, 1)} title="Move down">
                <ArrowDown />
              </Button>
            </>
          )}
          <Button variant="ghost" size="icon-xs" onClick={() => remove(id)} title="Remove">
            <X />
          </Button>
        </span>
      </li>
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex h-[82vh] flex-col sm:max-w-5xl">
        <DialogHeader>
          <DialogTitle>
            Talkgroups on {zone.name} · {knob.index} <span className="font-mono">{knob.name}</span>
          </DialogTitle>
          <DialogDescription>
            {system.name}. Starred talkgroups are on the priority list and are checked first, in order. A knob holds up to {MAX_KNOB_TALKGROUPS}.
          </DialogDescription>
        </DialogHeader>

        <div className="grid min-h-0 flex-1 grid-cols-[1.4fr_1fr] gap-4">
          <section className="flex min-h-0 flex-col rounded-lg border">
            <div className="flex items-center gap-2 border-b p-2">
              <div className="relative flex-1">
                <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
                <Input autoFocus value={query} onChange={(e) => setQuery(e.target.value)} placeholder={`Search ${system.talkgroup_list.length} talkgroups`} className="h-8 pl-8" />
              </div>
              <Button size="sm" variant="outline" disabled={checked.size === 0 || room === 0} onClick={() => add([...checked])}>
                <Plus />
                Add {checked.size || ""}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={!canAddAll}
                onClick={() => add(addable.map((t) => t.group_id))}
                title={!canAddAll && room > 0 && addable.length > room ? `More than ${room} talkgroups; search to narrow the list` : undefined}
              >
                Add all {canAddAll ? Math.min(addable.length, room) : ""}
              </Button>
            </div>
            <div ref={scrollRef} className="min-h-0 flex-1 overflow-auto">
              <div style={{ height: virtual.getTotalSize(), position: "relative" }}>
                {virtual.getVirtualItems().map((v) => {
                  const t = matches[v.index];
                  const on = onKnob.has(t.group_id);
                  const clash = !on && tgidTaken(t);
                  const elsewhere = others.get(t.group_id);
                  return (
                    <label
                      key={t.group_id}
                      className={cn("absolute inset-x-0 flex cursor-pointer items-center gap-2 border-b px-2 text-sm hover:bg-muted/50", (on || clash) && "cursor-default text-muted-foreground")}
                      style={{ top: v.start, height: ROW_H }}
                      title={clash ? "Another talkgroup with this TGID is already on the knob" : t.note}
                    >
                      <input
                        type="checkbox"
                        className="accent-primary"
                        disabled={on || clash}
                        checked={on || checked.has(t.group_id)}
                        onChange={(e) => {
                          const next = new Set(checked);
                          if (e.target.checked) next.add(t.group_id);
                          else next.delete(t.group_id);
                          setChecked(next);
                        }}
                      />
                      <span className="w-12 shrink-0 font-mono text-xs tabular-nums">{t.tgid}</span>
                      <span className="w-32 shrink-0 truncate font-mono text-[13px]">{t.name}</span>
                      <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">{t.note}</span>
                      {elsewhere && <span className="shrink-0 rounded bg-sky-500/10 px-1.5 text-[11px] text-sky-700 dark:text-sky-300">{elsewhere.length > 1 ? `${elsewhere.length} knobs` : elsewhere[0]}</span>}
                    </label>
                  );
                })}
              </div>
              {matches.length === 0 && <div className="p-6 text-center text-sm text-muted-foreground">No talkgroups match.</div>}
            </div>
          </section>

          <section className="flex min-h-0 flex-col rounded-lg border">
            <div className="flex items-center justify-between border-b px-3 py-2.5 text-sm font-medium">
              On this knob
              <span className={cn("tabular-nums text-xs", total > MAX_KNOB_TALKGROUPS - 5 ? "text-amber-600 dark:text-amber-400" : "text-muted-foreground")}>
                {total}/{MAX_KNOB_TALKGROUPS}
              </span>
            </div>
            <div className="min-h-0 flex-1 overflow-auto">
              <div className="px-3 pt-2 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">Priority ({priority.length})</div>
              {priority.length === 0 ? (
                <p className="px-3 py-1.5 text-xs text-muted-foreground">Star a talkgroup to check it first.</p>
              ) : (
                <ul>{priority.map((id, i) => <Row key={id} id={id} pri index={i} />)}</ul>
              )}
              <div className="px-3 pt-3 text-[11px] font-medium tracking-wide text-muted-foreground uppercase">Others ({normal.length})</div>
              <ul>{normal.map((id, i) => <Row key={id} id={id} pri={false} index={i} />)}</ul>
              {total === 0 && <p className="px-3 py-1.5 text-xs text-destructive">A trunked knob needs at least one talkgroup.</p>}
            </div>
          </section>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button disabled={total === 0} onClick={apply}>
            Apply
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
