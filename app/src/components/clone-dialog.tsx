import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { ArrowRight, Check, CircleAlert, ShieldCheck, TriangleAlert, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { OpProgress } from "@/components/op-progress";
import { useProgress } from "@/hooks/use-progress";
import { api } from "@/lib/api";
import { formatTaken } from "@/lib/format";
import { useStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import type { BackupInfo, ClonePlan, ProfileCounts, WriteResult } from "@/lib/types";

type Phase = { kind: "idle" } | { kind: "planning" } | { kind: "ready"; plan: ClonePlan } | { kind: "writing" } | { kind: "done"; result: WriteResult } | { kind: "error"; message: string; plan?: ClonePlan };

const model21 = (m: string) => m.slice(0, 21);

function Check_({ ok, children }: { ok: boolean; children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-2 text-sm">
      {ok ? <Check className="mt-0.5 size-4 shrink-0 text-emerald-500" /> : <X className="mt-0.5 size-4 shrink-0 text-destructive" />}
      <span className={ok ? undefined : "text-destructive"}>{children}</span>
    </li>
  );
}

const COUNT_ROWS: [keyof ProfileCounts, string][] = [
  ["zones", "Zones"],
  ["knobs", "Knobs"],
  ["systems", "P25 systems"],
  ["talkgroups", "Talkgroups"],
  ["conventional_freqs", "Conventional frequencies"],
];

/** Write a backup's profile to a different pager of the same model, keeping that pager's identity. */
export default function CloneDialog({ source, open, onOpenChange }: { source: BackupInfo | null; open: boolean; onOpenChange: (o: boolean) => void }) {
  const { backups, device, settings, setBusy, refreshBackups } = useStore();
  const dev = device?.state === "connected" ? device.device : null;
  // The newest backup of every other pager is a possible target; it's what the target holds now.
  const targets = useMemo(() => {
    const seen = new Set<string>();
    return backups.filter((b) => {
      if (!source || b.serial === source.serial || seen.has(b.serial)) return false;
      seen.add(b.serial);
      return true;
    });
  }, [backups, source]);
  const [targetId, setTargetId] = useState<string | null>(null);
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [confirmed, setConfirmed] = useState(false);
  const progress = useProgress(phase.kind === "writing");
  const target = targets.find((b) => b.id === targetId) ?? null;

  useEffect(() => {
    if (!open) return;
    setConfirmed(false);
    setPhase({ kind: "idle" });
    setTargetId((targets.find((b) => b.serial === dev?.serial) ?? targets[0])?.id ?? null);
    // Pick a default target once per opening.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    if (!open || !source || !targetId) return;
    setConfirmed(false);
    setPhase({ kind: "planning" });
    api
      .planClone(source.id, targetId)
      .then((plan) => setPhase({ kind: "ready", plan }))
      .catch((e) => setPhase({ kind: "error", message: String(e) }));
  }, [open, source, targetId]);

  if (!source) return null;
  const plan = phase.kind === "ready" ? phase.plan : phase.kind === "error" ? phase.plan : undefined;
  const checks = {
    connected: !!dev,
    isTarget: !!dev && !!target && dev.serial === target.serial && dev.db_version === target.db_version,
    templates: !!settings?.templates_ok,
  };
  const allOk = Object.values(checks).every(Boolean) && confirmed;

  async function write() {
    if (!source || !target) return;
    setPhase({ kind: "writing" });
    setBusy("write");
    try {
      const result = await api.cloneToPager(source.id, target.id);
      setPhase({ kind: "done", result });
      toast.success(`${target.serial} programmed and verified`);
      await refreshBackups();
      if (!result.new_backup) toast.warning("The write worked, but backing up the new profile failed. Back up the target before its next change.");
    } catch (e) {
      setPhase({ kind: "error", message: String(e) });
    } finally {
      setBusy(null);
    }
  }

  const locked = phase.kind === "writing";
  const s = plan?.summary;
  return (
    <Dialog open={open} onOpenChange={(o) => !locked && onOpenChange(o)}>
      <DialogContent className="flex max-h-[90vh] flex-col sm:max-w-2xl" showCloseButton={!locked}>
        <DialogHeader>
          <DialogTitle>Write to another pager</DialogTitle>
          <DialogDescription>
            Sets up a second pager of the same model from this backup. Every zone, knob and system on the target is replaced with this backup's; the target keeps its own radio number, unit IDs and
            passwords.
          </DialogDescription>
        </DialogHeader>

        <div className="-mx-6 flex-1 space-y-4 overflow-auto px-6">
          {targets.length === 0 ? (
            <div className="rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground">
              No backup of another pager yet. Connect the pager you want to set up and choose Back up now on the Pager page; that backup is what this write starts from.
            </div>
          ) : (
            <div>
              <div className="mb-1.5 text-xs font-medium text-muted-foreground">Target pager (its newest backup)</div>
              <div className="space-y-1">
                {targets.map((b) => {
                  const sameModel = model21(b.model) === model21(source.model);
                  return (
                    <button
                      key={b.id}
                      disabled={locked}
                      onClick={() => setTargetId(b.id)}
                      className={cn("flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-left text-sm transition-colors hover:bg-muted", b.id === targetId && "border-primary bg-muted")}
                    >
                      <span className="font-mono">{b.serial}</span>
                      <span className={cn("font-mono text-xs", !sameModel && "text-destructive")}>{model21(b.model)}</span>
                      <span className="font-mono text-xs text-muted-foreground">{b.db_version}</span>
                      <span className="ml-auto text-xs text-muted-foreground">
                        {dev?.serial === b.serial && <span className="mr-2 text-emerald-600 dark:text-emerald-400">connected</span>}
                        backed up {formatTaken(b)}
                      </span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {phase.kind === "planning" && <OpProgress progress={null} fallback="Building the target's new profile" />}

          {s && (
            <>
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-xs text-muted-foreground">
                    <th className="w-28 py-1 text-left font-medium" />
                    <th className="py-1 text-left font-medium">From (this backup)</th>
                    <th className="w-6" />
                    <th className="py-1 text-left font-medium">To (target pager)</th>
                  </tr>
                </thead>
                <tbody className="font-mono text-xs">
                  {(
                    [
                      ["Model", model21(s.source.pager.model), model21(s.target.pager.model)],
                      ["Serial", s.source.pager.serial, s.target.pager.serial],
                      ["Radio DB", s.source.pager.db_version, s.target.pager.db_version],
                      ["Profile", s.source.profile, s.target.profile],
                    ] as const
                  ).map(([label, a, b]) => (
                    <tr key={label} className="border-t">
                      <td className="py-1.5 font-sans text-muted-foreground">{label}</td>
                      <td className="py-1.5">{a}</td>
                      <td>
                        <ArrowRight className="size-3 text-muted-foreground" />
                      </td>
                      <td className="py-1.5">{b}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {s.regenerated && (
                <p className="text-xs text-muted-foreground">
                  The target runs {s.target.pager.db_version}, so its radio database is generated for that version from the source's profile.
                </p>
              )}

              <div className="grid gap-4 sm:grid-cols-2">
                <div>
                  <div className="mb-1.5 text-xs font-medium text-muted-foreground">The target keeps its own</div>
                  <ul className="divide-y rounded-lg border text-sm">
                    {s.kept.map((k) => (
                      <li key={k.label} className="flex items-center gap-2 px-3 py-1.5">
                        <span className="text-xs text-muted-foreground">{k.label}</span>
                        <span className="ml-auto truncate font-mono text-xs">{k.value}</span>
                        {k.differs && <span className="shrink-0 rounded bg-amber-500/10 px-1 text-[10px] text-amber-700 dark:text-amber-300">source differs</span>}
                      </li>
                    ))}
                  </ul>
                </div>
                <div>
                  <div className="mb-1.5 text-xs font-medium text-muted-foreground">What changes on the target</div>
                  <ul className="divide-y rounded-lg border text-sm">
                    {COUNT_ROWS.map(([k, label]) => (
                      <li key={k} className="flex items-center gap-2 px-3 py-1.5">
                        <span className="text-xs text-muted-foreground">{label}</span>
                        <span className={cn("ml-auto font-mono text-xs tabular-nums", s.before[k] !== s.after[k] && "font-medium")}>
                          {s.before[k]} <ArrowRight className="inline size-3 text-muted-foreground" /> {s.after[k]}
                        </span>
                      </li>
                    ))}
                  </ul>
                  <div className="mt-2 space-y-1 text-xs">
                    {(
                      [
                        ["Zones added", s.zones_added, "text-emerald-700 dark:text-emerald-300"],
                        ["Zones removed", s.zones_removed, "text-destructive"],
                        ["Systems added", s.systems_added, "text-emerald-700 dark:text-emerald-300"],
                        ["Systems removed", s.systems_removed, "text-destructive"],
                      ] as const
                    )
                      .filter(([, l]) => l.length > 0)
                      .map(([label, list, color]) => (
                        <div key={label}>
                          <span className="text-muted-foreground">{label}: </span>
                          <span className={cn("font-mono", color)}>{list.join(", ")}</span>
                        </div>
                      ))}
                  </div>
                </div>
              </div>

              <ul className="space-y-1.5">
                <Check_ ok={checks.connected}>Pager connected</Check_>
                <Check_ ok={checks.isTarget}>It's the target pager ({s.target.pager.serial}){dev && !checks.isTarget && `; ${dev.serial} is connected`}</Check_>
                <Check_ ok={checks.templates}>Unication templates available</Check_>
              </ul>
              {plan!.experimental.length > 0 && (
                <div className="rounded-md border border-amber-500/40 bg-amber-500/5 p-3 text-xs text-amber-800 dark:text-amber-300">
                  Experimental: {plan!.experimental.join(", ")}. Built from Unication's rules but never tested on a pager.
                </div>
              )}
              <label className="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/5 p-3 text-sm">
                <input type="checkbox" className="mt-0.5 accent-destructive" checked={confirmed} disabled={locked} onChange={(e) => setConfirmed(e.target.checked)} />
                <span>
                  <TriangleAlert className="mr-1 inline size-3.5 text-destructive" />
                  Replace the whole profile on pager <span className="font-mono">{s.target.pager.serial}</span> with the one from <span className="font-mono">{s.source.pager.serial}</span>. Anything set up
                  only on {s.target.pager.serial} is lost, apart from the safety backup.
                </span>
              </label>
              <div className="flex items-start gap-2 rounded-md bg-muted/60 p-3 text-xs text-muted-foreground">
                <ShieldCheck className="mt-0.5 size-3.5 shrink-0" />
                A safety backup of the target is taken first, and the write stops if the target changed since its backup. The pager shows "Programming…", then reboots. Every file is read back and
                checked afterwards.
              </div>
            </>
          )}

          {phase.kind === "writing" && <OpProgress progress={progress} fallback="Starting" />}

          {phase.kind === "done" && (
            <div className="space-y-2 text-sm">
              <div className="flex items-center gap-2 font-medium text-emerald-600 dark:text-emerald-400">
                <Check className="size-4" />
                Programmed and verified
              </div>
              <p className="text-muted-foreground">All {phase.result.verified.length} files read back identical. The target's previous profile is saved as a safety backup.</p>
            </div>
          )}

          {phase.kind === "error" && (
            <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
              <CircleAlert className="mt-0.5 size-4 shrink-0" />
              <span className="break-words">{phase.message}</span>
            </div>
          )}
        </div>

        <DialogFooter>
          {phase.kind === "done" ? (
            <Button onClick={() => onOpenChange(false)}>Done</Button>
          ) : (
            <>
              <Button variant="outline" disabled={locked} onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button variant="destructive" disabled={phase.kind !== "ready" || !allOk} onClick={write}>
                Replace profile on {target?.serial ?? "target"}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
