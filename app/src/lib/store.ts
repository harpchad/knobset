import { create } from "zustand";
import { api } from "./api";
import type { Band, BackupInfo, BackupSystem, ConvKnobEdit, ConvKnobUpdate, ScanKnobEdit, DeviceStatus, Edits, Knob, KnobRename, KnobSystemEdit, KnobTalkgroupEdit, P25System, ProfileView, Settings, SystemEdit, Talkgroup, Zone, RrPlan, KnobCopy, KnobMove } from "./types";

/** GroupID of the "All" talkgroup every system starts with. */
export const ALL_GROUP_ID = "0000000000000000";

export type Page = "pager" | "backups" | "zones" | "systems" | "pager-settings" | "settings";

/** PPS's built-in knob voice prompt for a trunked mode (16 TG scan, 48 TG monitor). */
export const builtinPrompt = (mode: number) => (mode === 48 ? 11 : 6);

/** Key for per-knob pending edits. */
export const knobKey = (zone: string, knob: number) => `${zone}\u0000${knob}`;
const key = knobKey;

/** Priority list in order, then the rest, as stored on the knob. */
export function splitList(k: Knob): { priority: string[]; normal: string[] } {
  return {
    priority: k.talkgroup_list.filter((t) => t.priority).map((t) => t.group_id),
    normal: k.talkgroup_list.filter((t) => !t.priority).map((t) => t.group_id),
  };
}

/** Pending scans can't keep listing channels that won't exist (or can't be scanned). */
function withoutMembers(scans: Record<string, ScanKnobEdit>, gone: (string | null | undefined)[]): Record<string, ScanKnobEdit> {
  const g = new Set(gone.filter((x): x is string => !!x));
  if (g.size === 0) return scans;
  return Object.fromEntries(Object.entries(scans).map(([k, e]) => [k, { ...e, members: e.members.filter((m) => !g.has(m)), priority: e.priority && g.has(e.priority) ? null : e.priority }]));
}

/** Receive modes Knobset can copy, move and remove. */
export const MOVABLE_MODES = [1, 4, 16, 33, 36, 48];

const sameList = (a: string[], b: string[]) => a.length === b.length && a.every((x, i) => x === b[i]);

interface State {
  page: Page;
  device: DeviceStatus | null;
  backups: BackupInfo[];
  profile: ProfileView | null;
  loadingProfile: boolean;
  /** Pending edits, keyed by zone + knob. */
  renames: Record<string, KnobRename>;
  talkgroups: Record<string, KnobTalkgroupEdit>;
  /** Systems being put on knobs, keyed by zone + knob. */
  knobSystems: Record<string, KnobSystemEdit>;
  /** Conventional channels being put on empty positions, keyed by zone + knob. */
  convKnobs: Record<string, ConvKnobEdit>;
  /** Knobs to make empty, keyed by zone + knob. A new knob on the same key converts it. */
  removedKnobs: Record<string, { zone: string; knob: number }>;
  /** Edits to existing analog conventional knobs, keyed by zone + knob. */
  convUpdates: Record<string, ConvKnobUpdate>;
  /** Scan knobs being added or edited, keyed by zone + knob. */
  scanKnobs: Record<string, ScanKnobEdit>;
  /** Zones to add, in creation order. */
  newZones: { zone_id: string; name: string }[];
  /** New names for existing zones, by zone ID. */
  zoneRenames: Record<string, string>;
  /** IDs of empty zones to delete. */
  deletedZones: string[];
  /** Every zone ID in the new order, if moved. */
  zoneOrder: string[] | null;
  /** Knobs copied to empty positions, keyed by the target zone + knob. */
  knobCopies: Record<string, KnobCopy>;
  /** Knobs moved or swapped, keyed by the source zone + knob. */
  knobMoves: Record<string, KnobMove>;
  /** Changed pager settings by key (only values that differ from the backup). */
  pagerSettings: Record<string, number>;
  /** Unsaved P25 system edits: original state and current draft. New systems carry their
   * band; systems copied from another backup also carry where they come from. */
  systems: Record<string, { original: SystemEdit; draft: SystemEdit; band?: string; copyFrom?: { backup: string; source_id: string } }>;
  /** Systems to delete on the next write. */
  deletedSystems: string[];
  /** Allowed Rx ranges (Hz) per system, from the pager's model. */
  bands: Record<string, [number, number][]>;
  /** Bands a new system can use. */
  profileBands: Band[];
  settings: Settings | null;
  busy: string | null;

  setPage: (p: Page) => void;
  refreshDevice: () => Promise<void>;
  refreshBackups: () => Promise<BackupInfo[]>;
  loadSettings: () => Promise<void>;
  setSettings: (s: Settings) => void;
  openBackup: (id: string) => Promise<void>;
  renameKnob: (zone: string, knob: number, name: string) => void;
  pendingFor: (zone: string, knob: number) => string | undefined;
  setKnobTalkgroups: (zone: string, knob: Knob, priority: string[], normal: string[]) => void;
  /** The knob's list including unsaved edits. */
  talkgroupsFor: (zone: string, knob: Knob) => { priority: string[]; normal: string[]; edited: boolean };
  loadSystemDraft: (systemId: string) => Promise<void>;
  updateSystem: (systemId: string, f: (d: SystemEdit) => SystemEdit) => void;
  revertSystem: (systemId: string) => void;
  deleteTalkgroup: (systemId: string, groupId: string) => void;
  /** A system's talkgroups including unsaved edits. */
  effectiveTalkgroups: (systemId: string) => Talkgroup[] | undefined;
  systemEdited: (systemId: string) => boolean;
  /** Start a new system in `band`; returns its ID. */
  addSystem: (band: string) => string;
  /** Add a copy of `system` from backup `backupId` as a new system; returns its ID. */
  copySystem: (backupId: string, system: BackupSystem) => Promise<string>;
  /** Delete an existing system on the next write, or discard a new one. */
  deleteSystem: (systemId: string) => void;
  restoreSystem: (systemId: string) => void;
  /** Apply a RadioReference import: into a new system in `band` (with its
   * name, WACN and System ID), or appended to an existing system's draft.
   * Returns the system's ID. */
  applyImport: (target: { band: string; name: string; wacn: number; sysid: number } | { systemId: string }, plan: RrPlan) => string;
  /** A system as the knob editor sees it, including unsaved and new systems. */
  systemView: (systemId: string) => P25System | undefined;
  /** The knob at `index` including a pending system assignment (also for empty positions). */
  effectiveKnob: (zone: Zone, index: number) => Knob | undefined;
  /** Put `systemId` on a knob. For an empty position `opts` sets mode, name and prompt. */
  assignSystem: (zone: Zone, index: number, systemId: string, opts?: { mode?: number; name?: string }) => void;
  updateKnobSystem: (zone: string, index: number, patch: Partial<KnobSystemEdit>) => void;
  clearKnobSystem: (zone: string, index: number) => void;
  /** Start an analog conventional channel on an empty position. */
  addConvKnob: (zone: string, index: number) => void;
  updateConvKnob: (zone: string, index: number, patch: Partial<ConvKnobEdit>) => void;
  clearConvKnob: (zone: string, index: number) => void;
  removeKnob: (zone: string, index: number) => void;
  undoRemoveKnob: (zone: string, index: number) => void;
  /** Change settings of an existing analog conventional knob (pass the full current values merged). */
  setConvUpdate: (zone: string, index: number, patch: Omit<ConvKnobUpdate, "zone" | "knob">) => void;
  /** Switch a trunked knob between TG Scan (16) and TG Monitor (48). */
  setKnobMode: (zone: Zone, index: number, mode: number) => void;
  /** Start a scan knob on an empty position, or start editing an existing scan knob's members. */
  startScanKnob: (zone: string, index: number, mode: number) => void;
  updateScanKnob: (zone: string, index: number, patch: Partial<ScanKnobEdit>) => void;
  clearScanKnob: (zone: string, index: number) => void;
  /** Why a position can't be copied, moved, or take a copy or move now (null: it can). A copy source may be copied again. */
  positionLock: (zone: string, knob: number, copySource?: boolean) => string | null;
  /** The pending move with an end at this position. */
  moveAt: (zone: string, knob: number) => KnobMove | undefined;
  copyKnobTo: (zone: string, knob: number, toZone: string, toKnob: number, name: string) => void;
  renameKnobCopy: (toZone: string, toKnob: number, name: string) => void;
  clearKnobCopy: (toZone: string, toKnob: number) => void;
  /** Move to an empty position, or swap with the knob there. */
  moveKnobTo: (zone: string, knob: number, toZone: string, toKnob: number) => void;
  /** Undo the move with an end at this position. */
  clearKnobMove: (zone: string, knob: number) => void;
  /** Analog conventional channels a scan can list, including pending ones. */
  scanCandidates: () => { channel_id: string; zone: string; knob: number; name: string; target: string }[];
  /** Zones as they'll be after the next write: renamed, added, deleted, ordered. */
  zonesView: () => Zone[];
  addZone: () => string;
  /** Rename a zone; pending knob edits follow it to the new name. */
  renameZone: (zoneId: string, name: string) => void;
  deleteZone: (zoneId: string) => void;
  moveZone: (zoneId: string, dir: -1 | 1) => void;
  /** Drop a pending zone reorder and pending deletes of existing zones. */
  undoZoneLayout: () => void;
  /** Pending changes other than zone ones (zone reorders and deletes must be written alone). */
  hasNonZoneEdits: () => boolean;
  setSetting: (key: string, value: number) => void;
  undoSettings: (keys: string[]) => void;
  pendingCount: () => number;
  edits: () => Edits;
  clearEdits: () => void;
  setBusy: (b: string | null) => void;
}

export const useStore = create<State>((set, get) => ({
  page: "pager",
  device: null,
  backups: [],
  profile: null,
  loadingProfile: false,
  renames: {},
  talkgroups: {},
  knobSystems: {},
  convKnobs: {},
  scanKnobs: {},
  removedKnobs: {},
  convUpdates: {},
  newZones: [],
  zoneRenames: {},
  deletedZones: [],
  zoneOrder: null,
  knobCopies: {},
  knobMoves: {},
  pagerSettings: {},
  systems: {},
  deletedSystems: [],
  bands: {},
  profileBands: [],
  settings: null,
  busy: null,

  setPage: (page) => set({ page }),
  setBusy: (busy) => set({ busy }),

  refreshDevice: async () => {
    if (get().busy) return; // never poll during a backup or write
    try {
      const device = await api.deviceStatus();
      if (device.state !== "busy") set({ device });
    } catch (e) {
      set({ device: { state: "disconnected", reason: String(e) } });
    }
  },

  refreshBackups: async () => {
    const backups = await api.getBackups();
    set({ backups });
    return backups;
  },

  loadSettings: async () => set({ settings: await api.getSettings() }),
  setSettings: (settings) => set({ settings }),

  openBackup: async (id) => {
    set({ loadingProfile: true });
    try {
      const profile = await api.loadProfile(id);
      set({ profile, renames: {}, talkgroups: {}, knobSystems: {}, convKnobs: {}, scanKnobs: {}, removedKnobs: {}, convUpdates: {}, newZones: [], zoneRenames: {}, deletedZones: [], zoneOrder: null, knobCopies: {}, knobMoves: {}, pagerSettings: {}, systems: {}, deletedSystems: [], bands: {}, profileBands: [], page: "zones" });
      api.systemBands(id).then((bands) => set({ bands: { ...bands, ...get().bands } })).catch(() => {});
      api.profileBands(id).then((profileBands) => set({ profileBands })).catch(() => {});
    } finally {
      set({ loadingProfile: false });
    }
  },

  renameKnob: (zone, knob, name) => {
    const renames = { ...get().renames };
    const original = get()
      .zonesView()
      .find((z) => z.name === zone)
      ?.knobs.find((k) => k.index === knob)?.name;
    if (name === original) delete renames[key(zone, knob)];
    else renames[key(zone, knob)] = { zone, knob, name };
    set({ renames });
  },

  pendingFor: (zone, knob) => get().renames[key(zone, knob)]?.name,

  setKnobTalkgroups: (zone, knob, priority, normal) => {
    if (get().knobSystems[key(zone, knob.index)]) return get().updateKnobSystem(zone, knob.index, { priority, normal });
    const talkgroups = { ...get().talkgroups };
    const orig = splitList(knob);
    if (sameList(orig.priority, priority) && sameList(orig.normal, normal)) delete talkgroups[key(zone, knob.index)];
    else talkgroups[key(zone, knob.index)] = { zone, knob: knob.index, priority, normal };
    set({ talkgroups });
  },

  talkgroupsFor: (zone, knob) => {
    const a = get().knobSystems[key(zone, knob.index)];
    if (a) return { priority: a.priority, normal: a.normal, edited: true };
    const e = get().talkgroups[key(zone, knob.index)];
    const list = e ? { priority: e.priority, normal: e.normal } : splitList(knob);
    // Talkgroups deleted from the system (unsaved) also leave the knob.
    const d = knob.system_id ? get().systems[knob.system_id] : undefined;
    if (d) {
      const alive = new Set(d.draft.talkgroups.map((t) => t.group_id));
      return { priority: list.priority.filter((g) => alive.has(g)), normal: list.normal.filter((g) => alive.has(g)), edited: !!e };
    }
    return { ...list, edited: !!e };
  },

  loadSystemDraft: async (systemId) => {
    const profile = get().profile;
    if (!profile || get().systems[systemId]) return;
    const original = await api.loadSystem(profile.backup.id, systemId);
    if (!get().systems[systemId]) set({ systems: { ...get().systems, [systemId]: { original, draft: structuredClone(original) } } });
  },

  updateSystem: (systemId, f) => {
    const cur = get().systems[systemId];
    if (!cur) return;
    set({ systems: { ...get().systems, [systemId]: { ...cur, draft: f(structuredClone(cur.draft)) } } });
  },

  revertSystem: (systemId) => {
    const cur = get().systems[systemId];
    if (cur) set({ systems: { ...get().systems, [systemId]: { ...cur, draft: structuredClone(cur.original) } } });
  },

  deleteTalkgroup: (systemId, groupId) => {
    get().updateSystem(systemId, (d) => ({ ...d, talkgroups: d.talkgroups.filter((t) => t.group_id !== groupId) }));
    // Drop it from any pending knob edits of this system too.
    const zones = get().zonesView();
    const talkgroups = { ...get().talkgroups };
    for (const [k, e] of Object.entries(talkgroups)) {
      const knob = zones.find((z) => z.name === e.zone)?.knobs.find((x) => x.index === e.knob);
      if (knob?.system_id !== systemId) continue;
      talkgroups[k] = { ...e, priority: e.priority.filter((g) => g !== groupId), normal: e.normal.filter((g) => g !== groupId) };
    }
    set({ talkgroups });
  },

  effectiveTalkgroups: (systemId) => {
    const d = get().systems[systemId];
    if (d) return d.draft.talkgroups.map((t) => ({ group_id: t.group_id, tgid: t.tgid, name: t.name, note: t.note }));
    return get().profile?.summary.systems.find((s) => s.id === systemId)?.talkgroup_list;
  },

  systemEdited: (systemId) => {
    const d = get().systems[systemId];
    return !!d && (!!d.band || JSON.stringify(d.draft) !== JSON.stringify(d.original));
  },

  addSystem: (band) => {
    const id = crypto.randomUUID().toLowerCase();
    const taken = new Set([...(get().profile?.summary.systems ?? []).map((s) => s.name.trim()), ...Object.values(get().systems).map((s) => s.draft.name.trim())]);
    let name = "New System";
    for (let n = 2; taken.has(name); n++) name = `New System ${n}`;
    const blank: SystemEdit = {
      system_id: id,
      name,
      wacn: 1,
      sysid: 1,
      sites: [{ id: null, name: "Site 1", rfss: 1, site: 1 }],
      control_channels: [],
      talkgroups: [{ group_id: ALL_GROUP_ID, tgid: 65535, name: "All", note: "" }],
    };
    const ranges = get().profileBands.find((b) => b.code === band)?.ranges ?? [];
    set({ systems: { ...get().systems, [id]: { original: blank, draft: structuredClone(blank), band } }, bands: { ...get().bands, [id]: ranges } });
    return id;
  },

  copySystem: async (backupId, system) => {
    const state = await api.loadSystem(backupId, system.id);
    const id = crypto.randomUUID().toLowerCase();
    const taken = new Set([...(get().profile?.summary.systems ?? []).map((s) => s.name.trim()), ...Object.values(get().systems).map((s) => s.draft.name.trim())]);
    let name = state.name.trim();
    for (let n = 2; taken.has(name); n++) name = `${state.name.trim().slice(0, 14 - String(n).length - 1).trimEnd()} ${n}`;
    const copy: SystemEdit = { ...state, system_id: id, name };
    const codes: string[] = system.band.match(/.{2}/g) ?? [];
    const ranges = get().profileBands.filter((b) => codes.includes(b.code)).flatMap((b) => b.ranges);
    set({
      systems: { ...get().systems, [id]: { original: copy, draft: structuredClone(copy), band: system.band, copyFrom: { backup: backupId, source_id: system.id } } },
      bands: { ...get().bands, [id]: ranges },
    });
    return id;
  },


  deleteSystem: (systemId) => {
    const systems = { ...get().systems };
    const isNew = !!systems[systemId]?.band;
    delete systems[systemId];
    // Knobs being moved onto it go back to how they were.
    const knobSystems = Object.fromEntries(Object.entries(get().knobSystems).filter(([, e]) => e.system_id !== systemId));
    set({ systems, knobSystems, deletedSystems: isNew ? get().deletedSystems : [...new Set([...get().deletedSystems, systemId])] });
  },

  systemView: (systemId) => {
    const base = get().profile?.summary.systems.find((s) => s.id === systemId);
    const d = get().systems[systemId]?.draft;
    if (!d) return base;
    const talkgroup_list = d.talkgroups.map((t) => ({ group_id: t.group_id, tgid: t.tgid, name: t.name, note: t.note }));
    return {
      id: systemId,
      name: d.name,
      wacn: d.wacn,
      system_id: d.sysid,
      sites: d.sites.map((s) => ({ name: s.name, rfss: String(s.rfss), site_id: String(s.site) })),
      control_channels_hz: d.control_channels.map((c) => c.rx_hz),
      talkgroups: talkgroup_list.length,
      talkgroup_list,
      in_use: base?.in_use ?? false,
    };
  },

  effectiveKnob: (zone, index) => {
    const stored = (z: string, i: number) => get().zonesView().find((x) => x.name === z)?.knobs.find((k) => k.index === i);
    const moveIn = Object.values(get().knobMoves).find((m) => m.to_zone === zone.name && m.to_knob === index);
    if (moveIn) {
      const k = stored(moveIn.zone, moveIn.knob);
      return k && { ...k, index };
    }
    const moveOut = get().knobMoves[key(zone.name, index)];
    if (moveOut) {
      const k = moveOut.swap ? stored(moveOut.to_zone, moveOut.to_knob) : undefined;
      return k && { ...k, index };
    }
    const copy = get().knobCopies[key(zone.name, index)];
    if (copy) {
      const k = stored(copy.zone, copy.knob);
      return k && { ...k, index, name: copy.name, channel_id: copy.channel_id };
    }
    const orig = zone.knobs.find((k) => k.index === index);
    const sc = get().scanKnobs[key(zone.name, index)];
    if (sc)
      return {
        index,
        name: sc.name ?? orig?.name ?? "",
        mode: sc.mode,
        target: `${sc.members.length} channels`,
        talkgroups: 0,
        system_id: null,
        talkgroup_list: [],
        scan_members: sc.members,
        scan_priority: sc.priority,
      };
    const c = get().convKnobs[key(zone.name, index)];
    if (c) return { index, name: c.name, mode: 33, target: `${(c.rx_hz / 1e6).toFixed(5)} MHz`, talkgroups: 0, system_id: null, talkgroup_list: [] };
    const a = get().knobSystems[key(zone.name, index)];
    if (!a) {
      if (get().removedKnobs[key(zone.name, index)]) return undefined;
      const u = get().convUpdates[key(zone.name, index)];
      if (orig && u?.rx_hz !== undefined) return { ...orig, target: `${(u.rx_hz / 1e6).toFixed(5)} MHz` };
      return orig;
    }
    const list = [...a.priority.map((g) => ({ group_id: g, priority: true })), ...a.normal.map((g) => ({ group_id: g, priority: false }))];
    return {
      index,
      name: a.name ?? orig?.name ?? "",
      mode: a.mode,
      target: get().systemView(a.system_id)?.name ?? null,
      talkgroups: list.length,
      system_id: a.system_id,
      talkgroup_list: list,
    };
  },

  assignSystem: (zone, index, systemId, opts) => {
    const k = key(zone.name, index);
    // A position being removed counts as empty: the new knob replaces it.
    const orig = get().removedKnobs[k] ? undefined : zone.knobs.find((x) => x.index === index);
    const knobSystems = { ...get().knobSystems };
    if (orig?.system_id === systemId) {
      const mode = knobSystems[k]?.mode ?? orig.mode;
      if (mode === orig.mode) delete knobSystems[k];
      else knobSystems[k] = { ...knobSystems[k], system_id: systemId, ...splitList(orig) };
      return set({ knobSystems });
    }
    // Talkgroups belong to their system; only "All" carries over.
    const prev = knobSystems[k] ?? (orig ? { ...splitList(orig) } : { priority: [], normal: [] });
    const keep = (l: string[]) => l.filter((g) => g === ALL_GROUP_ID);
    const mode = knobSystems[k]?.mode ?? orig?.mode ?? opts?.mode ?? 16;
    knobSystems[k] = {
      zone: zone.name,
      knob: index,
      system_id: systemId,
      mode,
      channel_id: orig ? null : (knobSystems[k]?.channel_id ?? crypto.randomUUID().toLowerCase()),
      name: orig ? null : (knobSystems[k]?.name ?? opts?.name ?? `Knob ${index}`),
      voice_prompt: orig ? 0 : (knobSystems[k]?.voice_prompt ?? 0),
      priority: keep(prev.priority),
      normal: keep(prev.normal),
    };
    const talkgroups = { ...get().talkgroups };
    delete talkgroups[k];
    set({ knobSystems, talkgroups });
  },

  updateKnobSystem: (zone, index, patch) => {
    const k = key(zone, index);
    const cur = get().knobSystems[k];
    if (cur) set({ knobSystems: { ...get().knobSystems, [k]: { ...cur, ...patch } } });
  },

  clearKnobSystem: (zone, index) => {
    const knobSystems = { ...get().knobSystems };
    delete knobSystems[key(zone, index)];
    set({ knobSystems });
  },

  addConvKnob: (zone, index) => {
    const taken = new Set([...get().zonesView().flatMap((z) => z.knobs.map((k) => k.name)), ...Object.values(get().convKnobs).map((c) => c.name)]);
    let name = "New Channel";
    for (let n = 2; taken.has(name); n++) name = `New Channel ${n}`;
    // Replacing a removed knob keeps its name.
    if (get().removedKnobs[key(zone, index)]) name = get().zonesView().find((z) => z.name === zone)?.knobs.find((k) => k.index === index)?.name || name;
    // Start inside the pager's first band; the user types the real frequency.
    const rx = get().profileBands[0]?.ranges[0]?.[0] ?? 151_000_000;
    const edit: ConvKnobEdit = {
      zone,
      knob: index,
      name,
      rx_hz: rx,
      bandwidth: 1,
      squelch: { type: "carrier" },
      voice_prompt: 0,
      channel_id: crypto.randomUUID().toLowerCase(),
      freq_id: crypto.randomUUID().toLowerCase(),
    };
    set({ convKnobs: { ...get().convKnobs, [key(zone, index)]: edit } });
  },

  updateConvKnob: (zone, index, patch) => {
    const cur = get().convKnobs[key(zone, index)];
    if (!cur) return;
    const next = { ...cur, ...patch };
    // A digital channel can't be scanned: drop it from pending scan lists.
    set({ convKnobs: { ...get().convKnobs, [key(zone, index)]: next }, scanKnobs: next.digital ? withoutMembers(get().scanKnobs, [next.channel_id]) : get().scanKnobs });
  },

  clearConvKnob: (zone, index) => {
    const convKnobs = { ...get().convKnobs };
    const gone = convKnobs[key(zone, index)]?.channel_id;
    delete convKnobs[key(zone, index)];
    set({ convKnobs, scanKnobs: withoutMembers(get().scanKnobs, [gone]) });
  },

  removeKnob: (zone, index) => {
    const k = key(zone, index);
    const drop = <T,>(m: Record<string, T>) => Object.fromEntries(Object.entries(m).filter(([x]) => x !== k));
    const channel = get().zonesView().find((z) => z.name === zone)?.knobs.find((x) => x.index === index)?.channel_id;
    set({
      removedKnobs: { ...get().removedKnobs, [k]: { zone, knob: index } },
      renames: drop(get().renames),
      talkgroups: drop(get().talkgroups),
      knobSystems: drop(get().knobSystems),
      convUpdates: drop(get().convUpdates),
      scanKnobs: withoutMembers(drop(get().scanKnobs), [channel]),
    });
  },

  undoRemoveKnob: (zone, index) => {
    const k = key(zone, index);
    const drop = <T,>(m: Record<string, T>) => Object.fromEntries(Object.entries(m).filter(([x]) => x !== k));
    // Any replacement knob started on the position goes too, and a system
    // deleted after the removal comes back with the knob.
    const system = get().profile?.summary.zones.find((z) => z.name === zone)?.knobs.find((x) => x.index === index)?.system_id;
    set({
      removedKnobs: drop(get().removedKnobs),
      convKnobs: drop(get().convKnobs),
      knobSystems: drop(get().knobSystems),
      deletedSystems: get().deletedSystems.filter((x) => x !== system),
    });
  },

  setConvUpdate: (zone, index, patch) => {
    const k = key(zone, index);
    const cur = get().zonesView().find((z) => z.name === zone)?.knobs.find((x) => x.index === index)?.conv;
    if (!cur) return;
    const next: ConvKnobUpdate = { ...get().convUpdates[k], ...patch, zone, knob: index };
    // Keep only fields that differ from the knob as stored.
    const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
    for (const f of ["rx_hz", "bandwidth", "squelch", "alert_mode", "record_flag", "voice_prompt"] as const) {
      if (next[f] !== undefined && same(next[f], cur[f])) delete next[f];
    }
    const changed = ["rx_hz", "bandwidth", "squelch", "alert_mode", "record_flag", "voice_prompt"].some((f) => next[f as keyof ConvKnobUpdate] !== undefined);
    const convUpdates = { ...get().convUpdates };
    if (changed) convUpdates[k] = { ...next, freq_id: next.freq_id ?? crypto.randomUUID().toLowerCase() };
    else delete convUpdates[k];
    set({ convUpdates });
  },

  setKnobMode: (zone, index, mode) => {
    const k = key(zone.name, index);
    const orig = zone.knobs.find((x) => x.index === index);
    if (!orig?.system_id) return;
    const knobSystems = { ...get().knobSystems };
    const cur = knobSystems[k];
    const systemId = cur?.system_id ?? orig.system_id;
    const list = cur ? { priority: cur.priority, normal: cur.normal } : get().talkgroupsFor(zone.name, orig);
    if (mode === orig.mode && systemId === orig.system_id) {
      delete knobSystems[k];
      set({ knobSystems });
      // Keep a pending talkgroup change as an ordinary talkgroup edit.
      if (cur) get().setKnobTalkgroups(zone.name, orig, list.priority, list.normal);
      return;
    } else {
      knobSystems[k] = { zone: zone.name, knob: index, system_id: systemId, mode, channel_id: null, name: null, voice_prompt: 0, priority: list.priority, normal: list.normal };
      const talkgroups = { ...get().talkgroups };
      delete talkgroups[k];
      set({ talkgroups });
    }
    set({ knobSystems });
  },

  startScanKnob: (zone, index, mode) => {
    const orig = get().removedKnobs[key(zone, index)] ? undefined : get().zonesView().find((z) => z.name === zone)?.knobs.find((k) => k.index === index);
    const edit: ScanKnobEdit = orig
      ? { zone, knob: index, mode: orig.mode, members: orig.scan_members ?? [], priority: orig.scan_priority ?? null, name: null, voice_prompt: 0 }
      : { zone, knob: index, mode, members: [], priority: null, name: mode === 36 ? "Priority Scan" : "Scan", voice_prompt: 0 };
    set({ scanKnobs: { ...get().scanKnobs, [key(zone, index)]: edit } });
  },

  updateScanKnob: (zone, index, patch) => {
    const cur = get().scanKnobs[key(zone, index)];
    if (cur) set({ scanKnobs: { ...get().scanKnobs, [key(zone, index)]: { ...cur, ...patch } } });
  },

  clearScanKnob: (zone, index) => {
    const scanKnobs = { ...get().scanKnobs };
    delete scanKnobs[key(zone, index)];
    set({ scanKnobs });
  },

  positionLock: (zone, knob, copySource = false) => {
    const g = get();
    const k = key(zone, knob);
    if (g.renames[k] || g.talkgroups[k] || g.knobSystems[k] || g.convKnobs[k] || g.scanKnobs[k] || g.removedKnobs[k] || g.convUpdates[k]) return "has unsaved changes; write or undo them first";
    if (g.knobCopies[k]) return "gets a copy on the next write";
    if (g.moveAt(zone, knob)) return "is part of a move on the next write";
    if (!copySource && Object.values(g.knobCopies).some((c) => c.zone === zone && c.knob === knob)) return "is being copied; write or undo the copy first";
    return null;
  },

  moveAt: (zone, knob) => Object.values(get().knobMoves).find((m) => (m.zone === zone && m.knob === knob) || (m.to_zone === zone && m.to_knob === knob)),

  copyKnobTo: (zone, knob, toZone, toKnob, name) => {
    const src = get().zonesView().find((z) => z.name === zone)?.knobs.find((k) => k.index === knob);
    if (!src) return;
    const scan = src.mode === 4 || src.mode === 36;
    const copy: KnobCopy = { zone, knob, to_zone: toZone, to_knob: toKnob, name, channel_id: scan ? null : crypto.randomUUID().toLowerCase() };
    set({ knobCopies: { ...get().knobCopies, [key(toZone, toKnob)]: copy } });
  },

  renameKnobCopy: (toZone, toKnob, name) => {
    const cur = get().knobCopies[key(toZone, toKnob)];
    if (cur) set({ knobCopies: { ...get().knobCopies, [key(toZone, toKnob)]: { ...cur, name } } });
  },

  clearKnobCopy: (toZone, toKnob) => {
    const knobCopies = { ...get().knobCopies };
    const gone = knobCopies[key(toZone, toKnob)]?.channel_id;
    delete knobCopies[key(toZone, toKnob)];
    set({ knobCopies, scanKnobs: withoutMembers(get().scanKnobs, [gone]) });
  },

  moveKnobTo: (zone, knob, toZone, toKnob) => {
    const swap = !!get().zonesView().find((z) => z.name === toZone)?.knobs.some((k) => k.index === toKnob);
    set({ knobMoves: { ...get().knobMoves, [key(zone, knob)]: { zone, knob, to_zone: toZone, to_knob: toKnob, swap } } });
  },

  clearKnobMove: (zone, knob) => {
    const m = get().moveAt(zone, knob);
    if (!m) return;
    const knobMoves = { ...get().knobMoves };
    delete knobMoves[key(m.zone, m.knob)];
    set({ knobMoves });
  },

  scanCandidates: () => {
    const out: { channel_id: string; zone: string; knob: number; name: string; target: string }[] = [];
    for (const z of get().zonesView()) {
      for (const k of z.knobs) {
        if (!k.analog || !k.channel_id || get().convKnobs[key(z.name, k.index)] || get().removedKnobs[key(z.name, k.index)]) continue;
        // A moved channel is listed where it'll be.
        const m = get().moveAt(z.name, k.index);
        const at = !m ? { zone: z.name, knob: k.index } : m.zone === z.name && m.knob === k.index ? { zone: m.to_zone, knob: m.to_knob } : { zone: m.zone, knob: m.knob };
        out.push({ channel_id: k.channel_id, ...at, name: get().renames[key(z.name, k.index)]?.name ?? k.name, target: k.target ?? "" });
        for (const c of Object.values(get().knobCopies)) {
          if (c.zone === z.name && c.knob === k.index && c.channel_id) out.push({ channel_id: c.channel_id, zone: c.to_zone, knob: c.to_knob, name: c.name, target: k.target ?? "" });
        }
      }
    }
    for (const c of Object.values(get().convKnobs).filter((x) => !x.digital)) out.push({ channel_id: c.channel_id, zone: c.zone, knob: c.knob, name: c.name, target: `${(c.rx_hz / 1e6).toFixed(5)} MHz` });
    return out;
  },

  zonesView: () => {
    const { profile, newZones, zoneRenames, deletedZones, zoneOrder } = get();
    const all: Zone[] = [
      ...(profile?.summary.zones ?? []).filter((z) => !deletedZones.includes(z.id)).map((z) => ({ ...z, name: zoneRenames[z.id] ?? z.name })),
      ...newZones.map((z) => ({ id: z.zone_id, name: z.name, knobs: [], empty_knobs: [1, 2, 3, 4, 5, 6, 7, 8] })),
    ];
    if (!zoneOrder) return all;
    return zoneOrder.map((id) => all.find((z) => z.id === id)).filter((z): z is Zone => !!z);
  },

  addZone: () => {
    const taken = new Set(get().zonesView().map((z) => z.name.trim().toLowerCase()));
    let n = 1;
    while (taken.has(`zone ${n}`)) n++;
    const zone_id = crypto.randomUUID().toLowerCase();
    const zoneOrder = get().zoneOrder;
    set({ newZones: [...get().newZones, { zone_id, name: `Zone ${n}` }], zoneOrder: zoneOrder ? [...zoneOrder, zone_id] : null });
    return zone_id;
  },

  renameZone: (zoneId, name) => {
    const z = get().zonesView().find((x) => x.id === zoneId);
    if (!z || z.name === name) return;
    // The UI refuses a clashing name; never merge two zones' pending edits.
    if (get().zonesView().some((x) => x.id !== zoneId && x.name.trim().toLowerCase() === name.trim().toLowerCase())) return;
    const from = z.name;
    // Pending knob edits are keyed by zone name: move them to the new one.
    function move<T extends { zone: string; knob: number }>(m: Record<string, T>): Record<string, T> {
      return Object.fromEntries(Object.values(m).map((e) => (e.zone === from ? { ...e, zone: name } : e)).map((e) => [key(e.zone, e.knob), e]));
    }
    const { renames, talkgroups, knobSystems, convKnobs, newZones, zoneRenames, profile } = get();
    const isNew = newZones.some((x) => x.zone_id === zoneId);
    const original = profile?.summary.zones.find((x) => x.id === zoneId)?.name;
    const nextRenames = { ...zoneRenames };
    if (!isNew) {
      if (name === original) delete nextRenames[zoneId];
      else nextRenames[zoneId] = name;
    }
    const both = <T extends { zone: string; to_zone: string }>(e: T): T => ({ ...e, zone: e.zone === from ? name : e.zone, to_zone: e.to_zone === from ? name : e.to_zone });
    set({
      knobCopies: Object.fromEntries(Object.values(get().knobCopies).map(both).map((c) => [key(c.to_zone, c.to_knob), c])),
      knobMoves: Object.fromEntries(Object.values(get().knobMoves).map(both).map((m) => [key(m.zone, m.knob), m])),
      renames: move(renames),
      talkgroups: move(talkgroups),
      knobSystems: move(knobSystems),
      convKnobs: move(convKnobs),
      scanKnobs: move(get().scanKnobs),
      removedKnobs: move(get().removedKnobs),
      convUpdates: move(get().convUpdates),
      newZones: isNew ? newZones.map((x) => (x.zone_id === zoneId ? { ...x, name } : x)) : newZones,
      zoneRenames: nextRenames,
    });
  },

  deleteZone: (zoneId) => {
    const { newZones, deletedZones, zoneOrder } = get();
    const order = zoneOrder?.filter((id) => id !== zoneId) ?? null;
    if (newZones.some((z) => z.zone_id === zoneId)) {
      const z = newZones.find((x) => x.zone_id === zoneId)!;
      // Discarding a new zone drops the knobs started in it too.
      const keep = <T extends { zone: string }>(m: Record<string, T>) => Object.fromEntries(Object.entries(m).filter(([, e]) => e.zone !== z.name));
      const gone = [...Object.values(get().convKnobs).filter((c) => c.zone === z.name).map((c) => c.channel_id), ...Object.values(get().knobCopies).filter((c) => c.to_zone === z.name).map((c) => c.channel_id)];
      const away = <T extends { zone: string; to_zone: string }>(m: Record<string, T>) => Object.fromEntries(Object.entries(m).filter(([, e]) => e.zone !== z.name && e.to_zone !== z.name));
      return set({
        knobCopies: away(get().knobCopies),
        knobMoves: away(get().knobMoves),
        newZones: newZones.filter((x) => x.zone_id !== zoneId),
        zoneOrder: order,
        convKnobs: keep(get().convKnobs),
        knobSystems: keep(get().knobSystems),
        scanKnobs: withoutMembers(keep(get().scanKnobs), gone),
      });
    }
    set({ deletedZones: [...deletedZones, zoneId], zoneOrder: order });
  },

  moveZone: (zoneId, dir) => {
    const ids = get().zonesView().map((z) => z.id);
    const i = ids.indexOf(zoneId);
    const j = i + dir;
    if (i < 0 || j < 0 || j >= ids.length) return;
    [ids[i], ids[j]] = [ids[j], ids[i]];
    // Back to the original order means no reorder at all.
    const natural = [...(get().profile?.summary.zones ?? []).map((z) => z.id).filter((id) => !get().deletedZones.includes(id)), ...get().newZones.map((z) => z.zone_id)];
    set({ zoneOrder: ids.join() === natural.join() ? null : ids });
  },

  undoZoneLayout: () => set({ zoneOrder: null, deletedZones: [] }),

  hasNonZoneEdits: () => {
    const g = get();
    return Object.keys(g.renames).length + Object.keys(g.talkgroups).length + Object.keys(g.knobSystems).length + Object.keys(g.convKnobs).length + Object.keys(g.scanKnobs).length + Object.keys(g.removedKnobs).length + Object.keys(g.convUpdates).length + Object.keys(g.knobCopies).length + Object.keys(g.knobMoves).length + g.deletedSystems.length > 0 || Object.keys(g.systems).some((id) => g.systemEdited(id));
  },

  restoreSystem: (systemId) => set({ deletedSystems: get().deletedSystems.filter((x) => x !== systemId) }),

  applyImport: (target, plan) => {
    const isNew = "band" in target;
    const id = isNew ? get().addSystem(target.band) : target.systemId;
    const tgs = plan.talkgroups.map((t) => ({ group_id: crypto.randomUUID().toLowerCase(), tgid: t.tgid, name: t.name, note: t.note, imported: true }));
    get().updateSystem(id, (d) => ({
      ...d,
      ...(isNew ? { name: target.name, wacn: target.wacn, sysid: target.sysid } : {}),
      // A new system's placeholder site gives way to the imported ones.
      sites: isNew && plan.sites.length ? plan.sites : [...d.sites, ...plan.sites],
      control_channels: [...d.control_channels, ...plan.control_channels],
      talkgroups: [...d.talkgroups, ...tgs],
    }));
    return id;
  },

  setSetting: (key, value) => {
    const pagerSettings = { ...get().pagerSettings };
    if (get().profile?.summary.settings?.[key] === value) delete pagerSettings[key];
    else pagerSettings[key] = value;
    set({ pagerSettings });
  },

  undoSettings: (keys) => set({ pagerSettings: Object.fromEntries(Object.entries(get().pagerSettings).filter(([k]) => !keys.includes(k))) }),

  pendingCount: () =>
    Object.keys(get().pagerSettings).length +
    Object.keys(get().renames).length +
    Object.keys(get().talkgroups).length +
    Object.keys(get().knobSystems).length +
    Object.keys(get().convKnobs).length +
    Object.keys(get().scanKnobs).length +
    Object.keys(get().removedKnobs).length +
    Object.keys(get().convUpdates).length +
    Object.keys(get().knobCopies).length +
    Object.keys(get().knobMoves).length +
    get().newZones.length +
    Object.keys(get().zoneRenames).length +
    get().deletedZones.length +
    (get().zoneOrder ? 1 : 0) +
    Object.keys(get().systems).filter((id) => get().systemEdited(id)).length +
    get().deletedSystems.length,
  edits: () => {
    const entries = Object.values(get().systems);
    return {
      renames: Object.values(get().renames),
      talkgroups: Object.values(get().talkgroups),
      systems: entries.filter((e) => !e.band && get().systemEdited(e.draft.system_id)).map((e) => e.draft),
      new_systems: entries.filter((e) => e.band && !e.copyFrom).map((e) => ({ band: e.band!, system: e.draft })),
      copied_systems: entries.filter((e) => e.copyFrom).map((e) => ({ ...e.copyFrom!, system: e.draft })),
      deleted_systems: get().deletedSystems,
      knob_systems: Object.values(get().knobSystems),
      conv_knobs: Object.values(get().convKnobs)
        .filter((c) => !c.digital)
        .map(({ digital: _digital, ...c }) => c),
      digital_knobs: Object.values(get().convKnobs)
        .filter((c) => c.digital)
        .map((c) => ({
          zone: c.zone,
          knob: c.knob,
          name: c.name,
          rx_hz: c.rx_hz,
          protocol: c.digital!.protocol,
          talkgroups: c.digital!.talkgroups,
          repeater_mode: c.digital!.repeater_mode,
          voice_prompt: c.voice_prompt,
          channel_id: c.channel_id,
          freq_id: c.freq_id,
        })),
      new_zones: get().newZones,
      zone_renames: Object.entries(get().zoneRenames).map(([zone_id, name]) => ({ zone_id, name })),
      deleted_zones: get().deletedZones,
      zone_order: get().zoneOrder,
      scan_knobs: Object.values(get().scanKnobs),
      removed_knobs: Object.values(get().removedKnobs),
      conv_updates: Object.values(get().convUpdates),
      knob_copies: Object.values(get().knobCopies),
      knob_moves: Object.values(get().knobMoves),
      settings: get().pagerSettings,
    };
  },
  clearEdits: () => set({ renames: {}, talkgroups: {}, knobSystems: {}, convKnobs: {}, scanKnobs: {}, removedKnobs: {}, convUpdates: {}, newZones: [], zoneRenames: {}, deletedZones: [], zoneOrder: null, knobCopies: {}, knobMoves: {}, pagerSettings: {}, systems: {}, deletedSystems: [] }),
}));
