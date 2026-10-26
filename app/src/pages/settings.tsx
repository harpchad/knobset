import { useState } from "react";
import { toast } from "sonner";
import { CheckCircle2, CircleAlert, FileDown, FolderOpen, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { api, pickFile, pickFolder } from "@/lib/api";
import { useStore } from "@/lib/store";

export default function SettingsPage() {
  const { settings, setSettings, refreshBackups } = useStore();
  const [importing, setImporting] = useState(false);
  if (!settings) return null;
  const { mobile } = settings;

  async function importInstaller() {
    const p = await pickFile("PPS installer", ["zip", "exe", "msi"], mobile);
    if (!p) return;
    setImporting(true);
    try {
      setSettings(await api.importTemplates(p));
      toast.success("Templates imported");
    } catch (e) {
      toast.error("Couldn't import templates", { description: String(e) });
    } finally {
      setImporting(false);
    }
  }

  async function chooseTemplates() {
    const p = await pickFolder("Choose the Unication PPS DataBase folder");
    if (!p) return;
    try {
      setSettings(await api.setTemplatesDir(p));
      toast.success("Templates found");
    } catch (e) {
      toast.error("That folder doesn't have the templates", { description: String(e) });
    }
  }

  async function chooseBackups() {
    const p = await pickFolder("Choose where backups are kept");
    if (!p) return;
    try {
      setSettings(await api.setBackupsDir(p));
      await refreshBackups();
    } catch (e) {
      toast.error("Couldn't use that folder", { description: String(e) });
    }
  }

  return (
    <div className="mx-auto max-w-3xl space-y-6 p-8">
      <h1 className="text-xl font-semibold tracking-tight">Settings</h1>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Unication templates</CardTitle>
          <CardDescription>
            Writing to a pager needs the template databases that ship with Unication's free PPS software. They're Unication's files, so this app doesn't include them. Download PPS from Unication and import the download here (the .zip, or the setup .exe inside it); Knobset copies out just the templates.
            {!mobile && (
              <>
                {" "}
                If PPS is already installed, you can choose its <span className="font-mono">DataBase</span> folder instead.
              </>
            )}
          </CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-3">
          {settings.templates_ok ? <CheckCircle2 className="size-4 shrink-0 text-emerald-500" /> : <CircleAlert className="size-4 shrink-0 text-amber-500" />}
          <span className="min-w-0 flex-1 truncate font-mono text-xs">{mobile ? (settings.templates_ok ? "Imported" : "Not imported yet") : (settings.templates_dir ?? "Not set")}</span>
          <Button variant={settings.templates_ok ? "outline" : "default"} size="sm" disabled={importing} onClick={importInstaller}>
            {importing ? <Loader2 className="animate-spin" /> : <FileDown />}
            {importing ? "Importing…" : "Import installer…"}
          </Button>
          {!mobile && (
            <Button variant="ghost" size="sm" disabled={importing} onClick={chooseTemplates}>
              <FolderOpen />
              Choose folder…
            </Button>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Backups folder</CardTitle>
          <CardDescription>Backups contain your pager's full profile, including its unit ID. Keep them somewhere private.</CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-3">
          <span className="min-w-0 flex-1 truncate font-mono text-xs">{mobile ? "Files app › On My iPad › Knobset › Backups" : settings.backups_dir}</span>
          {!mobile && (
            <Button variant="outline" size="sm" onClick={chooseBackups}>
              <FolderOpen />
              Choose…
            </Button>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Connection</CardTitle>
          <CardDescription>The pager serves its own network over USB and answers at this address.</CardDescription>
        </CardHeader>
        <CardContent className="font-mono text-xs">{settings.pager_addr}</CardContent>
      </Card>
    </div>
  );
}
