//! Copying a P25 system from another backup (PPS's "Add System from Resources") against
//! private fixtures (skipped without them).

use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::compare::{TableStatus, compare};
use g5_codeplug::row::load_table;
use g5_codeplug::system_edit::{SystemCopy, copy_system, current_state};
use g5_codeplug::{Profile, Templates};

const NOW: &str = "2026-10-20 12:00:00";
const NEW_ID: &str = "c0b1e2d3-a4f5-4687-9a8b-7c6d5e4f3a2b";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture() -> Option<(Templates, PathBuf, PathBuf)> {
    let templates = Templates::new(std::env::var_os("G5_TEMPLATES").map(PathBuf::from).unwrap_or(repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"))).ok()?;
    let root = std::env::var_os("G5_BACKUPS").map(PathBuf::from).unwrap_or(repo().join("backups"));
    let mut dirs: Vec<PathBuf> = fs::read_dir(root)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| fs::read_to_string(p.join("MANIFEST.txt")).is_ok_and(|m| m.contains("DBV1.41b") || m.contains("DBV1.41c")))
        .collect();
    dirs.sort();
    Some((templates, dirs.first()?.clone(), dirs.last()?.clone()))
}

/// The source's system with the most talkgroups, and a copy request for it.
fn request(src: &Path, name: &str) -> (String, SystemCopy) {
    let p = Profile::from_backup(src).unwrap();
    let idx = load_table(&p.db, "tabP25TrunkingSecurity");
    let row = idx.iter().max_by_key(|r| r.i("TGIDCount")).unwrap();
    let id = row.s("P25TrunkingID").to_string();
    let mut system = current_state(&p, &id).unwrap();
    system.system_id = NEW_ID.into();
    system.name = name.into();
    (id.clone(), SystemCopy { backup: src.to_path_buf(), source_id: id, system })
}

#[test]
fn copied_system_keeps_every_row_and_setting() {
    let Some((tpl, src, dst)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let (sid, req) = request(&src, "Copied Sys");
    let p = Profile::from_backup(&dst).unwrap();
    let before = load_table(&p.db, "tabP25TrunkingSecurity").len();
    let ch = copy_system(&p, &tpl, &req, NOW).unwrap();
    assert!(ch.created);
    let tgs = req.system.talkgroups.iter().filter(|t| t.group_id != g5_codeplug::system_edit::ALL_GROUP_ID).count();
    assert_eq!((ch.sites_added, ch.ccs_added, ch.tgs_added), (req.system.sites.len(), req.system.control_channels.len(), tgs), "a copy reports its contents");
    assert_eq!(load_table(&p.db, "tabP25TrunkingSecurity").len(), before + 1);

    let source = Profile::from_backup(&src).unwrap();
    let (a, b) = (p.system_db(NEW_ID).unwrap(), source.system_db(&sid).unwrap());
    for d in compare(&a, &b).unwrap() {
        if d.table == "tabP25TrunkingSystem" {
            continue;
        }
        assert_eq!(d.status, TableStatus::Same, "{} differs from the source system", d.table);
    }
    let (new, old) = (load_table(&a, "tabP25TrunkingSystem").remove(0), load_table(&b, "tabP25TrunkingSystem").remove(0));
    for c in new.cols() {
        let want = match c.as_str() {
            "ID" => NEW_ID.to_string(),
            "Name" => "Copied Sys".into(),
            "SystemSource" => format!("resource_{sid}"),
            _ => old.s(c).to_string(),
        };
        assert_eq!(new.s(c), want, "tabP25TrunkingSystem.{c}");
    }
    let idx = |db: &sqlite2::Connection, id: &str| load_table(db, "tabP25TrunkingSecurity").into_iter().find(|r| r.s("P25TrunkingID") == id).unwrap();
    let (ni, oi) = (idx(&p.db, NEW_ID), idx(&source.db, &sid));
    for c in ni.cols() {
        let want = match c.as_str() {
            "P25TrunkingID" | "DBFileName" => NEW_ID.to_string(),
            "SystemName" => "Copied Sys".into(),
            "SystemSource" => format!("resource_{sid}"),
            _ => oi.s(c).to_string(),
        };
        assert_eq!(ni.s(c), want, "tabP25TrunkingSecurity.{c}");
    }

    // A second copy under the same ID, or with a taken name, is refused.
    assert!(copy_system(&p, &tpl, &req, NOW).is_err());
    let mut again = req.clone();
    again.system.system_id = "d1c2b3a4-9f8e-4d7c-8b6a-5f4e3d2c1b0a".into();
    assert!(copy_system(&p, &tpl, &again, NOW).unwrap_err().to_string().contains("already called"));
    assert!(!p.folder.join("d1c2b3a4-9f8e-4d7c-8b6a-5f4e3d2c1b0a.db").exists(), "a failed copy must leave no file behind");
}

#[test]
fn copy_refuses_a_band_the_pager_lacks() {
    let Some((tpl, src, dst)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let source = Profile::from_backup(&src).unwrap();
    let idx = load_table(&source.db, "tabP25TrunkingSecurity");
    let Some(vhf) = idx.iter().find(|r| r.s("BandTypeCode").contains("VX")) else { return eprintln!("skipping: no VHF system in the fixtures") };
    let mut system = current_state(&source, vhf.s("P25TrunkingID")).unwrap();
    system.system_id = NEW_ID.into();
    system.name = "VHF Copy".into();
    let req = SystemCopy { backup: src.clone(), source_id: vhf.s("P25TrunkingID").into(), system };
    let p = Profile::from_backup(&dst).unwrap();
    // The same pager with a UHF second band instead of VHF.
    let model = load_table(&p.db, "tabModel").remove(0).s("Model").replace("SXVX", "SXUC");
    p.db.execute("UPDATE tabModel SET Model = ?", &[Some(&model)]).unwrap();
    let e = copy_system(&p, &tpl, &req, NOW).unwrap_err().to_string();
    assert!(e.contains("this pager's bands are SX, UC"), "{e}");
    assert!(load_table(&p.db, "tabP25TrunkingSecurity").iter().all(|r| r.s("P25TrunkingID") != NEW_ID));
}
