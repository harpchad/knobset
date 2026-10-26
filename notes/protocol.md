# G-series pager wire protocol (from PPS V00-04-00-48)

Source: ILSpy decompile of `communication.dll` and `PPSLog.dll` (DataConvert),
cross-checked with `rea inspect-managed-native-boundaries` (protocol code is
pure managed; native imports are only Win32 USB enumeration, codecs, SQLite).

## Physical / link layer

- USB device VID `17FF` PID `A4A2`, Linux USB gadget exposing RNDIS
  (`tether_driver.inf`, includes the inbox `netrndis.inf`).
- Pager-side RNDIS MAC `BE:6E:3B:3E:6F:6B` (PPS finds the NIC by this MAC,
  `USBDetect.InitDeviceNetworkIP`).
- `USBDetect.InitDeviceNetworkIP` would force the host NIC to static
  `10.10.0.10/255.255.0.0` via `netsh`, but it has no callers in this build;
  host addressing comes from pager DHCP, the driver, or the user. Pager IP default `10.10.0.12` (PpsSetting.db `TabUSBSetting`);
  an Ethernet profile row also exists with `192.168.17.12` (unused in UI:
  only the USB conn type is offered).
- Liveness: ICMP ping to the pager every 3 s, 120 ms timeout
  (`U3DeviceList.ConnectedNetDevices`, `PingDevice`).

## Transport

- Plain TCP, client → pager port **32769** (`DeviceConnectionSetting.m_port`).
- No stream prefix on the TCP path. (`SocketComm` has a legacy `FE FE FE FE`
  leading-code async parser but `NetWorkDevice` never starts it.)
- One TCP connection is opened at handshake and reused until ByeBye.
  No heartbeat actually runs (`StartHeartbeat` is empty, the BEAT/ALIVE
  listener is never started); liveness is ICMP ping only.
- Observed (pager B, DBV1.41c, 2026-10-20): the handshake makes the pager
  show "PC Connected" and pause scanning. A TCP connect followed by an
  immediate close, with nothing sent, does neither, so Knobset polls with
  that and handshakes only when a pager first appears.
- Receive: read 12-byte header, take payload length from bytes 6..7, then
  read `len + 2` more bytes. Socket receive timeout 8 s; per-read cap 5 s.

## Packet header (12 bytes)

| Off | Size | Field | Notes |
|---|---|---|---|
| 0 | 1 | version/flags | low nibble `0x8` = CRC present, high nibble = protocol version (0) → `0x08` |
| 1 | 3 | opcode | see table |
| 4 | 2 | seq | big-endian; starts at 1, wraps 65535 → 1 |
| 6 | 2 | length | big-endian; payload length **excluding** the trailing CRC (0 if no payload) |
| 8 | 2 | ackSeq | big-endian; seq of the last packet received from the peer |
| 10 | 1 | type/state | high nibble = packet type, low nibble = state/error |
| 11 | 1 | header checksum | `(sum(bytes 0..10) & 0xFF) ^ 0xFF` |

Packet types: 0 Inform, 1 Request, 2 Response, 3 Accepted, 4 ACK, 5 NAK.

## Payload ("content")

```
[sessionID: 2 bytes BE, only when HasSection] [command params] [data] [CRC16: 2 bytes BE]
```

- CRC: reflected CCITT, poly `0x8408`, init `0x0000`, no final XOR
  (= CRC-16/KERMIT), computed over everything before the CRC, **stored
  high byte first**.
- Session ID: global counter from `Protocol.CreateSequenceID()` (same
  1..65535 scheme as seq), new per Read/Write/Command. The peer must echo it
  in its payloads (`ContentCheck` rejects mismatches).
- `ParsePacket` strips header, session ID (if any) and CRC.

## Opcodes

| Opcode | Name | Use |
|---|---|---|
| `00 00 00` | handshake | identify (no session) |
| `00 00 02` | command | treated as Command on receive |
| `00 00 04` | BEAT | heartbeat |
| `00 00 05` | ALIVE | pager announces itself; PPS re-handshakes |
| `00 20 00` | Continue | data chunk |
| `00 20 01` | Complete | end of transfer / end business |
| `00 20 02` | Cancel | |
| `00 20 03` | Abort | payload[0] = error |
| `00 20 04` | Password | |
| `00 FF FF` | ByeBye | disconnect |
| `01 00 01` | start read business | |
| `01 00 02` | start write business | param `01` |
| `01 00 03` | start backup | |
| `01 00 04` | start update | |
| `01 00 08` | start key programming | |
| `01 00 20` | forest-fire zone write | |
| `01 00 30` | access password (read) | payload: 4-byte BE length + password bytes; reply 1 byte |
| `01 00 31` | access password (write) | same layout |
| `01 01 01` | read path (dir listing) | |
| `01 01 02` | write file | |
| `01 01 03` | read file | |
| `01 01 04` | delete file | |
| `01 02 01` | set date/time | no session; payload `seq(2) zoneLen(1) "GMT±hh:mm:ss" "yyyyMMddHHmmss"` |
| `01 02 02` | reboot | no session; payload int16 msec, big-endian |

File-op payload layouts: see `operations.md`.

## Exchanges

### Handshake (`HighDeviceProtocol.HandsShake(product)`)

Request, no session, opcode `00 00 00`, type Request, content:
`[product=0x03][edition=0x01][busType=0x05 (BT_U4; enum BT_U3=3, ConsoleDemo=4, U4=5)][protocolMode BE = FF FF FF FF]` + CRC.
Full request content: `03 01 05 FF FF FF FF` + CRC.
Timeout 2000 ms, 1 try.

Response payload: `[b0][b1][b2][b3][deviceInfo...]` →
`HandsShakeInfo(b0,b1,b2,0, payload[4..])` then `AnalyzeHightDeviceInfo`
(field layout in `operations.md`).

### Request / response pattern (`StartCommand`)

1. Host sends Request (type 1) with `[session][params]`.
2. Pager answers one of:
   - **Accepted** (type 3): payload = seconds to wait (BE int); host then
     waits that long for the Response.
   - **ACK** (type 4): host waits for the following Response.
   - **Response** (type 2): payload = result.
   - Abort / Cancel opcodes end the exchange.
3. Host ACKs every good non-ACK packet: header with same opcode, seq `00 00`,
   ackSeq = peer seq, type 4. On checksum/CRC error it NAKs (type 5, state =
   error code 1 or 2).
4. Retries: 3, interval 8000 ms; request is resent on timeout.

### Read (`Read(cmd)`)

Response payload: `[deviceStatus(1)][totalLen BE]`. Status 0 normal,
4 locked, 255 unknown. Then the pager streams **Inform** packets with opcode
`00 20 00` (each `[session][chunk]`), host ACKs each, until Inform `00 20 01`
Complete. Total bytes must equal `totalLen`.

### Write (`Write(cmd, data)`)

Response payload `[deviceStatus]`. Host sends data in ≤2048-byte chunks as
Inform `00 20 00` `[session][chunk]`, waits for ACK each, then Inform
`00 20 01` with one data byte `00`, waits for ACK.

### Sequence validation on receive

- Header checksum must match.
- For ACK/NAK the decompiled check is effectively always true (bug in
  `SequenceID.Equal`), so ACK seq is not really enforced.
- Otherwise peer seq must be greater than last ackSeq (wrap 65535 → 0/1 ok);
  equal = duplicate (re-ACK and ignore).

## Device error codes (Error enum)

0 ok, 1 checksum, 2 CRC, 3 no data, 4 NAK, 249 password (packet[1] = tries
left), 250 status, 251 cancel, 252 abort, 253 duplicate ack seq, 254 chaos,
255 session ID mismatch.

## Verified on hardware (2026-10-17, macOS 26, one G5)

- Device descriptor: class 2/2/1 (CDC), `bcdDevice 0x0222`, **2
  configurations**, product string "Unication RNDIS USB net".
- Configuration 1 is **CDC-ECM** (interface 0 class 2/6, interface 1 class
  10). macOS binds its built-in `AppleUserECM` dext and creates `en17`, so no
  RNDIS driver is needed on macOS. Configuration 2 is presumably RNDIS (for
  Windows).
- Host-side MAC is `be:6e:3b:3e:6f:6b`, the MAC PPS looks for. The pager-side
  MAC is per unit.
- The pager runs a **DHCP server** at `10.10.0.12`: the host got
  `10.10.0.6/16`, lease 0xd2f00 s (~10 days). No static IP setup needed.
- ICMP ping to `10.10.0.12` answers (~1 ms); TCP `10.10.0.12:32769` accepts
  connections.

### Wire behavior (M0 probe, `g5 info` / `ls` / `backup`)

Test unit: `G5D66BE-SXVXENB702UNI`, firmware V1.33-08182022, radio DB
**DBV1.41b**, protocol version 1, protocol mode `0x00010517`.

- Pager packets carry flags `0x18` (version 1, CRC). Ours use `0x08`; the
  pager accepts them.
- The pager ACKs every Request (seq 0, ackSeq = our seq) before answering.
- Handshake Response comes back with opcode `00 00 02`, not `00 00 00`.
- Other Responses echo the request opcode and session ID; Inform chunks use
  `00 20 00` / `00 20 01` with the same session ID.
- Read-access probe with an empty password returns **1** (granted) on this
  unit.
- The handshake "password" field holds the pager's access password in
  **plaintext** (six digits here). Never display or log it.
- Band bits `00 00` and an empty manufacture date on this unit; the factory
  parameter is a 1-byte field holding only NUL.
- Full backup (14 files, ~2.3 MB) takes about 8.5 s.

Second unit: `G5B67BF-SXVXEN1401UNI`, firmware **V2.02** (internal
V1.35-T01G-01252024), radio DB **DBV1.41c**, hardware dated 2016, same
protocol mode `0x00010517`. So the latest public firmware still uses the 1.41
radio DB layout; DBV2.0x is not tied to firmware V2.x. Same file set plus an
empty `map.db`; live `u3db.db` identical to the uploaded copy.

Files found:

| Path | Notes |
|---|---|
| `/defpara/Applications/PPS/u3db_PC.zip` (+ `.md5`) | PPS profile zip: `<GUID>.db` + `<GUID>/` with 9 P25 system DBs, AlertToneTotal.db, VoicePromptTotal.db |
| `/defpara/Applications/PPS/u3db_PC.db` | PPS profile DB, SQLite 2.1 |
| `/defpara/Applications/PPS/u3db.db` (+ `.md5`) | radio DB as uploaded by PPS |
| `/defpara/Applications/PPS/SuperPassword.conf`, `Readme` | `Readme` says "for PPS download u3db.db" |
| `/root/Applications/u3db/u3db.db` | **live** radio DB; diverges from the uploaded copy when settings change on the pager (here: `tabBackLightModeSetting`) |
| `/root/Applications/u3db/{battery,business,morse,voicepager_battery}.db` | SQLite 2.1, pager-owned |
| `/root/Applications/u3db/keytab` | binary, likely encryption key material: treat as sensitive |

MD5 sidecars are uppercase hex, no newline, and match their files.

### Write path (verified 2026-10-17 on unit B, byte-identical restore, twice)

Sequence, exactly as PPS's `ProgrammingFiles`, all on one TCP connection:

1. `01 00 31` write-access probe with empty password: granted.
2. `01 00 02 [01]` start write business: rc 0.
3. Per file, `01 01 02` with params `[fileLen u32][pathLen u32][path][00]`;
   Response `[status 0]`; then Inform `00 20 00` chunks of 2048 bytes, each
   ACKed by the pager (ACK carries ackSeq = our chunk seq); then Inform
   `00 20 01` `[00]`, ACKed.
4. `01 02 01` set clock (no session): payload `[seq u16][zoneLen][GMT+05:00:00][yyyyMMddHHmmss]`;
   pager ACKs then sends a Response.
5. `00 20 01 [00]` commit: ACK within 30 ms, **Response (rc 0) after ~2.2 s**.

Timing: 5 files / 1.3 MB in ~5.2 s total.

After the commit the pager applies the profile in the background: about
1 to 3 s after the commit Response it stops answering mid-transfer and later
resets the TCP connection. A fresh connection 10 s after the commit works
(handshake, access, reads). The read-back of every file matched, and the live
`/root/Applications/u3db/u3db.db` was unchanged (identical data). The link
(ECM interface, DHCP lease) stays up; it is a service restart, not a USB
re-enumeration.

### What the pager shows (observed by the user during the generated-DB upload)

1. "Exporting data ..." then "Data exported successfully" (brief): our
   pre-write safety backup / read business.
2. "Programming..." during the write business.
3. "Programmed successfully" at commit.
4. The pager **reboots**. This is why the TCP session drops a few seconds
   after the commit Response. The USB network link and DHCP address were
   back by the time we reconnected (10 s).

### First real edit (verified 2026-10-17, unit B)

`g5 restore --rename-knob "Metro" 7 "G5 TEST"` edited the profile
working copy, regenerated the radio DB (changes only in `tabChannel` and
`tabReceivingMode`), repacked `u3db_PC.zip`, and programmed all five files.
After the reboot the pager's live DB had the new name and the user
confirmed **"G5 TEST" on the pager display** at that knob. The edit was
kept on the unit.

### First talkgroup-list edit (verified 2026-10-18, unit B, from the app)

Using Knobset on a second Mac, the user added one talkgroup to a TG-Scan
knob as priority #2. The profile row was written with SeqNo 2; after the
write the pager's live `tabRecvGroupList` for that knob listed the new
TGID at SeqNo 2 alongside the existing priority #1 and non-priority
entries. The live radio DB matched the upload in all 194 tables and both
MD5 sidecars verified. Only the contact tables changed, as the write guard
requires.

## Still open


### Uploading a generated radio DB (verified 2026-10-17, unit B)

`g5 restore --regenerate` uploaded a `u3db.db` built by our converter:
logically identical to PPS's (194/194 tables) but 3,335 bytes different in
page layout, with a fresh MD5. The pager accepted it, and afterwards the
live `/root/Applications/u3db/u3db.db` was our file copied into place: it
differs from the upload in only 2 bytes on one page (the pager writing its
runtime state after load) and matches it in every table. So on commit the
pager installs the uploaded `u3db.db` as its live DB.

- Linux: which driver binds (cdc_ether for config 1 expected).
- Windows: Windows has no inbox CDC-ECM driver, which is presumably why
  Unication ships an RNDIS INF; confirm how config 2 gets selected.
