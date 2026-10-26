import { useEffect, useState } from "react";
import { toast } from "sonner";
import { ArrowRight, Check, CircleAlert, ShieldCheck, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { OpProgress } from "@/components/op-progress";
import { useProgress } from "@/hooks/use-progress";
import { api } from "@/lib/api";
import { useStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import { formatValue, settingDef } from "@/lib/pager-settings";
import type { WritePlan, WriteResult } from "@/lib/types";

type Phase = { kind: "planning" } | { kind: "ready"; plan: WritePlan } | { kind: "writing" } | { kind: "done"; result: WriteResult } | { kind: "error"; message: string; plan?: WritePlan };

function Check_({ ok, children }: { ok: boolean; children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-2 text-sm">
      {ok ? <Check className="mt-0.5 size-4 shrink-0 text-emerald-500" /> : <X className="mt-0.5 size-4 shrink-0 text-destructive" />}
      <span className={ok ? undefined : "text-destructive"}>{children}</span>
    </li>
  );
}

export default function WriteDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const { profile, device, settings, setBusy, refreshBackups, openBackup, clearEdits, edits: getEdits, pendingCount } = useStore();
  const [phase, setPhase] = useState<Phase>({ kind: "planning" });
  const progress = useProgress(phase.kind === "writing");
  const edits = getEdits();
  const count = pendingCount();

  useEffect(() => {
    if (!open || !profile) return;
    setPhase({ kind: "planning" });
    api
      .planWrite(profile.backup.id, edits)
      .then((plan) => setPhase({ kind: "ready", plan }))
      .catch((e) => setPhase({ kind: "error", message: String(e) }));
    // Plan once per opening.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  if (!profile) return null;
  const dev = device?.state === "connected" ? device.device : null;
  const checks = {
    connected: !!dev,
    samePager: !!dev && dev.serial === profile.backup.serial && dev.db_version === profile.backup.db_version,
    latest: profile.is_latest,
    templates: !!settings?.templates_ok,
  };
  const allOk = Object.values(checks).every(Boolean);

  async function write() {
    if (!profile) return;
    setPhase({ kind: "writing" });
    setBusy("write");
    try {
      const result = await api.writeToPager(profile.backup.id, edits);
      setPhase({ kind: "done", result });
      toast.success("Pager programmed and verified");
      clearEdits();
      // Keep editing from a backup of the pager as written (taken by the write itself).
      try {
        const b = result.new_backup ?? (await api.backupNow());
        await refreshBackups();
        await openBackup(b.id);
      } catch (e) {
        await refreshBackups();
        toast.warning(`The write worked, but backing up the new profile failed (${String(e)}). Take a backup before your next change.`);
      }
    } catch (e) {
      setPhase({ kind: "error", message: String(e) });
    } finally {
      setBusy(null);
    }
  }

  const locked = phase.kind === "writing";
  return (
    <Dialog open={open} onOpenChange={(o) => !locked && onOpenChange(o)}>
      <DialogContent className="sm:max-w-lg" showCloseButton={!locked}>
        <DialogHeader>
          <DialogTitle>Write to pager</DialogTitle>
          <DialogDescription>Your changes are applied to the profile from this backup, then programmed the same way Unication's software does it.</DialogDescription>
        </DialogHeader>

        {phase.kind === "planning" && <OpProgress progress={null} fallback="Building the new profile" />}

        {(phase.kind === "ready" || (phase.kind === "error" && phase.plan)) && (
          <div className="space-y-4">
            <div>
              <div className="mb-1.5 text-xs font-medium text-muted-foreground">Changes</div>
              <ul className="max-h-64 divide-y overflow-auto rounded-lg border">
                {(phase.kind === "ready" ? phase.plan : phase.plan!).systems.map((c) => {
                  const plural = (n: number, w: string) => `${n} ${w}${n === 1 ? "" : "s"}`;
                  const parts = c.deleted
                    ? ["removed from the profile"]
                    : c.created
                      ? [`new system · ${plural(c.sites_added, "site")} · ${plural(c.ccs_added, "control channel")} · ${plural(c.tgs_added, "talkgroup")}`]
                      : [
                    ...c.properties_changed.map((p) => `${p} changed`),
                    c.sites_added && `+${c.sites_added} site${c.sites_added > 1 ? "s" : ""}`,
                    c.sites_removed && `−${c.sites_removed} site${c.sites_removed > 1 ? "s" : ""}`,
                    c.sites_changed && `${c.sites_changed} site${c.sites_changed > 1 ? "s" : ""} edited`,
                    c.sites_reordered && "sites reordered",
                    c.ccs_added && `+${c.ccs_added} control channel${c.ccs_added > 1 ? "s" : ""}`,
                    c.ccs_removed && `−${c.ccs_removed} control channel${c.ccs_removed > 1 ? "s" : ""}`,
                    c.ccs_changed && `${c.ccs_changed} control channel${c.ccs_changed > 1 ? "s" : ""} edited`,
                    c.tgs_added && `+${c.tgs_added} talkgroup${c.tgs_added > 1 ? "s" : ""}`,
                    c.tgs_removed && `−${c.tgs_removed} talkgroup${c.tgs_removed > 1 ? "s" : ""}`,
                    c.tgs_changed && `${c.tgs_changed} talkgroup${c.tgs_changed > 1 ? "s" : ""} edited`,
                    c.tgs_settings_changed && `settings changed on ${c.tgs_settings_changed} talkgroup${c.tgs_settings_changed > 1 ? "s" : ""}`,
                    c.knob_entries_removed && `removed from ${c.knob_entries_removed} knob list entr${c.knob_entries_removed > 1 ? "ies" : "y"}`,
                        ].filter(Boolean);
                  return (
                    <li key={`sys-${c.system_id}`} className="space-y-1 px-3 py-2 text-sm">
                      <div className="flex items-center gap-2">
                        <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">P25 system</span>
                        <span className={cn("truncate font-mono", c.deleted && "line-through decoration-muted-foreground/60")}>{c.name}</span>
                      </div>
                      <div className="pl-30 text-xs text-muted-foreground">{parts.join(" · ")}</div>
                    </li>
                  );
                })}
                {(phase.kind === "ready" ? phase.plan : phase.plan!).talkgroups.map((t) => (
                  <li key={`tg-${t.zone}-${t.knob}`} className="space-y-1 px-3 py-2 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                        {t.zone} · {t.knob}
                      </span>
                      <span className="truncate font-mono">{t.knob_name}</span>
                      <span className="ml-auto shrink-0 text-xs text-muted-foreground">{t.count} talkgroups</span>
                    </div>
                    <div className="flex flex-wrap gap-1 pl-30">
                      {t.added.map((g) => (
                        <span key={`a${g.tgid}`} className="rounded bg-emerald-500/10 px-1.5 font-mono text-xs text-emerald-700 dark:text-emerald-300">+ {g.tgid} {g.name}</span>
                      ))}
                      {t.removed.map((g) => (
                        <span key={`r${g.tgid}`} className="rounded bg-destructive/10 px-1.5 font-mono text-xs text-destructive">− {g.tgid} {g.name}</span>
                      ))}
                      {t.priority_changed && <span className="rounded bg-amber-500/10 px-1.5 text-xs text-amber-700 dark:text-amber-300">priority list changed</span>}
                    </div>
                  </li>
                ))}
                {(phase.kind === "ready" ? phase.plan : phase.plan!).knob_systems.map((k) => (
                  <li key={`ks-${k.zone}-${k.knob}`} className="space-y-1 px-3 py-2 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                        {k.zone} · {k.knob}
                      </span>
                      <span className="truncate font-mono">{k.knob_name}</span>
                      <span className="ml-auto shrink-0 text-xs text-muted-foreground">{k.talkgroups} talkgroups</span>
                    </div>
                    <div className="flex items-center gap-1.5 pl-30 text-xs text-muted-foreground">
                      {k.new_knob ? (
                        <span>new {k.mode === 48 ? "TG Monitor" : "TG Scan"} knob on {k.new_system}</span>
                      ) : k.mode_switched && k.old_system === k.new_system ? (
                        <span>switched to {k.mode === 48 ? "TG Monitor" : "TG Scan"}</span>
                      ) : (
                        <>
                          <span className="line-through decoration-muted-foreground/60">{k.old_system}</span>
                          <ArrowRight className="size-3 shrink-0" />
                          <span>{k.new_system}</span>
                        </>
                      )}
                    </div>
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).zones ?? []).map((z, i) => (
                  <li key={`z-${z.zone_id}-${i}`} className="flex items-center gap-2 px-3 py-2 text-sm">
                    <span className="w-28 shrink-0 text-xs text-muted-foreground">zone</span>
                    <span className="truncate font-mono">{z.name}</span>
                    <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                      {z.created ? "new zone" : z.deleted ? "deleted" : z.renamed_from ? `renamed from ${z.renamed_from}` : "zones reordered"}
                      {z.renumbers && " · later channels renumber"}
                    </span>
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).digital_knobs ?? []).map((d) => (
                  <li key={`dk-${d.zone}-${d.knob}`} className="space-y-1 px-3 py-2 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                        {d.zone} · {d.knob}
                      </span>
                      <span className="truncate font-mono">{d.knob_name}</span>
                      <span className="ml-auto shrink-0 font-mono text-xs text-muted-foreground">{(d.rx_hz / 1e6).toFixed(5)} MHz</span>
                    </div>
                    <div className="pl-30 text-xs text-muted-foreground">
                      new {d.protocol} Monitor knob · {d.talkgroups} talkgroup{d.talkgroups !== 1 && "s"}
                      {d.new_groups > 0 && " (renumbers the radio's talkgroup contacts)"}
                    </div>
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).removed_knobs ?? []).map((r) => (
                  <li key={`rk-${r.zone}-${r.knob}`} className="space-y-1 px-3 py-2 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                        {r.zone} · {r.knob}
                      </span>
                      <span className="truncate font-mono line-through decoration-destructive/60">{r.knob_name}</span>
                      <span className="ml-auto shrink-0 text-xs text-destructive">removed</span>
                    </div>
                    {r.scans_changed.length > 0 && (
                      <div className="pl-30 text-xs text-muted-foreground">
                        dropped from scan {r.scans_changed.join(", ")}
                        {r.priority_moved.length > 0 && `; priority moved to the next channel in ${r.priority_moved.join(", ")}`}
                      </div>
                    )}
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).conv_updates ?? []).map((u) => (
                  <li key={`cu-${u.zone}-${u.knob}`} className="flex items-center gap-2 px-3 py-2 text-sm">
                    <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                      {u.zone} · {u.knob}
                    </span>
                    <span className="truncate font-mono">{u.knob_name}</span>
                    <span className="ml-auto shrink-0 truncate text-xs text-muted-foreground">{u.changed.join(", ")}</span>
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).scan_knobs ?? []).map((c) => (
                  <li key={`sk-${c.zone}-${c.knob}`} className="space-y-1 px-3 py-2 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                        {c.zone} · {c.knob}
                      </span>
                      <span className="truncate font-mono">{c.knob_name}</span>
                      <span className="ml-auto shrink-0 text-xs text-muted-foreground">{c.members.length} channels</span>
                    </div>
                    <div className="pl-30 text-xs text-muted-foreground">
                      {c.new_knob ? `new ${c.mode === 36 ? "Priority" : "Normal"} Scan knob` : "scan list changed"}
                    </div>
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).conv_knobs ?? []).map((c) => (
                  <li key={`ck-${c.zone}-${c.knob}`} className="space-y-1 px-3 py-2 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                        {c.zone} · {c.knob}
                      </span>
                      <span className="truncate font-mono">{c.knob_name}</span>
                      <span className="ml-auto shrink-0 font-mono text-xs text-muted-foreground">{(c.rx_hz / 1e6).toFixed(5)} MHz</span>
                    </div>
                    <div className="pl-30 text-xs text-muted-foreground">
                      new conventional Monitor knob · {c.bandwidth === 2 ? "25" : "12.5"} kHz · {c.squelch}
                    </div>
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).knob_copies ?? []).map((c) => (
                  <li key={`kc-${c.to_zone}-${c.to_knob}`} className="space-y-1 px-3 py-2 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                        {c.to_zone} · {c.to_knob}
                      </span>
                      <span className="truncate font-mono">{c.knob_name}</span>
                      <span className="ml-auto shrink-0 text-xs text-muted-foreground">new {c.kind} knob</span>
                    </div>
                    <div className="pl-30 text-xs text-muted-foreground">
                      copy of {c.source_name} ({c.zone} · {c.knob})
                    </div>
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).knob_moves ?? []).map((m) => (
                  <li key={`km-${m.zone}-${m.knob}`} className="space-y-1 px-3 py-2 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                        {m.zone} · {m.knob}
                      </span>
                      <span className="truncate font-mono">{m.knob_name}</span>
                      <ArrowRight className="size-3.5 shrink-0 text-muted-foreground" />
                      <span className="shrink-0 text-xs">
                        {m.to_zone} · {m.to_knob}
                      </span>
                    </div>
                    <div className="pl-30 text-xs text-muted-foreground">
                      {m.swapped_with ? `swapped with ${m.swapped_with} · ` : ""}channels renumber
                      {m.scans.length > 0 && ` · scan ${m.scans.join(", ")} follows`}
                    </div>
                  </li>
                ))}
                {((phase.kind === "ready" ? phase.plan : phase.plan!).settings ?? []).map((c) => {
                  const def = settingDef(c.key);
                  return (
                    <li key={`set-${c.key}`} className="flex items-center gap-2 px-3 py-2 text-sm">
                      <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">pager setting</span>
                      <span className="truncate">{def ? (def.group ? `${def.label} (menu item)` : def.label) : c.key}</span>
                      <span className="ml-auto flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground">
                        <span className="line-through decoration-muted-foreground/60">{formatValue(def, c.old)}</span>
                        <ArrowRight className="size-3 shrink-0" />
                        <span className="font-medium text-foreground">{formatValue(def, c.new)}</span>
                      </span>
                    </li>
                  );
                })}
                {(phase.kind === "ready" ? phase.plan : phase.plan!).renames.map((r) => (
                  <li key={`${r.zone}-${r.knob}`} className="flex items-center gap-2 px-3 py-2 text-sm">
                    <span className="w-28 shrink-0 truncate text-xs text-muted-foreground">
                      {r.zone} · {r.knob}
                    </span>
                    <span className="truncate font-mono line-through decoration-muted-foreground/60">{r.old}</span>
                    <ArrowRight className="size-3.5 shrink-0 text-muted-foreground" />
                    <span className="truncate font-mono font-medium">{r.new}</span>
                  </li>
                ))}
              </ul>
            </div>
            <ul className="space-y-1.5">
              <Check_ ok={checks.connected}>Pager connected</Check_>
              <Check_ ok={checks.samePager}>It's the pager this backup came from ({profile.backup.serial})</Check_>
              <Check_ ok={checks.latest}>Editing its newest backup</Check_>
              <Check_ ok={checks.templates}>Unication templates available</Check_>
            </ul>
            {((phase.kind === "ready" ? phase.plan : phase.plan!).experimental ?? []).length > 0 && (
              <div className="rounded-md border border-amber-500/40 bg-amber-500/5 p-3 text-xs text-amber-800 dark:text-amber-300">
                Experimental: {(phase.kind === "ready" ? phase.plan : phase.plan!).experimental.join(", ")}. Built from Unication's rules but never tested on a pager. If the knob stays silent or the
                pager misbehaves, restore the safety backup from the Backups page.
              </div>
            )}
            <div className="flex items-start gap-2 rounded-md bg-muted/60 p-3 text-xs text-muted-foreground">
              <ShieldCheck className="mt-0.5 size-3.5 shrink-0" />
              A safety backup is taken first. The pager shows "Programming…", then reboots. Every file is read back and checked afterwards. Takes about 30 seconds; don't unplug the pager.
            </div>
          </div>
        )}

        {phase.kind === "writing" && <OpProgress progress={progress} fallback="Starting" />}

        {phase.kind === "done" && (
          <div className="space-y-2 text-sm">
            <div className="flex items-center gap-2 font-medium text-emerald-600 dark:text-emerald-400">
              <Check className="size-4" />
              Programmed and verified
            </div>
            <p className="text-muted-foreground">All {phase.result.verified.length} files read back identical. The previous profile is saved as a safety backup.</p>
          </div>
        )}

        {phase.kind === "error" && (
          <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
            <CircleAlert className="mt-0.5 size-4 shrink-0" />
            <span className="break-words">{phase.message}</span>
          </div>
        )}

        <DialogFooter>
          {phase.kind === "done" ? (
            <Button onClick={() => onOpenChange(false)}>Done</Button>
          ) : (
            <>
              <Button variant="outline" disabled={locked} onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button disabled={phase.kind !== "ready" || !allOk} onClick={write}>
                Write {count} change{count !== 1 && "s"}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
