//! Tauri backend: thin commands over `g5-core`. All pager I/O is blocking and
//! runs on a worker thread; a device lock keeps status polling from ever
//! interleaving with a backup or a write.

use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, anyhow};
use g5_codeplug::summary::Summary;
use g5_codeplug::{Profile, Templates};
use g5_core::Progress;
use g5_core::backup::{BackupInfo, backup, list_backups};
use g5_core::device::{DeviceSummary, authorize_read, connect, reachable};
use g5_core::write::{Edits, RenameApplied, TalkgroupChange, WriteResult, edit_set, program};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

mod keep_alive;

type CmdResult<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    /// Unication PPS `DataBase` folder (templates are never bundled).
    templates_dir: Option<PathBuf>,
    backups_dir: Option<PathBuf>,
    pager_addr: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { templates_dir: None, backups_dir: None, pager_addr: g5_proto::DEFAULT_ADDR.into() }
    }
}

struct AppState {
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    default_backups: PathBuf,
    /// Where [`import_templates`] puts the extracted `DataBase` folder.
    imported_templates: PathBuf,
    /// Held for every pager session. Holds the last handshake's summary while the
    /// pager stays reachable, so polling doesn't handshake (see [`device_status`]).
    device: Arc<Mutex<Option<DeviceSummary>>>,
}

impl AppState {
    fn snapshot(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    fn backups_dir(&self) -> PathBuf {
        self.snapshot().backups_dir.unwrap_or_else(|| self.default_backups.clone())
    }

    fn addr(&self) -> anyhow::Result<SocketAddr> {
        self.snapshot().pager_addr.parse().context("invalid pager address in settings")
    }

    fn save(&self, s: Settings) -> anyhow::Result<()> {
        if let Some(dir) = self.settings_path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&self.settings_path, serde_json::to_vec_pretty(&s)?)?;
        *self.settings.lock().unwrap() = s;
        Ok(())
    }

    fn backup_path(&self, id: &str) -> anyhow::Result<PathBuf> {
        if id.contains('/') || id.contains('\\') || id.starts_with('.') {
            return Err(anyhow!("invalid backup id"));
        }
        let p = self.backups_dir().join(id);
        if !p.join("MANIFEST.txt").is_file() {
            return Err(anyhow!("backup {id} not found"));
        }
        Ok(p)
    }

    fn templates(&self) -> anyhow::Result<PathBuf> {
        let dir = self.snapshot().templates_dir.context("choose the Unication PPS DataBase folder in Settings first")?;
        Templates::new(&dir)?;
        Ok(dir)
    }
}

#[derive(Serialize)]
struct SettingsView {
    templates_dir: Option<PathBuf>,
    templates_ok: bool,
    backups_dir: PathBuf,
    pager_addr: String,
    /// iOS: no folder pickers or Finder, so the UI hides those.
    mobile: bool,
    /// ARM64 build. On Windows that means no Unication driver; the hint differs.
    arm: bool,
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> SettingsView {
    let s = state.snapshot();
    SettingsView {
        templates_ok: s.templates_dir.as_deref().is_some_and(|d| Templates::new(d).is_ok()),
        templates_dir: s.templates_dir,
        backups_dir: state.backups_dir(),
        pager_addr: s.pager_addr,
        mobile: cfg!(mobile),
        arm: cfg!(target_arch = "aarch64"),
    }
}

/// Extract the templates from the PPS installer into app storage and use them.
/// `path` may be a plain path or a `file://` URL (what the iOS picker returns).
#[tauri::command]
async fn import_templates(state: State<'_, AppState>, path: String) -> CmdResult<SettingsView> {
    let src = match tauri::Url::parse(&path) {
        Ok(u) if u.scheme() == "file" => u.to_file_path().map_err(|_| format!("can't read {path}"))?,
        _ => PathBuf::from(&path),
    };
    let dest = state.imported_templates.clone();
    let imported = tauri::async_runtime::spawn_blocking(move || g5_codeplug::installer::import_installer(&src, &dest))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mut s = state.snapshot();
    s.templates_dir = Some(imported.templates.dir);
    state.save(s).map_err(err)?;
    Ok(get_settings(state))
}

#[tauri::command]
fn set_templates_dir(state: State<'_, AppState>, path: PathBuf) -> CmdResult<SettingsView> {
    // Accept either the DataBase folder itself or a folder containing it.
    let dir = [path.clone(), path.join("DataBase")].into_iter().find(|d| Templates::new(d).is_ok()).ok_or_else(|| {
        format!("{} doesn't look like the Unication PPS DataBase folder (no Data_G4A.db)", path.display())
    })?;
    let mut s = state.snapshot();
    s.templates_dir = Some(dir);
    state.save(s).map_err(err)?;
    Ok(get_settings(state))
}

#[tauri::command]
fn set_backups_dir(state: State<'_, AppState>, path: PathBuf) -> CmdResult<SettingsView> {
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    let mut s = state.snapshot();
    s.backups_dir = Some(path);
    state.save(s).map_err(err)?;
    Ok(get_settings(state))
}

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum DeviceStatus {
    Connected { device: DeviceSummary },
    Disconnected { reason: String },
    Busy,
}

/// Polled every few seconds. A handshake makes the pager show "PC Connected" and
/// interrupts scanning, so only the first poll after the pager appears handshakes;
/// later polls just check that it still accepts a connection. Unplugging clears the
/// cache, and so does any write (the pager reboots), so the next pager is read fresh.
#[tauri::command]
async fn device_status(state: State<'_, AppState>) -> CmdResult<DeviceStatus> {
    let addr = state.addr().map_err(err)?;
    let lock = state.device.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Ok(mut cached) = lock.try_lock() else { return DeviceStatus::Busy };
        if let Some(device) = cached.clone() {
            if reachable(addr) {
                return DeviceStatus::Connected { device };
            }
            *cached = None;
        }
        match connect(addr) {
            Ok((client, info)) => {
                client.bye();
                let device = DeviceSummary::from(&info);
                *cached = Some(device.clone());
                DeviceStatus::Connected { device }
            }
            Err(e) => DeviceStatus::Disconnected { reason: format!("{e:#}") },
        }
    })
    .await
    .map_err(|e| e.to_string())
}

fn emitter(app: &AppHandle) -> impl FnMut(Progress) + use<> {
    let app = app.clone();
    move |p: Progress| {
        let _ = app.emit("op-progress", p);
    }
}

#[tauri::command]
async fn backup_now(app: AppHandle, state: State<'_, AppState>) -> CmdResult<BackupInfo> {
    let (addr, root, lock) = (state.addr().map_err(err)?, state.backups_dir(), state.device.clone());
    let mut progress = emitter(&app);
    let _awake = keep_alive::hold(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let mut cached = lock.lock().unwrap();
        let (mut client, info) = connect(addr)?;
        *cached = Some(DeviceSummary::from(&info));
        let r = authorize_read(&mut client, None).and_then(|()| backup(&mut client, &info, &root, &mut progress));
        client.bye();
        r
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

#[tauri::command]
fn get_backups(state: State<'_, AppState>) -> Vec<BackupInfo> {
    list_backups(&state.backups_dir())
}

#[derive(Serialize)]
struct ProfileView {
    backup: BackupInfo,
    summary: Summary,
    /// True when this is the newest backup of its pager (edits are only written from the newest).
    is_latest: bool,
}

#[tauri::command]
async fn load_profile(state: State<'_, AppState>, backup_id: String) -> CmdResult<ProfileView> {
    let path = state.backup_path(&backup_id).map_err(err)?;
    let root = state.backups_dir();
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<ProfileView> {
        let backup = BackupInfo::read(&path).context("reading backup")?;
        let profile = Profile::from_backup(&path)?;
        let summary = Summary::read(&profile)?;
        // Newest backup of this pager, or one holding the same profile (e.g. the
        // safety backup of a write that stopped before programming).
        let is_latest = list_backups(&root)
            .iter()
            .find(|b| b.serial == backup.serial)
            .is_some_and(|b| b.id == backup.id || g5_core::write::same_profile(&path, std::path::Path::new(&b.path)));
        Ok(ProfileView { backup, summary, is_latest })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

/// Editable state of one P25 system in a backup.
#[tauri::command]
async fn load_system(state: State<'_, AppState>, backup_id: String, system_id: String) -> CmdResult<g5_codeplug::system_edit::SystemEdit> {
    let path = state.backup_path(&backup_id).map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<_> {
        let profile = Profile::from_backup(&path)?;
        Ok(g5_codeplug::system_edit::current_state(&profile, &system_id)?)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

/// Rx band ranges (Hz) the pager behind this backup accepts, per system.
#[tauri::command]
async fn system_bands(state: State<'_, AppState>, backup_id: String) -> CmdResult<std::collections::HashMap<String, Vec<(i64, i64)>>> {
    let path = state.backup_path(&backup_id).map_err(err)?;
    let templates = state.templates().map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<_> {
        let profile = Profile::from_backup(&path)?;
        let tpl = Templates::new(&templates)?;
        let mut out = std::collections::HashMap::new();
        for r in g5_codeplug::row::load_table(&profile.db, "tabP25TrunkingSecurity") {
            out.insert(r.s("P25TrunkingID").to_string(), g5_codeplug::system_edit::band_ranges(&tpl, r.s("BandTypeCode"))?);
        }
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

/// Bands a new system can use on the pager behind this backup.
#[tauri::command]
async fn profile_bands(state: State<'_, AppState>, backup_id: String) -> CmdResult<Vec<g5_codeplug::system_edit::Band>> {
    let path = state.backup_path(&backup_id).map_err(err)?;
    let templates = state.templates().map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<_> {
        let profile = Profile::from_backup(&path)?;
        Ok(g5_codeplug::system_edit::profile_bands(&profile, &Templates::new(&templates)?)?)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

use g5_codeplug::rr_import::{self, ImportPlan, Selection, Sites, Talkgroups};

fn rr_err(e: g5_codeplug::Error) -> String {
    match e {
        g5_codeplug::Error::Invalid(m) => m,
        e => e.to_string(),
    }
}

/// Parse a RadioReference site table (CSV download or pasted page text).
#[tauri::command]
fn rr_parse_sites(text: String) -> CmdResult<Sites> {
    rr_import::parse_sites(&text).map_err(rr_err)
}

/// Parse a RadioReference talkgroup table (CSV download or pasted page text).
#[tauri::command]
fn rr_parse_talkgroups(text: String) -> CmdResult<Talkgroups> {
    rr_import::parse_talkgroups(&text).map_err(rr_err)
}

/// What importing `selection` would add to `existing` (None: a new system).
#[tauri::command]
fn rr_plan(existing: Option<g5_codeplug::system_edit::SystemEdit>, ranges: Vec<(i64, i64)>, sites: Sites, talkgroups: Talkgroups, selection: Selection) -> CmdResult<ImportPlan> {
    rr_import::plan_import(existing.as_ref(), &ranges, &sites, &talkgroups, &selection).map_err(rr_err)
}

#[derive(Serialize)]
struct WritePlan {
    renames: Vec<RenameApplied>,
    talkgroups: Vec<TalkgroupChange>,
    systems: Vec<g5_codeplug::system_edit::SystemChange>,
    knob_systems: Vec<g5_codeplug::knob_system::KnobSystemChange>,
    conv_knobs: Vec<g5_codeplug::conv_knob::ConvKnobChange>,
    zones: Vec<g5_codeplug::zone_edit::ZoneChange>,
    scan_knobs: Vec<g5_codeplug::scan_knob::ScanKnobChange>,
    removed_knobs: Vec<g5_codeplug::knob_remove::KnobRemovalChange>,
    conv_updates: Vec<g5_codeplug::knob_remove::ConvKnobUpdateChange>,
    digital_knobs: Vec<g5_codeplug::digital_knob::DigitalKnobChange>,
    knob_copies: Vec<g5_codeplug::knob_copy::KnobCopyChange>,
    knob_moves: Vec<g5_codeplug::knob_copy::KnobMoveChange>,
    settings: Vec<g5_codeplug::settings::SettingChange>,
    /// Changes converted by PPS's rules but never checked on a pager.
    experimental: Vec<String>,
    changed_tables: Vec<String>,
    files: Vec<(String, usize)>,
}

fn build_set(path: &Path, templates: &Path, edits: &Edits) -> anyhow::Result<g5_core::write::WriteSet> {
    edit_set(path, templates, edits)
}

/// The app names the backup a copied system comes from by ID; the core wants its folder.
fn resolve_copies(state: &AppState, edits: &mut Edits) -> anyhow::Result<()> {
    for c in &mut edits.copied_systems {
        c.backup = state.backup_path(&c.backup.to_string_lossy())?;
    }
    Ok(())
}

#[tauri::command]
async fn plan_write(state: State<'_, AppState>, backup_id: String, mut edits: Edits) -> CmdResult<WritePlan> {
    let path = state.backup_path(&backup_id).map_err(err)?;
    resolve_copies(&state, &mut edits).map_err(err)?;
    let templates = state.templates().map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<WritePlan> {
        let set = build_set(&path, &templates, &edits)?;
        Ok(WritePlan {
            renames: set.renames,
            talkgroups: set.talkgroups,
            systems: set.systems,
            knob_systems: set.knob_systems,
            conv_knobs: set.conv_knobs,
            zones: set.zones,
            scan_knobs: set.scan_knobs,
            removed_knobs: set.removed_knobs,
            conv_updates: set.conv_updates,
            digital_knobs: set.digital_knobs,
            knob_copies: set.knob_copies,
            knob_moves: set.knob_moves,
            settings: set.settings,
            experimental: set.experimental,
            changed_tables: set.changed_tables,
            files: set.files.iter().map(|(p, d)| (p.to_string(), d.len())).collect(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

#[tauri::command]
async fn write_to_pager(app: AppHandle, state: State<'_, AppState>, backup_id: String, mut edits: Edits) -> CmdResult<WriteResult> {
    let path = state.backup_path(&backup_id).map_err(err)?;
    resolve_copies(&state, &mut edits).map_err(err)?;
    let templates = state.templates().map_err(err)?;
    let (addr, root, lock) = (state.addr().map_err(err)?, state.backups_dir(), state.device.clone());
    let mut progress = emitter(&app);
    let _awake = keep_alive::hold(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let mut cached = lock.lock().unwrap();
        *cached = None; // the pager reboots; the next poll reads it again
        progress(Progress::step("Preparing files"));
        let set = build_set(&path, &templates, &edits)?;
        program(addr, None, &path, &set, &root, &mut progress)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

/// A P25 system in some backup, for picking one to copy.
#[derive(Serialize)]
struct BackupSystem {
    id: String,
    name: String,
    /// Band code(s) of the system, e.g. `SX` or `SXVX`.
    band: String,
    wacn: i64,
    sysid: i64,
    sites: usize,
    control_channels: usize,
    talkgroups: usize,
}

#[tauri::command]
async fn backup_systems(state: State<'_, AppState>, backup_id: String) -> CmdResult<Vec<BackupSystem>> {
    let path = state.backup_path(&backup_id).map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<_> {
        let profile = Profile::from_backup(&path)?;
        let summary = Summary::read(&profile)?;
        let index = g5_codeplug::row::load_table(&profile.db, "tabP25TrunkingSecurity");
        Ok(summary
            .systems
            .into_iter()
            .map(|s| BackupSystem {
                band: index.iter().find(|r| r.s("P25TrunkingID") == s.id).map(|r| r.s("BandTypeCode").to_string()).unwrap_or_default(),
                sites: s.sites.len(),
                control_channels: s.control_channels_hz.len(),
                talkgroups: s.talkgroups,
                id: s.id,
                name: s.name,
                wacn: s.wacn,
                sysid: s.system_id,
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

#[derive(Serialize)]
struct ClonePlan {
    summary: g5_core::clone::CloneSummary,
    /// Radio DB tables that change on the target.
    changed_tables: Vec<String>,
    experimental: Vec<String>,
    files: Vec<(String, usize)>,
}

/// What writing `source_id`'s profile to the pager `target_id` is a backup of would do.
#[tauri::command]
async fn plan_clone(state: State<'_, AppState>, source_id: String, target_id: String) -> CmdResult<ClonePlan> {
    let (src, tgt) = (state.backup_path(&source_id).map_err(err)?, state.backup_path(&target_id).map_err(err)?);
    let templates = state.templates().map_err(err)?;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<ClonePlan> {
        let (set, summary) = g5_core::clone::clone_set(&src, &tgt, &templates)?;
        Ok(ClonePlan { summary, changed_tables: set.changed_tables, experimental: set.experimental, files: set.files.iter().map(|(p, d)| (p.to_string(), d.len())).collect() })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

/// Write `source_id`'s profile to the attached pager, which must be the one `target_id` is a backup of.
#[tauri::command]
async fn clone_to_pager(app: AppHandle, state: State<'_, AppState>, source_id: String, target_id: String) -> CmdResult<WriteResult> {
    let (src, tgt) = (state.backup_path(&source_id).map_err(err)?, state.backup_path(&target_id).map_err(err)?);
    let templates = state.templates().map_err(err)?;
    let (addr, root, lock) = (state.addr().map_err(err)?, state.backups_dir(), state.device.clone());
    let mut progress = emitter(&app);
    let _awake = keep_alive::hold(&app);
    tauri::async_runtime::spawn_blocking(move || {
        let mut cached = lock.lock().unwrap();
        *cached = None; // the pager reboots; the next poll reads it again
        progress(Progress::step("Preparing files"));
        let (set, summary) = g5_core::clone::clone_set(&src, &tgt, &templates)?;
        g5_core::clone::program_clone(addr, None, &summary, &tgt, &set, &root, &mut progress)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

/// On iOS, backups live in Documents/Backups, which the Files app shows as
/// On My iPad › Knobset › Backups. Desktop keeps them in app data.
fn default_backups(app: &tauri::App, data: &Path) -> tauri::Result<PathBuf> {
    if !cfg!(target_os = "ios") {
        return Ok(data.join("backups"));
    }
    let dir = app.path().document_dir()?.join("Backups");
    move_backups(&data.join("backups"), &dir);
    Ok(dir)
}

/// One-time move of backups taken before they lived in Documents. Skips any
/// name already present at the destination; failures only log.
fn move_backups(old: &Path, new: &Path) {
    let Ok(entries) = fs::read_dir(old) else { return };
    if let Err(e) = fs::create_dir_all(new) {
        return tracing::warn!("can't create {}: {e}", new.display());
    }
    for entry in entries.filter_map(|e| e.ok()) {
        let dest = new.join(entry.file_name());
        if !dest.exists()
            && let Err(e) = fs::rename(entry.path(), &dest)
        {
            tracing::warn!("can't move backup {}: {e}", entry.path().display());
        }
    }
    let _ = fs::remove_dir(old);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let config = app.path().app_config_dir()?;
            let data = app.path().app_data_dir()?;
            let settings_path = config.join("settings.json");
            let settings = fs::read(&settings_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
            app.manage(AppState {
                settings: Mutex::new(settings),
                settings_path,
                default_backups: default_backups(app, &data)?,
                imported_templates: data.join("templates"),
                device: Arc::new(Mutex::new(None)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_templates_dir,
            import_templates,
            set_backups_dir,
            device_status,
            backup_now,
            get_backups,
            load_profile,
            load_system,
            system_bands,
            profile_bands,
            rr_parse_sites,
            rr_parse_talkgroups,
            rr_plan,
            plan_write,
            write_to_pager,
            backup_systems,
            plan_clone,
            clone_to_pager,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
