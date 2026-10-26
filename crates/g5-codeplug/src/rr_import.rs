//! Import a P25 system from RadioReference: the site and talkgroup CSVs that
//! Premium subscribers can download, or the same two tables copied from the
//! system's web page and pasted as text. Spec: notes/radioreference-import.md.
//!
//! Parsing is lenient: a bad row becomes a [`Problem`] and the rest still
//! imports. [`plan_import`] then turns the rows the user kept into additions
//! for a new or existing system, applying the pager's limits.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::system_edit::{ControlChannelEdit, SiteEdit, SystemEdit, MAX_CC_NOTE, MAX_CONTROL_CHANNELS, MAX_NAME, MAX_SITES, MAX_TALKGROUPS};
use crate::Error;

/// One row the parser skipped or changed, by 1-based line number in the input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RrFreq {
    pub hz: i64,
    /// RadioReference marks control channels `c` and alternates `a`.
    pub control: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RrSite {
    pub line: usize,
    pub rfss: i64,
    pub site: i64,
    pub nac: String,
    pub description: String,
    pub county: String,
    /// Control channels first, then by frequency.
    pub freqs: Vec<RrFreq>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sites {
    pub sites: Vec<RrSite>,
    /// Whether any frequency carries a control marker. Without markers every
    /// frequency counts as a control channel.
    pub marked: bool,
    pub problems: Vec<Problem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Encryption {
    Clear,
    Partial,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RrTalkgroup {
    pub line: usize,
    pub tgid: i64,
    pub alpha_tag: String,
    /// RadioReference's CSV code: `D`, `DE`, `De`, `T`, `TE`, `A`, ...
    /// (the web page's "D Enc" and "D enc" are folded to `DE` and `De`).
    pub mode: String,
    pub description: String,
    pub tag: String,
    pub category: String,
    pub encryption: Encryption,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Talkgroups {
    pub talkgroups: Vec<RrTalkgroup>,
    pub problems: Vec<Problem>,
}

/// Parse a RadioReference site table: the CSV download, or the table pasted
/// from the system page (one block per site, `Ctrl` under each control
/// channel).
pub fn parse_sites(text: &str) -> Result<Sites, Error> {
    let records = records(text);
    let mut out = match records.iter().find(|r| !r.fields.iter().all(|f| f.is_empty())) {
        Some(first) if is_site_header(&first.fields) => sites_from_table(&records, first.line)?,
        _ => sites_from_page(text),
    };
    if out.sites.is_empty() {
        return Err(Error::Invalid("no sites found; expected RadioReference's site CSV or the site table copied from the system page".into()));
    }
    out.marked = out.sites.iter().any(|s| s.freqs.iter().any(|f| f.control));
    // Control channels first, then by frequency, as the system page lists them.
    for s in &mut out.sites {
        s.freqs.sort_by_key(|f| (!f.control, f.hz));
    }
    Ok(out)
}

/// Parse a RadioReference talkgroup table: the CSV download, or the table
/// pasted from the system page.
pub fn parse_talkgroups(text: &str) -> Result<Talkgroups, Error> {
    let records = records(text);
    let header = records.iter().find(|r| !r.fields.iter().all(|f| f.is_empty())).filter(|r| tg_columns(&r.fields).is_some());
    let mut rows = Vec::new();
    let mut problems = Vec::new();
    if let Some(h) = header {
        let cols = tg_columns(&h.fields).unwrap();
        for r in records.iter().filter(|r| r.line > h.line && !r.fields.iter().all(|f| f.is_empty())) {
            let get = |i: Option<usize>| i.and_then(|i| r.fields.get(i)).map(|s| s.trim()).unwrap_or("");
            match talkgroup(r.line, get(Some(cols.decimal)), get(cols.mode), get(Some(cols.alpha_tag)), get(cols.description), get(cols.tag), get(cols.category)) {
                Ok(t) => rows.push(t),
                Err(m) => problems.push(Problem { line: r.line, message: m }),
            }
        }
    } else {
        // Page paste: DEC, HEX, Mode, Alpha Tag, Description, Tag, Category[, updated].
        // A "listen online" badge splits some rows: the decimal ends up alone
        // on one line and the next line starts with "Online".
        let mut pending: Option<(usize, String)> = None;
        for r in &records {
            let f: Vec<&str> = r.fields.iter().map(|s| s.trim()).collect();
            if f.iter().all(|s| s.is_empty()) {
                continue;
            }
            if f.len() == 1 && f[0].bytes().all(|b| b.is_ascii_digit()) {
                pending = Some((r.line, f[0].to_string()));
                continue;
            }
            let (line, dec) = match pending.take() {
                Some((line, dec)) if !f[0].bytes().all(|b| b.is_ascii_digit()) => (line, dec),
                _ => (r.line, f[0].to_string()),
            };
            if f.len() < 4 {
                problems.push(Problem { line, message: "not a talkgroup row".into() });
                continue;
            }
            let hex_matches = dec.parse::<i64>().ok().zip(i64::from_str_radix(f[1], 16).ok()).is_some_and(|(d, h)| d == h);
            if !hex_matches {
                problems.push(Problem { line, message: "not a talkgroup row (expected decimal and hex IDs first)".into() });
                continue;
            }
            let at = |i: usize| f.get(i).copied().unwrap_or("");
            match talkgroup(line, &dec, at(2), at(3), at(4), at(5), at(6)) {
                Ok(t) => rows.push(t),
                Err(m) => problems.push(Problem { line, message: m }),
            }
        }
    }
    let mut seen = HashMap::new();
    let mut talkgroups = Vec::new();
    for t in rows {
        if let Some(first) = seen.get(&t.tgid) {
            problems.push(Problem { line: t.line, message: format!("TGID {} is listed again (first on line {first}); kept the first", t.tgid) });
            continue;
        }
        seen.insert(t.tgid, t.line);
        talkgroups.push(t);
    }
    if talkgroups.is_empty() {
        return Err(Error::Invalid("no talkgroups found; expected RadioReference's talkgroup CSV or the talkgroup table copied from the system page".into()));
    }
    problems.sort_by_key(|p| p.line);
    Ok(Talkgroups { talkgroups, problems })
}

/// What the user kept in the preview.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// (RFSS, Site) of the sites to import.
    pub sites: Vec<(i64, i64)>,
    pub tgids: Vec<i64>,
    /// Import every frequency of the chosen sites as a control channel, not
    /// only the marked ones.
    #[serde(default)]
    pub all_freqs: bool,
}

impl Selection {
    /// The defaults: every site, every talkgroup that isn't fully encrypted,
    /// marked frequencies only.
    pub fn defaults(sites: &Sites, tgs: &Talkgroups) -> Self {
        Self {
            sites: sites.sites.iter().map(|s| (s.rfss, s.site)).collect(),
            tgids: tgs.talkgroups.iter().filter(|t| t.encryption != Encryption::Full).map(|t| t.tgid).collect(),
            all_freqs: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewTalkgroup {
    pub tgid: i64,
    pub name: String,
    pub note: String,
}

/// A name that had to change to fit the pager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Renamed {
    pub kind: String,
    pub from: String,
    pub to: String,
}

/// Existing items the import didn't mention, for the user to review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotInImport {
    pub sites: Vec<String>,
    pub control_channels: Vec<i64>,
    pub talkgroups: Vec<NewTalkgroup>,
}

/// Additions to apply to the target system. New talkgroups need GUIDs, which
/// the caller assigns.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPlan {
    pub sites: Vec<SiteEdit>,
    pub control_channels: Vec<ControlChannelEdit>,
    pub talkgroups: Vec<NewTalkgroup>,
    /// Already in the system, so left as they are.
    pub sites_present: usize,
    pub control_channels_present: usize,
    pub talkgroups_present: usize,
    pub renamed: Vec<Renamed>,
    /// Chosen frequencies outside the band, left out.
    pub out_of_band: Vec<i64>,
    pub not_in_import: NotInImport,
}

/// Work out what to add to `existing` (None for a new system) so it holds
/// the selected sites, control channels and talkgroups. Never renames,
/// renumbers or removes anything already there. `ranges` are the band's
/// receive ranges in Hz.
pub fn plan_import(existing: Option<&SystemEdit>, ranges: &[(i64, i64)], sites: &Sites, tgs: &Talkgroups, sel: &Selection) -> Result<ImportPlan, Error> {
    let mut plan = ImportPlan::default();
    let mut present_hz = HashSet::new();
    let chosen_sites: HashSet<(i64, i64)> = sel.sites.iter().copied().collect();
    let chosen_tgs: HashSet<i64> = sel.tgids.iter().copied().collect();
    let empty = SystemEdit { system_id: String::new(), name: String::new(), wacn: 0, sysid: 0, sites: vec![], control_channels: vec![], talkgroups: vec![] };
    let base = existing.unwrap_or(&empty);

    let have_sites: HashSet<(i64, i64)> = base.sites.iter().map(|s| (s.rfss, s.site)).collect();
    let mut have_hz: HashSet<i64> = base.control_channels.iter().map(|c| c.rx_hz).collect();
    let have_tgs: HashSet<i64> = base.talkgroups.iter().map(|t| t.tgid).collect();
    let in_band = |hz: i64| ranges.iter().any(|&(lo, hi)| (lo..=hi).contains(&hz));

    // Site names must be unique within a system, and shortening can collide.
    let mut site_names: HashSet<String> = base.sites.iter().map(|s| s.name.clone()).collect();
    let use_all = sel.all_freqs || !sites.marked;
    for s in sites.sites.iter().filter(|s| chosen_sites.contains(&(s.rfss, s.site))) {
        if have_sites.contains(&(s.rfss, s.site)) {
            plan.sites_present += 1;
        } else {
            let mut name = fit_name(&s.description, || format!("Site {}-{}", s.rfss, s.site));
            if site_names.contains(&name) {
                name = unique_name(&name, &format!(" {}-{}", s.rfss, s.site), &site_names);
            }
            site_names.insert(name.clone());
            if name != s.description {
                plan.renamed.push(Renamed { kind: "site".into(), from: s.description.clone(), to: name.clone() });
            }
            plan.sites.push(SiteEdit { id: None, name, rfss: s.rfss, site: s.site });
        }
        for f in s.freqs.iter().filter(|f| use_all || f.control) {
            if !in_band(f.hz) {
                if !plan.out_of_band.contains(&f.hz) {
                    plan.out_of_band.push(f.hz);
                }
            } else if have_hz.insert(f.hz) {
                plan.control_channels.push(ControlChannelEdit { id: None, rx_hz: f.hz, note: ascii(&s.description).chars().take(MAX_CC_NOTE).collect() });
            } else if base.control_channels.iter().any(|c| c.rx_hz == f.hz) {
                present_hz.insert(f.hz);
            }
        }
    }

    plan.control_channels_present = present_hz.len();

    for t in tgs.talkgroups.iter().filter(|t| chosen_tgs.contains(&t.tgid)) {
        // 65535 is the shared "All" talkgroup every system already has.
        if have_tgs.contains(&t.tgid) || t.tgid == 65535 {
            plan.talkgroups_present += 1;
            continue;
        }
        let name = fit_name(&t.alpha_tag, || t.tgid.to_string());
        if name != t.alpha_tag {
            plan.renamed.push(Renamed { kind: "talkgroup".into(), from: t.alpha_tag.clone(), to: name.clone() });
        }
        // tabGroup_New.Note is VARCHAR(240).
        plan.talkgroups.push(NewTalkgroup { tgid: t.tgid, name, note: ascii(&t.description).chars().take(240).collect() });
    }

    let imported_sites: HashSet<(i64, i64)> = sites.sites.iter().map(|s| (s.rfss, s.site)).collect();
    let imported_hz: HashSet<i64> = sites.sites.iter().flat_map(|s| s.freqs.iter().map(|f| f.hz)).collect();
    let imported_tgs: HashSet<i64> = tgs.talkgroups.iter().map(|t| t.tgid).collect();
    plan.not_in_import = NotInImport {
        sites: base.sites.iter().filter(|s| !imported_sites.contains(&(s.rfss, s.site))).map(|s| s.name.clone()).collect(),
        control_channels: base.control_channels.iter().filter(|c| !imported_hz.contains(&c.rx_hz)).map(|c| c.rx_hz).collect(),
        // The shared "All" talkgroup (65535) is never in RadioReference.
        talkgroups: base
            .talkgroups
            .iter()
            .filter(|t| !imported_tgs.contains(&t.tgid) && t.tgid != 65535)
            .map(|t| NewTalkgroup { tgid: t.tgid, name: t.name.clone(), note: t.note.clone() })
            .collect(),
    };

    for (what, have, add, max) in [
        ("sites", base.sites.len(), plan.sites.len(), MAX_SITES),
        ("control channels", base.control_channels.len(), plan.control_channels.len(), MAX_CONTROL_CHANNELS),
        // A new system also gets the template's "All" talkgroup.
        ("talkgroups", existing.map_or(1, |e| e.talkgroups.len()), plan.talkgroups.len(), MAX_TALKGROUPS),
    ] {
        if have + add > max {
            return Err(Error::Invalid(format!("the system would have {} {what}; the pager holds at most {max}", have + add)));
        }
    }
    Ok(plan)
}

// --- Sites -------------------------------------------------------------------

fn is_site_header(fields: &[String]) -> bool {
    let n: Vec<String> = fields.iter().map(|f| norm(f)).collect();
    n.iter().any(|f| f == "rfss") && n.iter().any(|f| f == "sitedec" || f == "sitehex")
}

fn sites_from_table(records: &[Record], header_line: usize) -> Result<Sites, Error> {
    let header = &records.iter().find(|r| r.line == header_line).unwrap().fields;
    let col = |names: &[&str]| header.iter().position(|h| names.contains(&norm(h).as_str()));
    let (Some(rfss), Some(freq_col)) = (col(&["rfss"]), col(&["frequencies", "freqs"])) else {
        return Err(Error::Invalid("the site table needs RFSS and Frequencies columns".into()));
    };
    let (dec, hex) = (col(&["sitedec"]), col(&["sitehex"]));
    let (nac, descr, county) = (col(&["sitenac", "nac"]), col(&["description"]), col(&["countyname", "county"]));

    let mut out = Sites { sites: vec![], marked: false, problems: vec![] };
    let mut seen = HashMap::new();
    for r in records.iter().filter(|r| r.line > header_line && !r.fields.iter().all(|f| f.trim().is_empty())) {
        let get = |i: Option<usize>| i.and_then(|i| r.fields.get(i)).map(|s| s.trim()).unwrap_or("");
        let Some(rfss) = get(Some(rfss)).parse::<i64>().ok().filter(|v| (0..=255).contains(v)) else {
            out.problems.push(Problem { line: r.line, message: format!("RFSS {:?} isn't 0 to 255", get(Some(rfss))) });
            continue;
        };
        // PPS also falls back to the hex column when the decimal one is unusable.
        let site = get(dec).parse::<i64>().ok().filter(|v| (0..=255).contains(v)).or_else(|| i64::from_str_radix(get(hex), 16).ok().filter(|v| (0..=255).contains(v)));
        let Some(site) = site else {
            out.problems.push(Problem { line: r.line, message: "site number isn't 0 to 255".into() });
            continue;
        };
        if let Some(first) = seen.insert((rfss, site), r.line) {
            out.problems.push(Problem { line: r.line, message: format!("site {rfss}-{site} is listed again (first on line {first}); kept the first") });
            continue;
        }
        let mut freqs = Vec::new();
        // Every column from Frequencies on holds one frequency.
        for f in r.fields.iter().skip(freq_col).map(|s| s.trim()).filter(|s| !s.is_empty()) {
            match parse_freq(f) {
                Some(fr) => push_freq(&mut freqs, fr),
                None => out.problems.push(Problem { line: r.line, message: format!("frequency {f:?} isn't a number of MHz; left out") }),
            }
        }
        out.sites.push(RrSite { line: r.line, rfss, site, nac: get(nac).into(), description: clean(get(descr)), county: clean(get(county)), freqs });
    }
    Ok(out)
}

/// The site table as pasted from the system page:
///
/// ```text
/// 1 (1)<TAB>001 (1)<TAB>3A1<TAB>North County Simulcast<TAB>North, ZZ<TAB>
/// 2 freqs · 1 control
/// 851.0125
/// Ctrl
/// 851.2875
/// ```
fn sites_from_page(text: &str) -> Sites {
    let mut out = Sites { sites: vec![], marked: false, problems: vec![] };
    let mut seen = HashMap::new();
    let mut skip_current = false;
    // The "24 freqs · 3 control" line under each site, to check against.
    let mut expected: HashMap<usize, (usize, usize, usize)> = HashMap::new();
    for (i, raw) in text.lines().enumerate() {
        let line = i + 1;
        let l = raw.trim();
        if l.is_empty() {
            continue;
        }
        let f: Vec<&str> = raw.split('\t').map(|s| s.trim()).collect();
        let ids = if f.len() >= 2 { id_pair(f[0]).zip(id_pair(f[1])) } else { None };
        if let Some((rfss, site)) = ids {
            {
                if !(0..=255).contains(&rfss) || !(0..=255).contains(&site) {
                    out.problems.push(Problem { line, message: format!("site {rfss}-{site}: RFSS and site must be 0 to 255") });
                    skip_current = true;
                    continue;
                }
                if let Some(first) = seen.insert((rfss, site), line) {
                    out.problems.push(Problem { line, message: format!("site {rfss}-{site} is listed again (first on line {first}); kept the first") });
                    skip_current = true;
                    continue;
                }
                skip_current = false;
                let at = |i: usize| clean(f.get(i).copied().unwrap_or(""));
                out.sites.push(RrSite { line, rfss, site, nac: at(2), description: at(3), county: at(4), freqs: vec![] });
                continue;
            }
        }
        if skip_current {
            continue;
        }
        let Some(current) = out.sites.last_mut() else {
            // A copied header row ("RFSS  Site  NAC ...") is expected here.
            if !l.contains("RFSS") {
                out.problems.push(Problem { line, message: "text before the first site; left out".into() });
            }
            continue;
        };
        if let Some(counts) = freq_counts(l) {
            expected.insert(out.sites.len() - 1, (line, counts.0, counts.1));
            continue;
        }
        if l.eq_ignore_ascii_case("ctrl") || l.eq_ignore_ascii_case("alt") {
            match current.freqs.last_mut() {
                Some(fr) => fr.control = true,
                None => out.problems.push(Problem { line, message: format!("{l} without a frequency above it") }),
            }
        } else if let Some(fr) = parse_freq(l) {
            push_freq(&mut current.freqs, fr);
        } else {
            out.problems.push(Problem { line, message: format!("unrecognized line {l:?}; left out") });
        }
    }
    for (i, (line, freqs, control)) in expected {
        let s = &out.sites[i];
        let got = (s.freqs.len(), s.freqs.iter().filter(|f| f.control).count());
        if got != (freqs, control) {
            out.problems.push(Problem {
                line,
                message: format!("site {}-{} lists {freqs} frequencies ({control} control) but {} ({}) were found; was the paste cut short?", s.rfss, s.site, got.0, got.1),
            });
        }
    }
    out.problems.sort_by_key(|p| p.line);
    out
}

/// `"24 freqs · 3 control"` -> (24, 3).
fn freq_counts(l: &str) -> Option<(usize, usize)> {
    let (freqs, rest) = l.split_once(" freqs")?;
    let control = rest.trim_start_matches(|c: char| !c.is_ascii_digit()).split_whitespace().next()?;
    Some((freqs.trim().parse().ok()?, control.parse().ok()?))
}

/// `"3 (3)"` or `"001 (1)"`: decimal, then hex in parentheses (must agree).
fn id_pair(s: &str) -> Option<i64> {
    let (dec, rest) = s.split_once(" (")?;
    let hex = rest.strip_suffix(')')?;
    let (d, h) = (dec.trim().parse::<i64>().ok()?, i64::from_str_radix(hex.trim(), 16).ok()?);
    (d == h).then_some(d)
}

fn push_freq(freqs: &mut Vec<RrFreq>, fr: RrFreq) {
    match freqs.iter_mut().find(|f| f.hz == fr.hz) {
        Some(f) => f.control |= fr.control,
        None => freqs.push(fr),
    }
}

/// `"851.887500c"` -> 851_887_500 Hz, control. Exact decimal arithmetic.
fn parse_freq(s: &str) -> Option<RrFreq> {
    let s = s.trim();
    let body = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let marker = &s[body.len()..];
    let control = matches!(marker, "c" | "C" | "a" | "A");
    if !(marker.is_empty() || control) {
        return None;
    }
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (keep, extra) = frac.split_at(frac.len().min(6));
    if extra.bytes().any(|b| b != b'0') {
        return None;
    }
    let mhz: i64 = whole.parse().ok().filter(|&m| m < 10_000)?;
    let micro: i64 = format!("{keep:0<6}").parse().ok()?;
    Some(RrFreq { hz: mhz * 1_000_000 + micro, control })
}

// --- Talkgroups --------------------------------------------------------------

struct TgColumns {
    decimal: usize,
    alpha_tag: usize,
    mode: Option<usize>,
    description: Option<usize>,
    tag: Option<usize>,
    category: Option<usize>,
}

fn tg_columns(fields: &[String]) -> Option<TgColumns> {
    let n: Vec<String> = fields.iter().map(|f| norm(f)).collect();
    let col = |names: &[&str]| n.iter().position(|h| names.contains(&h.as_str()));
    Some(TgColumns {
        decimal: col(&["decimal", "dec"])?,
        alpha_tag: col(&["alphatag", "alpha"])?,
        mode: col(&["mode"]),
        description: col(&["description"]),
        tag: col(&["tag"]),
        category: col(&["category"]),
    })
}

fn talkgroup(line: usize, dec: &str, mode: &str, alpha: &str, descr: &str, tag: &str, category: &str) -> Result<RrTalkgroup, String> {
    let tgid = dec.parse::<i64>().ok().filter(|v| (1..=65535).contains(v)).ok_or_else(|| format!("TGID {dec:?} isn't 1 to 65535"))?;
    let mode = fold_mode(mode);
    let encryption = match mode.chars().last() {
        Some('E') if mode.len() > 1 => Encryption::Full,
        Some('e') => Encryption::Partial,
        _ => Encryption::Clear,
    };
    Ok(RrTalkgroup { line, tgid, alpha_tag: clean(alpha), mode, description: clean(descr), tag: clean(tag), category: clean(category), encryption })
}

/// The web page writes the CSV's `DE` as "D Enc" and `De` as "D enc".
fn fold_mode(mode: &str) -> String {
    let mut words = mode.split_whitespace();
    let base = words.next().unwrap_or("").to_string();
    match words.next() {
        Some("Enc") => base + "E",
        Some("enc") => base + "e",
        _ => base,
    }
}

// --- Shared ------------------------------------------------------------------

/// Runs of whitespace become one space, as a browser shows them.
fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Header text compared without case, spaces or punctuation.
fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

/// The pager takes printable ASCII only.
fn ascii(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_graphic() || c == ' ' { c } else { '?' }).collect::<String>().trim().to_string()
}

/// Standard abbreviations, tried on whole words (and CamelCase parts) when a
/// name is too long. RadioReference's tags already use most short forms
/// (FD, PD, Co, Tac), so this only covers the long words that remain.
const ABBREVIATIONS: &[(&str, &str)] = &[
    ("Administration", "Admin"),
    ("Airport", "Arpt"),
    ("Airports", "Arpts"),
    ("Ambulance", "Amb"),
    ("Animal", "Anml"),
    ("Center", "Ctr"),
    ("Channel", "Ch"),
    ("Command", "Cmd"),
    ("Common", "Com"),
    ("Control", "Ctrl"),
    ("Corrections", "Corr"),
    ("County", "Co"),
    ("Department", "Dept"),
    ("Dispatch", "Disp"),
    ("District", "Dist"),
    ("East", "E"),
    ("Emergency", "Emerg"),
    ("Event", "Evt"),
    ("Events", "Evts"),
    ("Facility", "Fac"),
    ("Fireground", "FG"),
    ("Highway", "Hwy"),
    ("Hospital", "Hosp"),
    ("Industrial", "Ind"),
    ("Inspect", "Insp"),
    ("Inspection", "Insp"),
    ("International", "Intl"),
    ("Interop", "IOP"),
    ("Investigations", "Inv"),
    ("Maintenance", "Maint"),
    ("Manage", "Mgmt"),
    ("Management", "Mgmt"),
    ("Medical", "Med"),
    ("North", "N"),
    ("Office", "Ofc"),
    ("Operations", "Ops"),
    ("Parks", "Pks"),
    ("Patrol", "Ptl"),
    ("Police", "Pol"),
    ("Public", "Pub"),
    ("Regional", "Reg"),
    ("Rescue", "Resc"),
    ("School", "Sch"),
    ("Schools", "Schs"),
    ("Security", "Sec"),
    ("Service", "Svc"),
    ("Services", "Svcs"),
    ("Sheriff", "Sher"),
    ("Simulcast", "Simul"),
    ("South", "S"),
    ("Station", "Sta"),
    ("Street", "St"),
    ("Supervise", "Supv"),
    ("Supervisor", "Supv"),
    ("Tactical", "Tac"),
    ("Tower", "Twr"),
    ("Training", "Trng"),
    ("Transit", "Trnst"),
    ("University", "Univ"),
    ("Utilities", "Util"),
    ("Water", "Wtr"),
    ("West", "W"),
    ("Works", "Wks"),
];

/// Fit a name into the pager's 14 printable ASCII characters, keeping as
/// much meaning as possible, in this order, stopping as soon as it fits:
/// drop spaces around `/ & + -`; attach a trailing number to the word before
/// it; abbreviate (biggest saving first); join words, if that's enough; and
/// last, cut, keeping a short last word or trailing number so "Airports 1"
/// and "Airports 2", or "Springs FD" and "Springs PD", stay distinct.
fn fit_name(s: &str, fallback: impl FnOnce() -> String) -> String {
    let mut n = clean(&ascii(s));
    if n.is_empty() {
        return fallback();
    }
    let fits = |n: &str| n.chars().count() <= MAX_NAME;
    if fits(&n) {
        return n;
    }
    for sep in ['/', '&', '+', '-'] {
        n = n.replace(&format!(" {sep}"), &sep.to_string()).replace(&format!("{sep} "), &sep.to_string());
    }
    if !fits(&n)
        && let Some((head, num)) = n.rsplit_once(' ')
        && !num.is_empty()
        && num.bytes().all(|b| b.is_ascii_digit())
    {
        n = format!("{head}{num}");
    }
    while !fits(&n) {
        // Rightmost of the biggest savings, so leading words stay readable.
        let best = word_spans(&n)
            .into_iter()
            .filter_map(|(a, b)| ABBREVIATIONS.iter().find(|(w, _)| *w == &n[a..b]).map(|(w, short)| (w.len() - short.len(), a, b, *short)))
            .max_by_key(|&(saving, a, ..)| (saving, a));
        let Some((_, a, b, short)) = best else { break };
        n.replace_range(a..b, short);
    }
    if !fits(&n) && fits(&n.replace(' ', "")) {
        while !fits(&n) {
            n = n.replacen(' ', "", 1);
        }
    }
    if !fits(&n) {
        // Keep what tells similar names apart: a short last word ("FD",
        // "PD", "Tac2") or else a trailing number.
        let last = n.rsplit_once(' ').map(|(_, w)| w).filter(|w| (1..=4).contains(&w.len()));
        let digits = n.len() - n.trim_end_matches(|c: char| c.is_ascii_digit()).len();
        let tail = match last {
            Some(w) => &n[n.len() - w.len() - 1..],
            None if (1..6).contains(&digits) => &n[n.len() - digits..],
            None => "",
        };
        let head: String = n[..n.len() - tail.len()].chars().take(MAX_NAME - tail.len()).collect();
        n = format!("{}{tail}", head.trim_end());
    }
    n
}

/// Byte ranges of words, splitting CamelCase too: "PubWorks1" -> "Pub",
/// "Works"; "ZTPDTac" -> "ZTPD", "Tac".
fn word_spans(s: &str) -> Vec<(usize, usize)> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        if b[i].is_ascii_uppercase() {
            let mut k = i;
            while k < b.len() && b[k].is_ascii_uppercase() {
                k += 1;
            }
            if k - i > 1 && k < b.len() && b[k].is_ascii_lowercase() {
                // The last capital starts the next word.
                i = k - 1;
                out.push((start, i));
                continue;
            }
            i = k;
        }
        while i < b.len() && b[i].is_ascii_lowercase() {
            i += 1;
        }
        out.push((start, i));
    }
    out
}

/// `name` shortened to make room for `suffix`, then numbered if that's taken too.
fn unique_name(name: &str, suffix: &str, taken: &HashSet<String>) -> String {
    let with = |suffix: &str| format!("{}{suffix}", name.chars().take(MAX_NAME.saturating_sub(suffix.len())).collect::<String>().trim_end());
    let first = with(suffix);
    if !taken.contains(&first) {
        return first;
    }
    (2..).map(|n| with(&format!(" {n}"))).find(|c| !taken.contains(c)).unwrap()
}

struct Record {
    line: usize,
    fields: Vec<String>,
}

/// Split CSV or tab-separated text into records. The delimiter is a tab if
/// the first non-blank line has one, else a comma. CSV may quote fields,
/// with `""` escapes and embedded newlines. RadioReference's export doesn't
/// escape quotes inside a field (`"Police 3 "Secure channel"",...`), so a
/// quote only ends a field when a delimiter or line end follows it.
/// Tab-separated text is taken literally, since a pasted description can
/// start with a quote. Handles CRLF and a UTF-8 BOM.
fn records(text: &str) -> Vec<Record> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let sep = if first.contains('\t') { '\t' } else { ',' };
    let quoting = sep == ',';
    let c: Vec<char> = text.chars().collect();
    let ends_field = |i: usize| matches!(c.get(i), None | Some('\n') | Some('\r')) || c.get(i) == Some(&sep);
    let mut out = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let (mut line, mut start) = (1, 1);
    let mut quoted = false;
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '"' if quoted && ends_field(i + 1) => quoted = false,
            // An escaped quote, unless the pair is a literal quote followed
            // by the closing one.
            '"' if quoted && c.get(i + 1) == Some(&'"') && !ends_field(i + 2) => {
                field.push('"');
                i += 1;
            }
            '"' if quoting && !quoted && field.is_empty() => quoted = true,
            '\n' if quoted => {
                line += 1;
                field.push('\n');
            }
            '\r' if !quoted && c.get(i + 1) == Some(&'\n') => {}
            '\n' => {
                fields.push(std::mem::take(&mut field));
                out.push(Record { line: start, fields: std::mem::take(&mut fields) });
                line += 1;
                start = line;
            }
            ch if ch == sep && !quoted => fields.push(std::mem::take(&mut field)),
            ch => field.push(ch),
        }
        i += 1;
    }
    if !field.is_empty() || !fields.is_empty() {
        fields.push(field);
        out.push(Record { line: start, fields });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_case_words() {
        let words = |s: &str| word_spans(s).into_iter().map(|(a, b)| s[a..b].to_string()).collect::<Vec<_>>();
        assert_eq!(words("PubWorks1"), ["Pub", "Works"]);
        assert_eq!(words("ZTPDTac 2"), ["ZTPD", "Tac"]);
        assert_eq!(words("Fire/EMS"), ["Fire", "EMS"]);
    }

    #[test]
    fn frequencies_are_exact() {
        assert_eq!(parse_freq("851.912500c"), Some(RrFreq { hz: 851_912_500, control: true }));
        assert_eq!(parse_freq("853.7875"), Some(RrFreq { hz: 853_787_500, control: false }));
        assert_eq!(parse_freq("769.40625a"), Some(RrFreq { hz: 769_406_250, control: true }));
        assert_eq!(parse_freq("851.0000000"), Some(RrFreq { hz: 851_000_000, control: false }));
        assert_eq!(parse_freq("851.0000001"), None);
        assert_eq!(parse_freq("851.1x"), None);
        assert_eq!(parse_freq("Ctrl"), None);
    }

    #[test]
    fn modes_fold_to_csv_codes() {
        assert_eq!(fold_mode("D Enc"), "DE");
        assert_eq!(fold_mode("D enc"), "De");
        assert_eq!(fold_mode("T"), "T");
        let t = talkgroup(1, "3", "DE", "x", "", "", "").unwrap();
        assert_eq!(t.encryption, Encryption::Full);
        assert_eq!(talkgroup(1, "3", "De", "x", "", "", "").unwrap().encryption, Encryption::Partial);
        assert_eq!(talkgroup(1, "3", "E", "x", "", "", "").unwrap().encryption, Encryption::Clear);
    }

    #[test]
    fn records_handle_quotes_and_crlf() {
        let r = records("a,\"b,\"\"c\"\"\",d\r\n\"multi\nline\",x\r\n");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].fields, ["a", "b,\"c\"", "d"]);
        assert_eq!((r[1].line, r[1].fields.clone()), (2, vec!["multi\nline".to_string(), "x".to_string()]));
        // RadioReference's unescaped inner quotes.
        let r = records("1,\"Police 3 \"Secure channel\"\",\"Law Tac\"\n");
        assert_eq!(r[0].fields, ["1", "Police 3 \"Secure channel\"", "Law Tac"]);
        let r = records("\"say \"hi\" now\",x\n");
        assert_eq!(r[0].fields, ["say \"hi\" now", "x"]);
    }

    #[test]
    fn names_fit_the_pager() {
        let fit = |s: &str| fit_name(s, || "x".into());
        assert_eq!(fit("Elm FD Simulcast"), "Elm FD Simul");
        assert_eq!(fit("W A and R -2 "), "W A and R -2");
        assert_eq!(fit("Bayou Fire / EMS"), "Bayou Fire/EMS");
        assert_eq!(fit("Ox Co Airports 1"), "Ox Co Arpts1");
        assert_eq!(fit("ElmoreSchoolSEC3"), "ElmoreSchSEC3");
        assert_eq!(fit("Avalon Police PS"), "Avalon Pol PS");
        assert_eq!(fit("AvalonEmrgManage"), "AvalonEmrgMgmt");
        assert_eq!(fit("ZT  PublicWorks1"), "ZT PubWorks1");
        assert_eq!(fit("Maple Creek Fire"), "MapleCreekFire");
        assert_eq!(fit("MaplewdHlFD Tac1"), "MaplewdHl Tac1");
        assert_eq!(fit("WillowbrkSpgs FD"), "WillowbrkSp FD");
        assert_eq!(fit("RideZT C Private"), "RideZTCPrivate");
        assert_eq!(fit("North County Simulcast"), "North Co Simul");
        assert_eq!(fit_name("Caf\u{e9}", || "x".into()), "Caf?");
        assert_eq!(fit_name("  ", || "Site 3-1".into()), "Site 3-1");
    }
}
