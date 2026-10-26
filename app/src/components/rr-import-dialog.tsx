import { useEffect, useMemo, useRef, useState } from "react";
import { ChevronRight, FileUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { HexInput } from "@/components/hex-input";
import { api } from "@/lib/api";
import { hex, mhz } from "@/lib/format";
import { useStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import type { RrPlan, RrProblem, RrSites, RrTalkgroup, RrTalkgroups } from "@/lib/types";

const SELECT = "h-8 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50";
const MAX_NAME = 14;
const NO_SITES: RrSites = { sites: [], marked: false, problems: [] };
const NO_TGS: RrTalkgroups = { talkgroups: [], problems: [] };
const NO_RANGES: [number, number][] = [];

/** Ref setting a checkbox's indeterminate state (no HTML attribute for it). */
const indeterminate = (v: boolean) => (el: HTMLInputElement | null) => {
  if (el) el.indeterminate = v;
};

/** "2 sites", "1 control channel"; zero counts left out, joined as a list. */
function counts(parts: [number, string][]): string {
  const xs = parts.filter(([n]) => n > 0).map(([n, w]) => `${n} ${w}${n !== 1 ? "s" : ""}`);
  return xs.length <= 1 ? (xs[0] ?? "") : `${xs.slice(0, -1).join(", ")} and ${xs[xs.length - 1]}`;
}

type Parsed<T> = { text: string; value?: T; error?: string };

/** One table: paste it or choose the downloaded CSV. */
function Source<T extends { problems: RrProblem[] }>({
  label,
  hint,
  state,
  summary,
  onText,
}: {
  label: string;
  hint: string;
  state: Parsed<T>;
  summary: (v: T) => string;
  onText: (text: string) => void;
}) {
  const file = useRef<HTMLInputElement>(null);
  const [showProblems, setShowProblems] = useState(false);
  const problems = state.value?.problems ?? [];
  return (
    <div className="flex min-w-0 flex-1 flex-col gap-1.5">
      <div className="flex items-center justify-between">
        <Label>{label}</Label>
        <Button variant="ghost" size="xs" onClick={() => file.current?.click()}>
          <FileUp />
          Choose file…
        </Button>
        <input
          ref={file}
          type="file"
          accept=".csv,.txt,text/csv,text/plain"
          className="hidden"
          onChange={async (e) => {
            const f = e.target.files?.[0];
            if (f) onText(await f.text());
            e.target.value = "";
          }}
        />
      </div>
      <textarea
        value={state.text}
        onChange={(e) => onText(e.target.value)}
        placeholder={hint}
        spellCheck={false}
        className="h-24 resize-none rounded-md border border-input bg-transparent px-2 py-1.5 font-mono text-[11px] leading-snug shadow-xs outline-none placeholder:font-sans placeholder:text-xs focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
      />
      <div className="min-h-4 text-xs">
        {state.error ? (
          <span className="text-destructive">{state.error}</span>
        ) : state.value ? (
          <span className="text-muted-foreground">
            {summary(state.value)}
            {problems.length > 0 && (
              <>
                {" · "}
                <button className="text-amber-700 underline-offset-2 hover:underline dark:text-amber-400" onClick={() => setShowProblems(!showProblems)}>
                  {problems.length} row{problems.length !== 1 && "s"} skipped or changed
                </button>
              </>
            )}
          </span>
        ) : null}
      </div>
      {showProblems && problems.length > 0 && (
        <ul className="max-h-24 overflow-auto rounded-md bg-muted/50 p-2 text-[11px] text-muted-foreground">
          {problems.map((p, i) => (
            <li key={i}>
              Line {p.line}: {p.message}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function useParsed<T>(parse: (text: string) => Promise<T>): [Parsed<T>, (text: string) => void] {
  const [state, setState] = useState<Parsed<T>>({ text: "" });
  const seq = useRef(0);
  const onText = (text: string) => {
    const n = ++seq.current;
    setState({ text });
    if (!text.trim()) return;
    parse(text).then(
      (value) => n === seq.current && setState({ text, value }),
      (e) => n === seq.current && setState({ text, error: String(e) })
    );
  };
  return [state, onText];
}

const ENC_LABEL: Record<string, string> = { full: "Encrypted", partial: "Partly encrypted" };

function TalkgroupChooser({ tgs, chosen, setChosen }: { tgs: RrTalkgroup[]; chosen: Set<number>; setChosen: (s: Set<number>) => void }) {
  const [open, setOpen] = useState<Set<string>>(new Set());
  const groups = useMemo(() => {
    const m = new Map<string, RrTalkgroup[]>();
    for (const t of tgs) m.set(t.category || "Uncategorized", [...(m.get(t.category || "Uncategorized") ?? []), t]);
    return [...m.entries()];
  }, [tgs]);
  const toggle = (ids: number[], on: boolean) => {
    const next = new Set(chosen);
    for (const id of ids) {
      if (on) next.add(id);
      else next.delete(id);
    }
    setChosen(next);
  };
  const encrypted = tgs.filter((t) => t.encryption === "full").map((t) => t.tgid);
  const encOn = encrypted.filter((id) => chosen.has(id)).length;
  return (
    <div className="flex min-h-0 flex-1 flex-col gap-1.5">
      <div className="flex items-center gap-3 text-xs">
        <span className="font-medium">
          Talkgroups <span className="font-normal text-muted-foreground">({chosen.size} of {tgs.length})</span>
        </span>
        <button className="text-primary hover:underline" onClick={() => toggle(tgs.map((t) => t.tgid), true)}>
          All
        </button>
        <button className="text-primary hover:underline" onClick={() => toggle(tgs.map((t) => t.tgid), false)}>
          None
        </button>
        {encrypted.length > 0 && (
          <label className="ml-auto flex items-center gap-1.5 text-muted-foreground">
            <input type="checkbox" className="accent-primary" checked={encOn === encrypted.length} ref={indeterminate(encOn > 0 && encOn < encrypted.length)} onChange={(e) => toggle(encrypted, e.target.checked)} />
            Include {encrypted.length} encrypted
          </label>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-auto rounded-md border">
        {groups.map(([cat, list]) => {
          const on = list.filter((t) => chosen.has(t.tgid)).length;
          const isOpen = open.has(cat);
          return (
            <div key={cat} className="border-b last:border-b-0">
              <div className="flex items-center gap-2 px-2 py-1 text-sm hover:bg-muted/50">
                <input
                  type="checkbox"
                  className="accent-primary"
                  checked={on === list.length}
                  ref={indeterminate(on > 0 && on < list.length)}
                  onChange={(e) => toggle(list.map((t) => t.tgid), e.target.checked)}
                />
                <button
                  className="flex min-w-0 flex-1 items-center gap-1 text-left"
                  onClick={() => {
                    const next = new Set(open);
                    if (isOpen) next.delete(cat);
                    else next.add(cat);
                    setOpen(next);
                  }}
                >
                  <ChevronRight className={cn("size-3.5 shrink-0 text-muted-foreground transition-transform", isOpen && "rotate-90")} />
                  <span className="truncate">{cat}</span>
                  <span className="ml-auto shrink-0 pl-2 font-mono text-[11px] text-muted-foreground tabular-nums">
                    {on}/{list.length}
                  </span>
                </button>
              </div>
              {isOpen &&
                list.map((t) => (
                  <label key={t.tgid} className="flex cursor-pointer items-center gap-2 py-0.5 pr-2 pl-9 text-xs hover:bg-muted/50" title={t.description}>
                    <input type="checkbox" className="accent-primary" checked={chosen.has(t.tgid)} onChange={(e) => toggle([t.tgid], e.target.checked)} />
                    <span className="w-12 shrink-0 font-mono tabular-nums">{t.tgid}</span>
                    <span className="w-36 shrink-0 truncate font-mono">{t.alpha_tag}</span>
                    <span className="min-w-0 flex-1 truncate text-muted-foreground">{t.description}</span>
                    {ENC_LABEL[t.encryption] && (
                      <span className={cn("shrink-0 rounded px-1 text-[10px]", t.encryption === "full" ? "bg-destructive/10 text-destructive" : "bg-amber-500/10 text-amber-700 dark:text-amber-400")}>
                        {ENC_LABEL[t.encryption]}
                      </span>
                    )}
                  </label>
                ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function RrImportDialog({ open, onOpenChange, onImported }: { open: boolean; onOpenChange: (o: boolean) => void; onImported: (systemId: string) => void }) {
  const { profile, profileBands, bands, systems: drafts, deletedSystems, loadSystemDraft, applyImport } = useStore();
  const [sites, setSitesText] = useParsed(api.rrParseSites);
  const [tgs, setTgsText] = useParsed(api.rrParseTalkgroups);
  const [target, setTarget] = useState("");
  const [name, setName] = useState("");
  const [wacn, setWacn] = useState(0);
  const [sysid, setSysid] = useState(0);
  const [chosenSites, setChosenSites] = useState<Set<string>>(new Set());
  const [chosenTgs, setChosenTgs] = useState<Set<number>>(new Set());
  const [allFreqs, setAllFreqs] = useState(false);
  const [plan, setPlan] = useState<{ value?: RrPlan; error?: string }>({});
  const [showRenamed, setShowRenamed] = useState(false);

  const s = sites.value ?? NO_SITES;
  const t = tgs.value ?? NO_TGS;
  // Defaults whenever a table is (re)parsed: every site, and talkgroups that aren't fully encrypted.
  useEffect(() => setChosenSites(new Set(s.sites.map((x) => `${x.rfss}-${x.site}`))), [s]);
  useEffect(() => setChosenTgs(new Set(t.talkgroups.filter((x) => x.encryption !== "full").map((x) => x.tgid))), [t]);

  const existing = (profile?.summary.systems ?? []).filter((x) => !deletedSystems.includes(x.id));
  const newSystems = Object.values(drafts).filter((d) => d.band);
  const allFreqHz = s.sites.flatMap((x) => x.freqs.filter((f) => f.control || allFreqs || !s.marked).map((f) => f.hz));
  const fit = (ranges: [number, number][]) => allFreqHz.filter((hz) => ranges.some(([lo, hi]) => hz >= lo && hz <= hi)).length;
  // Until the user picks, aim at a new system in the band that holds the most frequencies.
  const bestBand = [...profileBands].sort((a, b) => fit(b.ranges) - fit(a.ranges))[0]?.code;
  const tgt = target || (bestBand ? `new:${bestBand}` : "");
  const isNew = tgt.startsWith("new:");
  const band = isNew ? profileBands.find((b) => `new:${b.code}` === tgt) : undefined;
  const systemId = isNew ? undefined : tgt.slice("sys:".length);
  const ranges = band?.ranges ?? (systemId ? bands[systemId] : undefined) ?? NO_RANGES;
  const draft = systemId ? drafts[systemId]?.draft : undefined;

  useEffect(() => {
    if (systemId) loadSystemDraft(systemId);
  }, [systemId, loadSystemDraft]);

  const selection = useMemo(
    () => ({ sites: s.sites.filter((x) => chosenSites.has(`${x.rfss}-${x.site}`)).map((x) => [x.rfss, x.site] as [number, number]), tgids: [...chosenTgs], all_freqs: allFreqs }),
    [s, chosenSites, chosenTgs, allFreqs]
  );
  const ready = !!(sites.value || tgs.value) && (isNew || !!draft);
  useEffect(() => {
    if (!ready) return setPlan({});
    let live = true;
    api.rrPlan(isNew ? null : draft!, ranges, s, t, selection).then(
      (value) => live && setPlan({ value }),
      (e) => live && setPlan({ error: String(e) })
    );
    return () => {
      live = false;
    };
  }, [ready, isNew, draft, ranges, s, t, selection]);

  const taken = [...existing.map((x) => x.name.trim()), ...newSystems.map((d) => d.draft.name.trim())];
  const nameError = !isNew
    ? null
    : !name.trim()
      ? "Name the system"
      : name.length > MAX_NAME
        ? `At most ${MAX_NAME} characters`
        : !/^[\x20-\x7E]+$/.test(name)
          ? "Printable characters only"
          : taken.includes(name.trim())
            ? "Another system has this name"
            : wacn === 0 || sysid === 0
              ? "Enter the WACN and System ID from the RadioReference page"
              : null;
  const p = plan.value;
  const adds = p ? p.sites.length + p.control_channels.length + p.talkgroups.length : 0;
  const blocker = plan.error ?? nameError ?? (isNew && p && (p.sites.length === 0 || p.control_channels.length === 0) ? "A new system needs at least one site and one control channel" : null) ?? (p && adds === 0 ? "Nothing new to add" : null);

  const reset = () => {
    setSitesText("");
    setTgsText("");
    setTarget("");
    setName("");
    setWacn(0);
    setSysid(0);
    setAllFreqs(false);
    setShowRenamed(false);
  };
  const doImport = () => {
    if (!p || blocker) return;
    const id = applyImport(band ? { band: band.code, name: name.trim(), wacn, sysid } : { systemId: systemId! }, p);
    reset();
    onOpenChange(false);
    onImported(id);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex max-h-[90vh] flex-col sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>Import from RadioReference</DialogTitle>
          <DialogDescription>
            On the system's RadioReference page, download the site and talkgroup CSVs (Premium), or select each table and paste it here. Nothing reaches the pager until you write.
          </DialogDescription>
        </DialogHeader>

        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto">
          <div className="flex gap-4">
            <Source
              label="Sites"
              hint="Paste the site table, or choose trs_sites_….csv"
              state={sites}
              onText={setSitesText}
              summary={(v) => `${v.sites.length} sites · ${v.sites.reduce((n, x) => n + x.freqs.filter((f) => f.control).length, 0)} control channels${v.marked ? "" : " (no markers: every frequency counts)"}`}
            />
            <Source label="Talkgroups" hint="Paste the talkgroup table, or choose trs_tg_….csv" state={tgs} onText={setTgsText} summary={(v) => `${v.talkgroups.length} talkgroups`} />
          </div>

          {(sites.value || tgs.value) && (
            <>
              <div className="flex flex-wrap items-end gap-4">
                <div className="w-80 space-y-1.5">
                  <Label htmlFor="rr-target">Import into</Label>
                  <select id="rr-target" className={SELECT} value={tgt} onChange={(e) => setTarget(e.target.value)}>
                    {profileBands.map((b) => (
                      <option key={b.code} value={`new:${b.code}`}>
                        New {b.name} system{s.sites.length ? ` · ${fit(b.ranges)} of ${allFreqHz.length} channels fit` : ""}
                      </option>
                    ))}
                    {existing.map((x) => (
                      <option key={x.id} value={`sys:${x.id}`}>
                        Add to {drafts[x.id]?.draft.name ?? x.name}
                      </option>
                    ))}
                  </select>
                </div>
                {isNew && (
                  <>
                    <div className="space-y-1.5">
                      <Label htmlFor="rr-name">System name</Label>
                      <Input id="rr-name" value={name} maxLength={MAX_NAME} className="w-40 font-mono" onChange={(e) => setName(e.target.value)} />
                    </div>
                    <div className="space-y-1.5">
                      <Label>WACN (hex)</Label>
                      <HexInput aria-label="WACN" value={wacn} max={0xfffff} digits={5} className="w-24" onChange={setWacn} />
                    </div>
                    <div className="space-y-1.5">
                      <Label>System ID (hex)</Label>
                      <HexInput aria-label="System ID" value={sysid} max={0xfff} digits={3} className="w-20" onChange={setSysid} />
                    </div>
                    <p className="w-full text-xs text-muted-foreground">The WACN and System ID aren't in the files; copy them from the top of the RadioReference page.</p>
                  </>
                )}
              </div>

              <div className="grid min-h-64 grid-cols-[minmax(0,2fr)_minmax(0,3fr)] gap-4">
                <div className="flex min-h-0 flex-col gap-1.5">
                  <div className="flex items-center gap-3 text-xs">
                    <span className="font-medium">
                      Sites <span className="font-normal text-muted-foreground">({selection.sites.length} of {s.sites.length})</span>
                    </span>
                    {s.marked && (
                      <label className="ml-auto flex items-center gap-1.5 text-muted-foreground" title="RadioReference marks control channels; the rest are voice channels the pager doesn't need.">
                        <input type="checkbox" className="accent-primary" checked={allFreqs} onChange={(e) => setAllFreqs(e.target.checked)} />
                        Voice channels too
                      </label>
                    )}
                  </div>
                  <div className="min-h-0 flex-1 overflow-auto rounded-md border">
                    {s.sites.length === 0 && <p className="p-2 text-xs text-muted-foreground">No site table: only talkgroups will be added.</p>}
                    {s.sites.map((x) => {
                      const k = `${x.rfss}-${x.site}`;
                      const used = x.freqs.filter((f) => f.control || allFreqs || !s.marked);
                      const out = used.filter((f) => !ranges.some(([lo, hi]) => f.hz >= lo && f.hz <= hi)).length;
                      return (
                        <label key={k} className="flex cursor-pointer items-start gap-2 border-b px-2 py-1 last:border-b-0 hover:bg-muted/50" title={used.map((f) => mhz(f.hz)).join(", ")}>
                          <input
                            type="checkbox"
                            className="mt-0.5 accent-primary"
                            checked={chosenSites.has(k)}
                            onChange={(e) => {
                              const next = new Set(chosenSites);
                              if (e.target.checked) next.add(k);
                              else next.delete(k);
                              setChosenSites(next);
                            }}
                          />
                          <span className="min-w-0 flex-1">
                            <span className="block truncate text-sm">{x.description || `Site ${k}`}</span>
                            <span className="font-mono text-[11px] text-muted-foreground">
                              RFSS {hex(x.rfss, 2)} · Site {hex(x.site, 2)} · {used.length} channel{used.length !== 1 && "s"}
                              {out > 0 && <span className="text-destructive"> · {out} outside the band</span>}
                            </span>
                          </span>
                        </label>
                      );
                    })}
                  </div>
                </div>
                {t.talkgroups.length > 0 ? (
                  <TalkgroupChooser tgs={t.talkgroups} chosen={chosenTgs} setChosen={setChosenTgs} />
                ) : (
                  <p className="text-xs text-muted-foreground">No talkgroup table: only sites and control channels will be added.</p>
                )}
              </div>

              {p && (
                <div className="space-y-1 rounded-md bg-muted/50 p-3 text-xs">
                  <p>
                    {adds > 0 ? (
                      <b>
                        Adds{" "}
                        {counts([
                          [p.sites.length, "site"],
                          [p.control_channels.length, "control channel"],
                          [p.talkgroups.length, "talkgroup"],
                        ])}
                      </b>
                    ) : (
                      <b>Adds nothing</b>
                    )}
                    {!isNew && p.sites_present + p.control_channels_present + p.talkgroups_present > 0 && (
                      <span className="text-muted-foreground">
                        {"; already there and left as they are: "}
                        {counts([
                          [p.sites_present, "site"],
                          [p.control_channels_present, "control channel"],
                          [p.talkgroups_present, "talkgroup"],
                        ])}
                      </span>
                    )}
                    .
                  </p>
                  {p.out_of_band.length > 0 && (
                    <p className="text-destructive">
                      Left out, outside {band ? `the ${band.name} band` : "this system's band"}: {p.out_of_band.map(mhz).join(", ")} MHz
                    </p>
                  )}
                  {p.renamed.length > 0 && (
                    <p className="text-muted-foreground">
                      <button className="underline-offset-2 hover:underline" onClick={() => setShowRenamed(!showRenamed)}>
                        {p.renamed.length} name{p.renamed.length !== 1 && "s"} shortened to fit the pager's {MAX_NAME} characters
                      </button>
                      ; rename them in the editor afterwards if you like.
                    </p>
                  )}
                  {showRenamed && (
                    <ul className="max-h-28 overflow-auto font-mono text-[11px] text-muted-foreground">
                      {p.renamed.map((r, i) => (
                        <li key={i}>
                          {r.from} → {r.to}
                        </li>
                      ))}
                    </ul>
                  )}
                  {!isNew && p.not_in_import.sites.length + p.not_in_import.control_channels.length + p.not_in_import.talkgroups.length > 0 && (
                    <p className="text-muted-foreground">
                      Not in RadioReference, kept:{" "}
                      {counts([
                        [p.not_in_import.sites.length, "site"],
                        [p.not_in_import.control_channels.length, "control channel"],
                        [p.not_in_import.talkgroups.length, "talkgroup"],
                      ])}
                      {p.not_in_import.talkgroups.length > 0 && ` (${p.not_in_import.talkgroups.slice(0, 5).map((x) => `${x.tgid} ${x.name}`).join(", ")}${p.not_in_import.talkgroups.length > 5 ? ", …" : ""})`}.
                      Remove any you don't want in the editor.
                    </p>
                  )}
                </div>
              )}
            </>
          )}
        </div>

        <DialogFooter className="items-center">
          {blocker && <span className="mr-auto text-xs text-destructive">{blocker}</span>}
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button disabled={!p || !!blocker} onClick={doImport}>
            Import
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
