# RadioReference integration in OEM PPS

Paths are relative to `../decomp/`.

- CTRL = `RadioReference/RadioReference/RadioReferenceController.cs`
- FORM = `RadioReference/RadioReference/RadioReferenceForm.cs`
- LOGIN = `RadioReference/RadioReference/RadioReferenceLoginForm.cs`
- SEARCH = `RadioReference/RadioReference/SearchForm.cs`
- START = `RadioReference/RadioReference/RadioReferenceStarter.cs`
- HANDLER = `View/View.RadioReference/RadioReferencehandler.cs`
- PROXY = `RadioReference/RadioReference.ServiceReference/RRWsdlPortTypeClient.cs`

## Endpoint and auth

- SOAP 1.1 over plain HTTP: `http://api.radioreference.com/soap2/index.php`,
  namespace `http://api.radioreference.com/soap2`, `version = "latest"`,
  30 s receive timeout (G_Series_PPS/app.config:46-53).
- `authInfo`: username, password, appKey, version, style.
- App key: a vendor (Unication) key hardcoded at CTRL:101. **Our app needs its
  own RR app key**; we must not reuse theirs.
- Premium only: login rejected unless `subExpireDate` is "Never" or in the
  future (CTRL:69, LOGIN:116-124).
- "Remember me" stores user/password in `<AppDir>\login2` / `<AppDir>\login`,
  AES-ECB with a static key in the binary (CTRL:30, Global/Global/Encrypt.cs).
  Credentials travel in cleartext over HTTP.

## Methods used

Called from the UI: `getUserData`, `getCountryInfo`, `getStateInfo`,
`getCountyInfo`, `getAgencyInfo`, `getTrsDetails`, `getTrsSites`,
`getTrsTalkgroups` (tgCid/tgTag/tgDec = 0), `getTrsTalkgroupCats`,
`getSubcatFreqs`, `getZipcodeInfo` (only if no cached CSV).

Wrapped but only used by an uncalled US crawler `getUSA()` (CTRL:1205):
`getTrsType`, `getTrsVoice`, `getTag`, `getTrsFlavor`. Proxy-only, never
called: `getTrsBySysid`, `searchStateFreq`, `searchCountyFreq`, FCC methods,
metro methods, etc. `getCountryList` is hardcoded to US=1, CA=2, AU=16.

## UI flow

1. Status bar button → `RadioReferenceStarter.start()`; first run unzips
   `OffLine.zip` (48 MB, 99,105 cached RR CSVs from Jun to Aug 2020) to
   `%ProgramData%\Unication\RadioReference\OffLine\`.
2. Login dialog (Connect / Offline / Cancel). "Offline" uses only cached CSVs.
3. "Import Radio Reference" form:
   - Step 1: pick Country / State / County.
   - Step 2: Agencies, P25 Trunking Systems (only `sType == 8`), or
     Conventional, then "Get Data".
   - Data tab: checkbox trees of sites → control channels and TG
     categories → TGIDs. Options: "Import all control channels" (default) vs
     "primary & alternate"; "Create new Zone/Knob" (default on).
   - Buttons: "Import to Profile", "Import to Public Resource".
4. Search dialog by US zip or RR sid (RR's internal id, not P25 SysID).

## Mapping into PPS tables (HANDLER `dealWithTrunking` 151-456)

| PPS | From RR | Notes |
|---|---|---|
| `tabP25TrunkingSystem.Name` | `sName` | truncated to 14 |
| `WacnID`, `SystemID` | `sysid[0].wacn/sysid` (hex) | only first sysid; defaults FFFFF / FFF |
| `SystemSource` | `RadioReference-{sid}` | |
| `tabP25TrunkingSite` | `siteDescr`, `rfss`, `siteNumber` | NAC, RAN, lat/lon dropped |
| `tabP25TrunkingControlCH` | `freq` × 1e6 Hz, Rx = Tx | order d, a, then rest |
| `tabGroup_New` | `tgAlpha` (14), `tgDec`, `tgDescr` | AlertMode 3, tone 9999 |

- Default "all control channels" also imports RR voice channels (empty
  `use`) as control channels.
- Talkgroups: **no filtering on `enc`, `tgMode`, tags; no dedupe**; RR
  categories are flattened and lost. Empty selection adds "All Call" 65535.
- Optional zone creation: one zone, knob 1 = mode 16 (trunking TG scan).
- Conventional (`dealWithAgency`): Rx = Tx = `out` freq (input ignored),
  drops NXDN/D-STAR/YSF/SSB/Telm, maps PL/DPL/NAC/DMR color code.
- Band check: every frequency must be inside the profile model's band.

## Bugs and limitations

- P25 trunking only; US/CA/AU only; zip search US only.
- Silent caps: 256 sites, 256 control channels, zones ≥ 64 aborts.
- Name truncation bug: trimmed vs untrimmed length mismatch yields "" for
  some long names (HANDLER:142-149).
- Offline CSV parser reads `sVoice`, `sFlavor`, `sCity` from the `sType`
  column (CTRL:336-338).

## Implications for our app

- Register our own RR app key, use HTTPS if the endpoint supports it, keep
  credentials in the OS keychain.
- Cache per user only and respect RR terms; do not ship a bulk RR dump.
- Do better than OEM: filter encrypted TGs, keep RR categories (map to scan
  lists), distinguish control vs voice channels, choose site subsets by
  location, and show a diff before import.
- Also support RR CSV export import (forum reports say the CSV downloads
  need Premium too; pasting the page tables works without it; spec in
  radioreference-import.md): sites need `RFSS`, `Site Dec`, `Description`, `Frequencies`;
  talkgroups need `Decimal`, `Alpha Tag`, `Mode`, `Description`, `Tag`.
