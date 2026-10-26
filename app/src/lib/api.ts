import type {
  Band,
  BackupInfo,
  BackupSystem,
  ClonePlan,
  DeviceStatus,
  Edits,
  ProfileView,
  Progress,
  RrPlan,
  RrSelection,
  RrSites,
  RrTalkgroups,
  Settings,
  SystemEdit,
  WritePlan,
  WriteResult,
} from "./types";

/** True inside the Tauri app; false in a plain browser (uses the mock backend). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  const { mockInvoke } = await import("./mock");
  return mockInvoke<T>(cmd, args);
}

export const api = {
  getSettings: () => call<Settings>("get_settings"),
  setTemplatesDir: (path: string) => call<Settings>("set_templates_dir", { path }),
  importTemplates: (path: string) => call<Settings>("import_templates", { path }),
  setBackupsDir: (path: string) => call<Settings>("set_backups_dir", { path }),
  deviceStatus: () => call<DeviceStatus>("device_status"),
  backupNow: () => call<BackupInfo>("backup_now"),
  getBackups: () => call<BackupInfo[]>("get_backups"),
  loadProfile: (backupId: string) => call<ProfileView>("load_profile", { backupId }),
  loadSystem: (backupId: string, systemId: string) => call<SystemEdit>("load_system", { backupId, systemId }),
  systemBands: (backupId: string) => call<Record<string, [number, number][]>>("system_bands", { backupId }),
  profileBands: (backupId: string) => call<Band[]>("profile_bands", { backupId }),
  rrParseSites: (text: string) => call<RrSites>("rr_parse_sites", { text }),
  rrParseTalkgroups: (text: string) => call<RrTalkgroups>("rr_parse_talkgroups", { text }),
  rrPlan: (existing: SystemEdit | null, ranges: [number, number][], sites: RrSites, talkgroups: RrTalkgroups, selection: RrSelection) =>
    call<RrPlan>("rr_plan", { existing, ranges, sites, talkgroups, selection }),
  planWrite: (backupId: string, edits: Edits) => call<WritePlan>("plan_write", { backupId, edits }),
  writeToPager: (backupId: string, edits: Edits) => call<WriteResult>("write_to_pager", { backupId, edits }),
  backupSystems: (backupId: string) => call<BackupSystem[]>("backup_systems", { backupId }),
  planClone: (sourceId: string, targetId: string) => call<ClonePlan>("plan_clone", { sourceId, targetId }),
  cloneToPager: (sourceId: string, targetId: string) => call<WriteResult>("clone_to_pager", { sourceId, targetId }),
};

/** Subscribe to long-operation progress. Returns an unsubscribe function. */
export async function onProgress(cb: (p: Progress) => void): Promise<() => void> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<Progress>("op-progress", (e) => cb(e.payload));
  }
  const { mockProgress } = await import("./mock");
  return mockProgress(cb);
}

export async function pickFolder(title: string): Promise<string | null> {
  if (!inTauri) return "/Users/demo/Unication/G_Series_PPS/DataBase";
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ directory: true, title });
  return typeof r === "string" ? r : null;
}

/** Pick one file. `extensions` filters on desktop only: iOS can't map
 * .exe/.msi to file types reliably, and the backend validates anyway. */
export async function pickFile(title: string, extensions: string[], mobile: boolean): Promise<string | null> {
  if (!inTauri) return "/Users/demo/Downloads/Unication_Pager_PPS_Setup.zip";
  const { open } = await import("@tauri-apps/plugin-dialog");
  const filters = mobile ? undefined : [{ name: title, extensions }];
  const r = await open({ title, filters });
  return typeof r === "string" ? r : null;
}

/** Show a file in Finder, File Explorer or the Linux file manager. */
export async function revealInFileManager(path: string) {
  if (!inTauri) return;
  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
  await revealItemInDir(path);
}
