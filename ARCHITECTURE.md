# Architecture

Knobset is a Rust core with two front ends: the `g5` command-line tool and
a Tauri 2 app (React and TypeScript) for macOS, iPad, Linux and Windows.
The detailed specs are in [`notes/`](notes/README.md); this is the map.

## The pager link

The pager is a USB network device. It enumerates as CDC-ECM (macOS, iPad
and Linux bind it with no driver) or RNDIS (Windows), serves DHCP on
`10.10.0.0/16`, and listens on TCP `10.10.0.12:32769`. Knobset is only
ever a client of that one address: it runs no servers and makes no
internet requests. Programming a pager means transferring files, mostly
SQLite 2 databases. The byte-level protocol is in
[`notes/protocol.md`](notes/protocol.md).

## Components

```text
app (Tauri + React)     g5-cli
        \                 /
         +---- g5-core ---+        backups, the guarded write, cloning
              /        \
      g5-codeplug     g5-proto     profile model and edits | wire protocol
            |
         sqlite2                   vendored SQLite 2.8.17 + safe wrapper
```

| Crate | Responsibility |
|---|---|
| `g5-proto` | Framing (header checksum, CRC), handshake, access checks, file reads and writes. Writes are limited to the files PPS itself programs (`WRITABLE_PATHS`). |
| `sqlite2` | Builds SQLite 2.8.17 with the `cc` crate and wraps it with bound parameters and owned results. |
| `g5-codeplug` | The profile model and every edit (rename, talkgroups, systems, knobs, zones, settings), the radio-DB generator for DBV1.41b to e, RadioReference import, and template import from Unication's installer. |
| `g5-core` | Backups and the guarded write flow, shared by the CLI and the app so both behave the same. |
| `g5-cli` | `g5`: hardware testing and scripting (`info`, `backup`, `restore`, `clone`, `compare`, ...). |
| `app/` | The desktop and iPad app. The Rust side exposes `g5-core` as Tauri commands; the React side is the UI. |

Unication's template databases come from the user's own copy of PPS
(imported in Settings or with `g5 import-templates`); they're never in
the repository or the builds.

## Data flow

**Backup.** Handshake, pass the pager's read-access check, read the
profile files, and store them with their MD5s in a timestamped folder.
The pager's own `.md5` files are checked against the data.

**Edit.** Load a backup into the profile model, apply edits in memory,
and regenerate the pager's radio database from Unication's templates the
way PPS does. Nothing touches the pager yet.

**Write.** `g5_core::write::program`:

1. requires the backup to come from this exact pager (model, serial,
   radio DB version) and verifies its MD5s and file formats;
2. checks the regenerated database against the tables that kind of edit
   may change (the write guard), and refuses anything else;
3. takes a fresh safety backup, and refuses if the pager changed since
   the source backup;
4. programs the files in PPS's order;
5. reconnects after the pager reboots and verifies every file byte for
   byte.

## Builds and releases

CI (GitHub Actions) runs tests, clippy and CodeQL on pull requests,
fuzzes the parsers weekly, and builds the Linux and Windows installers on
release tags, with Sigstore build provenance attestations. The Mac DMG
(signed and notarized) and the iPad build are made with the scripts in
`scripts/macos` and `scripts/ios`. See the README's Releasing section.

## Security

[SECURITY.md](SECURITY.md) covers reporting and scope;
[`notes/assurance-case.md`](notes/assurance-case.md) argues why Knobset's
security requirements are met.
