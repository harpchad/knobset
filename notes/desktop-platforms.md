# Linux and Windows desktop builds

Knobset's Mac DMG and iPad build are made locally and signed
(`scripts/macos/release.sh`, `scripts/ios/release.sh`). The Linux and
Windows builds come from GitHub Actions (`.github/workflows/desktop.yml`)
and are unsigned. This note covers what each OS needs for the pager's USB
link, where the app keeps its files, and what hasn't been checked on real
hardware yet.

## What CI builds

| OS | Runner | Artifacts | Floor |
|---|---|---|---|
| Linux x86_64 | `ubuntu-22.04` | `Knobset_<v>_amd64.AppImage`, `Knobset_<v>_amd64.deb` | glibc 2.35 and WebKitGTK 4.1 (Ubuntu 22.04, Debian 12, Fedora 36 or newer) |
| Windows x64 | `windows-latest` | `Knobset_<v>_x64-setup.exe` (NSIS) | Windows 10 1903 or later with WebView2 (Windows 11 ships it) |

When it runs:

- Every PR touching `crates/`, `app/`, `Cargo.*` or the workflow: frontend
  type-check, `cargo clippy --workspace --all-targets -D warnings` and
  `cargo test --workspace` on Ubuntu. The fixture tests skip, since CI has
  no pager backups or Unication templates.
- Installers build on `v*` tags, on manual runs (Actions › Desktop › Run
  workflow), and on PRs labelled `build-installers` or touching the
  workflow file. The Windows job also runs the core crates' tests, the only
  check of the MSVC build of SQLite 2 and of Windows file semantics.
- On a tag, a last job attaches the installers to the GitHub Release for
  that tag. It fails with a clear message if the tag doesn't match
  `tauri.conf.json` or the release doesn't exist yet; create the release
  and rerun the job. Run artifacts are kept for 14 days either way.

Minutes: standard GitHub-hosted runners are free for public repos, so
none of this is billed once Knobset is public. While it was private, the
free tier was 2,000 minutes a month and a Windows minute counted as two.
Measured on the PR that added the workflow:

| Run | Check | Linux installer | Windows installer | Billed |
|---|---|---|---|---|
| Cold caches | 9 min | 11 min | 17 min (34 billed) | about 54 min |
| Warm caches | 3 min | 6 min | 8 min (16 billed) | about 25 min |

Caches are per branch and nothing runs on `main`, so a tag build always
starts cold: about 55 billed minutes a release, and 3 to 9 per PR push.

No MSI: NSIS installs per user without admin rights, and nobody deploys
Knobset with Group Policy. Adding `msi` to the Windows `bundles` is all it
takes if that changes. Windows on ARM gets a native installer from the
`windows-11-arm` runner (free on public repos); it would also run the x64
one under emulation. No Linux ARM build yet.

## The pager's USB link

The pager is a Linux USB gadget, VID `17FF` PID `A4A2`, product string
"Unication RNDIS USB net", with two configurations: CDC-ECM first and
(presumably) RNDIS second (see `protocol.md`). Whichever one the host binds, the result
is a network adapter. The pager runs a DHCP server at `10.10.0.12` and
hands the host an address in `10.10.0.0/16` (10.10.0.6 so far). Its lease
has no router and no DNS server, so it never takes over the default route
or DNS. Knobset then opens plain TCP to `10.10.0.12:32769`. No USB driver
code is involved in the app, and it needs no USB permissions.

### macOS and iPadOS

Built-in `AppleUserECM` binds the ECM configuration. Nothing to install.
macOS 15 and later ask once whether Knobset may reach the local network.

### Linux

The kernel picks the ECM configuration and binds `cdc_ether` (if a kernel
ever picks the RNDIS one, `rndis_host` covers it). Both are modules in
every mainstream distro kernel. Nothing to install, and no udev rule is
needed, since Knobset only uses sockets.

Check it after plugging the pager in and letting it boot:

```sh
ip -br addr        # expect something like: enxbe6e3b3e6f6b  UP  10.10.0.6/16
ping -c 3 10.10.0.12
```

The interface is usually named `enx` plus the MAC (`enxbe6e3b3e6f6b`),
sometimes `usb0`. `dmesg | grep -i cdc_ether` shows the bind.

With NetworkManager (every desktop distro), the device shows up as a new
wired connection and gets its address by DHCP on its own. Gotchas:

- **Device unmanaged.** `nmcli device status` lists it as `unmanaged`.
  Ubuntu Server images and some minimal installs ship
  `/usr/lib/NetworkManager/conf.d/10-globally-managed-devices.conf`, which
  leaves Ethernet devices to netplan, and a device named in
  `/etc/network/interfaces` is unmanaged too. Either let NetworkManager
  have it:

  ```sh
  sudo nmcli device set enxbe6e3b3e6f6b managed yes   # until reboot
  ```

  or make that permanent with a file in `/etc/NetworkManager/conf.d/`:

  ```ini
  [keyfile]
  unmanaged-devices=except:mac:be:6e:3b:3e:6f:6b
  ```

  (that only overrides a broader `unmanaged-devices` line; if there's none,
  `managed yes` alone is enough).

- **No NetworkManager** (systemd-networkd, e.g. Ubuntu Server with netplan's
  default renderer): networkd ignores interfaces it has no config for. Add
  `/etc/systemd/network/50-g5-pager.network` and
  `sudo systemctl restart systemd-networkd`:

  ```ini
  [Match]
  MACAddress=be:6e:3b:3e:6f:6b

  [Network]
  DHCP=ipv4

  [DHCPv4]
  UseRoutes=false
  UseDNS=false
  ```

- **The pager reboots after a write.** On macOS the USB link and its
  lease stay up through that (it's a service restart, not a USB
  re-enumeration), and Knobset retries the read-back for about a minute.
  If Linux drops the address anyway, NetworkManager renews it well inside
  that window.

- **ModemManager.** It probes CDC devices for modems and shouldn't touch a
  network-only gadget. If the link flaps for the first half minute after
  plugging in, tell it to keep away with a udev rule in
  `/etc/udev/rules.d/70-g5-pager.rules`:

  ```
  ATTRS{idVendor}=="17ff", ATTRS{idProduct}=="a4a2", ENV{ID_MM_DEVICE_IGNORE}="1"
  ```

The host-side MAC above is the one seen so far, and the one PPS looks
for. If yours differs, use what `ip link` shows.

### Windows

Windows has no in-box CDC-ECM driver (Windows 11's USB networking class
driver is CDC-NCM, which doesn't apply), so the pager needs its RNDIS side.
The in-box RNDIS driver evidently doesn't claim the pager on its own, or
Unication wouldn't ship an INF for it. Without a driver, expect it under
Other devices in Device Manager with a warning icon.

Unication's driver package fixes that. It's in both of Unication's
downloads: the standalone `unication_windows_rndis_driver_setup.zip`, and
the PPS setup itself, which carries `Unication_Device_Driver_Setup.exe` in
its payload. If PPS has ever been installed on the PC, look in Device
Manager before installing the driver again. What's in it, checked in the extracted copies:

- `tether_driver.inf`, `DriverVer 07/24/2014, 1.0.0.0`, class `Net`. It
  matches `USB\VID_17FF&PID_A4A2` and does nothing but point Windows at
  its own built-in RNDIS driver (`include = netrndis.inf`,
  `needs = usbrndis6.ndi` on Windows 7 and later). It installs no `.sys`
  files.
- `unication_rndis.cat`, signed by Microsoft Windows Hardware
  Compatibility Publisher (WHQL) in 2015, so it installs on current
  Windows without test-signing or warnings.
- `dpinst.exe`, Microsoft's driver installer, which the setup wraps.
- Decorations for x86 and x64 only. On Windows on ARM the INF won't match;
  use the manual route below.

Since the INF only selects an in-box driver, there's a manual route that
needs no Unication download (untested on the pager): Device Manager ›
the pager's entry › Update driver › Browse my computer › Let me pick ›
Network adapters › Microsoft › Remote NDIS Compatible Device, and accept
the compatibility warning.

RNDIS on Windows 11: still in the box (`netrndis.inf`, `usb8023.sys`),
including 24H2, and Microsoft hasn't announced its removal. If that ever
changes, the fallback is a userspace RNDIS or ECM stack over WinUSB
(`nusb` plus `smoltcp`, see `architecture-proposal.md`).

Check it with `ipconfig`: an Ethernet adapter described as "USB Remote
NDIS Network Device" with an address in `10.10.0.x`, then
`ping 10.10.0.12`. A `169.254.x.x` address means DHCP didn't answer; give
the pager a few more seconds to boot, or unplug and replug it. Windows
files the link under "Unidentified network" on the Public profile. That's
fine: Knobset only makes outgoing connections, so Windows Firewall doesn't
prompt.

## Installing the unsigned builds

- **Windows.** Edge may flag the download as uncommon: … › Keep. Then
  SmartScreen says "Windows protected your PC": More info › Run anyway.
  The installer is per user and needs no admin rights. It fetches WebView2
  if it's missing (Windows 11 and current Windows 10 have it).
- **Linux AppImage.** `chmod +x Knobset_*.AppImage` and run it. It needs
  FUSE 2 (`libfuse2`, or `libfuse2t64` on Ubuntu 24.04 and later); without
  it, run it with `--appimage-extract-and-run`.
- **Linux .deb.** `sudo apt install ./Knobset_*_amd64.deb`, which pulls in
  WebKitGTK 4.1. It puts Knobset in the applications menu.
- On some NVIDIA and Wayland setups WebKitGTK draws a blank window; start
  it with `WEBKIT_DISABLE_DMABUF_RENDERER=1`.

**Signing is Chad's call, later.** Azure Trusted Signing is about $10 a
month (Basic tier) and Tauri supports it through a custom `signCommand`;
it removes "Unknown publisher" and lets SmartScreen reputation attach to
a stable identity. Until there are Windows users beyond testers, "More
info › Run anyway" is a reasonable price. Linux packages are routinely
unsigned; a signed apt repository isn't worth it at this size.

## Where the app keeps things

Tauri's app config and data dirs, with identifier `com.harpchad.knobset`:

| OS | Settings | Backups (default) and imported templates |
|---|---|---|
| macOS | `~/Library/Application Support/com.harpchad.knobset/settings.json` | same folder: `backups/`, `templates/` |
| Linux | `$XDG_CONFIG_HOME` or `~/.config/com.harpchad.knobset/settings.json` | `$XDG_DATA_HOME` or `~/.local/share/com.harpchad.knobset/`: `backups/`, `templates/` |
| Windows | `%APPDATA%\com.harpchad.knobset\settings.json` (Roaming) | same folder: `backups\`, `templates\` |
| iPadOS | app container | Documents/Backups (Files › On My iPad › Knobset) |

Settings › Backups folder moves backups anywhere. Uninstalling doesn't
remove any of this.

## Runtime differences

- **Show in file manager.** The backups list's folder button uses the
  opener plugin's reveal: Finder on macOS, File Explorer (with the item
  selected) on Windows, and on Linux the freedesktop `FileManager1` D-Bus
  call (Nautilus, Dolphin, Nemo, Thunar), falling back to the desktop
  portal. Its tooltip names the right one (`app/src/lib/platform.ts`).
- **Platform wording.** The Pager page's connection hint is per OS, from
  the webview's user agent (WKWebView, WebView2, WebKitGTK). iPadOS
  reports a Mac user agent and keeps the macOS text.
- **Keyboard.** Knobset defines no shortcuts of its own; Enter in a few
  fields is the only key handling. On macOS Tauri adds the standard app
  menu (⌘C, ⌘V, ⌘Q). Windows and Linux get no menu bar, and text fields
  use the webview's own Ctrl+C, Ctrl+V, Ctrl+Z; Alt+F4 closes the window.
- **Keeping the app awake.** `keep_alive.rs` only does something on iOS. A
  desktop write takes about 15 seconds including the pager's restart, far
  inside any sleep timer.
- **File paths on Windows.** The vendored SQLite 2 opens files with
  `CreateFileA`. The app's manifest
  (`app/src-tauri/windows-app-manifest.xml`) sets the UTF-8 active code
  page, so profile and backup paths under a non-ASCII user name open. The
  `g5` CLI has no such manifest, so on Windows it needs ASCII paths.
- **Templates.** "Choose folder" accepts an installed PPS `DataBase`
  folder, which on Windows is `Unication\G_Series_PPS_V00-04-00-48\DataBase`
  under `Program Files` or `Program Files (x86)` (from the MSI's layout). Importing the PPS download works the
  same on every OS.

## Untested on real Linux and Windows machines

CI proves the code compiles, the core tests pass on Ubuntu and Windows,
and the installers build. Nobody has yet:

- installed and launched either build, or seen the UI in WebView2 or
  WebKitGTK (layout, fonts, dark mode, dialogs);
- brought the pager's link up on Linux (`cdc_ether` binding, NetworkManager
  DHCP) or on Windows (Unication's driver on Windows 11, the manual
  Remote NDIS route, Windows on ARM);
- taken a backup or written to a pager from Linux or Windows, including
  the read-back after the pager reboots (RNDIS on Windows may drop the
  link where ECM on macOS doesn't);
- used "Show in file manager", the folder and file pickers, or template
  import on either OS;
- opened a backup under a non-ASCII Windows user name (the UTF-8 code page
  fix is untested);
- checked that the AppImage runs on a distro newer than Ubuntu 22.04.
