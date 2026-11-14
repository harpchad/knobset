import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Check, CircleAlert, ShieldCheck, TriangleAlert, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { OpProgress } from "@/components/op-progress";
import { useProgress } from "@/hooks/use-progress";
import { api } from "@/lib/api";
import { formatTaken } from "@/lib/format";
import { useStore } from "@/lib/store";
import type { BackupInfo, WriteResult } from "@/lib/types";

type Phase = { kind: "checking" } | { kind: "ready" } | { kind: "writing" } | { kind: "done"; result: WriteResult } | { kind: "error"; message: string; checked: boolean };

function Check_({ ok, children }: { ok: boolean; children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-2 text-sm">
      {ok ? <Check className="mt-0.5 size-4 shrink-0 text-emerald-500" /> : <X className="mt-0.5 size-4 shrink-0 text-destructive" />}
      <span className={ok ? undefined : "text-destructive"}>{children}</span>
    </li>
  );
}

/** Write a backup back to the pager it came from, exactly as it was taken. */
export default function RestoreDialog({ backup, open, onOpenChange }: { backup: BackupInfo | null; open: boolean; onOpenChange: (o: boolean) => void }) {
  const { device, setBusy, refreshBackups } = useStore();
  const dev = device?.state === "connected" ? device.device : null;
  const [phase, setPhase] = useState<Phase>({ kind: "checking" });
  const [confirmed, setConfirmed] = useState(false);
  const progress = useProgress(phase.kind === "writing");

  useEffect(() => {
    if (!open || !backup) return;
    setConfirmed(false);
    setPhase({ kind: "checking" });
    api
      .planRestore(backup.id)
      .then(() => setPhase({ kind: "ready" }))
      .catch((e) => setPhase({ kind: "error", message: String(e), checked: false }));
  }, [open, backup]);

  if (!backup) return null;
  const checks = {
    backup: phase.kind !== "checking" && !(phase.kind === "error" && !phase.checked),
    connected: !!dev,
    samePager: !!dev && dev.serial === backup.serial && dev.db_version === backup.db_version,
  };
  const allOk = Object.values(checks).every(Boolean) && confirmed;

  async function write() {
    if (!backup) return;
    setPhase({ kind: "writing" });
    setBusy("write");
    try {
      const result = await api.restoreToPager(backup.id);
      setPhase({ kind: "done", result });
      toast.success(`${backup.serial} restored and verified`);
      await refreshBackups();
      if (!result.new_backup) toast.warning("The restore worked, but backing up the pager afterwards failed. Back it up before its next change.");
    } catch (e) {
      setPhase({ kind: "error", message: String(e), checked: true });
    } finally {
      setBusy(null);
    }
  }

  const locked = phase.kind === "writing";
  return (
    <Dialog open={open} onOpenChange={(o) => !locked && onOpenChange(o)}>
      <DialogContent className="flex max-h-[90vh] flex-col sm:max-w-lg" showCloseButton={!locked}>
        <DialogHeader>
          <DialogTitle>Restore this backup</DialogTitle>
          <DialogDescription>
            Writes the profile back to pager <span className="font-mono">{backup.serial}</span> exactly as it was on {formatTaken(backup)}. Use it to undo a write that didn't work out.
          </DialogDescription>
        </DialogHeader>

        <div className="-mx-6 flex-1 space-y-4 overflow-auto px-6">
          {phase.kind === "checking" && <OpProgress progress={null} fallback="Checking the backup" />}

          {phase.kind !== "done" && phase.kind !== "checking" && (
            <>
              <ul className="space-y-1.5">
                <Check_ ok={checks.backup}>The backup is complete and its files check out</Check_>
                <Check_ ok={checks.connected}>Pager connected</Check_>
                <Check_ ok={checks.samePager}>
                  It's the pager this backup came from ({backup.serial}, {backup.db_version}){dev && !checks.samePager && `; ${dev.serial} (${dev.db_version}) is connected`}
                </Check_>
              </ul>
              <label className="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/5 p-3 text-sm">
                <input type="checkbox" className="mt-0.5 accent-destructive" checked={confirmed} disabled={locked || !checks.backup} onChange={(e) => setConfirmed(e.target.checked)} />
                <span>
                  <TriangleAlert className="mr-1 inline size-3.5 text-destructive" />
                  Replace the whole profile on pager <span className="font-mono">{backup.serial}</span> with this backup. Anything changed on the pager since {formatTaken(backup)} is lost, apart
                  from the safety backup.
                </span>
              </label>
              <div className="flex items-start gap-2 rounded-md bg-muted/60 p-3 text-xs text-muted-foreground">
                <ShieldCheck className="mt-0.5 size-3.5 shrink-0" />
                A safety backup of the pager is taken first. The pager shows "Programming…", then reboots. Every file is read back and checked afterwards.
              </div>
            </>
          )}

          {phase.kind === "writing" && <OpProgress progress={progress} fallback="Starting" />}

          {phase.kind === "done" && (
            <div className="space-y-2 text-sm">
              <div className="flex items-center gap-2 font-medium text-emerald-600 dark:text-emerald-400">
                <Check className="size-4" />
                Restored and verified
              </div>
              <p className="text-muted-foreground">All {phase.result.verified.length} files read back identical. The profile it replaced is saved as a safety backup.</p>
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
              <Button variant="destructive" disabled={phase.kind === "checking" || locked || !allOk} onClick={write}>
                Restore to {backup.serial}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
