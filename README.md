# Knobset

A modern, cross-platform replacement for Unication's Windows-only PPS for
programming G-series pagers, aimed at hobbyists who use a G5 as a P25
trunking scanner. Runs on macOS, iPad, Linux and Windows. Windows on ARM
has backed up and written a pager; Linux and Windows x64 haven't been
tried against one yet.

Status: early. Works on DBV1.41b to DBV1.41e pagers (firmware V1.33, V2.02
and V2.05 tested): back up, browse zones, knobs, systems and talkgroups, rename
knobs and edit a knob's talkgroup list, create and edit P25 systems (or
import them from RadioReference), then write to the pager with a safety
backup and byte-for-byte read-back verification.

## How it talks to the pager

The pager enumerates as a USB network adapter (CDC-ECM, so macOS and Linux
need no driver; RNDIS on Windows), serves DHCP on `10.10.0.0/16`, and
speaks a framed request/ACK protocol over TCP at `10.10.0.12:32769`.
Programming is file transfer of SQLite 2 databases. See [`notes/protocol.md`](notes/protocol.md).

## Layout

| Path | What |
|---|---|
| `crates/sqlite2` | Vendored SQLite 2.8.17 (public domain) + safe wrapper |
| `crates/g5-proto` | Wire protocol: framing, sessions, file read/write |
| `crates/g5-codeplug` | Profile model, edits, radio-DB generator (DBV1.41b to e) |
| `crates/g5-core` | Backups and the guarded write flow, shared by CLI and app |
| `crates/g5-cli` | `g5` command-line tool |
| `app/` | Knobset app for macOS, iPad, Linux and Windows (Tauri 2 + React) |
| `.github/workflows/` | CI: tests and clippy on PRs; Linux and Windows installers |
| `scripts/ios/` | App Store / TestFlight release script |
| `scripts/windows/` | Pager driver setup for Windows on ARM |
| `scripts/macos/` | Signed, notarized DMG release script |
| `scripts/icons/` | Regenerates every icon size from the SVG master |
| `scripts/release-checksums.sh` | Attaches `SHA256SUMS.txt` to a release |
| `fuzz/` | cargo-fuzz targets for the pager and RadioReference parsers |
| `notes/` | Reverse-engineering notes |

## Checking a download

Each release has a `SHA256SUMS.txt`, and the release page shows each
file's SHA-256 next to it. In the folder holding both, check a download
with `shasum -a 256 -c SHA256SUMS.txt --ignore-missing` on a Mac or
`sha256sum -c SHA256SUMS.txt --ignore-missing` on Linux. On Windows, run
`Get-FileHash .\Knobset_<version>_arm64-setup.exe` in PowerShell and
compare the hash with the file's line in `SHA256SUMS.txt` (case doesn't
matter).

A checksum shows the file is the one on the release page. In releases
after 0.6.1, the Linux and Windows installers also carry a signed build
provenance attestation, which shows the file was built by this repo's CI
from that release's tag. Check one with the GitHub CLI:

```sh
gh attestation verify Knobset_<version>_arm64-setup.exe -R harpchad/knobset \
  --source-ref refs/tags/v<version>
```

Keep `--source-ref`: without it, a build from any branch of the repo would
also pass. Each of those installers also has a `<file>.sigstore.json` on
the release, the same attestation as a file. With it, the check works
without looking the attestation up on GitHub:

```sh
gh attestation verify Knobset_<version>_arm64-setup.exe -R harpchad/knobset \
  --source-ref refs/tags/v<version> \
  --bundle Knobset_<version>_arm64-setup.exe.sigstore.json
```

That still downloads Sigstore's trusted root. To check on a machine with
no network, save it first with `gh attestation trusted-root >
trusted_root.jsonl` and add `--custom-trusted-root trusted_root.jsonl`.

The Mac DMG is built and notarized by Apple's tools instead, so it has no
attestation; Gatekeeper checks its signature when you open it.

## Installing on a Mac

Download `Knobset_<version>_universal.dmg` from the
[latest release](https://github.com/harpchad/knobset/releases/latest), open
it and drag Knobset to Applications. It runs natively on Apple silicon and
Intel Macs with macOS 13.3 or later, and it's signed and notarized, so
Gatekeeper opens it without a warning. On macOS 15 or later, the first time
you connect to the pager, macOS asks whether Knobset may find devices on
your local network: allow it, because that's how Knobset reaches the pager
over USB.

## Installing on iPad

Knobset for iPad is in beta on TestFlight; a public link is coming soon.
It needs iPadOS 16.4 or later and an iPad with USB-C. Plug the pager in
directly or through a USB-C hub: iPadOS sees it as an Ethernet adapter, so
there's nothing to install. The first time you connect, allow Knobset to
find devices on your local network.

Backups are saved in Files, under On My iPad > Knobset > Backups. To import
Unication's templates, download PPS on the iPad and choose it in Settings >
Import installer.

## Installing on Linux

Download `Knobset_<version>_amd64.AppImage` or `Knobset_<version>_amd64.deb`
from the same release. Both need glibc 2.35 and WebKitGTK 4.1 (Ubuntu
22.04, Debian 12, Fedora 36 or newer). Install the .deb with
`sudo apt install ./Knobset_<version>_amd64.deb`, or `chmod +x` the
AppImage and run it (it needs `libfuse2`, `libfuse2t64` on Ubuntu 24.04).

Plug in the pager: the kernel's `cdc_ether` driver binds it and
NetworkManager gets an address from it by DHCP, so there's nothing to
install. `ip -br addr` should show an `enx…` interface on `10.10.0.x`. If
`nmcli device status` lists it as unmanaged, or the machine runs
systemd-networkd instead, see
[`notes/desktop-platforms.md`](notes/desktop-platforms.md).

## Installing on Windows

Download `Knobset_<version>_x64-setup.exe` from the same release, or
`Knobset_<version>_arm64-setup.exe` for Windows on ARM. Either installs for
your user without admin rights and needs Windows 10 1903 or later.

They aren't signed yet, so Windows warns about them:

- SmartScreen says "Windows protected your PC": choose More info, then Run
  anyway. Edge may also call the download uncommon: choose … › Keep.
- Microsoft Defender has flagged the ARM installer as
  `Trojan:Win32/Bearfoos.A!ml`. That's a false positive from Defender's
  machine-learning model, and it's been reported to Microsoft. If Defender
  removes the file, check its SHA-256 against `SHA256SUMS.txt` first, then
  restore it from Windows Security › Virus & threat protection ›
  Protection history (Actions › Allow on device). Don't add a folder
  exclusion.

Windows on x64 needs Unication's RNDIS driver for the pager's USB link. It
ships inside PPS's setup and as Unication's separate "Windows RNDIS driver
setup" download. Once it's in, `ipconfig` shows a "USB Remote NDIS Network
Device" adapter on `10.10.0.x`.

Unication's driver doesn't install on Windows on ARM, and Device Manager
can't pick Windows' own Remote NDIS driver for the pager either. Instead,
plug in the pager, download `install-rndis.ps1` from the same release
([source](scripts/windows/install-rndis.ps1)) and run it once from an
administrator PowerShell in the folder you saved it to:

```powershell
powershell -ExecutionPolicy Bypass -File .\install-rndis.ps1
```

It only binds Windows' built-in driver, and downloads and installs nothing
else. Afterwards `ipconfig` shows a "Remote NDIS Compatible Device" on
`10.10.0.x`. Details for both are in
[`notes/desktop-platforms.md`](notes/desktop-platforms.md).

## Unication templates

Writing needs template databases that ship with Unication's free PPS. They
are Unication's files, so they are not in this repo. Download PPS from
Unication and import it in Settings (the .zip, its setup .exe, or
`PPS_Setup.msi`), or run `g5 import-templates`. On desktop you can also
point Knobset at an installed PPS `DataBase` folder.

## Development

You need stable Rust, Node 24 and
[Tauri 2's prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
cargo test --workspace          # golden tests need private pager backups in ./backups and skip without them
cd app && npm install && npm run tauri dev
```

Fuzzing needs nightly Rust; see [CONTRIBUTING.md](CONTRIBUTING.md).

## Releasing

A release has the Mac DMG, the Linux AppImage and .deb, the two Windows
installers (x64 and ARM), a `.sigstore.json` attestation bundle for each
Linux and Windows installer, the ARM driver script and `SHA256SUMS.txt`,
plus the iPad build on TestFlight.

1. Merge the release PR (version bump and CHANGELOG).
2. Create the release and its tag together, with the CHANGELOG section as
   the notes:
   `gh release create v<version> --target main --title "Knobset <version>" --notes-file <notes>`.
3. The new tag starts the Desktop workflow, which builds the Linux and
   Windows installers and attaches them, their `.sigstore.json` bundles,
   `install-rndis.ps1` and `SHA256SUMS.txt` to the release. If the tag was pushed before the
   release existed, rerun the workflow's last job once it does.
4. Run the Mac and iPad scripts below from a checkout of the tag.

## Releasing to TestFlight

From a clean checkout of the release tag:

```sh
ASC_KEY_ID=... ASC_ISSUER_ID=... scripts/ios/release.sh   # add a build number to re-upload a version
```

It archives, signs manually with the local Apple Distribution certificate,
uploads with the App Store Connect API key and waits for processing. The
one-time setup is in the script's header.

## Releasing the Mac app

From the same checkout, once the GitHub Release exists:

```sh
ASC_KEY_ID=... ASC_ISSUER_ID=... scripts/macos/release.sh
```

It builds a universal DMG, signs it with the local Developer ID Application
certificate, notarizes and staples both the app and the DMG with the same API
key, attaches the DMG to the release and refreshes `SHA256SUMS.txt`. The
one-time setup is in the script's header.

## Icons

`app/src-tauri/icons/app-icon.svg` is the master. After editing it, run
`scripts/icons/generate.sh`, which regenerates every size, strips alpha from
the iOS icons and builds `icon.icns` from a macOS-shaped variant.

## Contributing

Bug reports, feature requests, pager test results and pull requests are
welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) first: it covers what
can't go in the repo (Unication code and files, RadioReference data, pager
backups) and what a change to pager writes needs. Report security issues privately, as
described in [SECURITY.md](SECURITY.md).

## License

Mozilla Public License 2.0. See [LICENSE](LICENSE). Not affiliated with Unication.

The license covers the code, not the name: forks need a different name than
"Knobset".
