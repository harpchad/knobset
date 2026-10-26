# RadioReference import: spec

Decided with Chad 2026-10-18. Import fills the same pending edits the system
editor already makes (create a system, or add sites, control channels and
talkgroups to one), so it adds no new kind of pager write and goes through
the normal write dialog, safety backup and read-back.

## Inputs

Two files per system, both header-driven. RadioReference only offers the
CSV downloads to Premium subscribers (forum reports; the notes in
radioreference.md said otherwise). Free users paste the same tables copied
from the system's web page, which arrive tab-separated. One parser handles
both: detect comma or tab from the header line, match columns by name
(case- and space-insensitive), ignore unknown columns.

- **Sites**: `RFSS, Site Dec, Site Hex, Site NAC, Description, County Name,
  Lat, Lon, Range, Frequencies`, then one extra column per additional
  frequency (PPS folds every trailing column into Frequencies). A frequency
  may carry a letter suffix; RadioReference marks control channels `c` and
  alternates `a`. Site Dec above 255 falls back to Site Hex (PPS does the
  same). Needed: RFSS, Site Dec or Site Hex, Frequencies.
- **Talkgroups**: `Decimal, Hex, Alpha Tag, Mode, Description, Tag,
  Category` (some exports also have `Subfleet`). Needed: Decimal, Alpha Tag.
- Neither file carries the WACN or System ID; the user types them from the
  system page header. They're required for a new system and prefilled from
  the target system when updating one.

Checked against a 12-site regional system exported from RadioReference on 2026-10-18, CSV and
paste parse identically (12 sites, 137 frequencies, 917 talkgroups):

- **Page paste, sites**: no header. One block per site: `3 (3)<TAB>001
  (1)<TAB>NAC<TAB>Description<TAB>County, ST`, then `24 freqs · 3 control`,
  then one frequency per line with a `Ctrl` line under each control
  channel. The counts line is checked against what was parsed, to catch a
  paste that was cut short.
- **Page paste, talkgroups**: no header; `DEC, HEX, Mode, Alpha Tag,
  Description, Tag, Category, last updated`. Mode reads `D Enc` for the
  CSV's `DE` and `D enc` for `De`. A "listen online" badge splits some rows:
  the decimal sits alone on one line and the next starts with `Online`.
- The browser collapses runs of spaces; the CSV keeps them. Both are
  collapsed to one space.
- The CSV export doesn't escape quotes inside a field
  (`"Police 3 "Secure channel"",...`), so a quote only closes a field when a
  delimiter or line end follows. It also leaves a stray trailing backslash
  in one description.
- TGIDs can repeat (listed under two agencies); the first wins and the
  repeat is reported.
- One system can span bands: the test system has two 700 MHz sites among the 800 MHz
  ones. The G5's SX band covers both.
- Frequencies within a site are ordered control channels first, then by
  frequency, as the page lists them.

Real exports go in `/rr-data/` (gitignored). Tests use a synthetic fixture
with the same layouts, plus a CSV-vs-paste comparison of the real files that
skips when they're absent.

## Mapping

| Knobset | From | Rule |
|---|---|---|
| site name | Description | shortened (below), unique, flagged when changed |
| site RFSS / Site | RFSS / Site Dec (or Hex) | 0..255 each, else the row is an error |
| control channel | site Frequencies | see filtering; system-wide, deduped by Hz |
| control channel note | site Description | at most 64 characters |
| talkgroup name | Alpha Tag | shortened (below), flagged when changed |
| talkgroup TGID | Decimal | 1..65535, else the row is an error |
| talkgroup note | Description | |

Frequencies are MHz with up to 6 decimals, converted to Hz exactly (string
arithmetic, no float). Chosen frequencies outside the target band are left
out and listed, so a wrong band choice is obvious in the preview. Names and
notes are reduced to printable ASCII, which is all the pager takes.

## Shortening names

The pager shows 14 characters; RadioReference alpha tags run to 16 and
site descriptions much longer. Plain truncation made 196 of the test system's talkgroups
collapse into 63 groups of identical names ("Ox Co Airports 1/2/3" all
became "Ox Co Airports"). `fit_name` applies these steps in order and stops
as soon as the name fits:

1. Collapse whitespace; reduce to printable ASCII.
2. Drop spaces around `/ & + -`.
3. Attach a trailing number to the word before it ("Airports 1" ->
   "Airports1").
4. Abbreviate from a fixed list of standard forms (County -> Co, Police ->
   Pol, Simulcast -> Simul, Airports -> Arpts, ...), whole words and
   CamelCase parts, biggest saving first.
5. Join words, but only if that alone makes it fit ("Maple Creek Fire" ->
   "MapleCreekFire", the style RadioReference's editors use).
6. Cut, keeping a short last word or a trailing number ("WillowbrkSpgs FD"
   -> "WillowbrkSp FD").

On that system the only names left identical are tags RadioReference itself
lists twice. Site names must also be unique within a system; a collision
gets the site's RFSS-site number as a suffix.

## Defaults (preview can change all of them)

- Control channels: only frequencies marked `c` or `a`. If a file has no
  markers at all, every frequency, with a notice.
- Talkgroups: fully encrypted modes (`E`, `DE`, `TE`) start unchecked;
  partially encrypted (`De`, `Te`) and clear ones start checked. Grouped by
  Category with per-category toggles.
- Sites: all checked. Unchecking a site drops its frequencies unless another
  checked site shares them.

## New system vs update

- **New**: pick a band, enter name (default from the file name or blank),
  WACN, System ID. The new system keeps the template's "All" talkgroup.
- **Update an existing system** ("add missing, keep mine"): add sites whose
  (RFSS, Site) isn't present, control channels whose Hz isn't present, and
  talkgroups whose TGID isn't present. Never rename, renumber or remove.
  Existing items absent from the import are listed as "not in
  RadioReference" for manual removal in the editor.
- Limits are checked before the preview is accepted: 256 sites, 256 control
  channels, 8192 talkgroups per system.

## Layers

- `g5_codeplug::rr_import`: parse text into sites and talkgroups with
  per-row errors and flags, and plan an import against a band (new) or an
  existing `SystemEdit` (update). Pure, no I/O.
- `g5 rr-import --sites FILE --talkgroups FILE [--backup DIR --templates
  DIR --system NAME --band CODE --all-freqs --encrypted]`: preview only,
  for checking an export against a backup.
- Tauri command `rr_parse` for the app; the preview and the resulting drafts
  live in the frontend store like any other system edit.

## Out of scope for this milestone

- The RadioReference web service (needs our own app key and Premium login).
- Conventional frequencies and agencies.
- Creating knobs or zones from an import; the user assigns the system to a
  knob afterwards, as today.
