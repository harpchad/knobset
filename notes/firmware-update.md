# Firmware update (Unication's updater, V2.05 package)

Source: `gseries_voice_firmware_V2.05_20260928.exe` (a 7-Zip
self-extractor), its native Win32 updater, and one real update of pager C
(G5B67BF) from V2.02 to V2.05 on 2026-10-25. Knobset doesn't
flash firmware and has no plans to: the images are encrypted, the updater
is a closed native tool, and a failed flash is the one way to brick a
pager. What Knobset needs from an update is support for the radio DB
version it leaves behind ([`converter-1.41.md`](converter-1.41.md) 4.3).

## 1. Package

| Path | What |
|---|---|
| `Unication_Device_Upgrade.exe`, `libupgrade.dll`, `libupgradeg4.dll`, `pthreadGC2.dll` | 32-bit mingw updater and its transfer libraries |
| `for_user_file.ini` | Target IP list (`10.10.0.12`) and image paths per SoC |
| `G5A/Gx_Image_MFG_V1.35-T01S-08252026.bin` | Image for ad6900 pagers (the G5 test units). Encrypted; the tool decrypts it to `%TEMP%` before sending |
| `G5N/G5N_Image_V2.05-T01C_20260826.img` | Image for the newer SAMA5D3-based G5N |
| `G5A/DLL/DBV1.xx__to__DBV1.yy.bin`, `G5A/DLL/DBVersionUpgradeList.conf` | Radio DB migrations, one per version step, and the firmware-to-DB-version map |
| `G5A/sys/` | System update script and archive |
| `*.log` | Unication's own logs from building the package, appended to on each run |

Each migration `.bin` is a tar.gz holding a shell `upgrade` script and SQL.
The two that matter here:

- 1.41c to 1.41d rebuilds `tabTwoToneSignaling` with four detector
  threshold columns (`NSig/WSigPwrOn/OffThreshold`).
- 1.41d to 1.41e renames contact 99999 (the pager) to 9999999 and 65535
  (all call) to 9999998 across the contact tables.

The map lists V1.35-T01G-01252024 as DBV1.41c, V1.35-T01J-04112024 as
DBV1.41d and the V2.05 build as DBV1.41e.

## 2. How the updater talks to the pager

- Plain TCP to the IP in `for_user_file.ini`, outbound only (imports are
  `connect`/`send`/`recv`/`select`, no `bind`/`listen`). It probes the
  instruction-set version first to pick the ad6900 or SAMA5D3 path.
- It registers for Windows device-arrival notifications and, on some paths,
  waits for a network-adapter interface (`GUID_DEVINTERFACE_NET`) to
  arrive after rebooting the pager into update mode. That wait has a
  15-second timeout whose result is ignored, followed by a fixed 10-second
  sleep, so the tool carries on even if Windows never sees the pager
  re-enumerate. The ad6900 run on pager C didn't use that path at all.
- The other two arrival GUIDs are WinUSB interfaces for SAM-BA and
  manufacturing recovery modes on the G5N; not relevant to the G5.
- If the user data migration fails, the tool offers a "mandatory update"
  that discards user data. Answer No.

## 3. The real run (pager C, 2026-10-25)

Windows 11 ARM in a UTM VM on the Mac. Unication's RNDIS driver package has
x86/x64 sections only, so USB passthrough to ARM Windows binds the pager as
a COM port; instead the pager stayed on the Mac and the VM reached it
through UTM's shared network plus a temporary pf NAT rule on the pager's
interface:

```text
nat on en17 inet from 192.168.64.0/24 to 10.10.0.0/16 -> (en17)
```

Timeline from the tool's log and a watcher on the Mac:

| Time | Event |
|---|---|
| 14:10:15 | Version probe answered by the ad6900 path |
| 14:10:16 | Image decrypted, init sent |
| 14:10:22 to :31 | Pager reboots; macOS re-binds CDC-ECM and DHCP on its own |
| 14:10:33 | Transfer starts |
| 14:15:22 | Transfer done, "linux download success" |
| 14:15:25 to :40 | Pager reboots into V2.05 |

Afterwards the handshake reports software V2.05, internal version
V1.35-T01S-08252026, radio DB DBV1.41e. The keytab, business, morse and
battery DBs and the stored profile (`u3db_PC.db/.zip`) were byte-identical
to the pre-update backup. The radio DB was migrated in place and its
`.md5` sidecar left stale (see [`converter-1.41.md`](converter-1.41.md)
4.3); PPS or Knobset fixes the sidecar on the next write. Settings changed
on the pager's own menu (backlight here) live only in the radio DB, so any
rewrite from the profile, PPS's included, resets them.

PPS 0.4.0.48 also runs in that VM (install its three MSIs directly; skip
the driver installer) and reaches the pager the same way: it finds the
pager by pinging the configured IP, not by adapter MAC.
