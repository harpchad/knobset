//! edit_set end to end on private fixtures (skipped without them): a system edit plus a
//! knob edit in one batch must build a write set the guard accepts.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use g5_codeplug::Profile;
use g5_codeplug::summary::Summary;
use g5_codeplug::system_edit::{SiteEdit, TalkgroupEdit, current_state};
use g5_core::write::{Edits, KnobTalkgroupEdit, edit_set};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A RadioReference export kept for local tests: the first `rr-data/*/`
/// folder with a `{prefix}<sid>.csv` file. It's RadioReference's data and
/// stays out of git.
fn rr_file(rr_data: &Path, prefix: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(rr_data).ok()?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_dir()).collect();
    dirs.sort();
    dirs.iter().find_map(|d| {
        let mut files: Vec<PathBuf> = fs::read_dir(d).ok()?.filter_map(|e| e.ok().map(|e| e.path())).collect();
        files.sort();
        files.into_iter().find(|f| f.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(prefix) && n.ends_with(".csv")))
    })
}

/// The fixtures are real pager backups, so tests use stand-ins for zone and
/// knob names that would identify the owner. `fixture-names.txt` in the
/// backups folder (gitignored, one `stand-in=real name` per line) maps them
/// back; a name with no entry is used as is.
fn real(stand_in: &'static str) -> &'static str {
    static NAMES: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        fs::read_to_string(std::env::var_os("G5_BACKUPS").map(PathBuf::from).unwrap_or(repo().join("backups")).join("fixture-names.txt"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), &*Box::leak(v.trim().to_string().into_boxed_str())))
            .collect()
    });
    names.get(stand_in).copied().unwrap_or(stand_in)
}

fn fixture() -> Option<(PathBuf, PathBuf)> {
    let tpl = std::env::var_os("G5_TEMPLATES").map(PathBuf::from).unwrap_or(repo().join("extract/msi/Unication/G_Series_PPS_V00-04-00-48/DataBase"));
    if !tpl.join("Data_G4A.db").is_file() {
        return None;
    }
    let dir = fs::read_dir(std::env::var_os("G5_BACKUPS").map(PathBuf::from).unwrap_or(repo().join("backups")))
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| fs::read_to_string(p.join("MANIFEST.txt")).is_ok_and(|m| m.contains("DBV1.41b") || m.contains("DBV1.41c")))?;
    Some((dir, tpl))
}

#[test]
fn system_and_knob_edits_in_one_batch() {
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let (zone, knob) = s.zones.iter().flat_map(|z| z.knobs.iter().map(move |k| (z, k))).find(|(_, k)| k.system_id.is_some() && k.talkgroup_list.len() >= 2).unwrap();
    let sid = knob.system_id.clone().unwrap();

    // System: rename, add a site, add a talkgroup.
    let mut sys = current_state(&p, &sid).unwrap();
    sys.name = "Batch Test".into();
    let free = (0..=255).find(|n| !sys.sites.iter().any(|x| x.rfss == 7 && x.site == *n)).unwrap();
    sys.sites.push(SiteEdit { id: None, name: "Batch Site".into(), rfss: 7, site: free });
    let new_gid = "11111111-2222-4333-8444-555555555555".to_string();
    let tgid = (1..=65535).find(|n| !sys.talkgroups.iter().any(|t| t.tgid == *n)).unwrap();
    sys.talkgroups.push(TalkgroupEdit { group_id: new_gid.clone(), tgid, name: "Batch TG".into(), note: String::new(), ..Default::default() });

    // Knob: put the new talkgroup on it (it only exists after the system edit).
    let mut normal: Vec<String> = knob.talkgroup_list.iter().filter(|t| !t.priority).map(|t| t.group_id.clone()).collect();
    normal.push(new_gid);
    let priority = knob.talkgroup_list.iter().filter(|t| t.priority).map(|t| t.group_id.clone()).collect();

    let edits = Edits {
        systems: vec![sys],
        talkgroups: vec![KnobTalkgroupEdit { zone: zone.name.clone(), knob: knob.index, priority, normal }],
        ..Default::default()
    };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert_eq!(set.systems.len(), 1);
    assert_eq!((set.systems[0].sites_added, set.systems[0].tgs_added), (1, 1));
    assert_eq!(set.talkgroups[0].added.len(), 1);
    assert!(set.changed_tables.iter().any(|t| t == "tabSystemAlias"));
    assert!(set.changed_tables.iter().any(|t| t == "tabP25TrunkingSites"));
    assert!(set.changed_tables.iter().any(|t| t == "tabContacts"));
    assert_eq!(set.files.len(), 5);

    // Round trip: the profile stored on the pager (u3db_PC.zip) must carry every edit,
    // and u3db_PC.db must be the zip's root database.
    let zip = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_ZIP).unwrap().1;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    let back = Profile::from_zip(tmp.path()).unwrap();
    let st = current_state(&back, &sid).unwrap();
    assert_eq!(st.name, "Batch Test");
    assert!(st.sites.iter().any(|x| x.name == "Batch Site" && x.rfss == 7 && x.site == free), "site edit lost from the zip");
    assert!(st.talkgroups.iter().any(|t| t.name == "Batch TG"), "talkgroup edit lost from the zip");
    let k2 = Summary::read(&back).unwrap().zones.into_iter().find(|z| z.name == zone.name).unwrap().knobs.into_iter().find(|k| k.index == knob.index).unwrap();
    assert!(k2.talkgroup_list.iter().any(|t| t.group_id == "11111111-2222-4333-8444-555555555555"), "knob edit lost from the zip");
    let pc_db = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_DB).unwrap().1;
    assert_eq!(pc_db, &fs::read(&back.db_path).unwrap(), "u3db_PC.db must equal the zip's root DB");
}

#[test]
fn unchanged_system_edit_is_rejected_as_no_changes() {
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let sid = Summary::read(&p).unwrap().systems.into_iter().find(|s| s.in_use).unwrap().id;
    let edits = Edits { systems: vec![current_state(&p, &sid).unwrap()], ..Default::default() };
    assert!(edit_set(&dir, &tpl, &edits).is_err());
}

/// The common case after a RadioReference import: turn alerts and voice storage
/// on for a few talkgroups that are on knobs, in one write.
#[test]
fn talkgroup_settings_build_a_write_set() {
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let sid = s.systems.iter().find(|x| x.in_use).unwrap().id.clone();
    let on_knobs: Vec<String> = s.zones.iter().flat_map(|z| &z.knobs).filter(|k| k.system_id.as_deref() == Some(&sid)).flat_map(|k| k.talkgroup_list.iter().map(|t| t.group_id.clone())).collect();
    let mut sys = current_state(&p, &sid).unwrap();
    let mut picked = Vec::new();
    for t in sys.talkgroups.iter_mut().filter(|t| on_knobs.contains(&t.group_id)).take(5) {
        t.alert_mode = Some(t.alert_mode.unwrap() % 4 + 1);
        t.voice_storage = Some(!t.voice_storage.unwrap());
        picked.push((t.group_id.clone(), t.alert_mode, t.voice_storage));
    }
    let set = edit_set(&dir, &tpl, &Edits { systems: vec![sys], ..Default::default() }).unwrap();
    assert_eq!((set.systems[0].tgs_settings_changed, set.systems[0].tgs_changed), (picked.len(), 0));
    assert!(!set.changed_tables.is_empty());
    for t in &set.changed_tables {
        assert!(["tabRecvBehaviorByContactID", "tabRecvVoiceCallRecordByContactID"].contains(&t.as_str()), "unexpected change in {t}");
    }
    let zip = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_ZIP).unwrap().1;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    let back = current_state(&Profile::from_zip(tmp.path()).unwrap(), &sid).unwrap();
    for (gid, mode, vs) in picked {
        let t = back.talkgroups.iter().find(|t| t.group_id == gid).unwrap();
        assert_eq!((t.alert_mode, t.voice_storage), (mode, vs), "settings lost from the zip");
    }
}

#[test]
fn created_system_round_trips_through_the_zip() {
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let templates = g5_codeplug::Templates::new(&tpl).unwrap();
    let band = g5_codeplug::system_edit::profile_bands(&p, &templates).unwrap().remove(0);
    let new_id = "7e2d3c4b-5a69-4f98-8e7d-6c5b4a3f2e1d";
    let new = g5_codeplug::system_edit::NewSystem {
        band: band.code,
        system: g5_codeplug::system_edit::SystemEdit {
            system_id: new_id.into(),
            name: "Zip New".into(),
            wacn: 1,
            sysid: 2,
            sites: vec![SiteEdit { id: None, name: "Only".into(), rfss: 1, site: 1 }],
            control_channels: vec![g5_codeplug::system_edit::ControlChannelEdit { id: None, rx_hz: band.ranges[0].0 + 12_500, note: String::new() }],
            talkgroups: vec![],
        },
    };
    let edits = Edits { new_systems: vec![new], ..Default::default() };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert!(set.changed_tables.is_empty(), "radio DB changed: {:?}", set.changed_tables);
    assert_eq!(set.systems.iter().filter(|c| c.created).count(), 1);

    let zip = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_ZIP).unwrap().1;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    let names: Vec<String> = {
        let mut z = zip::ZipArchive::new(fs::File::open(tmp.path()).unwrap()).unwrap();
        (0..z.len()).map(|i| z.by_index(i).unwrap().name().unwrap().into_owned()).collect()
    };
    assert!(names.iter().any(|n| n.ends_with(&format!("/{new_id}.db"))), "new system missing from the zip");
    let back = Profile::from_zip(tmp.path()).unwrap();
    let st = current_state(&back, new_id).unwrap();
    assert_eq!((st.name.as_str(), st.sites.len(), st.talkgroups.len()), ("Zip New", 1, 1));
}

#[test]
fn new_system_on_new_knob_in_one_batch() {
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let templates = g5_codeplug::Templates::new(&tpl).unwrap();
    let band = g5_codeplug::system_edit::profile_bands(&p, &templates).unwrap().remove(0);
    let zones = g5_codeplug::row::load_table(&p.db, "tabZone_New");
    let empty = g5_codeplug::row::load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == 0).unwrap();
    let zone = zones.iter().find(|z| z.s("ID") == empty.s("ZoneID")).unwrap().s("Name").to_string();
    let (sid, tg) = ("1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d", "2b3c4d5e-6f70-4b8c-9dae-1f2a3b4c5d6e");
    let edits = Edits {
        new_systems: vec![g5_codeplug::system_edit::NewSystem {
            band: band.code,
            system: g5_codeplug::system_edit::SystemEdit {
                system_id: sid.into(),
                name: "Batch New".into(),
                wacn: 0xBEE00,
                sysid: 0x2A,
                sites: vec![SiteEdit { id: None, name: "Ridge".into(), rfss: 1, site: 3 }],
                control_channels: vec![g5_codeplug::system_edit::ControlChannelEdit { id: None, rx_hz: band.ranges[0].0 + 12_500, note: String::new() }],
                talkgroups: vec![TalkgroupEdit { group_id: tg.into(), tgid: 777, name: "Ridge Ops".into(), note: String::new(), ..Default::default() }],
            },
        }],
        knob_systems: vec![g5_codeplug::knob_system::KnobSystemEdit {
            zone: zone.clone(),
            knob: empty.i("KnobIndex"),
            system_id: sid.into(),
            mode: g5_codeplug::knob_system::MODE_TG_SCAN,
            channel_id: Some("3c4d5e6f-7081-4c9d-8ebf-2a3b4c5d6e7f".into()),
            name: Some("Ridge".into()),
            voice_prompt: 0,
            priority: vec![],
            normal: vec![tg.into()],
        }],
        ..Default::default()
    };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert_eq!(set.knob_systems.len(), 1);
    assert!(set.changed_tables.iter().any(|t| t == "tabChannel"), "{:?}", set.changed_tables);
    let zip = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_ZIP).unwrap().1;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    let back = Profile::from_zip(tmp.path()).unwrap();
    let k = Summary::read(&back).unwrap().zones.into_iter().find(|z| z.name == zone).unwrap().knobs.into_iter().find(|k| k.index == empty.i("KnobIndex")).unwrap();
    assert_eq!((k.name.as_str(), k.system_id.as_deref(), k.talkgroups), ("Ridge", Some(sid), 1));
}

/// An analog conventional knob goes through the write path; the guard allows
/// only the conventional channel tables, and a second change to the same knob
/// in one batch is refused.
#[test]
fn conventional_knob_builds_a_write_set() {
    use g5_codeplug::conv_knob::{BW_12_5, ConvKnobEdit, Squelch};
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let zones = g5_codeplug::row::load_table(&p.db, "tabZone_New");
    let r = g5_codeplug::row::load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == 0 && (1..=8).contains(&r.i("KnobIndex"))).unwrap();
    let zone = zones.iter().find(|z| z.s("ID") == r.s("ZoneID")).unwrap().s("Name").to_string();
    let conv = ConvKnobEdit {
        zone: zone.clone(),
        knob: r.i("KnobIndex"),
        name: "WX Test".into(),
        rx_hz: 162_550_000,
        bandwidth: BW_12_5,
        squelch: Squelch::Ctcss { tenths: 1000 },
        voice_prompt: 0,
        channel_id: "1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d".into(),
        freq_id: "2b3c4d5e-6f7a-4b8c-9d0e-1f2a3b4c5d6e".into(),
    };
    let set = edit_set(&dir, &tpl, &Edits { conv_knobs: vec![conv.clone()], ..Default::default() }).unwrap();
    assert_eq!(set.conv_knobs.len(), 1);
    assert!(set.changed_tables.iter().any(|t| t == "tabChannel"), "{:?}", set.changed_tables);
    assert!(set.changed_tables.iter().all(|t| g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES.contains(&t.as_str())), "{:?}", set.changed_tables);

    let rename = g5_core::write::KnobRename { zone, knob: conv.knob, name: "Other".into() };
    let err = edit_set(&dir, &tpl, &Edits { conv_knobs: vec![conv], renames: vec![rename], ..Default::default() }).unwrap_err();
    assert!(err.to_string().contains("another change in the same batch"), "{err}");
}

/// A new zone and a knob in it in one batch; a reorder on its own; and a
/// reorder mixed with a knob change is refused.
#[test]
fn zone_edits_build_write_sets() {
    use g5_codeplug::conv_knob::{BW_12_5, ConvKnobEdit, Squelch};
    use g5_codeplug::zone_edit::NewZone;
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let zid = "4d3c2b1a-0f9e-4d8c-8b7a-6f5e4d3c2b1a";
    let conv = ConvKnobEdit {
        zone: "Weather".into(),
        knob: 1,
        name: "WX Test".into(),
        rx_hz: 162_550_000,
        bandwidth: BW_12_5,
        squelch: Squelch::Carrier,
        voice_prompt: 0,
        channel_id: "5e4d3c2b-1a0f-4e9d-8c7b-6a5f4e3d2c1b".into(),
        freq_id: "6f5e4d3c-2b1a-4f0e-9d8c-7b6a5f4e3d2c".into(),
    };
    let edits = Edits { new_zones: vec![NewZone { zone_id: zid.into(), name: "Weather".into() }], conv_knobs: vec![conv.clone()], ..Default::default() };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert_eq!((set.zones.len(), set.conv_knobs.len()), (1, 1));
    let allowed: Vec<&str> = g5_codeplug::edit::ZONE_TABLES.iter().chain(g5_codeplug::edit::CONV_KNOB_CHANNEL_TABLES).copied().collect();
    assert!(set.changed_tables.iter().all(|t| allowed.contains(&t.as_str())), "{:?}", set.changed_tables);

    let p = Profile::from_backup(&dir).unwrap();
    let mut order: Vec<String> = g5_codeplug::row::load_table(&p.db, "tabZone_New").iter().map(|z| z.s("ID").to_string()).collect();
    order.reverse();
    let set = edit_set(&dir, &tpl, &Edits { zone_order: Some(order.clone()), ..Default::default() }).unwrap();
    assert!(set.zones[0].renumbers);
    assert!(set.changed_tables.iter().any(|t| t == "tabChannel"), "{:?}", set.changed_tables);

    let err = edit_set(&dir, &tpl, &Edits { zone_order: Some(order), conv_knobs: vec![conv], ..Default::default() }).unwrap_err();
    assert!(err.to_string().contains("on their own"), "{err}");
}

/// Converting a knob's type is a removal and a new knob on the same position
/// in one batch; anything else on a removed knob is refused.
#[test]
fn knob_conversions_build_write_sets() {
    use g5_codeplug::conv_knob::{BW_12_5, ConvKnobEdit, Squelch};
    use g5_codeplug::knob_remove::KnobRemoval;
    use g5_codeplug::knob_system::KnobSystemEdit;
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    // Trunked to conventional.
    let (zone, k) = s.zones.iter().find_map(|z| z.knobs.iter().find(|k| k.system_id.is_some()).map(|k| (z.name.clone(), k.clone()))).unwrap();
    let conv = ConvKnobEdit {
        zone: zone.clone(),
        knob: k.index,
        name: k.name.clone(),
        rx_hz: 154_430_000,
        bandwidth: BW_12_5,
        squelch: Squelch::Carrier,
        voice_prompt: 0,
        channel_id: "7a6b5c4d-3e2f-4a1b-8c9d-0e1f2a3b4c5d".into(),
        freq_id: "8b7c6d5e-4f3a-4b2c-9d0e-1f2a3b4c5d6e".into(),
    };
    let removal = KnobRemoval { zone: zone.clone(), knob: k.index };
    let set = edit_set(&dir, &tpl, &Edits { removed_knobs: vec![removal.clone()], conv_knobs: vec![conv], ..Default::default() }).unwrap();
    assert_eq!((set.removed_knobs.len(), set.conv_knobs.len()), (1, 1));
    let back = {
        let zip = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_ZIP).unwrap().1;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        fs::write(tmp.path(), zip).unwrap();
        Profile::from_zip(tmp.path()).unwrap()
    };
    let nk = Summary::read(&back).unwrap().zones.into_iter().find(|z| z.name == zone).unwrap().knobs.into_iter().find(|x| x.index == k.index).unwrap();
    assert_eq!((nk.mode, nk.name.as_str(), nk.target.as_deref()), (33, k.name.as_str(), Some("154.43000 MHz")));

    // Conventional to trunked.
    let (czone, ck) = s.zones.iter().find_map(|z| z.knobs.iter().find(|k| k.analog).map(|k| (z.name.clone(), k.clone()))).unwrap();
    let sys = s.systems.iter().find(|x| x.in_use).unwrap();
    let ks = KnobSystemEdit {
        zone: czone.clone(),
        knob: ck.index,
        system_id: sys.id.clone(),
        mode: 16,
        channel_id: Some("9c8d7e6f-5a4b-4c3d-8e2f-1a0b9c8d7e6f".into()),
        name: Some("Now Trunked".into()),
        voice_prompt: 0,
        priority: vec![],
        normal: vec![sys.talkgroup_list[1].group_id.clone()],
    };
    let result = edit_set(&dir, &tpl, &Edits { removed_knobs: vec![KnobRemoval { zone: czone.clone(), knob: ck.index }], knob_systems: vec![ks], ..Default::default() });
    match result {
        Ok(set) => assert!(set.knob_systems[0].new_knob),
        // The real conventional knobs are scan members; a scan that would drop below 2 refuses.
        Err(e) => assert!(e.to_string().contains("fewer than 2"), "{e}"),
    }

    // A rename with a trunked conversion names the new knob.
    let rename = g5_core::write::KnobRename { zone: zone.clone(), knob: k.index, name: "Renamed".into() };
    let sys = s.systems.iter().find(|x| x.in_use).unwrap();
    let ks = KnobSystemEdit {
        zone: zone.clone(),
        knob: k.index,
        system_id: sys.id.clone(),
        mode: 16,
        channel_id: Some("6a5b4c3d-2e1f-4a0b-9c8d-7e6f5a4b3c2d".into()),
        name: Some("Knob".into()),
        voice_prompt: 0,
        priority: vec![],
        normal: vec![sys.talkgroup_list[1].group_id.clone()],
    };
    let set = edit_set(&dir, &tpl, &Edits { removed_knobs: vec![removal.clone()], knob_systems: vec![ks], renames: vec![rename.clone()], ..Default::default() }).unwrap();
    let zip = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_ZIP).unwrap().1;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    let nk = Summary::read(&Profile::from_zip(tmp.path()).unwrap()).unwrap().zones.into_iter().find(|z| z.name == zone).unwrap().knobs.into_iter().find(|x| x.index == k.index).unwrap();
    assert_eq!((nk.mode, nk.name.as_str(), nk.system_id.as_deref()), (16, "Renamed", Some(sys.id.as_str())));

    // Without a new knob, a rename of a removed knob is refused.
    let err = edit_set(&dir, &tpl, &Edits { removed_knobs: vec![removal], renames: vec![rename], ..Default::default() }).unwrap_err();
    assert!(err.to_string().contains("removed and changed"), "{err}");
}

/// A DMR knob goes through the write path, flagged as experimental.
#[test]
fn digital_knob_builds_a_write_set() {
    use g5_codeplug::digital_knob::{DigitalKnobEdit, DigitalProtocol};
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    if g5_codeplug::row::load_table(&p.db, "tabProtocolMode").first().is_none_or(|m| m.s("DMRC") != "1") {
        return eprintln!("skipping: fixture pager has no DMR");
    }
    let zones = g5_codeplug::row::load_table(&p.db, "tabZone_New");
    let r = g5_codeplug::row::load_table(&p.db, "tabReceivingMode_NewG4").into_iter().find(|r| r.i("CurrentMode") == 0 && (1..=8).contains(&r.i("KnobIndex"))).unwrap();
    let zone = zones.iter().find(|z| z.s("ID") == r.s("ZoneID")).unwrap().s("Name").to_string();
    let d = DigitalKnobEdit {
        zone,
        knob: r.i("KnobIndex"),
        name: "DMR Test".into(),
        rx_hz: 155_310_000,
        protocol: DigitalProtocol::Dmr { slot: 1, color_code: 1 },
        talkgroups: vec![],
        repeater_mode: false,
        voice_prompt: 0,
        channel_id: "5b6a7988-9786-4a5b-c4d3-e2f1a0b9c8d7".replace('c', "a"),
        freq_id: "6a798897-86a5-4b4c-93e2-f1a0b9c8d7e6".into(),
    };
    let set = edit_set(&dir, &tpl, &Edits { digital_knobs: vec![d], ..Default::default() }).unwrap();
    assert_eq!(set.digital_knobs.len(), 1);
    assert_eq!(set.experimental, ["DMR conventional channel"]);
    assert!(set.changed_tables.iter().any(|t| t == "tabDMRSignaling"), "{:?}", set.changed_tables);
}

/// A RadioReference import goes through the normal write path: as a new
/// system, and added to an existing one. Uses a real export in /rr-data when
/// present (the one tested has 655 talkgroups with long descriptions), else
/// a small made-up table.
#[test]
fn radioreference_import_builds_a_write_set() {
    use g5_codeplug::rr_import::{Selection, parse_sites, parse_talkgroups, plan_import};
    use g5_codeplug::system_edit::{NewSystem, SystemEdit};

    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let rr = repo().join("rr-data");
    let read = |prefix| rr_file(&rr, prefix).map(fs::read_to_string);
    let (sites, tgs) = match (read("trs_sites_"), read("trs_tg_")) {
        (Some(Ok(s)), Some(Ok(t))) => (s, t),
        _ => (
            "RFSS,Site Dec,Site Hex,Description,Frequencies\n1,001,1,North County Simulcast,851.0125c,851.2875\n1,002,2,South,852.1625c\n".to_string(),
            "Decimal,Hex,Alpha Tag,Mode,Description,Tag,Category\n1,001,City EM 1,D,Emergency Management,Emergency Ops,City\n3,003,City PD,DE,Police,Law Dispatch,City\n".to_string(),
        ),
    };
    let (sites, tgs) = (parse_sites(&sites).unwrap(), parse_talkgroups(&tgs).unwrap());
    let sel = Selection::defaults(&sites, &tgs);
    let p = Profile::from_backup(&dir).unwrap();
    let templates = g5_codeplug::Templates::new(&tpl).unwrap();
    let band = g5_codeplug::system_edit::profile_bands(&p, &templates).unwrap().into_iter().max_by_key(|b| b.ranges.iter().any(|&(lo, _)| lo > 700_000_000)).unwrap();
    let guid = |i: usize| format!("{i:08x}-0000-4000-8000-{i:012x}");
    let tg_edits = |plan: &g5_codeplug::rr_import::ImportPlan, offset: usize| -> Vec<TalkgroupEdit> {
        plan.talkgroups.iter().enumerate().map(|(i, t)| TalkgroupEdit { group_id: guid(offset + i), tgid: t.tgid, name: t.name.clone(), note: t.note.clone(), imported: true, ..Default::default() }).collect()
    };

    // New system.
    let plan = plan_import(None, &band.ranges, &sites, &tgs, &sel).unwrap();
    let new_id = "6e2d3c4b-5a69-4f98-8e7d-6c5b4a3f2e1d";
    let new = NewSystem {
        band: band.code.clone(),
        system: SystemEdit {
            system_id: new_id.into(),
            name: "RR Import".into(),
            wacn: 0xBEE00,
            sysid: 0x2B0,
            sites: plan.sites.clone(),
            control_channels: plan.control_channels.clone(),
            talkgroups: tg_edits(&plan, 1),
        },
    };
    let set = edit_set(&dir, &tpl, &Edits { new_systems: vec![new], ..Default::default() }).unwrap();
    assert!(set.changed_tables.is_empty(), "radio DB changed: {:?}", set.changed_tables);
    let c = set.systems.iter().find(|c| c.created).unwrap();
    // The template's "All" talkgroup isn't counted as added.
    assert_eq!((c.sites_added, c.ccs_added, c.tgs_added), (plan.sites.len(), plan.control_channels.len(), plan.talkgroups.len()));

    // Added to the first existing system.
    let first = Summary::read(&p).unwrap().systems[0].id.clone();
    let mut existing = current_state(&p, &first).unwrap();
    let plan = plan_import(Some(&existing), &band.ranges, &sites, &tgs, &sel).unwrap();
    existing.sites.extend(plan.sites.clone());
    existing.control_channels.extend(plan.control_channels.clone());
    existing.talkgroups.extend(tg_edits(&plan, 100_000));
    let set = edit_set(&dir, &tpl, &Edits { systems: vec![existing], ..Default::default() }).unwrap();
    let c = set.systems.iter().find(|c| c.system_id == first).unwrap();
    assert_eq!((c.sites_added, c.ccs_added, c.tgs_added), (plan.sites.len(), plan.control_channels.len(), plan.talkgroups.len()));
    assert_eq!((c.sites_removed, c.ccs_removed, c.tgs_removed), (0, 0, 0));
    eprintln!("import into {first}: +{} sites, +{} control channels, +{} talkgroups", c.sites_added, c.ccs_added, c.tgs_added);
}

/// Unit B's newest backup (it has every knob kind, including a DMR knob).
fn unit_b() -> Option<(PathBuf, PathBuf)> {
    let (_, tpl) = fixture()?;
    let mut dirs: Vec<PathBuf> = fs::read_dir(std::env::var_os("G5_BACKUPS").map(PathBuf::from).unwrap_or(repo().join("backups"))).ok()?.filter_map(|e| e.ok().map(|e| e.path())).collect();
    dirs.sort();
    let dir = dirs.into_iter().rev().find(|d| {
        fs::read_to_string(d.join("MANIFEST.txt")).is_ok_and(|m| m.contains("DBV1.41c"))
            && Profile::from_backup(d).is_ok_and(|p| {
                let dmr: Vec<String> = g5_codeplug::row::load_table(&p.db, "tabProtocolSetting").iter().filter(|s| s.i("ProtocolType") == 4).map(|s| s.s("ID").to_string()).collect();
                g5_codeplug::row::load_table(&p.db, "tabSingleConvCHKnob").iter().any(|k| dmr.iter().any(|d| d == k.s("ProtocolSettingID")))
            })
    })?;
    Some((dir, tpl))
}

fn copy(zone: &str, knob: i64, to_zone: &str, to_knob: i64, ch: Option<&str>) -> g5_codeplug::knob_copy::KnobCopy {
    g5_codeplug::knob_copy::KnobCopy { zone: zone.into(), knob, to_zone: to_zone.into(), to_knob, name: "Copy".into(), channel_id: ch.map(String::from) }
}

fn mv(zone: &str, knob: i64, to_zone: &str, to_knob: i64, swap: bool) -> g5_codeplug::knob_copy::KnobMove {
    g5_codeplug::knob_copy::KnobMove { zone: zone.into(), knob, to_zone: to_zone.into(), to_knob, swap }
}

fn zip_profile(set: &g5_core::write::WriteSet) -> (Profile, tempfile::NamedTempFile) {
    let zip = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_ZIP).unwrap().1;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    (Profile::from_zip(tmp.path()).unwrap(), tmp)
}

/// Copies of every kind in one batch, with a scan that lists one of them;
/// moves and swaps alone and next to unrelated edits.
#[test]
fn knob_copies_and_moves_build_write_sets() {
    use g5_codeplug::knob_remove::KnobKind;
    use g5_codeplug::scan_knob::ScanKnobEdit;
    let Some((dir, tpl)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let s = Summary::read(&Profile::from_backup(&dir).unwrap()).unwrap();
    let knob = |z: &str, k: i64| s.zones.iter().find(|x| x.name == z).unwrap().knobs.iter().find(|x| x.index == k).unwrap().clone();
    let (c1, c2) = ("1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d", "2b3c4d5e-6f7a-4b8c-9d0e-1f2a3b4c5d6e");
    let scan = knob("Lake", 2);
    let mut members = scan.scan_members.clone();
    members.push(c2.into());
    let edits = Edits {
        knob_copies: vec![copy(real("Metro"), 1, "Test", 4, Some(c1)), copy("NOAA", 2, "Test", 5, Some(c2)), copy("NOAA", 1, "Test", 6, None), copy("Test", 2, "Test", 7, Some("3c4d5e6f-7a8b-4c9d-8e1f-2a3b4c5d6e7f"))],
        scan_knobs: vec![ScanKnobEdit { zone: "Lake".into(), knob: 2, mode: 36, members, priority: scan.scan_priority.clone(), name: None, voice_prompt: 0 }],
        ..Default::default()
    };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert_eq!(set.knob_copies.iter().map(|c| c.kind).collect::<Vec<_>>(), [KnobKind::Trunked, KnobKind::Conventional, KnobKind::Scan, KnobKind::Digital]);
    assert!(set.changed_tables.iter().any(|t| t == "tabP25TSetting") && set.changed_tables.iter().any(|t| t == "tabDMRSignaling"), "{:?}", set.changed_tables);
    assert!(!set.changed_tables.iter().any(|t| t == "tabContacts"), "copies add no contacts: {:?}", set.changed_tables);
    let (back, _tmp) = zip_profile(&set);
    let bz = Summary::read(&back).unwrap();
    let test = bz.zones.iter().find(|z| z.name == "Test").unwrap();
    assert_eq!(test.knobs.iter().filter(|k| k.name == "Copy").map(|k| k.index).collect::<Vec<_>>(), [4, 5, 6, 7]);
    assert!(bz.zones.iter().find(|z| z.name == "Lake").unwrap().knobs.iter().find(|k| k.index == 2).unwrap().scan_members.contains(&c2.to_string()));

    // A move on its own: the priority scan follows its priority channel.
    let set = edit_set(&dir, &tpl, &Edits { knob_moves: vec![mv("Lake", 3, "Test", 4, false)], ..Default::default() }).unwrap();
    assert_eq!(set.knob_moves[0].scans, [real("Lake 2 ElmwoodCo Fire")]);
    assert!(set.changed_tables.iter().all(|t| g5_codeplug::edit::ZONE_RENUMBER_TABLES.contains(&t.as_str())), "{:?}", set.changed_tables);
    let (back, _tmp) = zip_profile(&set);
    let moved = Summary::read(&back).unwrap().zones.into_iter().find(|z| z.name == "Test").unwrap().knobs.into_iter().find(|k| k.index == 4).unwrap();
    assert_eq!((moved.name.as_str(), moved.channel_id.as_ref()), (real("Elmwood Fire"), knob("Lake", 3).channel_id.as_ref()));

    // Next to unrelated edits: each stage is checked on its own and the plan lists both.
    let rename = g5_core::write::KnobRename { zone: real("Metro").into(), knob: 3, name: "Pine Co".into() };
    let edits = Edits { knob_moves: vec![mv("Lake", 3, "Test", 4, false), mv(real("Metro"), 1, "Test", 1, true)], knob_copies: vec![copy(real("Metro"), 2, "Test", 5, Some(c1))], renames: vec![rename], ..Default::default() };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert_eq!((set.knob_moves.len(), set.knob_copies.len(), set.renames.len()), (2, 1, 1));
    assert_eq!(set.knob_moves[1].swapped_with.as_deref(), Some("TEST"));
    assert!(set.changed_tables.iter().any(|t| t == "tabScanListSettings"), "{:?}", set.changed_tables);
    let (back, _tmp) = zip_profile(&set);
    let bz = Summary::read(&back).unwrap();
    let name = |z: &str, k: i64| bz.zones.iter().find(|x| x.name == z).unwrap().knobs.iter().find(|x| x.index == k).map(|x| x.name.clone());
    assert_eq!((name(real("Metro"), 1), name("Test", 1), name("Test", 4), name("Test", 5), name(real("Metro"), 3)), (Some("TEST".into()), Some(real("MTFD").into()), Some(real("Elmwood Fire").into()), Some("Copy".into()), Some("Pine Co".into())));
    assert_eq!(name("Lake", 3), None);
}

#[test]
fn knob_copy_and_move_conflicts_are_refused() {
    use g5_codeplug::conv_knob::{BW_12_5, ConvKnobEdit, Squelch};
    let Some((dir, tpl)) = unit_b() else { return eprintln!("skipping: fixtures not available") };
    let ch = "4d5e6f7a-8b9c-4d0e-9f1a-2b3c4d5e6f7a";
    let rename = |z: &str, k: i64| g5_core::write::KnobRename { zone: z.into(), knob: k, name: "X".into() };
    let conv = ConvKnobEdit { zone: "Test".into(), knob: 4, name: "C".into(), rx_hz: 154_430_000, bandwidth: BW_12_5, squelch: Squelch::Carrier, voice_prompt: 0, channel_id: ch.into(), freq_id: "5e6f7a8b-9c0d-4e1f-8a2b-3c4d5e6f7a8b".into() };
    let cases: Vec<(Edits, &str)> = vec![
        (Edits { knob_moves: vec![mv("Lake", 3, "Test", 4, false)], renames: vec![rename("Lake", 3)], ..Default::default() }, "moved and changed"),
        (Edits { knob_moves: vec![mv("Lake", 3, "Test", 4, false)], conv_knobs: vec![conv.clone()], ..Default::default() }, "moved and changed"),
        (Edits { knob_moves: vec![mv("Lake", 3, "Test", 4, false), mv("Lake", 4, "Test", 4, false)], ..Default::default() }, "moved and changed"),
        (Edits { knob_moves: vec![mv("Lake", 3, "Test", 4, false)], knob_copies: vec![copy("Lake", 3, "Test", 5, Some(ch))], ..Default::default() }, "moved and changed"),
        (Edits { knob_copies: vec![copy("Lake", 3, "Test", 5, Some(ch))], renames: vec![rename("Lake", 3)], ..Default::default() }, "copied and changed"),
        (Edits { knob_copies: vec![copy("Lake", 3, "Test", 4, Some(ch))], conv_knobs: vec![conv], ..Default::default() }, "gets a copy"),
        (Edits { knob_copies: vec![copy("Lake", 3, "Test", 4, Some(ch)), copy("Lake", 4, "Test", 4, Some("6f7a8b9c-0d1e-4f2a-9b3c-4d5e6f7a8b9c"))], ..Default::default() }, "gets a copy"),
        (Edits { knob_moves: vec![mv("Lake", 3, "Test", 4, false)], zone_order: Some(vec![]), ..Default::default() }, "on their own"),
        (Edits { knob_moves: vec![mv("Lake", 3, "Lake", 4, false)], ..Default::default() }, "swap the two knobs"),
        (Edits { knob_copies: vec![copy("Lake", 3, "Lake", 4, Some(ch))], ..Default::default() }, "isn't an empty position"),
    ];
    for (edits, want) in cases {
        let err = edit_set(&dir, &tpl, &edits).unwrap_err().to_string();
        assert!(err.contains(want), "expected {want:?}, got {err}");
    }
}

#[test]
fn settings_combine_with_a_knob_rename() {
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let s = Summary::read(&p).unwrap();
    let (zone, knob) = (s.zones[0].name.clone(), s.zones[0].knobs[0].index);
    let edits = Edits {
        renames: vec![g5_core::write::KnobRename { zone, knob, name: "Set Test".into() }],
        settings: [("hold.enabled".to_string(), 1), ("hold.timeout".to_string(), 5)].into(),
        ..Default::default()
    };
    let set = edit_set(&dir, &tpl, &edits).unwrap();
    assert_eq!(set.settings.len(), 2);
    let mut changed = set.changed_tables.clone();
    changed.sort();
    assert_eq!(changed, ["tabChannel", "tabFunctionsOnOff", "tabFunctionsParam", "tabReceivingMode"]);

    let zip = &set.files.iter().find(|(p, _)| *p == g5_core::write::PC_ZIP).unwrap().1;
    let tmp = tempfile::NamedTempFile::new().unwrap();
    fs::write(tmp.path(), zip).unwrap();
    let back = Summary::read(&Profile::from_zip(tmp.path()).unwrap()).unwrap();
    assert_eq!((back.settings["hold.enabled"], back.settings["hold.timeout"]), (1, 5), "settings lost from the zip");
}

#[test]
fn unchanged_settings_alone_are_no_changes() {
    let Some((dir, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let p = Profile::from_backup(&dir).unwrap();
    let cur = Summary::read(&p).unwrap().settings["hold.enabled"];
    let edits = Edits { settings: [("hold.enabled".to_string(), cur)].into(), ..Default::default() };
    assert!(edit_set(&dir, &tpl, &edits).unwrap_err().to_string().contains("no changes"));
}

/// A zone reorder renumbers digital channels too: their signaling rows are
/// keyed by channel ID. Runs on every backup that has a DMR or P25 conventional
/// knob.
#[test]
fn zone_reorder_renumbers_digital_channels() {
    let Some((_, tpl)) = fixture() else { return eprintln!("skipping: fixtures not available") };
    let root = std::env::var_os("G5_BACKUPS").map(PathBuf::from).unwrap_or(repo().join("backups"));
    let mut checked = 0;
    for dir in fs::read_dir(&root).unwrap().filter_map(|e| e.ok().map(|e| e.path())) {
        let Ok(radio) = sqlite2::Connection::open(&dir.join(g5_codeplug::source::BACKUP_RADIO_DB)) else { continue };
        let digital = ["tabDMRRepeaterSetting", "tabP25RepeaterSetting"]
            .iter()
            .any(|t| radio.query(&format!("SELECT count(*) FROM {t}"), &[]).is_ok_and(|r| r.rows[0][0].as_deref().is_some_and(|n| n != "0")));
        let Ok(p) = Profile::from_backup(&dir) else { continue };
        let mut order: Vec<String> = g5_codeplug::row::load_table(&p.db, "tabZone_New").iter().map(|z| z.s("ID").to_string()).collect();
        if !digital || order.len() < 2 {
            continue;
        }
        order.reverse();
        let set = edit_set(&dir, &tpl, &Edits { zone_order: Some(order), ..Default::default() }).unwrap_or_else(|e| panic!("{}: {e:#}", dir.display()));
        assert!(set.changed_tables.iter().any(|t| t.starts_with("tabDMR") || t.starts_with("tabP25R") || t == "tabP25Signaling"), "{:?}", set.changed_tables);
        checked += 1;
    }
    eprintln!("zone reorder with digital channels: {checked} backups");
}
