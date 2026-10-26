// Mirrors the Rust types serialized by the Tauri commands.

export interface DeviceSummary {
  model: string;
  serial: string;
  radio_number: string;
  sw_version: string;
  internal_version: string;
  db_version: string;
  hw_version: string;
  db_supported: boolean;
}

export type DeviceStatus =
  | { state: "connected"; device: DeviceSummary }
  | { state: "disconnected"; reason: string }
  | { state: "busy" };

export interface BackupInfo {
  id: string;
  path: string;
  model: string;
  serial: string;
  sw_version: string;
  db_version: string;
  taken: string; // YYYYMMDD-HHMMSSZ (UTC); older backups have no Z
  taken_at: string; // RFC 3339 UTC instant
  sidecars_ok: boolean;
  radio_db_sidecar_stale: boolean;
}

export interface KnobTalkgroup {
  group_id: string;
  priority: boolean;
}

export interface Knob {
  index: number;
  name: string;
  mode: number;
  target: string | null;
  talkgroups: number;
  system_id: string | null;
  talkgroup_list: KnobTalkgroup[];
  /** ChannelID of a conventional or trunked knob (scan lists refer to it). */
  channel_id?: string | null;
  /** Conventional knob with analog squelch (scannable). */
  analog?: boolean;
  /** Scan knobs: member ChannelIDs in scan order. */
  scan_members?: string[];
  scan_priority?: string | null;
  /** Analog conventional knobs: editable settings. */
  conv?: ConvSettings | null;
}

export interface Zone {
  /** tabZone_New.ID */
  id: string;
  name: string;
  knobs: Knob[];
  /** Positions with no channel that a system can be put on. */
  empty_knobs: number[];
}

export interface Site {
  name: string;
  rfss: string;
  site_id: string;
}

export interface Talkgroup {
  group_id: string;
  tgid: number;
  name: string;
  note: string;
}

export interface P25System {
  id: string;
  name: string;
  wacn: number;
  system_id: number;
  sites: Site[];
  control_channels_hz: number[];
  talkgroups: number;
  talkgroup_list: Talkgroup[];
  in_use: boolean;
}

export interface Summary {
  name: string;
  model: string;
  zones: Zone[];
  systems: P25System[];
  conventional_freqs: number;
  /** The pager supports DMR / P25 conventional channels. */
  dmr?: boolean;
  p25c?: boolean;
  /** Device-wide settings by key, in the profile's encoding (notes/pager-settings.md). */
  settings: Record<string, number>;
}

export interface ProfileView {
  backup: BackupInfo;
  summary: Summary;
  is_latest: boolean;
}

export interface KnobRename {
  zone: string;
  knob: number;
  name: string;
}

export interface RenameApplied {
  zone: string;
  knob: number;
  old: string;
  new: string;
}

export interface KnobTalkgroupEdit {
  zone: string;
  knob: number;
  priority: string[];
  normal: string[];
}

export interface SiteEdit {
  id: string | null;
  name: string;
  rfss: number;
  site: number;
}

export interface ControlChannelEdit {
  id: number | null;
  rx_hz: number;
  note: string;
}

export interface TalkgroupEdit {
  group_id: string;
  tgid: number;
  name: string;
  note: string;
  /** From a RadioReference import: written silent with no voice storage, as PPS's importer does. */
  imported?: boolean;
  /** Settings: undefined keeps the stored value, or for a new row the editor's (or importer's) default.
   * `AlertMode` 1 tone, 2 vibrate, 3 silent, 4 tone and vibrate. */
  alert_mode?: number;
  /** `AlertToneID`: "9999" default, "10000" chirp, "1".."8", "0" none; anything else is a custom tone from PPS. */
  alert_tone?: string;
  /** `RecordFlag`. */
  voice_storage?: boolean;
  /** `BackgroundColor` 0..7. */
  background?: number;
}

/** Complete desired state of one P25 system. */
export interface SystemEdit {
  system_id: string;
  name: string;
  wacn: number;
  sysid: number;
  sites: SiteEdit[];
  control_channels: ControlChannelEdit[];
  talkgroups: TalkgroupEdit[];
}

/** A band a new system can use (from the pager's model number). */
export interface Band {
  code: string;
  name: string;
  comment: string;
  ranges: [number, number][];
}

/** A system to add: complete state plus its band. */
export interface NewSystem {
  band: string;
  system: SystemEdit;
}

/** A system copied from another backup ("Add System from Resources" in PPS). */
export interface SystemCopy {
  /** Backup ID holding the system. */
  backup: string;
  /** P25TrunkingID of the system in that backup. */
  source_id: string;
  /** New system_id; row IDs are the source's. */
  system: SystemEdit;
}

/** A P25 system in some backup, for picking one to copy. */
export interface BackupSystem {
  id: string;
  name: string;
  /** Band code(s), e.g. "SX" or "SXVX". */
  band: string;
  wacn: number;
  sysid: number;
  sites: number;
  control_channels: number;
  talkgroups: number;
}

export interface SystemChange {
  system_id: string;
  name: string;
  created: boolean;
  deleted: boolean;
  properties_changed: string[];
  sites_added: number;
  sites_removed: number;
  sites_changed: number;
  sites_reordered: boolean;
  ccs_added: number;
  ccs_removed: number;
  ccs_changed: number;
  tgs_added: number;
  tgs_removed: number;
  tgs_changed: number;
  /** Existing talkgroups whose alert, tone, voice storage or colour changed. */
  tgs_settings_changed: number;
  knob_entries_removed: number;
  empty_knobs: string[];
}

/** Put a system on a knob (empty position, or a trunked knob moving system). */
export interface KnobSystemEdit {
  zone: string;
  knob: number;
  system_id: string;
  /** 16 TG scan or 48 TG monitor. */
  mode: number;
  /** New channel GUID (empty positions only). */
  channel_id: string | null;
  /** Alias (empty positions only). */
  name: string | null;
  /** 0 none, or the mode's built-in prompt (empty positions only). */
  voice_prompt: number;
  priority: string[];
  normal: string[];
}

export interface KnobSystemChange {
  zone: string;
  knob: number;
  knob_name: string;
  new_knob: boolean;
  mode: number;
  old_system: string | null;
  new_system: string;
  talkgroups: number;
  mode_switched?: boolean;
}

export interface Edits {
  renames: KnobRename[];
  talkgroups: KnobTalkgroupEdit[];
  systems: SystemEdit[];
  new_systems: NewSystem[];
  copied_systems: SystemCopy[];
  deleted_systems: string[];
  knob_systems: KnobSystemEdit[];
  conv_knobs: ConvKnobEdit[];
  new_zones: { zone_id: string; name: string }[];
  zone_renames: { zone_id: string; name: string }[];
  deleted_zones: string[];
  zone_order: string[] | null;
  scan_knobs: ScanKnobEdit[];
  removed_knobs: { zone: string; knob: number }[];
  conv_updates: ConvKnobUpdate[];
  digital_knobs: {
    zone: string;
    knob: number;
    name: string;
    rx_hz: number;
    protocol: DigitalProtocol;
    talkgroups: { id: number; name: string }[];
    repeater_mode: boolean;
    voice_prompt: number;
    channel_id: string;
    freq_id: string;
  }[];
  knob_copies: KnobCopy[];
  knob_moves: KnobMove[];
  /** Changed pager settings by key. */
  settings: Record<string, number>;
}

/** Copy a knob to an empty position (g5_codeplug::knob_copy::KnobCopy). */
export interface KnobCopy {
  zone: string;
  knob: number;
  to_zone: string;
  to_knob: number;
  name: string;
  /** New channel GUID; null for scan knobs. */
  channel_id: string | null;
}

/** Move a knob to an empty position, or swap it with the knob there. */
export interface KnobMove {
  zone: string;
  knob: number;
  to_zone: string;
  to_knob: number;
  swap: boolean;
}

export type KnobKind = "trunked" | "conventional" | "digital" | "scan";

export interface KnobCopyChange {
  zone: string;
  knob: number;
  to_zone: string;
  to_knob: number;
  source_name: string;
  knob_name: string;
  kind: KnobKind;
}

export interface KnobMoveChange {
  zone: string;
  knob: number;
  to_zone: string;
  to_knob: number;
  knob_name: string;
  kind: KnobKind;
  swapped_with: string | null;
  /** Scans listing a moved channel ("zone knob name"); they follow it. */
  scans: string[];
}

/** A scan knob on an empty position, or new members for an existing one. */
export interface ScanKnobEdit {
  zone: string;
  knob: number;
  /** 4 = Normal Scan, 36 = Priority Scan. */
  mode: number;
  members: string[];
  priority: string | null;
  /** New scan knobs only. */
  name: string | null;
  voice_prompt: number;
}

export interface ScanKnobChange {
  zone: string;
  knob: number;
  knob_name: string;
  new_knob: boolean;
  mode: number;
  members: string[];
  priority: string | null;
}

export interface ZoneChange {
  zone_id: string;
  name: string;
  created: boolean;
  deleted: boolean;
  renamed_from: string | null;
  renumbers: boolean;
}

export interface ConvSettings {
  rx_hz: number;
  bandwidth: number;
  /** null when the knob has several tones. */
  squelch: Squelch | null;
  /** 1 tone, 2 vibrate, 3 silent, 4 tone + vibrate. */
  alert_mode: number;
  /** Voice storage: 0 off, 1 ID match, 2 all voice. */
  record_flag: number;
  voice_prompt: number;
}

/** Changes to an existing analog conventional knob; absent fields stay. */
export interface ConvKnobUpdate {
  zone: string;
  knob: number;
  rx_hz?: number;
  bandwidth?: number;
  squelch?: Squelch;
  alert_mode?: number;
  record_flag?: number;
  voice_prompt?: number;
  freq_id?: string;
}

/** How a conventional channel opens (g5_codeplug::conv_knob::Squelch). */
export type Squelch =
  | { type: "carrier" }
  /** Tenths of Hz: 1273 = 127.3 Hz. */
  | { type: "ctcss"; tenths: number }
  /** Octal code as written ("023"). */
  | { type: "dcs"; code: string; inverted: boolean };

export type DigitalProtocol = { type: "dmr"; slot: number; color_code: number } | { type: "p25c"; nac: number; filter_nac: boolean };

/** Digital settings of a new conventional knob (frontend only; sent as a DigitalKnobEdit). */
export interface DigitalSettings {
  protocol: DigitalProtocol;
  /** Decimal IDs; empty means the all-call placeholder. */
  talkgroups: { id: number; name: string }[];
  repeater_mode: boolean;
}

export interface DigitalKnobChange {
  zone: string;
  knob: number;
  knob_name: string;
  rx_hz: number;
  protocol: string;
  dmr: boolean;
  talkgroups: number;
  new_groups: number;
}

/** An analog conventional channel on an empty knob position. */
export interface ConvKnobEdit {
  /** Set for a DMR or P25 conventional knob (sent as digital_knobs). */
  digital?: DigitalSettings;
  zone: string;
  knob: number;
  name: string;
  rx_hz: number;
  /** 1 = 12.5 kHz, 2 = 25 kHz. */
  bandwidth: number;
  squelch: Squelch;
  /** 0 = none, 2 = built-in "Monitor Mode". */
  voice_prompt: number;
  channel_id: string;
  freq_id: string;
}

export interface ConvKnobChange {
  zone: string;
  knob: number;
  knob_name: string;
  rx_hz: number;
  bandwidth: number;
  squelch: string;
}

export interface SettingChange {
  key: string;
  old: number;
  new: number;
}

export interface TalkgroupRef {
  tgid: number;
  name: string;
}

export interface TalkgroupChange {
  zone: string;
  knob: number;
  knob_name: string;
  added: TalkgroupRef[];
  removed: TalkgroupRef[];
  priority_changed: boolean;
  count: number;
}

export interface WritePlan {
  renames: RenameApplied[];
  talkgroups: TalkgroupChange[];
  systems: SystemChange[];
  knob_systems: KnobSystemChange[];
  conv_knobs: ConvKnobChange[];
  zones: ZoneChange[];
  scan_knobs: ScanKnobChange[];
  removed_knobs: { zone: string; knob: number; knob_name: string; kind: KnobKind; scans_changed: string[]; priority_moved: string[] }[];
  conv_updates: { zone: string; knob: number; knob_name: string; changed: string[] }[];
  digital_knobs: DigitalKnobChange[];
  knob_copies?: KnobCopyChange[];
  knob_moves?: KnobMoveChange[];
  settings: SettingChange[];
  /** Converted by PPS's rules but never checked on a pager. */
  experimental: string[];
  changed_tables: string[];
  files: [string, number][];
}

/** Model, serial and radio DB version: what the write checks compare. */
export interface PagerId {
  model: string;
  serial: string;
  db_version: string;
}

export interface CloneSide {
  backup_id: string;
  pager: PagerId;
  /** Profile name. */
  profile: string;
}

export interface ProfileCounts {
  zones: number;
  knobs: number;
  systems: number;
  talkgroups: number;
  conventional_freqs: number;
}

/** An identity field the target pager keeps. */
export interface KeptField {
  label: string;
  value: string;
  /** The source's value differs. */
  differs: boolean;
}

export interface CloneSummary {
  source: CloneSide;
  target: CloneSide;
  kept: KeptField[];
  before: ProfileCounts;
  after: ProfileCounts;
  zones_added: string[];
  zones_removed: string[];
  systems_added: string[];
  systems_removed: string[];
  /** The radio DB is regenerated for the target's radio DB version. */
  regenerated: boolean;
  identity_tables: string[];
}

export interface ClonePlan {
  summary: CloneSummary;
  /** Radio DB tables that change on the target. */
  changed_tables: string[];
  experimental: string[];
  files: [string, number][];
}

export interface WriteResult {
  safety_backup: BackupInfo;
  verified: string[];
  /** Backup of the pager as written; null if that backup failed. */
  new_backup: BackupInfo | null;
}

export interface Progress {
  message: string;
  done: number;
  total: number;
}

export interface Settings {
  templates_dir: string | null;
  templates_ok: boolean;
  backups_dir: string;
  pager_addr: string;
  /** iOS: no folder pickers or Finder. */
  mobile: boolean;
  /** ARM64 build (Windows on ARM needs a different driver). */
  arm: boolean;
}

// RadioReference import (g5_codeplug::rr_import).

export interface RrProblem {
  line: number;
  message: string;
}

export interface RrFreq {
  hz: number;
  control: boolean;
}

export interface RrSite {
  line: number;
  rfss: number;
  site: number;
  nac: string;
  description: string;
  county: string;
  /** Control channels first, then by frequency. */
  freqs: RrFreq[];
}

export interface RrSites {
  sites: RrSite[];
  /** Any frequency marked as a control channel; without markers all count. */
  marked: boolean;
  problems: RrProblem[];
}

export type RrEncryption = "clear" | "partial" | "full";

export interface RrTalkgroup {
  line: number;
  tgid: number;
  alpha_tag: string;
  mode: string;
  description: string;
  tag: string;
  category: string;
  encryption: RrEncryption;
}

export interface RrTalkgroups {
  talkgroups: RrTalkgroup[];
  problems: RrProblem[];
}

export interface RrSelection {
  /** [RFSS, Site] pairs. */
  sites: [number, number][];
  tgids: number[];
  all_freqs: boolean;
}

export interface RrNewTalkgroup {
  tgid: number;
  name: string;
  note: string;
}

export interface RrPlan {
  sites: SiteEdit[];
  control_channels: ControlChannelEdit[];
  talkgroups: RrNewTalkgroup[];
  sites_present: number;
  control_channels_present: number;
  talkgroups_present: number;
  renamed: { kind: string; from: string; to: string }[];
  out_of_band: number[];
  not_in_import: { sites: string[]; control_channels: number[]; talkgroups: RrNewTalkgroup[] };
}
