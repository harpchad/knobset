// Browser-only fake backend so the UI can be developed and previewed without a pager.
// All data here is invented.
import type { BackupInfo, BackupSystem, ClonePlan, Edits, P25System, Progress, ProfileView, RrSelection, RrSites, RrTalkgroups, Settings, SiteEdit, Summary, SystemEdit, Knob, KnobKind } from "./types";

const listeners = new Set<(p: Progress) => void>();
export function mockProgress(cb: (p: Progress) => void) {
  listeners.add(cb);
  return () => listeners.delete(cb);
}
const emit = (p: Progress) => listeners.forEach((l) => l(p));
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

const tg = (tgid: number, name: string, note = "") => ({ group_id: `tg-${tgid}`, tgid, name, note });
/** Invented per-talkgroup settings: the public works, parks and schools rows look like a silent RadioReference import. */
const tgSettings = (t: { tgid: number }) =>
  t.tgid >= 4000 && t.tgid < 5000
    ? { alert_mode: 3, alert_tone: "9999", voice_storage: false, background: 0 }
    : t.tgid === 1001
      ? { alert_mode: 4, alert_tone: "10000", voice_storage: true, background: 4 }
      : t.tgid === 9001
        ? { alert_mode: 1, alert_tone: "6f1d2c3b-4a59-4e87-9d6c-5b4a3f2e1d0c", voice_storage: true, background: 0 }
        : { alert_mode: 1, alert_tone: "9999", voice_storage: true, background: 0 };

const systems: P25System[] = [
  {
    id: "sys-metro",
    name: "Metro P25",
    wacn: 0xbee00,
    system_id: 0x3a1,
    sites: [
      { name: "Downtown", rfss: "1", site_id: "1" },
      { name: "North", rfss: "1", site_id: "2" },
      { name: "Airport", rfss: "1", site_id: "5" },
    ],
    control_channels_hz: [851_012_500, 851_287_500, 852_162_500, 852_887_500],
    talkgroups: 27,
    talkgroup_list: [
      tg(1001, "FD Dispatch", "Fire dispatch"),
      tg(1002, "FD Tac 1"),
      tg(1003, "FD Tac 2"),
      tg(1101, "EMS Dispatch", "Ambulance dispatch"),
      tg(1102, "EMS Hosp"),
      tg(2001, "PD North"),
      tg(2002, "PD South"),
      tg(3001, "Airport Ops"),
      tg(9001, "Interop 1", "Regional mutual aid"),
      ...["North", "South", "East", "West", "Central"].flatMap((d, i) => [
        tg(4001 + i * 10, `PW ${d}`, `Public works ${d.toLowerCase()} district`),
        tg(4002 + i * 10, `Parks ${d}`, `Parks and recreation, ${d.toLowerCase()}`),
        tg(4003 + i * 10, `Schools ${d}`, `School district ${d.toLowerCase()} buses`),
      ]),
      tg(5001, "Transit Ops", "Bus dispatch"),
      tg(5002, "Transit Sec", "Transit security"),
      tg(6001, "Hosp Net", "Hospital coordination"),
    ],
    in_use: true,
  },
  {
    id: "sys-state",
    name: "State Network",
    wacn: 0xbee00,
    system_id: 0x1ce,
    sites: [{ name: "County Simulcast", rfss: "3", site_id: "12" }],
    control_channels_hz: [769_406_250, 769_656_250],
    talkgroups: 4,
    talkgroup_list: [tg(10101, "Troop A"), tg(10102, "Troop A Tac"), tg(10500, "DOT District"), tg(11000, "State EOC")],
    in_use: true,
  },
];

/** Systems only the second demo pager has, so copying one shows something new. */
const otherSystems: P25System[] = [
  {
    id: "sys-county",
    name: "County VHF",
    wacn: 0xbee00,
    system_id: 0x2b7,
    sites: [{ name: "Courthouse", rfss: "1", site_id: "1" }, { name: "Water Tower", rfss: "1", site_id: "2" }],
    control_channels_hz: [155_115_000, 155_250_000],
    talkgroups: 3,
    talkgroup_list: [tg(301, "Sheriff Disp"), tg(302, "Sheriff Tac"), tg(310, "Road Dept")],
    in_use: true,
  },
  {
    id: "sys-uhf",
    name: "Campus UHF",
    wacn: 0xbee00,
    system_id: 0x4c2,
    sites: [{ name: "Main", rfss: "1", site_id: "1" }],
    control_channels_hz: [453_150_000],
    talkgroups: 2,
    talkgroup_list: [tg(501, "Campus PD"), tg(502, "Facilities")],
    in_use: true,
  },
];
const bandOf: Record<string, string> = { "sys-metro": "SX", "sys-state": "SX", "sys-county": "VX", "sys-uhf": "UC" };

const summary: Summary = {
  name: "Demo Profile",
  model: "G5D66BE-SXVXENB702UNI",
  conventional_freqs: 2,
  systems,
  dmr: true,
  p25c: true,
  settings: {
    "hold.enabled": 0,
    "hold.timeout": 0,
    "hold.unhold_alert": 0,
    "hold.remove": 0,
    "display.system_id": 0,
    "display.call_id": 0,
    "display.date": 0,
    "display.hour": 0,
    "display.rssi": 0,
    "alerts.receive_type": 2,
    "alerts.receive_volume": 5,
    "alerts.notify_type": 1,
    "alerts.notify_volume": 3,
    "alerts.oor_alert": 1,
    "alerts.oor_timer": 20,
    "alerts.in_range_timer": 5,
    "alerts.oor_repeat": 0,
    "alerts.oor_interval": 180,
    "alerts.oor_timeout": 900,
    "alerts.unread": 1,
    "alerts.unread_interval": 180,
    "backlight.mode": 1,
    "backlight.level": 5,
    "backlight.timeout": 10,
    "backlight.flash_on": 1,
    "backlight.flash_off": 1,
    "backlight.flash_duration": 0,
    "backlight.flash_voice": 0,
    "backlight.flash_text": 0,
    "backlight.flash_image": 0,
    "voice.record_limit": 600,
    ...Object.fromEntries([810, 811, 812, 816, 817, 820, 102, 103, 522, 841, 843, 844, 845, 846, 848, 849, 850, 853, 502].map((id) => [`menu.${id}`, 1])),
    "menu.814": 0,
    "menu.815": 0,
    "menu.852": 0,
  },
  zones: [
    {
      id: "a1a1a1a1-0000-4000-8000-000000000001",
      name: "Metro",
      knobs: [
        { index: 1, name: "Fire", mode: 16, target: "Metro P25", talkgroups: 3, system_id: "sys-metro", talkgroup_list: [{ group_id: "tg-1001", priority: true }, { group_id: "tg-1002", priority: false }, { group_id: "tg-1003", priority: false }] },
        { index: 2, name: "EMS", mode: 16, target: "Metro P25", talkgroups: 2, system_id: "sys-metro", talkgroup_list: [{ group_id: "tg-1101", priority: false }, { group_id: "tg-1102", priority: false }] },
        { index: 3, name: "Police", mode: 16, target: "Metro P25", talkgroups: 2, system_id: "sys-metro", talkgroup_list: [{ group_id: "tg-2001", priority: false }, { group_id: "tg-2002", priority: false }] },
        { index: 4, name: "Metro All", mode: 48, target: "Metro P25", talkgroups: 1, system_id: "sys-metro", talkgroup_list: [{ group_id: "tg-9001", priority: false }] },
        { index: 5, name: "Fireground", mode: 33, target: "154.28000 MHz", talkgroups: 0, system_id: null, talkgroup_list: [], channel_id: "c0000000-0000-4000-8000-000000000005", analog: true, conv: { rx_hz: 154280000, bandwidth: 1, squelch: { type: "ctcss", tenths: 1273 }, alert_mode: 1, record_flag: 1, voice_prompt: 0 } },
      ],
      empty_knobs: [6, 7, 8],
    },
    {
      id: "a1a1a1a1-0000-4000-8000-000000000002",
      name: "Highway",
      knobs: [
        { index: 1, name: "State Troop A", mode: 16, target: "State Network", talkgroups: 2, system_id: "sys-state", talkgroup_list: [{ group_id: "tg-10101", priority: false }, { group_id: "tg-10102", priority: false }] },
        { index: 2, name: "State All", mode: 48, target: "State Network", talkgroups: 0, system_id: "sys-state", talkgroup_list: [] },
        { index: 3, name: "Scan Locals", mode: 36, target: "3 channels", talkgroups: 0, system_id: null, talkgroup_list: [], scan_members: ["c0000000-0000-4000-8000-000000000204", "c0000000-0000-4000-8000-000000000005", "c0000000-0000-4000-8000-000000000205"], scan_priority: "c0000000-0000-4000-8000-000000000204" },
        { index: 4, name: "County Fire", mode: 33, target: "154.04000 MHz", talkgroups: 0, system_id: null, talkgroup_list: [], channel_id: "c0000000-0000-4000-8000-000000000204", analog: true, conv: { rx_hz: 154040000, bandwidth: 1, squelch: { type: "carrier" }, alert_mode: 3, record_flag: 0, voice_prompt: 0 } },
        { index: 5, name: "Mutual Aid", mode: 33, target: "154.19000 MHz", talkgroups: 0, system_id: null, talkgroup_list: [], channel_id: "c0000000-0000-4000-8000-000000000205", analog: true, conv: { rx_hz: 154190000, bandwidth: 1, squelch: { type: "dcs", code: "023", inverted: false }, alert_mode: 1, record_flag: 1, voice_prompt: 0 } },
      ],
      empty_knobs: [6, 7, 8],
    },
  ],
};

let backups: BackupInfo[] = [
  { id: "demo-20261017-165926Z", path: "/Users/demo/backups/demo-20261017-165926Z", model: summary.model, serial: "G5DEMO0001", sw_version: "V1.33-08182022", db_version: "DBV1.41b", taken: "20261017-165926Z", taken_at: "2026-10-17T16:59:26Z", sidecars_ok: true, radio_db_sidecar_stale: false },
  { id: "demo-20261016-091200Z", path: "/Users/demo/backups/demo-20261016-091200Z", model: summary.model, serial: "G5DEMO0001", sw_version: "V1.33-08182022", db_version: "DBV1.41b", taken: "20261016-091200Z", taken_at: "2026-10-16T09:12:00Z", sidecars_ok: true, radio_db_sidecar_stale: false },
  { id: "demo2-20261015-180500Z", path: "/Users/demo/backups/demo2-20261015-180500Z", model: summary.model, serial: "G5DEMO0002", sw_version: "V1.33-08182022", db_version: "DBV1.41b", taken: "20261015-180500Z", taken_at: "2026-10-15T18:05:00Z", sidecars_ok: true, radio_db_sidecar_stale: false },
  { id: "demo3-20261014-120000Z", path: "/Users/demo/backups/demo3-20261014-120000Z", model: "G5B67BF-SXVXEN1401UNI-0002-180625", serial: "G5DEMO0003", sw_version: "V2.02", db_version: "DBV1.41c", taken: "20261014-120000Z", taken_at: "2026-10-14T12:00:00Z", sidecars_ok: true, radio_db_sidecar_stale: false },
];
let settings: Settings = { templates_dir: "/Users/demo/Unication/G_Series_PPS/DataBase", templates_ok: true, backups_dir: "/Users/demo/backups", pager_addr: "10.10.0.12:32769", mobile: false };

const kindOf = (k: Knob): KnobKind => (k.system_id ? "trunked" : k.mode === 4 || k.mode === 36 ? "scan" : k.analog ? "conventional" : "digital");

/** A zone by the name knob edits use: its new name if the batch renames it. */
const zoneNamed = (edits: Edits, name: string) => summary.zones.find((z) => (edits.zone_renames ?? []).find((r) => r.zone_id === z.id)?.name === name || z.name === name);

/** Apply copies, then moves, to the fake profile the way the backend does. */
function applyCopiesAndMoves(edits: Edits) {
  const zoneOf = (name: string) => zoneNamed(edits, name);
  const place = (zone: string, k: Knob) => {
    const z = zoneOf(zone);
    if (!z) return;
    z.knobs = [...z.knobs.filter((x) => x.index !== k.index), k].sort((a, b) => a.index - b.index);
    z.empty_knobs = z.empty_knobs.filter((i) => i !== k.index);
  };
  const take = (zone: string, index: number) => {
    const z = zoneOf(zone);
    const k = z?.knobs.find((x) => x.index === index);
    if (!z || !k) return undefined;
    z.knobs = z.knobs.filter((x) => x !== k);
    z.empty_knobs = [...z.empty_knobs, index].sort((a, b) => a - b);
    return k;
  };
  for (const c of edits.knob_copies ?? []) {
    const k = zoneOf(c.zone)?.knobs.find((x) => x.index === c.knob);
    if (k) place(c.to_zone, { ...structuredClone(k), index: c.to_knob, name: c.name, channel_id: c.channel_id });
  }
  for (const m of edits.knob_moves ?? []) {
    const a = take(m.zone, m.knob);
    const b = m.swap ? take(m.to_zone, m.to_knob) : undefined;
    if (a) place(m.to_zone, { ...a, index: m.to_knob });
    if (b) place(m.zone, { ...b, index: m.knob });
  }
}

function stamp(d: Date) {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getUTCFullYear()}${p(d.getUTCMonth() + 1)}${p(d.getUTCDate())}-${p(d.getUTCHours())}${p(d.getUTCMinutes())}${p(d.getUTCSeconds())}Z`;
}

async function fakeBackup(): Promise<BackupInfo> {
  for (let i = 1; i <= 14; i++) {
    emit({ message: "Reading profile files", done: i, total: 14 });
    await sleep(80);
  }
  const now = new Date();
  now.setMilliseconds(0);
  const t = stamp(now);
  const b = { ...backups[0], id: `demo-${t}`, path: `/Users/demo/backups/demo-${t}`, taken: t, taken_at: now.toISOString().replace(".000Z", "Z") };
  backups = [b, ...backups];
  return b;
}

export async function mockInvoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  await sleep(150);
  switch (cmd) {
    case "get_settings":
      return settings as T;
    case "set_templates_dir":
      settings = { ...settings, templates_dir: String(args.path), templates_ok: true };
      return settings as T;
    case "import_templates":
      await sleep(1500);
      settings = { ...settings, templates_dir: "/Users/demo/Library/Application Support/com.harpchad.knobset/templates", templates_ok: true };
      return settings as T;
    case "set_backups_dir":
      settings = { ...settings, backups_dir: String(args.path) };
      return settings as T;
    case "device_status":
      return { state: "connected", device: { model: summary.model, serial: "G5DEMO0001", radio_number: "User-001", sw_version: "V1.33-08182022", internal_version: "V1.33-08182022", db_version: "DBV1.41b", hw_version: "GSXX-DEMO", db_supported: true } } as T;
    case "get_backups":
      return backups as T;
    case "backup_now":
      return (await fakeBackup()) as T;
    case "load_profile": {
      const b = backups.find((x) => x.id === args.backupId)!;
      return { backup: b, summary, is_latest: b.id === backups[0].id } satisfies ProfileView as T;
    }
    case "load_system": {
      const sys = [...systems, ...otherSystems].find((x) => x.id === args.systemId)!;
      return {
        system_id: sys.id,
        name: sys.name,
        wacn: sys.wacn,
        sysid: sys.system_id,
        sites: sys.sites.map((x, i) => ({ id: String(i + 1), name: x.name, rfss: +x.rfss, site: +x.site_id })),
        control_channels: sys.control_channels_hz.map((hz, i) => ({ id: i + 1, rx_hz: hz, note: "" })),
        talkgroups: sys.talkgroup_list.map((t) => ({ group_id: t.group_id, tgid: t.tgid, name: t.name, note: t.note, ...tgSettings(t) })),
      } satisfies SystemEdit as T;
    }
    case "system_bands":
      return { "sys-metro": [[763000000, 776000000], [851000000, 870000000]], "sys-state": [[763000000, 776000000], [851000000, 870000000]] } as T;
    case "profile_bands":
      return [
        { code: "SX", name: "700/800", comment: "(763~776, 851~870 MHz)", ranges: [[763000000, 776000000], [851000000, 870000000]] },
        { code: "VX", name: "VHF", comment: "(136~174 MHz)", ranges: [[136000000, 174000000]] },
      ] as T;
    // RadioReference import: canned tables (the real parser is Rust), a simple plan.
    case "rr_parse_sites":
      if (!String(args.text).trim().length) throw "no sites found";
      return {
        marked: true,
        problems: [{ line: 7, message: "frequency \"85x.1\" isn't a number of MHz; left out" }],
        sites: [
          { line: 2, rfss: 1, site: 1, nac: "3A1", description: "North County Simulcast", county: "North", freqs: [{ hz: 851012500, control: true }, { hz: 852162500, control: true }, { hz: 851287500, control: false }, { hz: 853112500, control: false }] },
          { line: 3, rfss: 1, site: 2, nac: "3A2", description: "South Tower", county: "South", freqs: [{ hz: 852162500, control: true }, { hz: 855387500, control: true }, { hz: 854062500, control: false }] },
          { line: 4, rfss: 2, site: 10, nac: "3B0", description: "East Lake 700", county: "East", freqs: [{ hz: 772706250, control: true }, { hz: 773881250, control: false }] },
        ],
      } as T;
    case "rr_parse_talkgroups": {
      if (!String(args.text).trim().length) throw "no talkgroups found";
      const cats = ["City Police", "City Fire", "County Sheriff", "Public Works", "Hospitals"];
      const modes: [string, string][] = [["D", "clear"], ["DE", "full"], ["De", "partial"], ["D", "clear"], ["T", "clear"]];
      const talkgroups = Array.from({ length: 60 }, (_, i) => {
        const [mode, encryption] = modes[i % modes.length];
        const cat = cats[Math.floor(i / 12)];
        return { line: i + 2, tgid: 1000 + i * 3, alpha_tag: `${cat.split(" ")[1]} ${["Dispatch", "Tac", "Ops", "Admin Channel", "Car-to-Car"][i % 5]} ${i}`, mode, description: `${cat} ${i}`, tag: "Law Dispatch", category: cat, encryption };
      });
      return { talkgroups, problems: [{ line: 30, message: "TGID 1084 is listed again (first on line 29); kept the first" }] } as T;
    }
    case "rr_plan": {
      const { existing, ranges, sites, talkgroups, selection } = args as {
        existing: SystemEdit | null; ranges: [number, number][]; sites: RrSites; talkgroups: RrTalkgroups; selection: RrSelection;
      };
      const keep = new Set(selection.sites.map(([r, x]) => `${r}-${x}`));
      const tg = new Set(selection.tgids);
      const haveHz = new Set(existing?.control_channels.map((c) => c.rx_hz));
      const haveTg = new Set(existing?.talkgroups.map((t) => t.tgid));
      const plan = { sites: [] as SiteEdit[], control_channels: [] as { id: null; rx_hz: number; note: string }[], talkgroups: [] as { tgid: number; name: string; note: string }[],
        sites_present: 0, control_channels_present: 0, talkgroups_present: 0, renamed: [] as { kind: string; from: string; to: string }[], out_of_band: [] as number[],
        not_in_import: { sites: [] as string[], control_channels: [] as number[], talkgroups: [] as { tgid: number; name: string; note: string }[] } };
      for (const x of sites.sites.filter((x) => keep.has(`${x.rfss}-${x.site}`))) {
        const name = x.description.slice(0, 14).trimEnd();
        if (name !== x.description) plan.renamed.push({ kind: "site", from: x.description, to: name });
        plan.sites.push({ id: null, name, rfss: x.rfss, site: x.site });
        for (const f of x.freqs.filter((f) => f.control || selection.all_freqs)) {
          if (!ranges.some(([lo, hi]) => f.hz >= lo && f.hz <= hi)) plan.out_of_band.push(f.hz);
          else if (!haveHz.has(f.hz)) (haveHz.add(f.hz), plan.control_channels.push({ id: null, rx_hz: f.hz, note: x.description }));
        }
      }
      for (const t of talkgroups.talkgroups.filter((t) => tg.has(t.tgid))) {
        if (haveTg.has(t.tgid)) { plan.talkgroups_present++; continue; }
        const name = t.alpha_tag.slice(0, 14).trimEnd();
        if (name !== t.alpha_tag) plan.renamed.push({ kind: "talkgroup", from: t.alpha_tag, to: name });
        plan.talkgroups.push({ tgid: t.tgid, name, note: t.description });
      }
      return plan as T;
    }
    case "plan_write": {
      const edits = args.edits as Edits;
      const noChange = {
        created: false, deleted: false, properties_changed: [] as string[], sites_added: 0, sites_removed: 0, sites_changed: 0, sites_reordered: false,
        ccs_added: 0, ccs_removed: 0, ccs_changed: 0, tgs_added: 0, tgs_removed: 0, tgs_changed: 0, tgs_settings_changed: 0, knob_entries_removed: 0, empty_knobs: [] as string[],
      };
      const knobOf = (zone: string, knob: number) => summary.zones.find((z) => z.name === zone)!.knobs.find((k) => k.index === knob)!;
      const tgRef = (id: string) => {
        const t = systems.flatMap((s) => s.talkgroup_list).find((x) => x.group_id === id)!;
        return { tgid: t.tgid, name: t.name };
      };
      return {
        renames: edits.renames.map((r) => ({ ...r, old: knobOf(r.zone, r.knob).name, new: r.name })),
        systems: [
          ...(edits.deleted_systems ?? []).map((id) => ({ ...noChange, system_id: id, name: systems.find((x) => x.id === id)!.name, deleted: true })),
          ...[...(edits.new_systems ?? []), ...(edits.copied_systems ?? [])].map(({ system: e }) => ({
            ...noChange, system_id: e.system_id, name: e.name, created: true,
            sites_added: e.sites.length, ccs_added: e.control_channels.length, tgs_added: e.talkgroups.filter((t) => t.group_id !== "0000000000000000").length,
          })),
          ...edits.systems.map((e) => {
          const o = systems.find((x) => x.id === e.system_id)!;
          const ids = new Set(o.talkgroup_list.map((t) => t.group_id));
          return {
            system_id: e.system_id, name: e.name,
            properties_changed: [e.name !== o.name && "name", e.wacn !== o.wacn && "WACN", e.sysid !== o.system_id && "System ID"].filter(Boolean) as string[],
            sites_added: e.sites.filter((x) => !x.id).length, sites_removed: Math.max(0, o.sites.length - e.sites.filter((x) => x.id).length), sites_changed: 0, sites_reordered: false,
            ccs_added: e.control_channels.filter((x) => x.id === null).length, ccs_removed: Math.max(0, o.control_channels_hz.length - e.control_channels.filter((x) => x.id !== null).length), ccs_changed: 0,
            tgs_added: e.talkgroups.filter((t) => !ids.has(t.group_id)).length, tgs_removed: o.talkgroup_list.filter((t) => !e.talkgroups.some((x) => x.group_id === t.group_id)).length,
            tgs_changed: e.talkgroups.filter((t) => o.talkgroup_list.some((x) => x.group_id === t.group_id && (x.name !== t.name || x.tgid !== t.tgid || x.note !== t.note))).length,
            tgs_settings_changed: e.talkgroups.filter((t) => {
              const x = o.talkgroup_list.find((y) => y.group_id === t.group_id);
              const s = x && tgSettings(x);
              return s && (s.alert_mode !== t.alert_mode || s.alert_tone !== t.alert_tone || s.voice_storage !== t.voice_storage || s.background !== t.background);
            }).length,
            knob_entries_removed: 0, empty_knobs: [], created: false, deleted: false,
          };
          }),
        ],
        talkgroups: edits.talkgroups.map((t) => {
          const k = knobOf(t.zone, t.knob);
          const before = k.talkgroup_list.map((x) => x.group_id);
          const after = [...t.priority, ...t.normal];
          const pri = k.talkgroup_list.filter((x) => x.priority).map((x) => x.group_id);
          return {
            zone: t.zone,
            knob: t.knob,
            knob_name: k.name,
            added: after.filter((g) => !before.includes(g)).map(tgRef),
            removed: before.filter((g) => !after.includes(g)).map(tgRef),
            priority_changed: pri.join() !== t.priority.join(),
            count: after.length,
          };
        }),
        knob_systems: (edits.knob_systems ?? []).map((k) => {
          const orig = knobOf(k.zone, k.knob);
          const sysName = (id: string) => systems.find((x) => x.id === id)?.name ?? edits.new_systems.find((n) => n.system.system_id === id)?.system.name ?? id;
          return {
            zone: k.zone, knob: k.knob, knob_name: k.name ?? orig?.name ?? "", new_knob: !orig, mode: k.mode,
            old_system: orig?.system_id ? sysName(orig.system_id) : null, new_system: sysName(k.system_id), talkgroups: k.priority.length + k.normal.length, mode_switched: !!orig && orig.mode !== k.mode,
          };
        }),
        zones: [
          ...(edits.new_zones ?? []).map((z) => ({ zone_id: z.zone_id, name: z.name, created: true, deleted: false, renamed_from: null, renumbers: false })),
          ...(edits.zone_renames ?? []).map((z) => ({ zone_id: z.zone_id, name: z.name, created: false, deleted: false, renamed_from: summary.zones.find((x) => x.id === z.zone_id)?.name ?? null, renumbers: false })),
          ...(edits.deleted_zones ?? []).map((id) => ({ zone_id: id, name: summary.zones.find((x) => x.id === id)?.name ?? id, created: false, deleted: true, renamed_from: null, renumbers: true })),
          ...(edits.zone_order ? [{ zone_id: edits.zone_order[0], name: "", created: false, deleted: false, renamed_from: null, renumbers: true }] : []),
        ],
        digital_knobs: (edits.digital_knobs ?? []).map((d) => ({
          zone: d.zone, knob: d.knob, knob_name: d.name, rx_hz: d.rx_hz, dmr: d.protocol.type === "dmr",
          protocol: d.protocol.type === "dmr" ? `DMR slot ${d.protocol.slot}, color code ${d.protocol.color_code}` : `P25 NAC ${d.protocol.nac.toString(16).toUpperCase().padStart(3, "0")}`,
          talkgroups: Math.max(1, d.talkgroups.length), new_groups: Math.max(1, d.talkgroups.length),
        })),
        experimental: [...new Set((edits.digital_knobs ?? []).map((d) => (d.protocol.type === "dmr" ? "DMR conventional channel" : "P25 conventional channel")))],
        removed_knobs: (edits.removed_knobs ?? []).map((r) => ({ zone: r.zone, knob: r.knob, knob_name: knobOf(r.zone, r.knob)?.name ?? "", kind: knobOf(r.zone, r.knob)?.system_id ? "trunked" : "conventional", scans_changed: [], priority_moved: [] })),
        conv_updates: (edits.conv_updates ?? []).map((u) => ({ zone: u.zone, knob: u.knob, knob_name: knobOf(u.zone, u.knob)?.name ?? "", changed: Object.keys(u).filter((k) => !["zone", "knob", "freq_id"].includes(k)) })),
        knob_copies: (edits.knob_copies ?? []).map((c) => {
          const k = zoneNamed(edits, c.zone)!.knobs.find((x) => x.index === c.knob)!;
          return { ...c, source_name: k.name, knob_name: c.name, kind: kindOf(k) };
        }),
        knob_moves: (edits.knob_moves ?? []).map((m) => {
          const at = (z: string, i: number) => zoneNamed(edits, z)!.knobs.find((x) => x.index === i)!;
          const k = at(m.zone, m.knob);
          const scans = summary.zones.flatMap((z) => z.knobs.filter((s) => k.channel_id && s.scan_members?.includes(k.channel_id)).map((s) => `${z.name} ${s.index} ${s.name}`));
          return { ...m, knob_name: k.name, kind: kindOf(k), swapped_with: m.swap ? at(m.to_zone, m.to_knob).name : null, scans };
        }),
        scan_knobs: (edits.scan_knobs ?? []).map((c) => ({ zone: c.zone, knob: c.knob, knob_name: c.name ?? knobOf(c.zone, c.knob)?.name ?? "", new_knob: c.name !== null, mode: c.mode, members: c.members, priority: c.priority })),
        conv_knobs: (edits.conv_knobs ?? []).map((c) => ({
          zone: c.zone, knob: c.knob, knob_name: c.name, rx_hz: c.rx_hz, bandwidth: c.bandwidth,
          squelch: c.squelch.type === "ctcss" ? `CTCSS ${(c.squelch.tenths / 10).toFixed(1)} Hz` : c.squelch.type === "dcs" ? `DCS ${c.squelch.code}${c.squelch.inverted ? " inverted" : ""}` : "carrier",
        })),
        settings: Object.entries(edits.settings ?? {}).map(([key, v]) => ({ key, old: summary.settings[key], new: v })),
        changed_tables: ["tabContacts", "tabRecvGroupList"],
        files: [["/defpara/Applications/PPS/u3db_PC.db", 437248], ["/defpara/Applications/PPS/u3db.db", 615424], ["/defpara/Applications/PPS/u3db.db.md5", 32], ["/defpara/Applications/PPS/u3db_PC.zip", 259852], ["/defpara/Applications/PPS/u3db_PC.zip.md5", 32]],
      } as T;
    }
    case "backup_systems": {
      const list = args.backupId === "demo2-20261015-180500Z" ? [...systems, ...otherSystems] : systems;
      return list.map((x) => ({
        id: x.id, name: x.name, band: bandOf[x.id] ?? "SX", wacn: x.wacn, sysid: x.system_id, sites: x.sites.length, control_channels: x.control_channels_hz.length, talkgroups: x.talkgroups,
      })) satisfies BackupSystem[] as T;
    }
    case "plan_clone": {
      const src = backups.find((b) => b.id === args.sourceId)!;
      const tgt = backups.find((b) => b.id === args.targetId)!;
      const [m1, m2] = [src.model.slice(0, 21), tgt.model.slice(0, 21)];
      if (m1 !== m2) throw `the source pager is a ${m1} and the target a ${m2} (hardware D vs B, protocols 66 vs 67, software BE vs BF, revision B702UNI vs 1401UNI); Knobset only writes a profile to another pager of the same model`;
      const side = (b: BackupInfo) => ({ backup_id: b.id, pager: { model: b.model, serial: b.serial, db_version: b.db_version }, profile: summary.name });
      const counts = { zones: summary.zones.length, knobs: summary.zones.reduce((n, z) => n + z.knobs.length, 0), systems: systems.length, talkgroups: systems.reduce((n, x) => n + x.talkgroups, 0), conventional_freqs: summary.conventional_freqs };
      return {
        summary: {
          source: side(src),
          target: side(tgt),
          kept: [
            { label: "Radio number", value: "User-002", differs: true },
            { label: "Pager alias", value: "Engine 7", differs: true },
            { label: "P25 trunking unit ID", value: "1234502", differs: true },
            { label: "Conventional unit IDs", value: "none", differs: false },
            { label: "Read password", value: "off", differs: false },
            { label: "Write password", value: "on", differs: true },
            { label: "PIN lock", value: "off", differs: false },
          ],
          before: { ...counts, zones: 1, knobs: 4, systems: 1, talkgroups: 27 },
          after: { ...counts, systems: counts.systems + otherSystems.length, talkgroups: counts.talkgroups + 5 },
          zones_added: ["Highway"],
          zones_removed: [],
          systems_added: ["State Network", ...otherSystems.map((x) => x.name)],
          systems_removed: [],
          regenerated: src.db_version !== tgt.db_version,
          identity_tables: ["tabAccessPassword", "tabMember_New", "tabPagerSUID"],
        },
        changed_tables: ["tabChannel", "tabContacts", "tabRadioBaseSetting", "tabReceivingMode", "tabRecvGroupList", "tabSystemAlias", "tabZone"],
        experimental: [],
        files: [["/defpara/Applications/PPS/u3db_PC.db", 437248], ["/defpara/Applications/PPS/u3db.db", 615424], ["/defpara/Applications/PPS/u3db.db.md5", 32], ["/defpara/Applications/PPS/u3db_PC.zip", 259852], ["/defpara/Applications/PPS/u3db_PC.zip.md5", 32]],
      } satisfies ClonePlan as T;
    }
    case "clone_to_pager": {
      emit({ message: "Connecting", done: 0, total: 0 });
      await sleep(400);
      const safety = await fakeBackup();
      const total = 1_312_588;
      for (let d = 0; d <= total; d += 65_536) {
        emit({ message: "Writing profile", done: Math.min(d, total), total });
        await sleep(60);
      }
      emit({ message: "Pager is rebooting to apply the profile", done: 0, total: 0 });
      await sleep(1500);
      emit({ message: "Verified", done: 0, total: 0 });
      return { safety_backup: safety, verified: ["u3db_PC.db", "u3db.db", "u3db.db.md5", "u3db_PC.zip", "u3db_PC.zip.md5"], new_backup: await fakeBackup() } as T;
    }
    case "write_to_pager": {
      emit({ message: "Connecting", done: 0, total: 0 });
      await sleep(400);
      const safety = await fakeBackup();
      const total = 1_312_588;
      for (let d = 0; d <= total; d += 65_536) {
        emit({ message: "Writing profile", done: Math.min(d, total), total });
        await sleep(60);
      }
      emit({ message: "Pager is rebooting to apply the profile", done: 0, total: 0 });
      await sleep(1500);
      const edits = args.edits as Edits;
      for (const r of edits.renames) {
        const k = summary.zones.find((z) => z.name === r.zone)?.knobs.find((x) => x.index === r.knob);
        if (k) k.name = r.name;
      }
      for (const t of edits.talkgroups) {
        const k = summary.zones.find((z) => z.name === t.zone)?.knobs.find((x) => x.index === t.knob);
        if (!k) continue;
        k.talkgroup_list = [...t.priority.map((g) => ({ group_id: g, priority: true })), ...t.normal.map((g) => ({ group_id: g, priority: false }))];
        k.talkgroups = k.talkgroup_list.length;
      }
      applyCopiesAndMoves(edits);
      summary.settings = { ...summary.settings, ...(edits.settings ?? {}) };
      emit({ message: "Verified", done: 0, total: 0 });
      return { safety_backup: safety, verified: ["u3db_PC.db", "u3db.db", "u3db.db.md5", "u3db_PC.zip", "u3db_PC.zip.md5"] } as T;
    }
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}
