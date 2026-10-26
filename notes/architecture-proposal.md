# Proposed architecture (draft for review)

Nothing here is scaffolded yet. Decisions marked **Decide** need Chad's call.

## Constraints from the reverse engineering

1. Transport is TCP to `10.10.0.12:32769`. Verified on hardware: the pager's
   USB config 1 is CDC-ECM, macOS binds it natively, and the pager serves
   DHCP. Plain OS sockets work on macOS; userspace RNDIS is only a fallback.
2. The pager stores SQLite **2.1** files. We need a 2.8.x engine to write them.
3. Writing a profile means generating `u3db.db` for the pager's reported
   U3DB version (DBV2.01 to 2.06) plus PPS-format copies (`u3db_PC.db`,
   `u3db_PC.zip`, MD5 sidecars) so Unication PPS can still read the pager.
   Both test pagers (firmware V1.33 and V2.02) report DBV1.41b/c, so the
   **1.41 converter is the first target**; 2.0x waits for test hardware.
4. Templates (`PPS_G4A.db`, `DB_Base_G4A.DBV2.0x.db`, per-model
   `MFG`/`BSS` DBs, `Data_G4A.db`) are Unication files.
5. Decompiled C# is a spec, not source we can copy. Reimplement.

## Recommended stack: Rust core + Tauri 2 + React/TypeScript

Why Rust:

- Memory-safe protocol code, one static core shared by CLI and GUI.
- If a platform ever lacks a usable driver, `nusb` + `smoltcp` give us a
  userspace RNDIS/ECM fallback. Not needed on macOS (verified).
- The `cc` crate builds vendored SQLite 2.8.17 cleanly for every target.
- One static core library also powers a CLI for hardware testing.

Why Tauri: native-sized binaries (~10 MB vs ~150 MB Electron), signed
macOS/Windows/Linux bundles, and a web UI where good data grids exist
(TanStack Table with virtualization handles 8192-row talkgroup lists).

Alternative: Go + Wails. Workable, but userspace RNDIS in Go means cgo
libusb plus gVisor's netstack, and cgo cross-builds are painful. I'd pick
Rust.

## Workspace layout

```
crates/
  g5-proto     framing, CRC/checksum, session, request/ACK/chunk state machine,
               file ops (list/read/write/delete), handshake parsing
  g5-link      Transport trait: OsTcp (system network) and UsbRndis (nusb + smoltcp);
               device discovery by VID 17FF / PID A4A2
  sqlite2      vendored SQLite 2.8.17 + safe wrapper (open, exec, query, copy template)
  g5-codeplug  PPS 8.0 profile model (typed layer over the profile DB), model-number
               decoding, band limits, validation (ProfileCheck rules + UI ranges),
               u3db DBV2.0x generator, .unipps import/export
  g5-import    RadioReference SOAP (own app key), RR CSV, SDRTrunk / Sentinel /
               ProScan CSV talkgroups
  g5-cli       probe, backup, read, write, convert (dev and power users)
app/           Tauri shell + React UI
```

Canonical storage: the PPS 8.0 profile DB itself (held as SQLite 3 in the
app, converted to 2.1 at write time). The typed layer covers what we expose;
untouched tables keep their template or read-back values, so round trips
don't lose settings we don't edit.

## Safety rails (non-negotiable)

- Allowlist of writable pager paths (the seven PPS writes). No firmware, no
  arbitrary paths, no delete outside the allowlist.
- Write only if model number, band codes and U3DB version match a tested
  converter. Unknown version means read-only.
- Before every write, pull and archive the pager's current `u3db_PC.zip`,
  `u3db.db` and MD5s with a timestamp. One-click restore.
- Show a human-readable diff (systems, talkgroups, scan lists, channels)
  before writing.
- Dry-run mode writes the generated files to disk for inspection.

## UX direction (scanner-first)

- **Systems:** one page per P25 system. Header (name, WACN, SysID), sites
  table with control channels inline, talkgroups as a spreadsheet grid with
  search, bulk edit, category column, encrypted flag, CSV paste.
- **Scan lists:** named, reusable talkgroup lists with a live 64-TG meter.
  Built from RR categories in a couple of clicks. Assigning a scan list to a
  knob replaces OEM's one-at-a-time "Add >>".
- **Zones and knobs:** visual 8-position knob per zone; drag a scan list,
  conventional channel or free-scan range onto a position. Mode-family
  changes warn with exactly what will be lost.
- **Import wizard:** RadioReference (premium login, keychain) or CSV.
  Location-aware site picker, skip encrypted TGs by default, merge into an
  existing system with a diff.
- **Device:** connect status, model/firmware/U3DB version, Read, Write
  (with diff and backup), backup history.
- Hidden plumbing: the "P25 Trunking" protocol entry and other OEM
  prerequisites are created automatically. Scanner-friendly defaults
  (TG hold/lockout on). Inline validation that never traps navigation.

## Milestones

| # | Goal | Needs a pager? |
|---|---|---|
| M0 ✅ | CLI probe: enumerate USB descriptors (RNDIS vs ECM), bring up link, handshake, list dirs, read and back up `u3db_PC.zip` + `u3db.db`. Validates the protocol notes. | Yes |
| M1 ✅ | SQLite 2 crate, profile reader (`g5 summary`), DBV1.41b/c radio-DB generator (`g5 convert`), golden test: both pagers regenerate 194/194 tables identical | No (after M0 backup) |
| M2 | Editor + PPS-format export; verify Unication PPS (Windows VM) imports it cleanly | VM |
| M3 ✅ | Write path: byte-identical restore, generated radio DB upload, and a real edit (knob rename) confirmed on the pager display. DBV1.41b/c only; DBV2.0x waits for hardware | Yes |
| M4a ✅ | Desktop app: pager status + backups, backups list, zones/knobs view with rename, P25 systems + talkgroup grid, guarded write dialog (`app/`) | Yes for writes |
| M4 | RadioReference + CSV import, scan-list builder | No |
| M5 | Custom tones/prompts (G.729A encoder), home-screen logo | Yes |

## Open decisions

1. **Decided (2026-10-17):** Rust core + Tauri 2 + React/TypeScript.
2. **Decided (2026-10-17):** don't ship Unication templates. First run
   extracts them from the user's copy of the free PPS installer (Inno Setup
   wrapping an MSI) into app data. Test copies stay gitignored.
3. **Decided (2026-10-18, replaces GPL-3.0-or-later from 2026-10-17):**
   MPL-2.0. GPL was picked so we could use `bcg729` (GPLv3) for G.729A, but
   the iPad target means App Store and TestFlight distribution, whose terms
   the FSF holds incompatible with the GPL. Knobset's own exception couldn't
   cover `bcg729`'s code, and nothing used it yet. MPL keeps file-level
   copyleft, allows a paid App Store release, and fits the MIT/Apache deps.
   Custom tones (M5) need a non-GPL G.729A encoder (patents expired ~2017),
   or tones become desktop-only with `bcg729` as an optional feature.
4. **Decide:** RadioReference app key. Needs an application to RR.
5. Hardware: Chad has two G5 models for M0 testing (details at M0).
