# Capacity limits, validation, and other findings

Paths are relative to `../decomp/View/`.

## Capacity limits (UI-enforced)

| Item | Limit | Evidence |
|---|---|---|
| P25 trunking systems per profile | 1024 | View.P25TSystem/P25TSyetemTotal.cs:32 |
| Sites per system | 256 | SiteListControl.cs:26 |
| Control channels per system (pool) | 256 | ControlCHControl.cs:25 |
| Talkgroups per system | 8192 | P25TGroupListControl.cs:31 |
| Subgroups / individual IDs per system | 999 / 999 | P25TSubGroupListControl.cs:30, P25TIndividualCallListControl.cs:21 |
| Talkgroups per trunking RX (scan) list | 64 (priority + non-priority share) | View.Zone/TrunkingRxGroupListControl.cs:24 |
| Conventional frequencies | 1024 | View.FrequencySetting/FreqSettingControl.cs:29 |
| Zones | 64 | View.Zone/ZoneListControl.cs:82 |
| Knob positions per zone | 8 (template ships 16 rows; verify on hardware) | ZoneListControl.cs:78 |
| Conventional scan list members | 2 to 16, 1 priority | View.Zone/ScanListControl.cs:234 |
| Custom alert tones / voice prompts | 100 each, ≤ 29.5 s, 8 kHz 16-bit mono | View.AlertTone/NewAlertToneControl.cs:155-219 |
| Canned messages | 1024 × 128 chars | CannedMessageSettingControl.cs:88 |
| Encryption keys | 1024 | EncryptionKeyControl.cs:111 |

Name lengths: 14 chars for almost every alias (system, site, TG, zone,
knob); profile name 32; control-channel note 64.

## ID ranges

- WACN 0 to 0xFFFFF; System ID 0 to 0xFFF; RFSS and Site ID 0 to 0xFF;
  SUID 0 to 0xFFFFFF.
- P25 TGID 1 to 65535; unit ID 0 to 16777215; NAC 0 to 0xFFF.
- DMR color code 0 to 15; DMR group 1 to 16776415 plus 16777200 to 16777215.

## Frequencies

- Band edges come from data (`tabRFBandTypeInfo` / `tabRFBandRanges`),
  selected by model-number band codes (View.Profile/BaseDataRepository.cs:79-125,
  ModelNumberInfo.cs:161-183). All G5 models are dual-band: 700/800 plus
  VHF, UHF, or low band.
- Entry is 5 decimals, spinner step 12.5 kHz; no raster check, only band.
- Bandwidth 12.5 / 20 / 25 kHz. Full-spectrum scan: 1 to 3 ranges, step
  1 Hz to 1 MHz.

## Validation worth copying

- System alias required and unique; at least one control channel unless
  full-spectrum scan.
- Sites: unique names, unique RFSS + Site ID pairs.
- Control channels: no duplicate Rx freq per system; in band.
- OEM gaps we should close: no duplicate WACN+SysID check, no duplicate TGID
  check.

## Model-driven features

- Per-model protocol flags in `tabProtocolMode` (CSQ, CTCSS, DTMF, 2-Tone,
  5-Tone, MDC1200, DVOA, P25C, DMRC, ARDS, P25T, DMRT, MPT1327, POCSAG,
  FLEX). P25 trunking page hidden unless `EnableP25T`.
- No P25 Phase 2 / TDMA flag exists anywhere in PPS; "PhaseII" strings refer
  to 2-Tone subgroup signalling. Phase 2 support is in pager firmware, not
  configuration.
- Encryption: AES only; device modes Disable / Uni Enhanced / Multi-Key / KVL.

## Login and access

- PPS login is local only: `Unication`/`Engineer` (engineer mode, skips DB)
  or `admin`/`admin` (MD5 in PpsSetting.db). Our app needs no login.
- Device access password: MD5 hex sent with `01 00 30` / `01 00 31`;
  3-try / 600 s lockout is PC-side only (PwdControl.db).
- Super Password writes `/root/Applications/u3db/SuperPassword.conf`.

## Audio

- Custom tones and prompts: MP3/WAV → 8 kHz 16-bit mono → G.729A (`.dat`,
  CodecG729ab.dll, 10 bytes per 10 ms frame) → `/defpara/Applications/chvoicerecord/<id>.dat`.
  OEM encodes the 44-byte WAV header as audio (bug).
- AMR, AES-to-WAV decrypt, and voice-memo download are dead code. Nothing
  reads recordings off the pager.

## Telemetry and other network calls

- Opt-in (`AutoLog`, default false) error email via `mail.uni.com.tw:25`,
  no TLS, with credentials hardcoded in PPSLog; can attach the profile.
- RadioReference SOAP; "open website" link. No update check.

## Profile file formats

- `.unipps`: header `FE FE FE FE`, version, product ID, versions, Unix
  time, count, CRC-CCITT; per profile length, CRC, ID string, zip of
  `<guid>.db` and `<guid>\` folder (View.Profile/ProfileManagerBiz.cs:273-330).
- P25 public-resource export zip: `P25TTotal.db` index plus one
  `<GUID>.db` per system (P25TSystem.db template).
- Clone from older M5 `*.DAT` / M6 `*.ppg` exists.

## No firmware upgrade

ProgramView only writes profile files. No firmware path in this build.
