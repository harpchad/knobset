//! RadioReference site and talkgroup exports (CSV or text pasted from RR's
//! pages), then the import plan built from them, name shortening included.
#![no_main]

use g5_codeplug::rr_import::{Selection, parse_sites, parse_talkgroups, plan_import};
use libfuzzer_sys::fuzz_target;

/// VHF and 700/800 MHz receive ranges, like a G5B67BF's.
const RANGES: &[(i64, i64)] = &[(136_000_000, 174_000_000), (764_000_000, 870_000_000)];

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else { return };
    let (Ok(sites), Ok(tgs)) = (parse_sites(text), parse_talkgroups(text)) else {
        return;
    };
    let sel = Selection::defaults(&sites, &tgs);
    let _ = plan_import(None, RANGES, &sites, &tgs, &sel);
});
