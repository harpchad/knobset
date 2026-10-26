import { useState } from "react";
import { toast } from "sonner";
import { Archive, Cable, CheckCircle2, FolderOpen, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { OpProgress } from "@/components/op-progress";
import { useProgress } from "@/hooks/use-progress";
import { api } from "@/lib/api";
import { formatTaken } from "@/lib/format";
import { usbLinkHint } from "@/lib/platform";
import { useStore } from "@/lib/store";

function Field({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className={mono ? "font-mono text-sm" : "text-sm"}>{value || "-"}</div>
    </div>
  );
}

export default function PagerPage() {
  const { device, backups, refreshBackups, openBackup, busy, setBusy } = useStore();
  const [running, setRunning] = useState(false);
  const progress = useProgress(running);

  const latest = device?.state === "connected" ? backups.find((b) => b.serial === device.device.serial) : undefined;

  async function backUp() {
    setRunning(true);
    setBusy("backup");
    try {
      const b = await api.backupNow();
      await refreshBackups();
      toast.success("Backup saved", { description: formatTaken(b) });
    } catch (e) {
      toast.error("Backup failed", { description: String(e) });
    } finally {
      setRunning(false);
      setBusy(null);
    }
  }

  return (
    <div className="mx-auto max-w-3xl space-y-6 p-8">
      <div>
        <h1 className="text-xl font-semibold tracking-tight">Pager</h1>
        <p className="text-sm text-muted-foreground">Connect a G-series pager with its USB cable. {usbLinkHint}</p>
      </div>

      {device?.state === "connected" ? (
        <Card>
          <CardHeader>
            <div className="flex items-start gap-3">
              <CheckCircle2 className="mt-0.5 size-5 text-emerald-500" />
              <div className="min-w-0 flex-1">
                <CardTitle className="font-mono text-base">{device.device.model.slice(0, 21)}</CardTitle>
                <CardDescription>Serial {device.device.serial}</CardDescription>
              </div>
              {device.device.db_supported ? (
                <Badge variant="secondary">Supported</Badge>
              ) : (
                <Badge variant="destructive">Read-only: {device.device.db_version} not supported yet</Badge>
              )}
            </div>
          </CardHeader>
          <CardContent className="space-y-5">
            <div className="grid grid-cols-3 gap-4">
              <Field label="Firmware" value={device.device.sw_version} />
              <Field label="Radio database" value={device.device.db_version} mono />
              <Field label="Radio number" value={device.device.radio_number} />
            </div>
            {running ? (
              <OpProgress progress={progress} fallback="Starting backup" />
            ) : (
              <div className="flex flex-wrap items-center gap-2">
                <Button onClick={backUp} disabled={!!busy}>
                  <Archive />
                  Back up now
                </Button>
                {latest && (
                  <Button variant="outline" onClick={() => openBackup(latest.id)}>
                    <FolderOpen />
                    Open latest backup
                  </Button>
                )}
                <span className="text-xs text-muted-foreground">
                  {latest ? `Last backup ${formatTaken(latest)}` : "No backup of this pager yet. Back up before editing."}
                </span>
              </div>
            )}
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardHeader>
            <div className="flex items-start gap-3">
              <Cable className="mt-0.5 size-5 text-muted-foreground" />
              <div>
                <CardTitle className="text-base">No pager connected</CardTitle>
                <CardDescription>Plug the pager in and turn it on. This page checks every few seconds.</CardDescription>
              </div>
            </div>
          </CardHeader>
          {device?.state === "disconnected" && (
            <CardContent>
              <div className="flex items-start gap-2 rounded-md bg-muted/60 p-3 text-xs text-muted-foreground">
                <TriangleAlert className="mt-0.5 size-3.5 shrink-0" />
                <span className="break-words">{device.reason}</span>
              </div>
            </CardContent>
          )}
        </Card>
      )}
    </div>
  );
}
