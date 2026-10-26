import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { CircleAlert, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { api } from "@/lib/api";
import { formatTaken, hex } from "@/lib/format";
import { useStore } from "@/lib/store";
import { cn } from "@/lib/utils";
import type { BackupSystem } from "@/lib/types";

const SELECT = "h-8 w-full rounded-md border border-input bg-transparent px-2 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50";

/** PPS's "Add System from Resources": copy a P25 system, with every site, control channel and
 * talkgroup setting, from any backup into the open profile as a new system. */
export function CopySystemDialog({ open, onOpenChange, onCopied }: { open: boolean; onOpenChange: (o: boolean) => void; onCopied: (id: string) => void }) {
  const { backups, profile, profileBands, copySystem } = useStore();
  const [all, setAll] = useState(false);
  const [backupId, setBackupId] = useState<string>("");
  const [list, setList] = useState<{ loading: boolean; systems: BackupSystem[]; error?: string }>({ loading: false, systems: [] });
  const [busy, setBusy] = useState<string | null>(null);

  // Newest backup of each pager unless asked for all of them.
  const choices = useMemo(() => {
    const seen = new Set<string>();
    return backups.filter((b) => all || (!seen.has(b.serial) && seen.add(b.serial)));
  }, [backups, all]);

  useEffect(() => {
    if (open && !choices.some((b) => b.id === backupId)) setBackupId(choices.find((b) => b.serial !== profile?.backup.serial)?.id ?? choices[0]?.id ?? "");
  }, [open, choices, backupId, profile]);

  useEffect(() => {
    if (!open || !backupId) return;
    setList({ loading: true, systems: [] });
    api
      .backupSystems(backupId)
      .then((systems) => setList({ loading: false, systems }))
      .catch((e) => setList({ loading: false, systems: [], error: String(e) }));
  }, [open, backupId]);

  const codes = profileBands.map((b) => b.code);
  const why = (s: BackupSystem) => {
    const parts = s.band.match(/.{2}/g) ?? [];
    if (parts.length === 0) return "has no band";
    const missing = parts.filter((c) => !codes.includes(c));
    return missing.length ? `band ${missing.join(", ")} isn't on this pager (${codes.join(", ")})` : null;
  };
  const here = new Set((profile?.summary.systems ?? []).map((s) => `${s.wacn}.${s.system_id}`));

  async function copy(s: BackupSystem) {
    setBusy(s.id);
    try {
      const id = await copySystem(backupId, s);
      toast.success(`Copied ${s.name}`, { description: "It's a pending change; put it on a knob or write it as it is." });
      onCopied(id);
      onOpenChange(false);
    } catch (e) {
      toast.error(`Couldn't copy ${s.name}`, { description: String(e) });
    } finally {
      setBusy(null);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex max-h-[85vh] flex-col sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>Copy a system from a backup</DialogTitle>
          <DialogDescription>Adds a copy of a P25 system from any backup, with its sites, control channels and talkgroups and all their settings. The copy is a new system; the original is untouched.</DialogDescription>
        </DialogHeader>
        <div className="space-y-1.5">
          <div className="flex items-center gap-3">
            <select className={SELECT} value={backupId} onChange={(e) => setBackupId(e.target.value)} aria-label="Backup">
              {choices.map((b) => (
                <option key={b.id} value={b.id}>
                  {b.serial} · {b.model.slice(0, 21)} · {formatTaken(b)}
                  {b.id === profile?.backup.id ? " (open)" : ""}
                </option>
              ))}
            </select>
            <label className="flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground">
              <input type="checkbox" className="accent-primary" checked={all} onChange={(e) => setAll(e.target.checked)} />
              All backups
            </label>
          </div>
        </div>
        <div className="-mx-6 min-h-40 flex-1 overflow-auto px-6">
          {list.loading ? (
            <div className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              Reading the backup
            </div>
          ) : list.error ? (
            <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
              <CircleAlert className="mt-0.5 size-4 shrink-0" />
              {list.error}
            </div>
          ) : list.systems.length === 0 ? (
            <div className="py-6 text-sm text-muted-foreground">This backup has no P25 systems.</div>
          ) : (
            <ul className="divide-y rounded-lg border">
              {list.systems.map((s) => {
                const blocked = why(s);
                return (
                  <li key={s.id} className="flex items-center gap-3 px-3 py-2">
                    <div className="min-w-0 flex-1">
                      <div className={cn("flex items-center gap-1.5 text-sm font-medium", blocked && "text-muted-foreground")}>
                        <span className="truncate">{s.name}</span>
                        <span className="shrink-0 rounded bg-muted px-1 font-mono text-[10px] text-muted-foreground">{s.band || "no band"}</span>
                        {here.has(`${s.wacn}.${s.sysid}`) && <span className="shrink-0 text-[11px] font-normal text-muted-foreground italic">same WACN and System ID as one in this profile</span>}
                      </div>
                      <div className="font-mono text-[11px] text-muted-foreground">
                        {hex(s.wacn, 5)}.{hex(s.sysid, 3)} · {s.sites} site{s.sites !== 1 && "s"} · {s.control_channels} control ch · {s.talkgroups} TGs
                      </div>
                      {blocked && <div className="text-[11px] text-destructive">Can't copy: {blocked}</div>}
                    </div>
                    <Button size="sm" variant="outline" disabled={!!blocked || busy !== null} onClick={() => copy(s)}>
                      {busy === s.id ? <Loader2 className="animate-spin" /> : null}
                      Copy
                    </Button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
