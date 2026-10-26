//! RadioReference import. The fixtures are made up, but have the layout and
//! quirks of real exports and page pastes (zero-padded site numbers, `c`
//! markers, trailing spaces in alpha tags, a repeated TGID, the "Online"
//! badge splitting a pasted row). Real files in /rr-data are compared too
//! when present; they're RadioReference's data and stay out of git.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use g5_codeplug::rr_import::{parse_sites, parse_talkgroups, plan_import, Encryption, Selection, Sites, Talkgroups};
use g5_codeplug::system_edit::{ControlChannelEdit, SiteEdit, SystemEdit, TalkgroupEdit};

const SITES_CSV: &str = "\
RFSS,Site Dec,Site Hex,Site NAC,Description,County Name,Lat,Lon,Range,Frequencies
1,001,1,3A1,\"North County Simulcast\",\"North\",45.1,-101.5,20,851.012500c,851.287500,852.162500c,853.112500
1,002,2,3A2,\"South Tower\",\"South\",44.9,-101.6,15,852.162500c,854.062500,855.387500c
2,010,a,3B0,\"East, Industrial\",\"East\",45.0,-101.3,5,856.412500c,857.412500
";

const SITES_PASTE: &str = "\
1 (1)\t001 (1)\t3A1\tNorth County Simulcast\tNorth, ZZ\t
4 freqs \u{b7} 2 control
851.0125
Ctrl
852.1625
Ctrl
851.2875
853.1125

1 (1)\t002 (2)\t3A2\tSouth Tower\tSouth, ZZ\t
3 freqs \u{b7} 2 control
852.1625
Ctrl
855.3875
Ctrl
854.0625

2 (2)\t010 (a)\t3B0\tEast, Industrial\tEast, ZZ\t
2 freqs \u{b7} 1 control
856.4125
Ctrl
857.4125
";

const TGS_CSV: &str = "\
Decimal,Hex,Alpha Tag,Mode,Description,Tag,Category
1,001,\"City EM 1\",\"D\",\"Emergency Management\",\"Emergency Ops\",\"City Emergency Management\"
3,003,\"City PD Dispatch\",\"DE\",\"Police Dispatch\",\"Law Dispatch\",\"City Police\"
4,004,\"City PD Tac \",\"De\",\"Police Tac, sometimes clear\",\"Law Tac\",\"City Police\"
11,00b,\"County Fire Simulcast\",\"D\",\"Fire Dispatch: \"\"Simulcast\"\"\",\"Fire Dispatch\",\"County Fire\"
40100,9ca4,\"Water Main\",\"T\",\"Water Main Dispatch\",\"Utilities\",\"Utilities\"
3,003,\"Dup PD\",\"DE\",\"Listed under a second agency\",\"Law Dispatch\",\"County Police\"
";

const TGS_PASTE: &str = "\
1\t001\tD\tCity EM 1\tEmergency Management\tEmergency Ops\tCity Emergency Management\t2013-10-12
3\t003\tD Enc\tCity PD Dispatch\tPolice Dispatch\tLaw Dispatch\tCity Police\t2013-10-12
4\t004\tD enc\tCity PD Tac\tPolice Tac, sometimes clear\tLaw Tac\tCity Police\t2013-10-12
11\t00b\tD\tCounty Fire Simulcast\tFire Dispatch: \"Simulcast\"\tFire Dispatch\tCounty Fire\t2013-10-12
40100
Online\t9ca4\tT\tWater Main\tWater Main Dispatch\tUtilities\tUtilities\t2014-06-22
3\t003\tD Enc\tDup PD\tListed under a second agency\tLaw Dispatch\tCounty Police\t2013-10-12
";

/// A site as (RFSS, site, description, [(Hz, control)]), for comparing formats.
type SiteView = (i64, i64, String, Vec<(i64, bool)>);

fn site_view(s: &Sites) -> Vec<SiteView> {
    s.sites.iter().map(|s| (s.rfss, s.site, s.description.clone(), s.freqs.iter().map(|f| (f.hz, f.control)).collect())).collect()
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

fn tg_view(t: &Talkgroups) -> Vec<(i64, String, String, String, String, String)> {
    t.talkgroups.iter().map(|t| (t.tgid, t.alpha_tag.clone(), t.mode.clone(), t.description.clone(), t.tag.clone(), t.category.clone())).collect()
}

#[test]
fn csv_and_paste_parse_the_same() {
    let (csv, paste) = (parse_sites(SITES_CSV).unwrap(), parse_sites(SITES_PASTE).unwrap());
    assert_eq!(site_view(&csv), site_view(&paste));
    assert!(csv.marked && paste.marked);
    assert_eq!(csv.problems, vec![]);
    assert_eq!(paste.problems, vec![]);
    let s = &csv.sites[2];
    assert_eq!((s.rfss, s.site, s.description.as_str(), s.nac.as_str()), (2, 10, "East, Industrial", "3B0"));
    assert_eq!(csv.sites[0].freqs[0].hz, 851_012_500);

    let (csv, paste) = (parse_talkgroups(TGS_CSV).unwrap(), parse_talkgroups(TGS_PASTE).unwrap());
    assert_eq!(tg_view(&csv), tg_view(&paste));
    assert_eq!(csv.talkgroups.len(), 5);
    let modes: Vec<_> = csv.talkgroups.iter().map(|t| (t.tgid, t.encryption)).collect();
    assert_eq!(modes, [(1, Encryption::Clear), (3, Encryption::Full), (4, Encryption::Partial), (11, Encryption::Clear), (40100, Encryption::Clear)]);
    assert_eq!(csv.talkgroups[2].alpha_tag, "City PD Tac");
    assert_eq!(csv.talkgroups[3].description, "Fire Dispatch: \"Simulcast\"");
    // The repeated TGID keeps its first row and is reported.
    for p in [&csv.problems, &paste.problems] {
        assert_eq!(p.len(), 1);
        assert!(p[0].message.contains("TGID 3 is listed again"), "{p:?}");
    }
    assert_eq!(csv.problems[0].line, 7);
    assert_eq!(paste.problems[0].line, 7);
}

#[test]
fn bad_rows_are_reported_not_fatal() {
    let sites = "RFSS,Site Dec,Site Hex,Site NAC,Description,County Name,Lat,Lon,Range,Frequencies\n\
                 1,001,1,,A,,,,,851.0125c,85x.1\n\
                 300,001,1,,B,,,,,851.0125c\n\
                 1,999,zz,,C,,,,,851.0125c\n\
                 1,001,1,,Again,,,,,852.0125c\n";
    let s = parse_sites(sites).unwrap();
    assert_eq!(s.sites.len(), 1);
    assert_eq!(s.sites[0].freqs.len(), 1);
    let lines: Vec<usize> = s.problems.iter().map(|p| p.line).collect();
    assert_eq!(lines, [2, 3, 4, 5]);

    let cut = "1 (1)\t001 (1)\t3A1\tNorth\tNorth, ZZ\t\n4 freqs \u{b7} 2 control\n851.0125\nCtrl\n";
    let s = parse_sites(cut).unwrap();
    assert!(s.problems[0].message.contains("cut short"), "{:?}", s.problems);

    assert!(parse_sites("nothing useful here\n").is_err());
    assert!(parse_talkgroups("Decimal,Alpha Tag\n0,zero\n70000,big\n").is_err());
}

const BAND_800: &[(i64, i64)] = &[(851_000_000, 869_000_000)];

#[test]
fn new_system_takes_marked_channels_and_clear_talkgroups() {
    let (sites, tgs) = (parse_sites(SITES_CSV).unwrap(), parse_talkgroups(TGS_CSV).unwrap());
    let sel = Selection::defaults(&sites, &tgs);
    assert_eq!(sel.tgids, [1, 4, 11, 40100]);
    let plan = plan_import(None, BAND_800, &sites, &tgs, &sel).unwrap();

    let names: Vec<_> = plan.sites.iter().map(|s| (s.name.as_str(), s.rfss, s.site)).collect();
    assert_eq!(names, [("North Co Simul", 1, 1), ("South Tower", 1, 2), ("East, Ind", 2, 10)]);
    // Marked only, and 852.1625 (shared by two sites) once.
    let hz: Vec<_> = plan.control_channels.iter().map(|c| c.rx_hz).collect();
    assert_eq!(hz, [851_012_500, 852_162_500, 855_387_500, 856_412_500]);
    assert_eq!(sites.sites[1].freqs.iter().map(|f| f.hz).collect::<Vec<_>>(), [852_162_500, 855_387_500, 854_062_500]);
    assert_eq!(plan.control_channels[0].note, "North County Simulcast");
    let tgs_added: Vec<_> = plan.talkgroups.iter().map(|t| (t.tgid, t.name.as_str())).collect();
    assert_eq!(tgs_added, [(1, "City EM 1"), (4, "City PD Tac"), (11, "Co Fire Simul"), (40100, "Water Main")]);
    let renamed: Vec<_> = plan.renamed.iter().map(|r| (r.kind.as_str(), r.to.as_str())).collect();
    assert_eq!(renamed, [("site", "North Co Simul"), ("site", "East, Ind"), ("talkgroup", "Co Fire Simul")]);
    assert!(plan.out_of_band.is_empty());

    let all = plan_import(None, BAND_800, &sites, &tgs, &Selection { all_freqs: true, ..sel.clone() }).unwrap();
    assert_eq!(all.control_channels.len(), 8);
    // A 700 MHz band leaves everything out.
    let wrong = plan_import(None, &[(769_000_000, 775_000_000)], &sites, &tgs, &sel).unwrap();
    assert!(wrong.control_channels.is_empty());
    assert_eq!(wrong.out_of_band.len(), 4);
}

#[test]
fn update_adds_only_whats_missing() {
    let (sites, tgs) = (parse_sites(SITES_CSV).unwrap(), parse_talkgroups(TGS_CSV).unwrap());
    let existing = SystemEdit {
        system_id: "x".into(),
        name: "Metro".into(),
        wacn: 0xBEE00,
        sysid: 0x1A3,
        sites: vec![
            SiteEdit { id: Some("1".into()), name: "My North".into(), rfss: 1, site: 1 },
            SiteEdit { id: Some("2".into()), name: "Old Site".into(), rfss: 9, site: 9 },
        ],
        control_channels: vec![
            ControlChannelEdit { id: Some(1), rx_hz: 851_012_500, note: String::new() },
            ControlChannelEdit { id: Some(2), rx_hz: 860_000_000, note: String::new() },
        ],
        talkgroups: vec![
            TalkgroupEdit { group_id: "0000000000000000".into(), tgid: 65535, name: "All".into(), note: String::new(), ..Default::default() },
            TalkgroupEdit { group_id: "a".into(), tgid: 1, name: "My EM".into(), note: String::new(), ..Default::default() },
            TalkgroupEdit { group_id: "b".into(), tgid: 500, name: "Gone".into(), note: String::new(), ..Default::default() },
        ],
    };
    let plan = plan_import(Some(&existing), BAND_800, &sites, &tgs, &Selection::defaults(&sites, &tgs)).unwrap();
    assert_eq!(plan.sites.iter().map(|s| (s.rfss, s.site)).collect::<Vec<_>>(), [(1, 2), (2, 10)]);
    assert_eq!(plan.sites_present, 1);
    assert_eq!(plan.control_channels.iter().map(|c| c.rx_hz).collect::<Vec<_>>(), [852_162_500, 855_387_500, 856_412_500]);
    assert_eq!(plan.control_channels_present, 1);
    assert_eq!(plan.talkgroups.iter().map(|t| t.tgid).collect::<Vec<_>>(), [4, 11, 40100]);
    assert_eq!(plan.talkgroups_present, 1);
    assert_eq!(plan.not_in_import.sites, ["Old Site"]);
    assert_eq!(plan.not_in_import.control_channels, [860_000_000]);
    assert_eq!(plan.not_in_import.talkgroups.iter().map(|t| t.tgid).collect::<Vec<_>>(), [500]);
}

#[test]
fn shortened_site_names_stay_unique() {
    let csv = "RFSS,Site Dec,Site Hex,Description,Frequencies\n\
               3,001,1,ZZ Hollins County Simulcast,851.0125c\n\
               3,003,3,ZZ Hollins County Corrections,851.2875c\n\
               3,004,4,ZZ Hollins County Courthouse,851.5125c\n";
    let sites = parse_sites(csv).unwrap();
    let tgs = parse_talkgroups(TGS_CSV).unwrap();
    let plan = plan_import(None, BAND_800, &sites, &tgs, &Selection::defaults(&sites, &tgs)).unwrap();
    let names: Vec<_> = plan.sites.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["ZZ Hollins Co", "ZZ Hollin Corr", "ZZ Hollins 3-4"]);

    let mut existing = SystemEdit { system_id: "x".into(), name: "x".into(), wacn: 1, sysid: 1, sites: vec![], control_channels: vec![], talkgroups: vec![] };
    existing.sites.push(SiteEdit { id: Some("1".into()), name: "ZZ Hollins Co".into(), rfss: 9, site: 9 });
    existing.sites.push(SiteEdit { id: Some("2".into()), name: "ZZ Hollins 3-1".into(), rfss: 9, site: 8 });
    let plan = plan_import(Some(&existing), BAND_800, &sites, &tgs, &Selection::defaults(&sites, &tgs)).unwrap();
    assert_eq!(plan.sites[0].name, "ZZ Hollins C 2");
}

#[test]
fn limits_are_enforced() {
    let mut csv = String::from("Decimal,Alpha Tag\n");
    for i in 1..=8200 {
        csv += &format!("{i},TG {i}\n");
    }
    let tgs = parse_talkgroups(&csv).unwrap();
    let sites = parse_sites(SITES_CSV).unwrap();
    let sel = Selection::defaults(&sites, &tgs);
    let err = plan_import(None, BAND_800, &sites, &tgs, &sel).unwrap_err();
    assert!(err.to_string().contains("at most 8192"), "{err}");
}

/// A real RadioReference export, when present: the CSV and the
/// page paste must agree.
#[test]
fn real_csv_matches_real_paste() {
    let rr = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rr-data");
    let (Some(sites_csv), Some(tg_csv)) = (rr_file(&rr, "trs_sites_"), rr_file(&rr, "trs_tg_")) else {
        return eprintln!("skipping: no RadioReference export in {}", rr.display());
    };
    let dir = sites_csv.parent().unwrap().to_path_buf();
    let read = |f: &str| fs::read_to_string(dir.join(f));
    let (Ok(sc), Ok(sp), Ok(tc), Ok(tp)) = (fs::read_to_string(&sites_csv), read("sites-paste.txt"), fs::read_to_string(&tg_csv), read("talkgroups-paste.txt")) else {
        eprintln!("skipping: incomplete RadioReference export in {}", dir.display());
        return;
    };
    let (sc, sp) = (parse_sites(&sc).unwrap(), parse_sites(&sp).unwrap());
    assert_eq!(site_view(&sc), site_view(&sp));
    assert!(sc.problems.is_empty() && sp.problems.is_empty(), "{:?} {:?}", sc.problems, sp.problems);
    let (tc, tp) = (parse_talkgroups(&tc).unwrap(), parse_talkgroups(&tp).unwrap());
    // The CSV export leaves a stray backslash where the page shows nothing.
    let unslash = |t: &Talkgroups| tg_view(t).into_iter().map(|mut r| (r.3 = r.3.trim_end_matches('\\').trim_end().to_string(), r).1).collect::<Vec<_>>();
    assert_eq!(unslash(&tc), unslash(&tp));
    assert_eq!(tc.problems.len(), tp.problems.len());
    let plan = plan_import(None, BAND_800, &sc, &tc, &Selection::defaults(&sc, &tc)).unwrap();
    // Shortening must not make distinct alpha tags look the same.
    let mut by_name: std::collections::HashMap<&str, HashSet<&str>> = Default::default();
    for (t, rr) in plan.talkgroups.iter().zip(tc.talkgroups.iter().filter(|t| t.encryption != g5_codeplug::rr_import::Encryption::Full)) {
        by_name.entry(t.name.as_str()).or_default().insert(rr.alpha_tag.as_str());
    }
    let merged: Vec<_> = by_name.iter().filter(|(_, tags)| tags.len() > 1).collect();
    assert!(merged.is_empty(), "shortening merged distinct tags: {merged:?}");
    eprintln!(
        "RadioReference export: {} sites, {} control channels, {} of {} talkgroups by default, {} renamed",
        plan.sites.len(),
        plan.control_channels.len(),
        plan.talkgroups.len(),
        tc.talkgroups.len(),
        plan.renamed.len()
    );
}
