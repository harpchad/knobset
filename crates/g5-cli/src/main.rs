//! `g5`: command-line access to Unication G-series pagers.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use g5_codeplug::compare::TableStatus;
use g5_codeplug::convert141::Variant;
use g5_codeplug::summary::{Summary, mode_label};
use g5_codeplug::{Profile, Templates};
use g5_core::Progress;
use g5_core::backup::{backup, manifest_field};
use g5_core::device::{authorize_read, connect, describe};
use g5_core::write::{Edits, KnobRename, WriteSet, edit_set, program, restore_set};
use g5_proto::DEFAULT_ADDR;

#[derive(Parser)]
#[command(version, about = "Tools for Unication G-series pagers")]
struct Cli {
    /// Pager address
    #[arg(long, global = true, default_value = DEFAULT_ADDR, env = "G5_ADDR")]
    addr: SocketAddr,
    /// Access password, if the pager has one set
    #[arg(long, global = true, env = "G5_PASSWORD", hide_env_values = true)]
    password: Option<String>,
    /// -v: protocol steps, -vv: hex dump of every packet
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Handshake and print what the pager reports about itself
    Info,
    /// List a directory on the pager
    Ls { path: String },
    /// Copy the pager's profile files into a timestamped local folder
    Backup {
        #[arg(long, default_value = "backups")]
        out: PathBuf,
    },
    /// Show systems, sites, talkgroups and zones in a backup (offline)
    Summary { backup: PathBuf },
    /// Generate the pager's radio DB from a backup's profile (offline)
    Convert {
        backup: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Radio DB version to target; defaults to the one recorded in the backup
        #[arg(long)]
        db_version: Option<String>,
        /// Folder holding Unication's template DBs (PPS `DataBase` folder)
        #[arg(long, env = "G5_TEMPLATES")]
        templates: PathBuf,
    },
    /// Write a backup back to the same pager, optionally with knob renames
    Restore {
        backup: PathBuf,
        /// Actually write. Without this, only the plan is shown.
        #[arg(long)]
        yes: bool,
        /// Where to put the automatic pre-write safety backup
        #[arg(long, default_value = "backups")]
        safety_out: PathBuf,
        /// Rename one knob first: ZONE KNOB NAME (repeatable)
        #[arg(long, num_args = 3, value_names = ["ZONE", "KNOB", "NAME"], action = clap::ArgAction::Append)]
        rename_knob: Vec<String>,
        /// Folder holding Unication's template DBs (needed with --rename-knob)
        #[arg(long, env = "G5_TEMPLATES")]
        templates: Option<PathBuf>,
    },
    /// Write one pager's backup to another pager of the same model, keeping the
    /// target's radio number, unit IDs and passwords
    Clone {
        /// Backup to copy the profile from
        source: PathBuf,
        /// The target pager's newest backup (the write refuses if the pager changed since)
        #[arg(long)]
        target: PathBuf,
        /// Actually write to the attached pager, which must be the target. Without this, only the plan is shown.
        #[arg(long)]
        yes: bool,
        /// Where to put the automatic pre-write safety backup
        #[arg(long, default_value = "backups")]
        safety_out: PathBuf,
        /// Folder holding Unication's template DBs (PPS `DataBase` folder)
        #[arg(long, env = "G5_TEMPLATES")]
        templates: PathBuf,
    },
    /// Compare two SQLite 2 databases table by table (offline)
    Compare { left: PathBuf, right: PathBuf },
    /// Preview a RadioReference import: the site and talkgroup CSVs, or the
    /// tables pasted from the system page into text files (offline)
    RrImport {
        /// Site table (trs_sites_*.csv, or pasted text)
        #[arg(long)]
        sites: PathBuf,
        /// Talkgroup table (trs_tg_*.csv, or pasted text)
        #[arg(long)]
        talkgroups: PathBuf,
        /// Backup to plan against; without it only the parse is shown
        #[arg(long)]
        backup: Option<PathBuf>,
        /// Folder holding Unication's template DBs (needed with --backup)
        #[arg(long, env = "G5_TEMPLATES")]
        templates: Option<PathBuf>,
        /// Band code; defaults to the band holding the most chosen frequencies
        #[arg(long)]
        band: Option<String>,
        /// Name of an existing system to add to, instead of a new one
        #[arg(long)]
        system: Option<String>,
        /// Every site frequency becomes a control channel, not only marked ones
        #[arg(long)]
        all_freqs: bool,
        /// Include fully encrypted talkgroups
        #[arg(long)]
        encrypted: bool,
    },
    /// Extract Unication's template DBs from the PPS installer (offline)
    ImportTemplates {
        /// The PPS download: the .zip, the setup .exe inside it, or PPS_Setup.msi
        installer: PathBuf,
        /// Folder to create (replaced if it exists)
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = match cli.verbose {
        0 => "warn",
        1 => "g5_proto=debug,g5_core=debug,warn",
        _ => "g5_proto=trace,g5::wire=trace,g5_core=debug,warn",
    };
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .with_writer(std::io::stderr)
        .init();
    let pw = cli.password.as_deref();

    match &cli.cmd {
        Cmd::Summary { backup } => summary(backup),
        Cmd::Convert { backup, out, db_version, templates } => convert(backup, out, db_version.as_deref(), templates),
        Cmd::Compare { left, right } => compare(left, right),
        Cmd::RrImport { sites, talkgroups, backup, templates, band, system, all_freqs, encrypted } => {
            rr_import(sites, talkgroups, backup.as_deref(), templates.as_deref(), band.as_deref(), system.as_deref(), *all_freqs, *encrypted)
        }
        Cmd::ImportTemplates { installer, out } => {
            let r = g5_codeplug::installer::import_installer(installer, out)?;
            println!("{} files ({} KB) in {}", r.files, r.bytes / 1024, r.templates.dir.display());
            Ok(())
        }
        Cmd::Restore { backup, yes, safety_out, rename_knob, templates } => restore(cli.addr, pw, backup, *yes, safety_out, rename_knob, templates.as_deref()),
        Cmd::Clone { source, target, yes, safety_out, templates } => clone(cli.addr, pw, source, target, *yes, safety_out, templates),
        Cmd::Info => {
            let (client, info) = connect(cli.addr)?;
            print!("{}", describe(&info));
            client.bye();
            Ok(())
        }
        Cmd::Ls { path } => {
            let (mut client, _) = connect(cli.addr)?;
            authorize_read(&mut client, pw)?;
            let r = client.list_dir(path).with_context(|| format!("listing {path}"));
            client.bye();
            for e in r? {
                println!("{:>10}  {}", e.size, e.name);
            }
            Ok(())
        }
        Cmd::Backup { out } => {
            let (mut client, info) = connect(cli.addr)?;
            let r = authorize_read(&mut client, pw).and_then(|()| backup(&mut client, &info, out, &mut print_progress));
            client.bye();
            let b = r?;
            let checks = if b.sidecars_ok {
                "pager MD5 checks OK"
            } else if b.radio_db_sidecar_stale {
                "radio DB MD5 stale, normal after a firmware update"
            } else {
                "pager MD5 checks FAILED"
            };
            println!("\nBackup written to {} ({checks})", b.path.display());
            Ok(())
        }
    }
}

fn print_progress(p: Progress) {
    if p.total > 0 {
        eprint!("\r{} ({}/{})        ", p.message, p.done, p.total);
        if p.done == p.total {
            eprintln!();
        }
    } else {
        eprintln!("{}", p.message);
    }
}

fn restore(addr: SocketAddr, pw: Option<&str>, src: &Path, yes: bool, safety_out: &Path, rename: &[String], templates: Option<&Path>) -> Result<()> {
    let set: WriteSet = if rename.is_empty() {
        restore_set(src)?
    } else {
        let renames = rename
            .chunks(3)
            .map(|c| Ok(KnobRename { zone: c[0].clone(), knob: c[1].parse().context("KNOB must be a number")?, name: c[2].clone() }))
            .collect::<Result<Vec<_>>>()?;
        edit_set(src, templates.context("--rename-knob needs --templates or G5_TEMPLATES")?, &Edits { renames, ..Default::default() })?
    };
    for r in &set.renames {
        println!("Rename {:?} knob {}: {:?} -> {:?}", r.zone, r.knob, r.old, r.new);
    }
    if !set.changed_tables.is_empty() {
        println!("Radio DB tables changed: {}", set.changed_tables.join(", "));
    }
    println!("Plan (one write business, set clock, commit):");
    for (path, data) in &set.files {
        println!("  {:>8} bytes  {path}", data.len());
    }
    if !yes {
        println!("\nDry run. Re-run with --yes to write.");
        return Ok(());
    }
    let r = program(addr, pw, src, &set, safety_out, &mut print_progress)?;
    println!("\nWrote and verified {} files. Safety backup: {}", r.verified.len(), r.safety_backup.path.display());
    Ok(())
}

fn clone(addr: SocketAddr, pw: Option<&str>, source: &Path, target: &Path, yes: bool, safety_out: &Path, templates: &Path) -> Result<()> {
    let (set, c) = g5_core::clone::clone_set(source, target, templates)?;
    let side = |s: &g5_core::clone::CloneSide| format!("{} {} ({}), profile {:?}", &s.pager.model[..21.min(s.pager.model.len())], s.pager.serial, s.pager.db_version, s.profile);
    println!("From: {}\nTo:   {}", side(&c.source), side(&c.target));
    if c.regenerated {
        println!("The radio DB is regenerated for the target's {}.", c.target.pager.db_version);
    }
    println!("\nThe target keeps its own:");
    for k in &c.kept {
        println!("  {:<22} {}{}", k.label, k.value, if k.differs { "  (the source's differs)" } else { "" });
    }
    let (b, a) = (&c.before, &c.after);
    println!("\nThe target's profile is replaced:");
    println!("  zones      {:>4} -> {}", b.zones, a.zones);
    println!("  knobs      {:>4} -> {}", b.knobs, a.knobs);
    println!("  systems    {:>4} -> {}", b.systems, a.systems);
    println!("  talkgroups {:>4} -> {}", b.talkgroups, a.talkgroups);
    for (label, list) in [("zones added", &c.zones_added), ("zones removed", &c.zones_removed), ("systems added", &c.systems_added), ("systems removed", &c.systems_removed)] {
        if !list.is_empty() {
            println!("  {label}: {}", list.join(", "));
        }
    }
    println!("\nRadio DB tables that change on the target: {}", if set.changed_tables.is_empty() { "none".to_string() } else { set.changed_tables.join(", ") });
    println!("Plan (one write business, set clock, commit):");
    for (path, data) in &set.files {
        println!("  {:>8} bytes  {path}", data.len());
    }
    if !yes {
        println!("\nDry run. Re-run with --yes to write to the attached pager (it must be {}).", c.target.pager.serial);
        return Ok(());
    }
    let r = g5_core::clone::program_clone(addr, pw, &c, target, &set, safety_out, &mut print_progress)?;
    println!("\nWrote and verified {} files. Safety backup: {}", r.verified.len(), r.safety_backup.path.display());
    Ok(())
}

fn summary(backup: &Path) -> Result<()> {
    let profile = Profile::from_backup(backup).with_context(|| format!("loading profile from {}", backup.display()))?;
    let s = Summary::read(&profile)?;
    println!("Profile: {}", s.name);
    println!("Model:   {}", s.model);
    println!("\nP25 systems ({}):", s.systems.len());
    for sys in &s.systems {
        println!(
            "  {:<14}  WACN {:05X}  SysID {:03X}  {:>2} sites  {:>3} control ch  {:>4} talkgroups{}",
            sys.name, sys.wacn, sys.system_id, sys.sites.len(), sys.control_channels_hz.len(), sys.talkgroups,
            if sys.in_use { "" } else { "  (not on any knob)" }
        );
    }
    println!("\nConventional frequencies: {}", s.conventional_freqs);
    for z in &s.zones {
        println!("\nZone {}:", z.name);
        for k in &z.knobs {
            let tg = if k.talkgroups > 0 { format!("  {} TGs", k.talkgroups) } else { String::new() };
            println!("  {:>2}  {:<14}  {:<20}  {}{tg}", k.index, k.name, mode_label(k.mode), k.target.as_deref().unwrap_or(""));
        }
    }
    Ok(())
}

fn convert(backup: &Path, out: &Path, db_version: Option<&str>, templates: &Path) -> Result<()> {
    let version = match db_version {
        Some(v) => v.to_string(),
        None => manifest_field(backup, "Radio DB version:").context("no --db-version given and none recorded in the backup")?,
    };
    let variant = Variant::for_db_version(&version).with_context(|| format!("radio DB version {version} is not supported yet (DBV1.41b to DBV1.41e are)"))?;
    let report = g5_codeplug::convert141::convert(&Profile::from_backup(backup)?, &Templates::new(templates)?, variant, out)?;
    println!("Wrote {} ({} statements, {version})", out.display(), report.statements);
    for u in &report.unverified {
        println!("warning: uses {u}, which the validation pagers did not cover");
    }
    Ok(())
}

fn compare(left: &Path, right: &Path) -> Result<()> {
    let (a, b) = (sqlite2::Connection::open(left)?, sqlite2::Connection::open(right)?);
    let diffs = g5_codeplug::compare::compare(&a, &b)?;
    let bad: Vec<_> = diffs.iter().filter(|d| d.status != TableStatus::Same).collect();
    for d in &bad {
        println!("{:<40} left={:<5} right={:<5} {:?}", d.table, d.left_rows, d.right_rows, d.status);
    }
    println!("{} tables, {} identical, {} different", diffs.len(), diffs.len() - bad.len(), bad.len());
    if !bad.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn rr_import(
    sites_path: &Path,
    tgs_path: &Path,
    backup: Option<&Path>,
    templates: Option<&Path>,
    band: Option<&str>,
    system: Option<&str>,
    all_freqs: bool,
    encrypted: bool,
) -> Result<()> {
    use g5_codeplug::rr_import::{Encryption, Selection, parse_sites, parse_talkgroups, plan_import};
    use g5_codeplug::system_edit::{current_state, profile_bands};

    let read = |p: &Path| std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()));
    let sites = parse_sites(&read(sites_path)?)?;
    let tgs = parse_talkgroups(&read(tgs_path)?)?;
    let freqs: usize = sites.sites.iter().map(|s| s.freqs.len()).sum();
    let marked: usize = sites.sites.iter().map(|s| s.freqs.iter().filter(|f| f.control).count()).sum();
    println!("{} sites, {freqs} frequencies ({marked} marked control)", sites.sites.len());
    let count = |e: Encryption| tgs.talkgroups.iter().filter(|t| t.encryption == e).count();
    println!(
        "{} talkgroups: {} clear, {} partly encrypted, {} encrypted",
        tgs.talkgroups.len(),
        count(Encryption::Clear),
        count(Encryption::Partial),
        count(Encryption::Full)
    );
    for (file, problems) in [(sites_path, &sites.problems), (tgs_path, &tgs.problems)] {
        for p in problems {
            println!("  {}:{}: {}", file.display(), p.line, p.message);
        }
    }
    if !sites.marked {
        println!("  no control channel markers: every frequency counts as a control channel");
    }

    let Some(backup) = backup else { return Ok(()) };
    let templates = Templates::new(templates.context("--backup needs --templates")?)?;
    let profile = Profile::from_backup(backup)?;
    let bands = profile_bands(&profile, &templates)?;
    let mut sel = Selection::defaults(&sites, &tgs);
    sel.all_freqs = all_freqs;
    if encrypted {
        sel.tgids = tgs.talkgroups.iter().map(|t| t.tgid).collect();
    }
    let chosen: Vec<i64> = sites.sites.iter().flat_map(|s| s.freqs.iter().filter(|f| f.control || all_freqs || !sites.marked).map(|f| f.hz)).collect();
    for b in &bands {
        let n = chosen.iter().filter(|&&hz| b.ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&hz))).count();
        println!("band {} ({}): {n} of {} chosen frequencies", b.code, b.name, chosen.len());
    }
    let summary = Summary::read(&profile)?;
    let existing = match system {
        Some(name) => {
            let s = summary.systems.iter().find(|s| s.name == name).with_context(|| format!("no system named {name:?} in the backup"))?;
            Some(current_state(&profile, &s.id)?)
        }
        None => None,
    };
    let in_band = |b: &&g5_codeplug::system_edit::Band| chosen.iter().filter(|&&hz| b.ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&hz))).count();
    let band = match band {
        Some(code) => bands.iter().find(|b| b.code == code).with_context(|| format!("the pager has no band {code}"))?,
        None => bands.iter().max_by_key(in_band).context("the pager reports no bands")?,
    };
    let plan = plan_import(existing.as_ref(), &band.ranges, &sites, &tgs, &sel)?;
    println!("\nInto {} ({} band):", system.map_or("a new system".to_string(), |n| format!("{n:?}")), band.code);
    println!("  add {} sites ({} already there)", plan.sites.len(), plan.sites_present);
    println!("  add {} control channels ({} already there)", plan.control_channels.len(), plan.control_channels_present);
    println!("  add {} talkgroups ({} already there)", plan.talkgroups.len(), plan.talkgroups_present);
    if !plan.out_of_band.is_empty() {
        let mhz: Vec<String> = plan.out_of_band.iter().map(|hz| format!("{:.5}", *hz as f64 / 1e6)).collect();
        println!("  left out, outside the band: {}", mhz.join(", "));
    }
    println!("  {} names shortened to 14 characters", plan.renamed.len());
    let n = &plan.not_in_import;
    if !(n.sites.is_empty() && n.control_channels.is_empty() && n.talkgroups.is_empty()) {
        println!("  in the system but not in RadioReference: {} sites, {} control channels, {} talkgroups", n.sites.len(), n.control_channels.len(), n.talkgroups.len());
    }
    Ok(())
}
