import { useState } from "react";
import { FolderSearch, Send, ShieldCheck, ShieldAlert } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { revealInFileManager } from "@/lib/api";
import { fileManagerName } from "@/lib/platform";
import { formatTaken } from "@/lib/format";
import { useStore } from "@/lib/store";
import CloneDialog from "@/components/clone-dialog";
import type { BackupInfo } from "@/lib/types";

export default function BackupsPage() {
  const { backups, openBackup, profile, loadingProfile, settings } = useStore();
  const [cloning, setCloning] = useState<BackupInfo | null>(null);
  const newest = new Map<string, string>();
  for (const b of backups) if (!newest.has(b.serial)) newest.set(b.serial, b.id);

  return (
    <div className="space-y-4 p-8">
      <div>
        <h1 className="text-xl font-semibold tracking-tight">Backups</h1>
        <p className="text-sm text-muted-foreground">Every backup is a full copy of the pager's profile files. A safety backup is taken automatically before every write.</p>
      </div>
      {backups.length === 0 ? (
        <div className="rounded-lg border border-dashed p-10 text-center text-sm text-muted-foreground">No backups yet. Connect a pager and choose Back up now.</div>
      ) : (
        <div className="rounded-lg border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Taken</TableHead>
                <TableHead>Model</TableHead>
                <TableHead>Serial</TableHead>
                <TableHead>Firmware</TableHead>
                <TableHead>Database</TableHead>
                <TableHead>Integrity</TableHead>
                <TableHead />
              </TableRow>
            </TableHeader>
            <TableBody>
              {backups.map((b) => (
                <TableRow key={b.id} data-state={profile?.backup.id === b.id ? "selected" : undefined}>
                  <TableCell className="whitespace-nowrap">
                    {formatTaken(b)}
                    {newest.get(b.serial) === b.id && <Badge variant="secondary" className="ml-2">Newest</Badge>}
                  </TableCell>
                  <TableCell className="font-mono text-xs">{b.model.slice(0, 21)}</TableCell>
                  <TableCell className="font-mono text-xs">{b.serial}</TableCell>
                  <TableCell className="text-xs">{b.sw_version}</TableCell>
                  <TableCell className="font-mono text-xs">{b.db_version}</TableCell>
                  <TableCell>
                    {b.sidecars_ok ? (
                      <span className="flex items-center gap-1 text-xs text-emerald-600 dark:text-emerald-400"><ShieldCheck className="size-3.5" />MD5 OK</span>
                    ) : b.radio_db_sidecar_stale ? (
                      <span
                        className="flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400"
                        title="A firmware update changed the radio DB without updating its checksum. The next write fixes it."
                      >
                        <ShieldAlert className="size-3.5" />Radio DB MD5 stale
                      </span>
                    ) : (
                      <span className="flex items-center gap-1 text-xs text-destructive"><ShieldAlert className="size-3.5" />Check failed</span>
                    )}
                  </TableCell>
                  <TableCell className="text-right whitespace-nowrap">
                    {!settings?.mobile && (
                      <Button variant="ghost" size="icon-sm" onClick={() => revealInFileManager(b.path)} title={`Show in ${fileManagerName}`}>
                        <FolderSearch />
                      </Button>
                    )}
                    <Button variant="ghost" size="icon-sm" onClick={() => setCloning(b)} title="Write to another pager…" aria-label="Write to another pager…">
                      <Send />
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={loadingProfile}
                      onClick={() => openBackup(b.id).catch((e) => toast.error("Couldn't open backup", { description: String(e) }))}
                    >
                      Open
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
      <CloneDialog source={cloning} open={!!cloning} onOpenChange={(o) => !o && setCloning(null)} />
    </div>
  );
}
