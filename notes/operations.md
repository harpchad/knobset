# G-series PPS high-level operations (PPS V00-04-00-48)

Companion to `protocol.md` (framing, header, CRC, Request/Accepted/Response,
Continue/Complete chunking). This file covers which opcodes, files and
payloads each user-visible operation uses, in order.

Paths are relative to `../decomp/`. Abbreviations:

- `HDP` = `communication/Unication.PPS.U3.CommunicationLayer/HighDeviceProtocol.cs`
- `GP` = `communication/Unication.PPS.U3.CommunicationLayer/GeneralProtocol.cs`
- `U3D` = `communication/Unication.PPS.U3.BussinessLayer/U3Device.cs`
- `U3DL` = `communication/Unication.PPS.U3.BussinessLayer/U3DeviceList.cs`
- `HSI` = `PPSLog/Unication.PPS.U3.CommunicationLayer/HandsShakeInfo.cs`
- `PC` = `View/View.ProgramVersion/ProgramController.cs`
- `RPD` = `View/View.ReadDevice/ReadProfileFromDeviceControl.cs`

## 0. Conventions used below

"Content" means the payload after the 12-byte header, before the trailing
CRC (see `protocol.md`). `S` = 2-byte session ID (present only when
`HasSection` is true). All multi-byte integers are big-endian unless noted.
Strings are UTF-8 (`DataConvert.ToBytes(string)` = `Encoding.UTF8`).

### Path/string encoding (used by every file op)

```
[len BE u32 = strlen(path) + 1] [path bytes] [00]
```

`len` counts the trailing NUL. Note PPS computes it from `string.Length`
(UTF-16 code units), not the UTF-8 byte count, so only ASCII paths are
safe. Example for `/defpara/Applications/PPS/`
(26 chars): `00 00 00 1B 2F 64 65 ... 2F 00`.

### Three call shapes (all in HDP, each allocates a fresh session ID)

| Shape | Request content | Response content | Then |
|---|---|---|---|
| `Command(op+params)` (HDP:179) | `S params` | `S [rc]` + more | `rc` byte returned; 0 = success |
| `Read(op+params)` (HDP:15) | `S params` | `S [devStatus][totalLen BE u32]` | pager streams Inform `00 20 00` chunks, then Inform `00 20 01` |
| `Write(op+params, data)` (HDP:39) | `S params` | `S [devStatus]` | host streams `00 20 00` chunks (<=2048 B), then `00 20 01` with data `00` |

`devStatus` (GP:439): 0 normal, 4 = "device had been locked", 255/other =
unknown error (throws).

### Business brackets

Multi-file operations are wrapped in a "business":

- Start: `Command(<start opcode> + [param])`, must return `rc == 0`
  (U3D:296).
- Stop: `Command(00 20 01 + [code])` (U3D:301). `code` 0 = commit/normal
  end, 1 = abort (sent from exception handlers). Note `ENDOPCODE` reuses the
  Complete opcode `00 20 01`, but as a Request with a session, not an
  Inform.

| Start opcode | Param | Used by |
|---|---|---|
| `01 00 01` STARTREAD | `00` | `ReadU3Files(string[])`, `ReadU3Parameters` |
| `01 00 02` STARTWRITE | `01` | `ProgrammingFiles`, `WriteU3Files` |
| `01 00 03` STARTBACKUP | n/a | defined, never used |
| `01 00 04` STARTUPDATE | `00` | `UpdateDatabase` (dead code) |
| `01 00 08` STARTKEY | `01` | `ProgrammingKey` (dead code) |
| `01 00 20` forest-fire zone write | n/a | defined, never used |

## 1. Connect / identify

### Discovery (what actually runs)

1. `Program.Main` calls `U3DeviceList.Instance.InitializeNetDevices()`
   (`G_Series_PPS/New_PPS/Program.cs:57`). The USB/WinUSB
   `InitializeDevices()` is never called.
2. `InitSet` ctor loads `ConnectionStatus.ConnType/IP/Port` from
   PpsSetting.db (`View/View.Profile/InitSet.cs:181-187`). ConnType 1 = "USB"
   (the RNDIS TCP path), 2 = Ethernet; anything else and `GeneralProtocol`
   throws for `DeviceType.Net` (GP:137-142). The exe.config
   `CommunicationType=3` key is never read by any code.
3. Background thread `ConnectedNetDevices` (U3DL:564-601), every 3 s:
   - ICMP ping `ConnectionStatus.IP`, 32 bytes of `a`, 120 ms timeout, DF set
     (`PingDevice`, U3DL:603-632).
   - Ping fails and a device is listed: `DeleteNetDevice` (no ByeBye, socket
     not closed).
   - Ping OK and no device: new `U3Device(HighProtocol)` then `HandsShake()`;
     on success `AddNetDevice`.
4. `USBDetect.InitDeviceNetworkIP` (netsh static 10.10.0.10/16 on the NIC
   with MAC `BE6E3B3E6F6B`) has **no callers** in any shipped assembly
   (checked decompiled C# and raw member names in every DLL/EXE). In this
   build PPS does not configure the host IP itself; the RNDIS driver
   package, DHCP from the pager, or the user must. Worth correcting in
   `protocol.md`.
5. TCP: `NetWorkDevice.Open()` connects only if no socket exists; only
   `U3Device.HandsShake()` calls `Open()` (which first `Close()`s). So one TCP
   connection is opened per handshake and reused for every later operation
   until ByeBye/Close.
6. Single-device app: every UI path uses `U3DeviceList.Instance.Devices[0]`.

### Handshake (opcode `00 00 00`, no session)

Built in `HDP.HandsShake(byte[] product)` (HDP:90-120), called from
`U3Device.HandsShake()` (U3D:241-284). Timeout 2000 ms, 1 try.

Request content (7 bytes, then CRC):

| Off | Value | Source |
|---|---|---|
| 0 | `03` | product ID; appSettings `ProductID` if present (absent in shipped config) |
| 1 | `01` | `Description.Edition` |
| 2 | `05` | `(byte)Protocol.BusType` = `BT_U4` |
| 3..6 | `FF FF FF FF` | `Description.ProtocolMode` BE |

Correction for `protocol.md`: `BusinessType` is `BT_U3 = 3, BT_ConsoleDemo,
BT_U4`, so `BT_U4` = **5**, not 4
(`communication/Unication.PPS.U3.CommunicationLayer/BusinessType.cs`).
ProtocolMode: `U3DeviceList` ctor calls `CreateDescriptionByNode(8000, 1,
ProtocolMode)` and the getter turns 0 into `uint.MaxValue` (U3DL:160-187).

Response content (HasSection false, so no session; CRC stripped):

```
[productID u8][productType u8][businessID u8][reserved u8]  deviceInfo...
```

`deviceInfo` is parsed by `HSI.AnalyzeHightDeviceInfo(byte[])`
(HSI:249-345). It is a sequence of length-prefixed strings via
`GetFieldValue` (HSI:347-370):

```
field := [len BE u32, counts trailing NUL] [len-1 bytes UTF-8] [00]
len == 0  -> "" (only the 4 length bytes consumed)
```

Field order:

| # | Encoding | Field | Notes |
|---|---|---|---|
| 1 | string | model + serial | if 56 chars: `ModelNO = [0..40)`, `SerialNO = [40..56)`; otherwise `ModelNO = null`, `SerialNO = whole`. (The file-based parser also accepts 60 = 40 + 20; the wire parser does not.) |
| 2 | string | `Password` | parsed, never used by PPS |
| 3 | string | `RadioNo` | |
| 4 | 2 raw bytes `b0 b1` | band/board bitfield | `functionType = b0>>6`; `boardType = (b0>>2)&0xF`; `secondaryBand = ((b0&3)<<3) | ((b1>>5)&7)`; `primaryBand = b1&0x1F` |
| 5 | string | `ManufactureDate` | |
| 6 | string | version tuple | split on `:`: `SWVersion:InternalVersion:U3DBVersion:ProtocolVersion` (fewer parts tolerated; 1 part sets SW and Internal to the same value). Sets statics `g_swVersion`, `g_protocolVersion`. |
| 7 | string | `HWVersion` | |
| 8 | string | `FactoryParameter` | |
| 9 | u32 BE | `ProtocolMode` | |

Model numbers are 40-char fields, space or zero padded; PPS uses the first
21 chars (e.g. `G5H67AB-SXUDENBX00UNI`). Parsing:
`View/View.Profile/ModelNumberCode.cs` (v1 layout: product `[0,2)`,
hardware `[2]`, protocol `[3,5)`, software `[5,7)`, `-`, RF band 1
`[8,10)`, RF band 2 `[10,12)`, language `[12,14)`, cert `[14]`, safety
`[15]`, model ver `[16,18)`, spec ver `[18,21)`).

Fields PPS actually consumes: `ModelNO`, `SerialNO`, `SWVersion`,
`U3DBVersion` (status bar `View/View.StatusBar/DeviceInfoFrom.cs:34-40`,
Program/Read gating), plus `g_protocolVersion`/`g_swVersion` (password
gating). ProductID/Type/BusinessID are stored but never compared
(`IsSameTypeMachine` has no callers).

### Heartbeat: not active

- `U3Device.StartHeartbeat()` and `StopHeartbeat()` are empty
  (U3D:1386-1388, 1459-1461). The `Heartbeat()` loop (U3D:1390-1457, which
  would send `Command(00 00 04)` every 2 s and re-handshake on rc 2) is never
  started.
- `Listen()` (U3D:400-438, which handles pager-initiated `00 FF FF` ByeBye
  and `00 00 05` ALIVE by re-handshaking) is only started by `StartListen()`,
  which has no callers.
- Net effect: liveness is ICMP ping only. Our app does not need BEAT/ALIVE
  for parity, but should tolerate the pager sending them.

### ByeBye (`00 FF FF`)

`HDP.ByeBye` (HDP:169-177): Request (type 1), current seq, **no content at
all** (length 0, no session, no CRC). Waits up to 100 ms for any reply, then
`U3Device.ByeBye` closes the socket (U3D:311-331). Sent by
`DisconnectDevices` (U3DL:634-683, sends it twice to net devices): on app
exit (`Program.cs:66`), after a password-check comm error
(`AccessPasswordHandler.cs:99,145`, `DeviceAccessPasswordForm.cs:231`), and
when the connection setting is saved (`DeviceConnectionSetting.cs:102`).

### UpdateDateTime (`01 02 01`, no session)

`HDP.UpdateDateTime` (HDP:122-145). Content:

```
[seq BE u16] [zoneLen u8] [zone ASCII] [yyyyMMddHHmmss ASCII, local time]
```

- `seq` is `SeqID.ToBytes()` evaluated before the header is built, so the
  header seq is this value + 1. It occupies the slot a session ID would.
- `zone` = `"GMT+hh:mm:ss"` when local is behind UTC, `"GMT-hh:mm:ss"` when
  ahead (POSIX sign inversion). Central daylight time sends `GMT+05:00:00`.
  PPS derives it from `(Now - UtcNow).ToString()` with two clock reads, so
  it can send `04:59:59`; we should compute it properly.
- Host sends, reads up to two packets (100 ms each), checks the last one and
  ACKs it.
- Only caller: end of `ProgrammingFiles` (U3D:531), inside the write
  business, right before StopBusiness.

### RebootDevice (`01 02 02`, no session)

`HDP.RebootDevice(short msec)` (HDP:147-167). Content: `[msec BE i16]`.
Same send/receive pattern as UpdateDateTime. **No callers** anywhere: PPS
never reboots the pager after programming.

### ExecuteCommand and `Config/*.cmd`

- `U3Device.ExecuteCommand(string)` calls `Protocol.ExecuteCommand`, whose
  base implementation returns 0; `HighDeviceProtocol` does not override it.
  It sends nothing. No callers.
- `Config/readProfile.cmd` is the 11 ASCII bytes `readProfile`;
  `Config/getDeviceInfo.cmd` is the 13 bytes `getDeviceInfo` (no newline).
  Neither name appears in any shipped assembly (searched ASCII and UTF-16 in
  every DLL/EXE). Vestigial; likely intended as payloads for an
  ExecuteCommand opcode that this build never implemented.

## 2. Access password

### Wire (`HDP.accessPassword` HDP:548-578, `accessPasswordWrite` HDP:580-610)

| Opcode | Purpose |
|---|---|
| `01 00 30` | read access check (before Read Profile) |
| `01 00 31` | write access check (before Program) |

Request: type 1, new session, content:

```
S [pwLen BE u32] [pw bytes]
```

Response: after stripping S and CRC, exactly 1 byte; anything else, or a
CRC/checksum failure, makes PPS return 3. Note it uses only two 100 ms
receive windows, so the pager must answer fast.

| rc | Meaning in PPS |
|---|---|
| 0 or 1 | pass |
| 2 | denied, prompt for password |
| other / 3 | comm error; PPS calls `DisconnectDevices()` (ByeBye) |

### Flow (`View/View.DevicePassword/AccessPasswordHandler.cs`)

1. Version gate (`notCompatibleVersionRead/Write`, lines 16-57): if
   `g_protocolVersion > 0` always check. Otherwise skip the check on old
   firmware: read is skipped when SW contains `V0.1-0.12B_T` with build <
   34; write is skipped unless SW contains that prefix with build > 40 (or
   40 with suffix letter > `n`).
2. Probe with an **empty password**: `pwLen = 0`, no bytes (lines 86-101,
   132-147). rc 0/1 means no password set or not required.
3. On rc 2, `DeviceAccessPasswordForm` asks the user, then sends
   `MD5(ASCII(password.Trim()))` as **32 uppercase hex ASCII chars**
   (`DeviceAccessPasswordForm.cs:195-233`; md5 helper
   `Global/Global.Common/Common.cs:200-210` produces lowercase, the form
   calls `.ToUpper()`). So `pwLen = 0x20`.

### Lockout is client-side only

`tabPwdControlGlobal` in PwdControl.db (defaults 3 tries, 600 s;
`View/View.Setting/PwdControlDataRepository.cs:15-24`) and
`tabPwdControlUser` (type 0 read, 1 write; count + reset timestamp). The
pager is never told about failed attempts. Separately the pager can report
`devStatus 4` (locked) on Read/Write starts, and the protocol has an
`Error.Password` (249) path with "tries left" in `packet[1]` (GP:418-419),
so the device may enforce its own lockout. Unknown until tested.

### Super Password (Other Tools menu)

`View/View.SuperPassword/SuperPasswordControl.cs:74-101`: 4 to 16 chars,
writes a local file

```
[SuperPassword]\r\n
Password=<MD5 upper hex>\r\n
```

and pushes it with `ProgrammingFile(path, "/root/Applications/u3db/SuperPassword.conf", 01 01 02)`,
a bare WRITEFILE with **no Start/Stop business, no mutex, no access
password check**. Presumably sets or overrides the device access password.

### PPS login (local only)

`tabLoginPPS` in PpsSetting.db holds `admin` / `21232f297a57a5a743894a0e4a801fc3`
(lowercase MD5 of `admin`). Checked locally by `View/View.Login/Login.cs`
before the main form opens; never sent to the pager. Our app can drop it.

## 3. File operations (U3D)

### READPATH `01 01 01`: directory listing

`ReadU3Files(string u3Path)` (U3D:1121-1169); variant with sizes at
U3D:1067-1119.

- Shape: `Read`. **Not** wrapped in a business and no mutex.
- Request params: path encoding (section 0), e.g. `/defpara/Applications/PPS/`
  with trailing slash.
- Response data (after the Read stream is reassembled), repeated to end:

```
[nameLen BE u32, counts NUL] [name bytes] [00] [size BE u32]
```

Names are bare file names (compared against `"u3db_PC.zip"`, `"u3db.db"`).
Whether directories appear is unknown.

### READFILE `01 01 03`

`ReadU3Files(string[] files, out byte[][])` (U3D:694-759):

```
Command 01 00 01 [00]                 start read business
for each file:
  Read 01 01 03  <path encoding>      file bytes = reassembled stream
Command 00 20 01 [00]                 stop (always, in finally)
```

`ReadU3Parameters(file)` (U3D:636-692) is the same for one file.

### WRITEFILE `01 01 02`

Request params (U3D:493-508, also 1298-1309):

```
[fileLen BE u32] [pathLen BE u32 = strlen+1] [path bytes] [00]
```

followed by the `Write` data phase (file bytes in <=2048-byte Continue
chunks, each ACKed, then Complete with `00`).

`ProgrammingFiles(pcfiles[], u3files[])` (U3D:457-547), the one Program
uses:

```
Command 01 00 02 [01]                 start write business; rc != 0 -> -2
for each (pc, dst):  Write 01 01 02 <params> <file bytes>   false -> -3 (no stop sent!)
  exception -> Command 00 20 01 [01], rethrow
UpdateDateTime 01 02 01               (inside the business)
Command 00 20 01 [00]                 stop/commit
```

`WriteU3Files(byte[][], paths[])` (U3D:761-830) is identical minus the
date/time set. No callers. `ProgrammingFile(pc, dst, op)` (U3D:1284-1315)
is a bare single write with no business.

### DELETEFILE `01 01 04`

`DeleteU3File(path)` (U3D:1317-1348): `Command(01 01 04 + <path encoding>)`,
rc 0 = success. No callers in the UI (only the dead `UpdateDatabase`).

### ProgrammingKey `01 00 08`

`ProgrammingKey(keyTable)` (U3D:832-898): start `01 00 08 [01]`, WRITEFILE to
`/root/Applications/u3db/keytab`, stop `[00]`. No callers in this build
(member name only exists in communication.dll). Encryption keys go into
u3db.db instead.

## 4. Write profile to pager ("Program")

### Entry points

- Edit Profile toolbar Program button:
  `View/View.EditProfileUI/EditProfileControl.cs:1066` calls
  `ProgramController.Program(profileDbPath, this)`.
- `ProgramView` and `DetailReportControl` only call `CheckProfile`.

### Pre-flight checks (PC:61-169), in order

1. `CheckProfile`: local profile validation (`View/View.ProfileCheck/ProfileCheckHandler.cs`); errors open a report and abort.
2. A device is connected; not already programming.
3. Device model (first 21 chars) must parse as a valid `ModelNumberCode`
   (else "device does NOT have a model number").
4. Device model must match an enabled row of `tabModelNumberInfo`
   (`Enable=1`, Data_G4A.db) on product, hardware, protocol, software, both
   RF bands and language (`ModelNumberCode.isContain`).
5. Device `U3DBVersion` (from handshake) must be a substring match in
   `tabSupportU3DBVersion` (Data_G4A.db: `DBV1.41a`..`DBV1.41e`,
   `DBV2.01`..`DBV2.06`). Same check as `CommonFunction.CheckSupportU3DBVersion`
   (`View/View.Common/CommonFunction.cs:1070-1081`). Note it is
   `IndexOf`, so a short/garbage version could false-match.
6. If profile model != device model: product code must match; RF band
   codes must match exactly; Tow Truck (software code starting `T`)
   mismatch is blocked both ways; a hardware downgrade to B/D or protocols
   the device lacks trigger a confirm dialog (`ProgramQuestion`) listing
   protocols that will be stripped (`getRemoveProtoclList`, PC:189-203).
7. Access password write check (`01 00 31`, section 2).

### PC-side build (`ProgramData`, PC:205-260)

1. Copy profile `<guid>.db` and its resource folder `<guid>/` to
   `%UserData%\Temp\Convert\`.
2. `ChangeDB` (PC:262-393): if models differ, rewrite the profile's model to
   the device model, copy that model's `MFG_G4A.db`/`BSS_G4A.db` from
   `DataBase/<model>/`, clamp frequencies to the device bands, delete
   protocols the device lacks.
3. `ConvertToU3DB` (PC:395-439): base DB
   `DataBase/DB_Base/DB_Base_G4A.<U3DBVersion>.db` copied to
   `U3DB_Public.db`, `convertPublic`; copied to `U3DB_Member.db`,
   `convertPrivate`. Converter class chosen by DB version
   (`PPSU4ParamConverterFactory.cs`). Result: the pager-format `u3db.db`.
   Both PPS and base DBs are **SQLite 2.1** files.
4. Tone map: custom alert tones become `G4-Tone-<id>` (ids from 100) and
   channel voice prompts `G4-ChVoice-<nnn>`
   (`PPSU4ParamConverter_02P01.cs:521-539`).

### Files sent, in this order (`ProgramFile`, PC:441-526)

| # | PC file | Pager path |
|---|---|---|
| 1 | converted profile copy `<guid>.db` (PPS schema) | `/defpara/Applications/PPS/u3db_PC.db` |
| 2 | `U3DB_Member.db` (pager schema) | `/defpara/Applications/PPS/u3db.db` |
| 3 | MD5 of #2 | `/defpara/Applications/PPS/u3db.db.md5` |
| 4 | zip (below) | `/defpara/Applications/PPS/u3db_PC.zip` |
| 5 | MD5 of #4 | `/defpara/Applications/PPS/u3db_PC.zip.md5` |
| 6..n | each `<resource>/*.wav` encoded to `.dat`, only if its base name is in the tone map | `/defpara/Applications/chvoicerecord/<G4-Tone-NNN or G4-ChVoice-NNN>.dat` |
| last | `<resource>/customer.png` if present | `/defpara/Applications/homescreen/customer.png` |

Then UpdateDateTime and StopBusiness `00 20 01 [00]` (see ProgrammingFiles
above). All in one write business.

- MD5 sidecar: 32 **uppercase** hex chars, no newline, no BOM
  (`DataCheck.MD5File`/`ByteArrayToHexString`,
  `communication/Unication.PPS.U3.CommunicationLayer/DataCheck.cs:92-117,163-174`;
  `File.WriteAllText`). The data file is always written before its `.md5`.
- Zip (`getProfileZip`, PC:534-556, DotNetZip, deflate, parallel deflate
  off): `<guid>.db` at the root plus directory `<guid>/` containing the
  profile resource folder (wav/dat tones, `customer.png`, P25 trunking
  system DBs `<id>.db`). This is what Read Profile pulls back.
- `.dat` audio (`View/Global/G729Audio.cs:48-102`): the whole `.wav` file
  (header included) is chopped into 160-byte frames (80 x 16-bit samples,
  zero padded) and run through `CodecG729ab.dll` with VAD off; output is the
  raw concatenation of 10-byte G.729A frames, no header. Implies 8 kHz
  16-bit mono source wavs.
- Legacy constants `U3_*_OLD` (`/root/Applications/u3db/...`) are used only
  by the read fallback and Super Password.

### What makes the pager apply it

Nothing explicit. No reboot (`RebootDevice` has no callers), no
ExecuteCommand. The only signals are the business stop `00 20 01 [00]` and
the md5 sidecars. Presumably the pager validates `u3db.db` against
`u3db.db.md5` on commit (or on next boot) and reloads. Needs a live capture
to confirm whether the pager reboots itself.

## 4b. Read profile from pager (for completeness)

Entry: `View/View.Profile/ProfileListUI.cs:312` then `RPD.readDevice()`
(RPD:96-147). Requires a model number; then access password read check
(`01 00 30`).

`ReadProfileFromDevice` (RPD:154-260):

1. READPATH `/defpara/Applications/PPS/`; if it lists `u3db_PC.zip`, use
   that. Else READPATH `/root/Applications/u3db/`; zip there, else fall
   back to READFILE `/root/Applications/u3db/u3db_PC.db`.
2. READFILE the chosen file (one read business), unzip into
   `Temp\DeviceData\`.
3. If handshake `U3DBVersion` is empty: READFILE
   `/opt/Qtopia/etc/default/Settings/version.conf`, take the line starting
   `Version` and map `G4B64BF-AP-V1.01` to `DBV1.00`, `V0.9-0.14A_T11C` to
   `DBV1.01`, `V0.9-0.14A_T11F` to `DBV1.02`
   (`CommonFunction.FetchDBVersionFromDevice`, CommonFunction.cs:1083-1116).
4. If the DB version is supported: READPATH `/root/Applications/u3db/` and,
   if listed, READFILE `/root/Applications/u3db/u3db.db` (old path only;
   `RestoreU3DBToTemp`, RPD:262-286) to merge on-device changes back.
5. Local: schema upgrade (`DBUpdateChecker`), model fix-ups, import as a
   profile.

## 5. Firmware / software upgrade

None in this build. Searched View/, communication/ and all assemblies:
no firmware, `.bin`, `.img` or flashing code. What exists:

- `View/View.UpdateVersion/*`: local migrations of PPS profile DB schemas.
- `STARTUPDATEOPCODE 01 00 04` + `UpdateDatabase` (U3D:900-1045): legacy U3
  "web database" push (MissingCar/MissingPeople/Suspect/Map DBs and
  `.dat`/`.idx` images under `/root/Applications/DatabaseSearch/` and
  `/root/Applications/image/gps/src/`, version check via `*.ver` files with
  `Version:` lines, deletes stale images with `01 01 04`). Unreachable.
- RadioUSB.dll prints "in bootloader" when its control request sees status
  `0x88` (section 7): the legacy USB transport could detect a bootloader,
  but nothing here flashes.
- M5/M6 "Clone to Gx Pager" only imports `.DAT` profile files locally.

Risk surface for our app: WRITEFILE and DELETEFILE take arbitrary absolute
paths (PPS itself writes under `/defpara/Applications/`, `/root/Applications/u3db/`),
so a path bug can overwrite or delete system files on a Qtopia/Linux
device. Whitelist destination paths.

## 6. Safety checklist for our app (derived from PPS)

1. Handshake first; refuse if `ModelNO` is null/`<None>` or not a valid
   model code.
2. Model must be in the enabled model list (product, hardware, protocol,
   software, both bands, language).
3. `U3DBVersion` must be one we have a converter and `DB_Base_G4A.<ver>.db`
   for. Use exact match, not substring.
4. Profile-to-device: same product code and RF bands; no Tow Truck
   cross-programming; strip protocols the device lacks.
5. Access password write check before any write business.
6. Write order: data file, then its `.md5` (uppercase hex); everything in
   one `01 00 02 [01]` business; on any failure send `00 20 01 [01]`
   (PPS skips this on a false return from WriteDevice; we should not).
7. Integrity: per-packet CRC16 (managed code) plus MD5 sidecars. No
   CheckCRC.dll involvement.
8. Only write to the seven known destination paths.
9. Never call reboot/delete unless proven necessary.

## 7. Native DLLs

REA (`rea-agents` 6.5.0) results: `inspect-artifact` only classifies both
as PE with no children; `inspect-binary-layout` is ELF/Linux-x64 only
(`capability_unavailable`); `analyze` and `execute-rizin-command` need
Ghidra/Hopper/IDA/rizin, none installed. Exports, imports and code below are
from static `objdump -p/-d` (Xcode llvm-objdump). Raw outputs:
`../scratch/{CheckCRC,RadioUSB}.artifact.json`,
`../scratch/RadioUSB.asm`.

### CheckCRC.dll (sha256 `63720568...541e2a`, PE32 i386, MinGW, 2013)

Imports: KERNEL32 (atom functions, MinGW runtime), msvcrt, USER32
`MessageBoxA`. Exports:

| Export | Behavior |
|---|---|
| `HelloWorld()` | MessageBox "Hello World from DLL!" (test stub) |
| `int LSB_CRC(int mode, u8 *data, int len, u8 *crc)` cdecl | mode 1: compute CRC into `crc[0..1]`; mode 2: compare against `crc[0..1]`. Returns 0 ok, -1 bad args/mismatch. |

Internal `CRC_CCITT_LSB`: init 0, per byte 8 rounds of
`crc = ((crc ^ b) & 1) ? (crc >> 1) ^ 0x8408 : crc >> 1; b >>= 1`, no final
XOR, output **high byte first**. That is CRC-16/KERMIT parameters
(reflected 0x1021) with big-endian byte order, bit-for-bit the same as the
managed `DataCheck.CRC_CCITT_LSB`
(`communication/Unication.PPS.U3.CommunicationLayer/DataCheck.cs:58-73`).
No `DllImport("CheckCRC.dll")` exists in any assembly: unused. Do not port.

### RadioUSB.dll (internal name `U3_USB_transport_layer.dll`, sha256 `dd683408...2285d3`)

PE32 i386, MinGW. Imports KERNEL32, msvcrt, setupapi
(`SetupDiGetClassDevsA`, `SetupDiEnumDeviceInterfaces`, ...); loads
`winusb.dll` dynamically and re-exports `WinUsb_*` pointers. Exports:
`USB_initial`, `USB_release`, `USB_read_sync/async`,
`USB_write_sync/async`, `USB_get_overlapped_result`, `USB_EP0_sync`,
`request_service_start/stop`, `dll_loaded_flag`, `interface_private_datas`.

`request_service_start/stop(service)`: vendor control OUT
`bmRequestType 0x40, bRequest 1 (start) / 0 (stop), wValue = service,
wIndex = GetTickCount() & 0xFFFF, wLength 0`; then up to 10 polls of
control IN `0xC0, bRequest 0, wLength 8`. Reply byte 1 `0x88` = "in
bootloader"; `0xFF` = busy (sleep 200 ms); byte 0 `0x40` with bytes 4..5
echoing the nonce and bytes 2..3 zero = success.

Who calls it: only `USBDevice` (`communication/Unication.PPS.U3.CommunicationLayer/USBDevice.cs:180-197`,
`USB_initial(2)` then stop/start service on interface 2), which is only
constructed by `U3DeviceList.InitializeUSBDevices` for paths matching
`vid_17ff&pid_0002&mi_01` (U3DL:514-553), reached only from
`InitializeDevices()`, which nothing calls. The G-series pager is
`17FF:A4A2` RNDIS and goes through `NetWorkDevice`/TCP. RadioUSB.dll is
dead for G-series; it belongs to the older U3 (PID 0002) WinUSB transport.

## Open questions (need a pager or capture)

- Does the pager reboot or reload on its own after `00 20 01 [00]`? How long
  until it answers ping/handshake again?
- READPATH on a missing directory, READFILE on a missing file: Abort,
  error rc, or empty data?
- Does the listing include directories or only files?
- What do access rc 0 vs 1 mean (no password vs correct password)? Does the
  pager lock itself after N wrong hashes?
- Does the pager require `u3db_PC.db`/zip at all, or only `u3db.db` + md5?
- Does Super Password conf take effect without a business bracket?
- How does the host get 10.10.0.10 now that PPS never runs netsh (DHCP from
  the gadget?).
