import { useMemo, useRef, useState } from "react";
import { ArrowDown, ArrowUp, Bell, BellOff, Mic, Search, Vibrate, X } from "lucide-react";
import {
  columnFilteringFeature,
  createColumnHelper,
  createFilteredRowModel,
  createSortedRowModel,
  filterFn_includesString,
  globalFilteringFeature,
  rowSortingFeature,
  sortFn_alphanumeric,
  tableFeatures,
  useTable,
  type SortingState,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { hex } from "@/lib/format";
import type { TalkgroupEdit } from "@/lib/types";
import { ALERT_MODES, ALERT_TONES, BACKGROUNDS, alertShort, isBuiltinTone, settingsOf, toneAudible, toneLabel, type TgSettings } from "@/lib/tg-settings";
import { SELECT, Swatch } from "@/components/talkgroup-dialog";

export type TgRow = TalkgroupEdit & { knobs: string[]; status?: "new" | "changed" };

const features = tableFeatures({
  columnFilteringFeature,
  globalFilteringFeature,
  rowSortingFeature,
  filteredRowModel: createFilteredRowModel(),
  sortedRowModel: createSortedRowModel(),
  filterFns: { includesString: filterFn_includesString },
  sortFns: { alphanumeric: sortFn_alphanumeric },
});
const col = createColumnHelper<typeof features, TgRow>();
const columns = col.columns([
  col.accessor("tgid", { header: "TGID", cell: (c) => c.getValue() }),
  col.accessor((r) => hex(r.tgid, 4), { id: "hex", header: "Hex" }),
  col.accessor("name", { header: "Alias" }),
  col.accessor((r) => alertShort(settingsOf(r).alert_mode), { id: "alert", header: "Alert" }),
  col.accessor((r) => (settingsOf(r).voice_storage ? "Stored" : ""), { id: "storage", header: "Storage" }),
  col.accessor("note", { header: "Description" }),
  col.accessor((r) => r.knobs.join(", "), { id: "knobs", header: "On knobs" }),
]);
const WIDTHS = "grid-cols-[1rem_5rem_4.5rem_10rem_8.5rem_4rem_minmax(0,1fr)_minmax(0,12rem)]";
const ROW_H = 32;

function AlertCell({ s }: { s: TgSettings }) {
  const Icon = s.alert_mode === 3 ? BellOff : s.alert_mode === 2 ? Vibrate : Bell;
  const tone = toneAudible(s.alert_mode) ? toneLabel(s.alert_tone) : "";
  return (
    <span className={cn("flex min-w-0 items-center gap-1 text-xs", s.alert_mode === 3 && "text-muted-foreground")} title={tone ? `${alertShort(s.alert_mode)} · ${tone}` : alertShort(s.alert_mode)}>
      <Icon className="size-3.5 shrink-0" />
      <span className="truncate">{alertShort(s.alert_mode)}</span>
      {tone && tone !== "Default tone" && <span className="truncate text-muted-foreground">· {tone.replace("Tone ", "T")}</span>}
    </span>
  );
}

interface Props {
  data: TgRow[];
  onRowClick?: (row: TgRow) => void;
  actions?: React.ReactNode;
  /** Selected GroupIDs for bulk edit; omit to hide selection. */
  selected?: Set<string>;
  onSelectedChange?: (s: Set<string>) => void;
  onBulkApply?: (patch: Partial<TgSettings>) => void;
}

export function TalkgroupTable({ data, onRowClick, actions, selected, onSelectedChange, onBulkApply }: Props) {
  const [globalFilter, setGlobalFilter] = useState("");
  const [sorting, setSorting] = useState<SortingState>([{ id: "tgid", desc: false }]);
  const [anchor, setAnchor] = useState<number | null>(null);
  const table = useTable({
    features,
    columns,
    data,
    state: { globalFilter, sorting },
    onGlobalFilterChange: setGlobalFilter,
    onSortingChange: setSorting,
    globalFilterFn: "includesString",
  });
  const rows = table.getRowModel().rows;
  const scrollRef = useRef<HTMLDivElement>(null);
  const virtual = useVirtualizer({ count: rows.length, getScrollElement: () => scrollRef.current, estimateSize: () => ROW_H, overscan: 12 });
  const onKnobs = useMemo(() => data.filter((d) => d.knobs.length > 0).length, [data]);
  const selectable = !!selected && !!onSelectedChange;
  const shown = rows.map((r) => r.original.group_id);
  const shownSelected = selected ? shown.filter((g) => selected.has(g)).length : 0;
  const allShown = shown.length > 0 && shownSelected === shown.length;

  const toggle = (i: number, shift: boolean) => {
    if (!selected || !onSelectedChange) return;
    const next = new Set(selected);
    const g = shown[i];
    const on = !selected.has(g);
    if (shift && anchor !== null) {
      for (let j = Math.min(anchor, i); j <= Math.max(anchor, i); j++) on ? next.add(shown[j]) : next.delete(shown[j]);
    } else on ? next.add(g) : next.delete(g);
    setAnchor(i);
    onSelectedChange(next);
  };
  const toggleShown = () => {
    if (!selected || !onSelectedChange) return;
    const next = new Set(selected);
    for (const g of shown) allShown ? next.delete(g) : next.add(g);
    onSelectedChange(next);
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col rounded-lg border">
      <div className="flex items-center gap-3 border-b p-2">
        <div className="relative w-72">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input value={globalFilter} onChange={(e) => setGlobalFilter(e.target.value)} placeholder="Search TGID, alias, alert or description" className="h-8 pl-8" />
        </div>
        <span className="text-xs text-muted-foreground">
          {rows.length === data.length ? `${data.length} talkgroups` : `${rows.length} of ${data.length}`} · {onKnobs} on knobs
        </span>
        <div className="ml-auto">{actions}</div>
      </div>
      {selectable && selected.size > 0 && onBulkApply && (
        <BulkBar count={selected.size} hidden={selected.size - shownSelected} onApply={onBulkApply} onClear={() => onSelectedChange(new Set())} />
      )}
      <div className={cn("grid items-center gap-3 border-b bg-muted/40 px-3 py-1.5 text-xs font-medium text-muted-foreground", WIDTHS)}>
        {selectable ? (
          <input
            type="checkbox"
            aria-label={allShown ? "Unselect the talkgroups shown" : "Select the talkgroups shown"}
            title={allShown ? "Unselect the talkgroups shown" : `Select the ${shown.length} talkgroups shown`}
            className="accent-primary"
            checked={allShown}
            ref={(el) => {
              if (el) el.indeterminate = shownSelected > 0 && !allShown;
            }}
            onChange={toggleShown}
          />
        ) : (
          <span />
        )}
        {table.getHeaderGroups()[0].headers.map((h) => (
          <button key={h.id} className="flex min-w-0 items-center gap-1 text-left" onClick={h.column.getToggleSortingHandler()}>
            <span className="truncate">
              <table.FlexRender header={h} />
            </span>
            {h.column.getIsSorted() === "asc" && <ArrowUp className="size-3" />}
            {h.column.getIsSorted() === "desc" && <ArrowDown className="size-3" />}
          </button>
        ))}
      </div>
      <div ref={scrollRef} className="min-h-0 flex-1 overflow-auto">
        <div style={{ height: virtual.getTotalSize(), position: "relative" }}>
          {virtual.getVirtualItems().map((v) => {
            const row = rows[v.index];
            const t = row.original;
            const s = settingsOf(t);
            const isSel = selected?.has(t.group_id) ?? false;
            return (
              <div
                key={row.id}
                role={onRowClick ? "button" : undefined}
                tabIndex={onRowClick ? 0 : undefined}
                onClick={onRowClick ? () => onRowClick(t) : undefined}
                onKeyDown={onRowClick ? (e) => e.key === "Enter" && onRowClick(t) : undefined}
                className={cn(
                  "absolute inset-x-0 grid items-center gap-3 border-b px-3 text-sm",
                  WIDTHS,
                  t.knobs.length > 0 && "bg-sky-500/5",
                  t.status && "bg-amber-500/10",
                  isSel && "bg-primary/10",
                  onRowClick && "cursor-pointer hover:bg-muted/60",
                )}
                style={{ top: v.start, height: ROW_H }}
              >
                {selectable ? (
                  <input
                    type="checkbox"
                    aria-label={`Select ${t.name}`}
                    className="accent-primary"
                    checked={isSel}
                    onClick={(e) => {
                      e.stopPropagation();
                      toggle(v.index, e.shiftKey);
                    }}
                    onKeyDown={(e) => e.stopPropagation()}
                    onChange={() => {}}
                  />
                ) : (
                  <span />
                )}
                <span className="font-mono text-xs tabular-nums">{t.tgid}</span>
                <span className="font-mono text-xs tabular-nums text-muted-foreground">{hex(t.tgid, 4)}</span>
                <span className="flex min-w-0 items-center gap-1.5">
                  <span className="truncate font-mono text-[13px]">{t.name}</span>
                  {s.background !== 0 && <Swatch value={s.background} className="size-2.5 shrink-0 rounded-full border" />}
                </span>
                <AlertCell s={s} />
                <span className="text-xs">{s.voice_storage && <Mic className="size-3.5 text-muted-foreground" aria-label="Voice storage on" />}</span>
                <span className="truncate text-xs text-muted-foreground">{t.note}</span>
                <span className="truncate text-xs">{t.knobs.join(", ")}</span>
              </div>
            );
          })}
        </div>
        {rows.length === 0 && <div className="p-6 text-center text-sm text-muted-foreground">No talkgroups match.</div>}
      </div>
    </div>
  );
}

const KEEP = "";

function BulkBar({ count, hidden, onApply, onClear }: { count: number; hidden: number; onApply: (p: Partial<TgSettings>) => void; onClear: () => void }) {
  const [mode, setMode] = useState(KEEP);
  const [tone, setTone] = useState(KEEP);
  const [storage, setStorage] = useState(KEEP);
  const [bg, setBg] = useState(KEEP);
  const any = mode !== KEEP || tone !== KEEP || storage !== KEEP || bg !== KEEP;
  const cls = cn(SELECT, "h-7 w-auto text-xs");
  return (
    <div className="flex flex-wrap items-center gap-2 border-b bg-primary/5 px-3 py-1.5 text-xs">
      <span className="font-medium">
        {count} selected{hidden > 0 && <span className="font-normal text-muted-foreground"> ({hidden} not shown)</span>}
      </span>
      <select aria-label="Set alert for selected" className={cls} value={mode} onChange={(e) => setMode(e.target.value)}>
        <option value={KEEP}>Alert: keep</option>
        {ALERT_MODES.map((m) => (
          <option key={m.value} value={m.value}>
            Alert: {m.label}
          </option>
        ))}
      </select>
      <select aria-label="Set alert tone for selected" className={cls} value={tone} onChange={(e) => setTone(e.target.value)}>
        <option value={KEEP}>Tone: keep</option>
        {ALERT_TONES.map((x) => (
          <option key={x.value} value={x.value}>
            Tone: {x.label}
          </option>
        ))}
      </select>
      <select aria-label="Set voice storage for selected" className={cls} value={storage} onChange={(e) => setStorage(e.target.value)}>
        <option value={KEEP}>Storage: keep</option>
        <option value="on">Storage: on</option>
        <option value="off">Storage: off</option>
      </select>
      <select aria-label="Set background for selected" className={cls} value={bg} onChange={(e) => setBg(e.target.value)}>
        <option value={KEEP}>Colour: keep</option>
        {BACKGROUNDS.map((b) => (
          <option key={b.value} value={b.value}>
            Colour: {b.label}
          </option>
        ))}
      </select>
      <Button
        size="xs"
        disabled={!any}
        onClick={() => {
          const p: Partial<TgSettings> = {};
          if (mode !== KEEP) p.alert_mode = Number(mode);
          if (tone !== KEEP) p.alert_tone = tone;
          if (storage !== KEEP) p.voice_storage = storage === "on";
          if (bg !== KEEP) p.background = Number(bg);
          onApply(p);
          setMode(KEEP);
          setTone(KEEP);
          setStorage(KEEP);
          setBg(KEEP);
        }}
      >
        Apply to {count}
      </Button>
      <Button size="xs" variant="ghost" className="ml-auto" onClick={onClear}>
        <X />
        Clear selection
      </Button>
    </div>
  );
}

/** Apply a bulk patch; custom tones from PPS are never overwritten. Returns the new list and how many custom tones were kept. */
export function applyBulk(tgs: TalkgroupEdit[], ids: Set<string>, patch: Partial<TgSettings>): { talkgroups: TalkgroupEdit[]; keptCustom: number } {
  let keptCustom = 0;
  const talkgroups = tgs.map((t) => {
    if (!ids.has(t.group_id)) return t;
    const next = { ...t, ...patch };
    if (patch.alert_tone !== undefined && !isBuiltinTone(settingsOf(t).alert_tone)) {
      next.alert_tone = t.alert_tone;
      keptCustom++;
    }
    return next;
  });
  return { talkgroups, keptCustom };
}
