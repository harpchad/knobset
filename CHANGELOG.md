# Changelog

All notable changes to Knobset are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org).

## [Unreleased]

### Added

- Linux builds for 64-bit ARM: an `aarch64` AppImage and an `arm64` .deb,
  for a Raspberry Pi 4 or 5 or a Linux VM on Apple silicon. Not yet run
  against a pager.
- Screenshots in the README, made from the app's demo data by
  `scripts/screenshots/capture.py`, which also draws the GitHub social
  preview card.
- `PRIVACY.md`: Knobset collects nothing; what stays on the device, and
  what TestFlight, GitHub and email handle.
- A public TestFlight link for the iPad beta, in the README. Anyone can
  join; build 0.6.2 passed Beta App Review on 2026-10-28.

### Changed

- The x64 Linux AppImage and the x64 Windows installer have now run
  against a pager (0.6.2 on Arch with Wayland, and on Windows 11 with
  Unication's driver): link, template import, backup and a knob write
  all work. The notes cover Arch's `enp…u…` interface name, running the
  AppImage without FUSE 2, and the "USB Serial Device" binding Windows
  shows before the driver is installed. The .deb and the ARM builds ran
  in Ubuntu VMs, and the README now says Ubuntu Server and cloud images
  need `linux-modules-extra` for the pager's link.

### Fixed

- Drop-down menus in dark mode on Linux showed light text on a light box,
  so the selected system or mode looked blank. The page now tells the
  web view it's dark, so WebKitGTK (and WebView2 on Windows) draws its
  native controls dark too.

## [0.6.2] - 2026-10-27

A security fix for the command-line tool, Windows on ARM support that
works with a real pager, and verifiable downloads. Pager writes are
unchanged from 0.6.1.

### Security

- `g5 -vv` printed the pager's access password in plain text (from the
  handshake response) and its MD5 (from the access checks). The packet
  trace now shows only their length. If you saved `-vv` output from a
  pager with an access password, delete it.

### Added

- `install-rndis.ps1`, attached to each release, binds Windows' own Remote
  NDIS driver to the pager on Windows on ARM. Device Manager can't do it
  there, so the 0.6.1 advice to pick the driver by hand doesn't work.
- `SHA256SUMS.txt` on each release, for checking downloads.
- Signed build provenance attestations for the Linux and Windows
  installers, checked with `gh attestation verify`, and a `.sigstore.json`
  bundle next to each one so `--bundle` doesn't have to fetch the
  attestation from GitHub.
- Weekly fuzzing (cargo-fuzz) of the pager frame and response parsers and
  the RadioReference import.
- Tests that zip and MSI template imports refuse paths escaping their
  folder and zip symlinks.
- Project documents: GOVERNANCE.md, CODE_OF_CONDUCT.md (Contributor
  Covenant 2.1), ROADMAP.md, ARCHITECTURE.md, a security assurance case
  (`notes/assurance-case.md`), a Quick start in the README, and the
  vulnerability response process in SECURITY.md.
- CI and review: CodeQL code scanning, an OpenSSF Scorecard workflow,
  Dependabot updates for GitHub Actions with every action pinned to a
  commit SHA, tests and clippy on every pull request and push to `main`,
  and CodeRabbit reviews (it comments and approves, but never commits).
  Merging to `main` needs an approving review, a passing "Test and
  clippy" check and an up-to-date branch.
- `osv-scanner.toml` and `app/osv-scanner.toml` record seven reviewed
  dependency advisories, each with its reason and a review date. Five are
  in the Linux build through Tauri's GTK and D-Bus crates, but Knobset
  never calls their affected code (four are only "unmaintained" notices);
  `block-buffer` 0.10 needs a caught panic, and release builds abort on
  panic; `deepmerge` is a development dependency and isn't in any build.
- The OpenSSF Best Practices badge (passing), plus release and license
  badges, at the top of the README.

### Changed

- The Pager page tells Windows on ARM users to run `install-rndis.ps1`
  instead of installing Unication's driver.
- The README covers Defender's false positive on the ARM installer, and
  Windows on ARM is now tested against a pager: backup, write and
  read-back.

### Fixed

- A packet whose content is shorter than its header says is now an error
  instead of a panic (no caller could send one, but the parser is public).

## [0.6.1] - 2026-10-26

The first release from the public repository. Pager behaviour is the same
as 0.6.0.

### Added

- A native Windows on ARM installer (`Knobset_<version>_arm64-setup.exe`),
  built by CI alongside the x64 one. Not yet tried on a pager; Unication's
  RNDIS driver is x64 only, so ARM needs Windows' own Remote NDIS driver,
  picked by hand in Device Manager.
- `CONTRIBUTING.md` and `SECURITY.md`, and an iPad install section in the
  README.

## [0.6.0] - 2026-10-25

On a pager so far: display format, talkgroup colour and talkgroup alert
with voice storage (pager B), and a write to a DBV1.41e pager (pager C).
The other features pass offline tests against real backups and are
waiting on the steps in `notes/pager-test-plan.md`.

### Added

- Pagers on radio DB DBV1.41d and DBV1.41e (firmware V2.05 and the
  builds before it), with the 1.41e contact numbering PPS uses. Verified
  against a PPS-written 1.41e radio DB from a real pager: all 194 tables
  identical. Cloning between a 1.41c and a 1.41e pager generates the radio
  DB for the target's version.
- Write a backup to another pager of the same model (Backups › Write to
  another pager, or the button above an open profile). The target's zones,
  knobs and systems are replaced with the backup's, while it keeps its own
  radio number, alias, unit IDs, access passwords and PIN lock; PPS would
  copy those too. The dialog shows both pagers side by side, the identity
  fields kept, and what changes on the target, and the write needs an
  explicit confirmation. Pagers of different models are refused with the
  differences spelled out; a target on another radio DB version gets its
  radio DB generated for that version. The target's safety backup comes
  first and the write stops if the target changed since its newest backup.
  `g5 clone` previews and writes from the command line.
- Copy a P25 system from any backup into the open profile (P25 systems ›
  Copy from a backup), like PPS's Add System from Resources: sites, control
  channels and talkgroups come over with every setting, as a new system you
  can edit before writing. Systems in a band the pager doesn't have can't
  be copied.
- Copy a knob to an empty position in the same zone or another (Copy to...
  on the knob's panel): trunked, analog, DMR, P25 conventional and scan
  knobs. The copy gets a new channel with the same system, talkgroups,
  frequency, squelch, settings and voice prompt, or for a scan the same
  channels, as Unication's paste writes it. It starts with the original's
  name, which you can change before writing, and a scan edit in the same
  write can list it.
- Move a knob to an empty position, or swap two knobs (Move to...), within
  a zone or across zones. The channel keeps its identity, so scans that list
  it and their priority channel follow it to its new place. The pager numbers
  channels by position, so a move renumbers them: Knobset applies moves last
  and checks they changed nothing but the numbering. A moved knob takes no
  other edit in the same write.
- Pager settings page for the device-wide settings PPS keeps on its general
  settings pages, edited in the open backup's profile and written with the
  rest of your changes. Only settings you change are written, each within
  PPS's range, and the write guard allows only the radio tables that
  setting reaches.
  - Talkgroup hold and lockout: the hold and block soft keys, the hold
    timeout (0 to 99 minutes) and the un-hold beep, all off by default in
    PPS. Blocking is temporary on the pager (turning the knob clears it);
    there's no lockout list a profile can pre-load.
  - Display format: decimal or hex talkgroup and system IDs, date format,
    12 or 24 hour clock, and signal strength as bars or dBm (pagers with
    radio DB 1.41c or newer).
  - Alerts: call alert and notification tone volume (by the volume knob, or
    fixed from mute to 5), the out-of-range beep with its timers and
    repeat, and the unread message reminder.
  - Backlight and screen: automatic or manual brightness (10 to 100%), the
    screen timeout (0 keeps it on), and the receive flash for calls and
    messages with its on, off and duration times.
  - Voice storage: the longest stored voice message, 1 to 10 minutes.
  - Pager menu items: show or hide each item of the pager's Function Status
    and Function Setting menus, such as TGID Hold Timeout (change the hold
    timeout on the pager), In/Out Range Alert and Factory Setting. PPS's
    top-level functions (All Band Scan, message boxes and so on) aren't
    offered yet.
- Talkgroup settings: set a talkgroup's alert (tone, vibrate, silent, or
  tone and vibrate), alert tone (default, chirp or Tone 1 to 8), voice
  storage and background colour in its dialog on the P25 systems page, the
  way PPS's talkgroup editor does. The talkgroup list shows each one's alert
  and voice storage. Tick several talkgroups, or search and tick the header
  to take everything shown (searching "Silent" finds every silent one), and
  set any of the four on all of them at once: after a RadioReference import
  leaves hundreds of talkgroups silent, turning alerts on for the few you
  care about is one step. Custom tones made in PPS are kept as they are.
  Only these settings' radio tables may change on the write.
- Linux and Windows builds: an AppImage and a .deb for x86_64 Linux
  (Ubuntu 22.04 or newer) and an installer for 64-bit Windows 10 and 11,
  built by GitHub Actions and attached to each release. They're unsigned,
  so Windows SmartScreen asks once (More info, Run anyway). Linux needs no
  driver for the pager; Windows needs Unication's RNDIS driver, which comes
  with PPS. Neither has been tried against a pager yet. The backups
  list's folder button now names Finder, File Explorer or the file manager
  to match.

### Fixed

- The pager no longer flashes "PC Connected" and stops scanning every few
  seconds while Knobset is open. Knobset reads the pager's details once when
  it's plugged in and then only checks that it's still there, which the
  pager doesn't notice. Backups and writes still show "PC Connected", as
  they must.
- Moving or deleting a zone was refused on a pager with a DMR or P25
  conventional knob: the renumbering check didn't know those channels'
  signaling rows are numbered by channel too.
- The first backup after a firmware update showed "Check failed" and
  couldn't be written from, because the update migrates the radio DB
  without updating its checksum file. Backups now show "Radio DB MD5
  stale" for that case, and writes accept it when the radio DB is exactly
  what came off the pager, sending a corrected checksum.

## [0.5.0] - 2026-10-19

### Added

- Import a P25 system from RadioReference (P25 systems › Import from
  RadioReference). Use the site and talkgroup CSVs Premium subscribers can
  download, or paste the two tables straight from the system's page, which
  needs no subscription. Choose which sites and talkgroup categories to keep.
  Only the frequencies RadioReference marks as control channels are used, and
  fully encrypted talkgroups start unchecked. Import into a new system, or
  add to an existing one: anything already there is left as it is, and
  nothing is removed. Names longer than the pager's 14 characters are
  shortened with standard abbreviations (County to Co, Police to Pol, and so
  on), keeping trailing numbers and short suffixes like FD and PD so similar
  talkgroups stay distinct; frequencies outside the band are
  left out, and the result is an ordinary pending change you can review
  before writing. Imported talkgroups are silent with no voice storage, as
  in PPS's own RadioReference import. `g5 rr-import` previews an import from
  the command line.
- Put an analog conventional channel on an empty knob position: one
  frequency in Monitor mode, 12.5 or 25 kHz, with carrier squelch, a CTCSS
  tone or a DCS code (normal or inverted), and the built-in voice prompt off
  by default. Each knob gets its own frequency entry, written the way PPS
  writes one.
- Zones: add a zone (8 empty positions), rename one, move zones left or right
  (the pager starts on the first zone after a write), and delete a zone with
  no knobs. Moves and deletes renumber the radio's channels, so they're
  written on their own, and Knobset checks the renumbering changed nothing
  else. Pending knob edits follow a zone when it's renamed.
- Scan knobs: put a Normal or Priority Scan on an empty position, listing 2
  to 16 analog conventional knobs from any zone in scan order (with a
  starred priority channel for Priority Scan), or edit an existing scan's
  list. The new scan knob is written exactly as PPS writes one.
- Remove a knob (trunked, conventional or scan). Scans that listed a removed
  channel drop it, and a priority scan whose priority channel goes moves its
  priority to the next channel; a removal that would leave a scan with fewer
  than 2 channels is refused. Put a different kind of knob on the emptied
  position to convert a knob from conventional to trunked or back. Removing
  the last knob on a P25 system lets you delete the system in the same write.
- Edit an analog conventional knob in place: frequency, bandwidth, squelch,
  alert, voice storage and voice prompt.
- Switch a trunked knob between TG Scan and TG Monitor.
- Experimental: P25 conventional (NAC, optionally only that NAC) and DMR
  (timeslot, color code) channels in Monitor mode, with an optional
  talkgroup list (an "All" placeholder otherwise). Built from Unication's
  rules, and the write dialog says they're experimental. A DMR knob has
  been written to a pager, which kept it, but no audio has been confirmed
  yet; P25 conventional hasn't been on a pager. DMR is offered only on
  pagers whose model includes it.

### Fixed

- After a write, Knobset backs up the pager as written and opens that
  backup, so the next change can start right away. If that backup fails,
  the write is still reported as successful, with a warning.
- A safety backup from a write that stopped before programming no longer
  forces a fresh backup: a backup with the same profile as the newest one
  counts as current.
- The write dialog no longer shows how long the write took. The time
  covered only the file transfer, not the backups, reboot and read-back,
  so it was several times too short.
- Backup times on iPad showed UTC as if it were local time. Backups now
  record their time in UTC (folder names end in `Z`), and Knobset shows them
  in local time with the time zone, for example "Oct 19, 2026, 5:14 AM CDT".
- "Add all" in a knob's talkgroup list works without a search when every
  remaining talkgroup fits on the knob. Larger systems still need a search
  first, and the "All" placeholder is never added in bulk.
- A backup that stops partway, for example because the pager was unplugged,
  no longer leaves an empty or partial folder behind.

## [0.4.1] - 2026-10-18

### Added

- Mac download: a universal DMG (Apple silicon and Intel) attached to each
  GitHub Release, signed with Developer ID and notarized so Gatekeeper opens
  it without a warning. Requires macOS 13.3 or later.

### Changed

- The Mac app icon now has the macOS rounded-square shape and margin instead
  of a full-bleed square.
- The iPad app now requires iPadOS 16.4 or later. Its interface is built for
  Safari 16.4, so on iPadOS 15 it installed but could render incorrectly.

## [0.4.0] - 2026-10-18

### Added

- App icon: a stylized G5 pager on teal, replacing Tauri's placeholder.
- iPad support: Knobset runs on iPadOS 15 or later with the pager plugged into
  the iPad's USB-C port. iPadOS drives the pager's USB network adapter
  natively. Backing up, viewing, editing and writing all work. On iPad,
  backups appear in the Files app (On My iPad › Knobset › Backups), and a
  backup or write keeps running if you switch apps or the screen would
  auto-lock.
- Import the templates straight from Unication's PPS download (the .zip, the
  setup .exe inside it, or `PPS_Setup.msi`). Knobset extracts just the
  `DataBase` folder into its own storage, so you no longer need Windows or an
  installed copy of PPS. Choosing an installed `DataBase` folder still works
  on desktop. Also available as `g5 import-templates`.

### Changed

- Relicensed from GPL-3.0-or-later to MPL-2.0, so Knobset can be distributed
  through the App Store and TestFlight.

### Fixed

- Opening a backup failed on systems where SQLite 2 had no writable temp
  directory. It now uses the OS temp directory.
- A backup that couldn't be opened now shows an error. Before, the Open
  button looked like it did nothing.

## [0.3.0] - 2026-10-18

### Added

- Create P25 trunking systems in either of the pager model's bands, with
  sites, control channels and talkgroups entered up front. Unlike PPS there's
  no placeholder control channel, and a failed create leaves nothing behind.
- Delete P25 systems that no knob uses; Knobset refuses while a knob still
  uses one.
- Put a P25 system on an empty knob position (TG Scan or TG Monitor, voice
  prompt off by default), or move a trunked knob to another system, then pick
  its talkgroups. Conventional knobs can't be converted yet.

### Security

- Upgrade the `zip` crate from 2 to 9 (RUSTSEC-2026-0367, -0368, -0369).
  Knobset never used the affected `extract` API; profile zips with symlink
  entries are now rejected outright.
- Move the `shadcn` CLI to devDependencies; the app's runtime dependencies
  audit clean.
- Dependabot version updates for Cargo and npm.

## [0.2.0] - 2026-10-18

### Added

- Edit P25 trunking systems: name, WACN and System ID; add, edit, reorder
  and remove sites; add, edit and remove control channels (checked against
  the pager's bands); add, edit and delete talkgroups. Deleting a talkgroup
  also removes it from that system's knob lists.
- Write dialog summarizes system changes.
- Site editor labels RFSS and Site IDs as hex (RadioReference's Hex column)
  and shows each value's decimal equivalent in the field.

## [0.1.0] - 2026-10-18

First working version, verified on two G5 pagers (DBV1.41b and DBV1.41c).

### Added

- Wire protocol for G-series pagers over USB CDC-ECM/RNDIS (TCP
  `10.10.0.12:32769`): handshake, access checks, directory listing, chunked
  file reads and writes, guarded to the paths PPS programs.
- `g5` CLI: `info`, `ls`, `backup`, `summary`, `convert`, `compare`,
  `restore` (with `--rename-knob`).
- Vendored SQLite 2.8.17 (thread-safe build) for the pager's database format.
- Radio-database generator for DBV1.41b and DBV1.41c, matching Unication
  PPS output table for table on real pagers.
- Safe write flow: backup must match the pager, MD5 checks, automatic safety
  backup, refusal if the pager changed since the source backup, reboot-aware
  read-back verification.
- Knobset desktop app (Tauri 2 + React): pager status and backups, backup
  list, zones and knobs with inline rename, knob talkgroup picker with
  priority ordering, P25 systems view, guarded write dialog.
