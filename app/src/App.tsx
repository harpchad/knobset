import { useEffect, useState } from "react";
import { Archive, CircleDot, RadioTower, Send, Settings as SettingsIcon, SlidersHorizontal, Smartphone, Upload } from "lucide-react";
import { Toaster } from "@/components/ui/sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { TooltipProvider } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useStore, type Page } from "@/lib/store";
import { formatTaken } from "@/lib/format";
import { useSystemTheme } from "@/hooks/use-system-theme";
import PagerPage from "@/pages/pager";
import BackupsPage from "@/pages/backups";
import ZonesPage from "@/pages/zones";
import SystemsPage from "@/pages/systems";
import PagerSettingsPage from "@/pages/pager-settings";
import SettingsPage from "@/pages/settings";
import WriteDialog from "@/components/write-dialog";
import CloneDialog from "@/components/clone-dialog";

const NAV: { page: Page; label: string; icon: typeof Smartphone; needsProfile?: boolean }[] = [
  { page: "pager", label: "Pager", icon: Smartphone },
  { page: "backups", label: "Backups", icon: Archive },
  { page: "zones", label: "Zones & knobs", icon: CircleDot, needsProfile: true },
  { page: "systems", label: "P25 systems", icon: RadioTower, needsProfile: true },
  { page: "pager-settings", label: "Pager settings", icon: SlidersHorizontal, needsProfile: true },
];

export default function App() {
  useSystemTheme();
  const { page, setPage, profile, device, refreshDevice, refreshBackups, loadSettings } = useStore();
  const [writeOpen, setWriteOpen] = useState(false);
  const [cloneOpen, setCloneOpen] = useState(false);
  const pending = useStore((s) => s.pendingCount());

  useEffect(() => {
    loadSettings();
    refreshBackups();
    refreshDevice();
    const t = setInterval(refreshDevice, 5000);
    return () => clearInterval(t);
  }, [loadSettings, refreshBackups, refreshDevice]);

  const connected = device?.state === "connected";

  return (
    <TooltipProvider>
      {/* Safe-area padding keeps iPad content clear of the status bar and home indicator (0 on desktop). */}
      <div className="flex h-screen bg-background pt-[env(safe-area-inset-top)] pr-[env(safe-area-inset-right)] text-foreground">
        <aside className="flex w-56 shrink-0 flex-col border-r bg-sidebar pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)]">
          <div className="px-4 pt-5 pb-4">
            <div className="text-sm font-semibold tracking-tight">Knobset</div>
            <div className="mt-1 flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className={cn("size-2 rounded-full", connected ? "bg-emerald-500" : "bg-muted-foreground/40")} />
              {connected ? device.device.model.slice(0, 7) + " connected" : "No pager"}
            </div>
          </div>
          <nav className="flex flex-1 flex-col gap-0.5 px-2">
            {NAV.map(({ page: p, label, icon: Icon, needsProfile }) => {
              const disabled = needsProfile && !profile;
              return (
                <button
                  key={p}
                  disabled={disabled}
                  onClick={() => setPage(p)}
                  className={cn(
                    "flex items-center gap-2.5 rounded-md px-2.5 py-1.5 text-sm transition-colors",
                    page === p ? "bg-sidebar-accent font-medium text-sidebar-accent-foreground" : "text-sidebar-foreground/80 hover:bg-sidebar-accent/60",
                    disabled && "pointer-events-none opacity-40",
                  )}
                >
                  <Icon className="size-4" />
                  {label}
                </button>
              );
            })}
          </nav>
          <div className="px-2 pb-3">
            <button
              onClick={() => setPage("settings")}
              className={cn(
                "flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-sm",
                page === "settings" ? "bg-sidebar-accent font-medium" : "text-sidebar-foreground/80 hover:bg-sidebar-accent/60",
              )}
            >
              <SettingsIcon className="size-4" />
              Settings
            </button>
          </div>
        </aside>

        <main className="flex min-w-0 flex-1 flex-col pb-[env(safe-area-inset-bottom)]">
          {profile && (page === "zones" || page === "systems" || page === "pager-settings") && (
            <header className="flex items-center gap-3 border-b px-6 py-3">
              <div className="min-w-0">
                <div className="truncate text-sm font-medium">{profile.summary.name || "Untitled profile"}</div>
                <div className="text-xs text-muted-foreground">
                  {profile.backup.model.slice(0, 21)} · backup from {formatTaken(profile.backup)}
                  {!profile.is_latest && <span className="text-amber-600 dark:text-amber-400"> · not the newest backup</span>}
                </div>
              </div>
              <div className="ml-auto flex items-center gap-2">
                {pending > 0 && <Badge variant="secondary">{pending} unsaved change{pending > 1 ? "s" : ""}</Badge>}
                <Button
                  size="sm"
                  variant="outline"
                  disabled={pending > 0}
                  title={pending > 0 ? "Writes this backup as it is; write or discard your changes first" : "Set up another pager of the same model from this backup"}
                  onClick={() => setCloneOpen(true)}
                >
                  <Send />
                  Write to another pager…
                </Button>
                <Button size="sm" disabled={pending === 0} onClick={() => setWriteOpen(true)}>
                  <Upload />
                  Write to pager
                </Button>
              </div>
            </header>
          )}
          <div className="min-h-0 flex-1 overflow-auto">
            {page === "pager" && <PagerPage />}
            {page === "backups" && <BackupsPage />}
            {page === "zones" && <ZonesPage />}
            {page === "systems" && <SystemsPage />}
            {page === "pager-settings" && <PagerSettingsPage />}
            {page === "settings" && <SettingsPage />}
          </div>
        </main>
      </div>
      <WriteDialog open={writeOpen} onOpenChange={setWriteOpen} />
      <CloneDialog source={cloneOpen ? (profile?.backup ?? null) : null} open={cloneOpen} onOpenChange={setCloneOpen} />
      <Toaster richColors position="bottom-right" />
    </TooltipProvider>
  );
}
