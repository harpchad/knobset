//! A system copied from another backup goes through edit_set like a new system
//! (private fixtures, skipped without them).

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::Profile;
use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::row::load_table;
use g5_codeplug::system_edit::{SystemCopy, current_state};
use g5_core::write::{Edits, PC_ZIP, edit_set};

const NEW_ID: &str = "5f4e3d2c-1b0a-4987-8a6b-5c4d3e2f1a0b";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture() -> Option<(PathBuf, PathBuf, PathBuf)> {
    let tpl = std::env::var_os("G5_TEMPLATES").map(PathBuf::from).unwrap_or(repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"));
    if !tpl.join("Data_G4A.db").is_file() {
        return None;
    }
    let mut dirs: Vec<PathBuf> = fs::read_dir(std::env::var_os("G5_BACKUPS").map(PathBuf::from).unwrap_or(repo().join("backups")))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| fs::read_to_string(p.join("MANIFEST.txt")).is_ok_and(|m| m.contains("DBV1.41b") || m.contains("DBV1.41c")))
        .collect();
    dirs.sort();
    Some((dirs.last()?.clone(), dirs.first()?.clone(), tpl))
}

/// A copy of `src`'s system with the most talkgroups, under a new ID and a free name.
fn copy_of(src: &Path) -> SystemCopy {
    let p = Profile::from_backup(src).unwrap();
    let row = load_table(&p.db, "tabP25TrunkingSecurity").into_iter().max_by_key(|r| r.i("TGIDCount")).unwrap();
    let mut system = current_state(&p, row.s("P25TrunkingID")).unwrap();
    system.system_id = NEW_ID.into();
    system.name = "From Backup".into();
    SystemCopy { backup: src.to_path_buf(), source_id: row.s("P25TrunkingID").into(), system }
}

#[test]
fn copied_system_writes_nothing_to_the_radio_until_a_knob_uses_it() {
    let Some((dir, src, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let copy = copy_of(&src);
    let edits = Edits { copied_systems: vec![copy.clone()], ..Default::default() };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert!(set.changed_tables.is_empty(), "radio DB changed: {:?}", set.changed_tables);
    assert_eq!(set.systems.iter().filter(|c| c.created).count(), 1);

    // The zip carries the copy, row for row the source's system.
    let zip = &set.files.iter().find(|(p, _)| *p == PC_ZIP).unwrap().1;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    let back = Profile::from_zip(tmp.path()).unwrap();
    let source = Profile::from_backup(&src).unwrap();
    for d in compare(&back.system_db(NEW_ID).unwrap(), &source.system_db(&copy.source_id).unwrap()).unwrap() {
        assert!(d.table == "tabP25TrunkingSystem" || d.status == TableStatus::Same, "{} differs", d.table);
    }
    let st = current_state(&back, NEW_ID).unwrap();
    assert_eq!(st.name, "From Backup");
    assert_eq!((st.sites.len(), st.control_channels.len(), st.talkgroups.len()), (copy.system.sites.len(), copy.system.control_channels.len(), copy.system.talkgroups.len()));

    // Deleting it in the same batch is a conflict.
    let bad = Edits { copied_systems: vec![copy], deleted_systems: vec![NEW_ID.into()], ..Default::default() };
    assert!(edit_set(&dir, &tpl, &bad).is_err());
}

#[test]
fn copied_system_on_a_new_knob_passes_the_guard() {
    let Some((dir, src, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let copy = copy_of(&src);
    let p = Profile::from_backup(&dir).unwrap();
    let empty = load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == 0).unwrap();
    let zone = load_table(&p.db, "tabZone_New").into_iter().find(|z| z.s("ID") == empty.s("ZoneID")).unwrap().s("Name").to_string();
    let tg = copy.system.talkgroups.iter().find(|t| t.group_id != g5_codeplug::system_edit::ALL_GROUP_ID).unwrap().group_id.clone();
    let edits = Edits {
        copied_systems: vec![copy],
        knob_systems: vec![g5_codeplug::knob_system::KnobSystemEdit {
            zone: zone.clone(),
            knob: empty.i("KnobIndex"),
            system_id: NEW_ID.into(),
            mode: g5_codeplug::knob_system::MODE_TG_SCAN,
            channel_id: Some("6a5b4c3d-2e1f-4a0b-9c8d-7e6f5a4b3c2d".into()),
            name: Some("Copied".into()),
            voice_prompt: 0,
            priority: vec![],
            normal: vec![tg],
        }],
        ..Default::default()
    };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert!(set.changed_tables.iter().any(|t| t == "tabSystemAlias"), "{:?}", set.changed_tables);
    assert!(set.changed_tables.iter().any(|t| t == "tabP25TrunkingControlChannels"), "{:?}", set.changed_tables);
    let mut allowed: Vec<&str> = Vec::new();
    for l in [g5_codeplug::edit::SYSTEM_EDIT_TABLES, g5_codeplug::edit::TALKGROUP_EDIT_TABLES, g5_codeplug::edit::KNOB_CHANNEL_TABLES] {
        allowed.extend(l);
    }
    for t in &set.changed_tables {
        assert!(allowed.contains(&t.as_str()), "{t} changed");
    }
}
