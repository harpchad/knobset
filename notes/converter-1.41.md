# The PPS -> u3db converter for DBV1.41a-e, validated on two real pagers

Companion to [data-model.md](data-model.md) section 2.4 (DBV2.06). Both test
pagers report a 1.41 radio DB, so this is the converter that actually matters
for us. Paths in `file:line` references are relative to `decomp/` unless
stated otherwise; the converter files live in `View/View.ProgramVersion/`.

Units are called **unit A (1.41b)** (G5D66BE, firmware V1.33) and **unit B
(1.41c)** (G5B67BF, firmware V2.02). No serials, passwords, unit IDs or
agency names appear below.

Working files: `scratch/conv141/` (see section 4.1 for the tools).

## 0. Summary

- `DBV1.41b` -> `PPSU4ParamConverter_01P41b`, `DBV1.41c` ->
  `PPSU4ParamConverter_01P41c`, and so on through 1.41e
  (`PPSU4ParamConverterFactory.cs:9-23`). Knobset implements b through e;
  section 4.3 has the 1.41e validation. 1.41a
  and 1.41b are two separate copies of the same converter, both deriving
  straight from `PPSU4ParamConverterBase`; c, d and e form a chain on top of b.
- The 1.41 radio DB is the old 194-table layout. A "channel" is still a
  `ZoneKnobID = zone * 100 + knob`, but the data is spread over a dozen
  per-channel tables (`tabChannel`, `tabChannelAdvInfo`,
  `tabFunctionOnOffOfChan`, ...), sites and control channels are two separate
  tables, the system name sits in `tabSystemAlias`, and contacts use 99999
  (self), 65535 (all-call) and 10000+ (groups).
- **I re-implemented the 1.41b/1.41c converter in Python and ran it on both
  real profiles. It reproduces every row of every table in both uploaded
  `u3db.db` files, in the same rowid order: 69 tables written with data, 17
  tables written with values equal to the template, 108 tables left at the
  template. Zero mismatches.** So the code analysis below is confirmed, and
  both pagers were programmed by a PPS whose 1.41b/1.41c converter and menu
  reference data are identical to V00-04-00-48.
- The P25 trunking path is simpler than in 2.06: one `tabP25TrunkingSignaling`
  row per distinct (system, protocol setting), control channels de-duplicated
  per system by frequency, sites de-duplicated by (RFSS, site), talkgroups as
  contacts with address `TGID + 2^31`, receive lists in `tabRecvGroupList`
  keyed by the system SigID.
- Tooling bug found on the way: `scratch/dump_sqlite2.sh` (and so every
  `scratch/db3/*.sqlite3` copy) silently turns numeric-looking strings into
  numbers. `'0000000000000000'` (the CSQ protocol ID) becomes `'0'`. Section
  4.1 has a lossless loader.

## 1. Classes, inheritance and version deltas

```
PPSU4ParamConverterBase (abstract)
 |- PPSU4ParamConverter_01P41a          (DBV1.41a, standalone copy, 4847 lines)
 |- PPSU4ParamConverter_01P41b          (DBV1.41b, standalone copy, 4899 lines)
     |- PPSU4ParamConverter_01P41c      (DBV1.41c, 1274 lines of overrides)
         |- PPSU4ParamConverter_01P41d  (DBV1.41d, one override)
             |- PPSU4ParamConverter_01P41e (DBV1.41e, ID constants only)
```

The 2.0x chain (`_02P01` ...) is separate and also derives from the base.

Contact ID constants (base `PPSU4ParamConverterBase.cs:3022-3055`, overrides
noted):

| Constant | Base (a, c, d) | 1.41b | 1.41e |
|---|---|---|---|
| `getMyContactId` (pager itself) | 99999 | 99999 | 9999999 |
| `getAllCallContactId` | 65535 | 65535 | 9999998 |
| `getFirstGroupContactId` | 1 | **10000** (`01P41b.cs:4585`) | 1 |
| `getFirstSubGroupContactId` | 100001 | (unused) | 10000000 |
| `getFirstSUIDContactId` (members) | 1 | 1 | 9000000 |
| `isGroupContactIDContainSGContactID` | true | true | false |
| `isMemberContactIDSkipMyContactID` | false | false | true |

1.41c and 1.41d inherit 10000 from 1.41b.

What each version changes:

| Version | Template change (data-model 6.2) | Converter change |
|---|---|---|
| 1.41a | (identical to b) | Hard-coded 65535 / 99999 / `m_groupMap.Count + 10000` / `m_memberMap.Count + 1`. Output equals 1.41b for P25T and CSQ knobs. Differences: P25C conventional `SquelchOption` passes the profile value only in selective mode (b maps 3 -> 0 in selective and 0/2 -> 3 in monitor); P25C sub-group receive rows are written even when `Alt = 0`, and sub-group receive rows get SeqNo 0 instead of the parent TG's SeqNo. |
| 1.41b | | Introduces the ID virtuals above, `convertMultiCHScan_tabFunctionOnOffOfChan`, `GetVoiceBufferingForceDrop` (returns 0). |
| 1.41c | `tabFunctionOnOffOfChan.ForceDrop`, `tabFunctionsOnOff.SignalLevel`, `tabFunctionsParam.StunChID`, `tabKMFSystem.Proxy_BT_MAC`, OTAR packet-data columns | `tabFunctionOnOffOfChan` gets an 18th value, `ForceDrop = VoiceBufferingForceDrop` when voice buffering is on (`01P41c.cs:603-642,817-820`); `tabHomeSettings` always updated and `tabFunctionsOnOff.SignalLevel = tabHomeSettings.StatusBarRssiShowType` (`:371-409`); `tabKMFSystem` 7 columns with `OTARBtMac`, one 18-column `tabP25OTARFeatureControl` row per KMF instead of per channel, `tabP25OTARFeatureControlByChannel.(channel, KMF index)` (`:19-185`); `convertRecvGroupList` no longer returns early when 2-tone sub-group binding is off, so revert-list pairs are written for any knob with revert groups (`:822-1274`, only that one `if` differs). |
| 1.41d | 2-tone detector thresholds in `tabTwoToneSignaling` | `convertTwoToneSignaling` writes `NSig/WSigPwrOn/OffThreshold` via `getThresholdOfToneDetector` (`01P41d.cs:13-50`). |
| 1.41e | (identical to d) | Only the ID constants (`01P41e.cs`). Group IDs run 1.. without per-knob resets, sub-groups from 10000000, members from 9000000. |

`CheckProfile` and `ChangeProfile` are no-ops in 1.41b (`01P41b.cs:85-92`).
`updateDataToPPS` (`:4590`) exists but nothing calls it.

## 2. Algorithm

### 2.1 Driver

`ProgramController.ProgramData` (`ProgramController.cs:205-260`) copies the
profile and its folder to `Temp\Convert\`, runs `ChangeDB` (model remap,
data-model 2.2; a no-op when the profile model equals the pager model, as for
both units), then `ConvertToU3DB` (`:395-439`):

1. Template = `DataBase\DB_Base\DB_Base_G4A.<version reported by pager>.db`,
   copied to `U3DB_Public.db`.
2. `init()` (`01P41b.cs:94-109`): zone map and channel map only (the base's
   group/member/P25T maps are not used in 1.41).
3. `convertPublic` on the copy, then copy to `U3DB_Member.db` and
   `convertPrivate` on that. `U3DB_Member.db` is what gets uploaded as
   `u3db.db`.

All SQL goes through `SQLiteDataBase.ExeNonQuery(List)` (5000 statements per
`begin; ... commit;`). Any failing statement throws and aborts programming, so
PPS output never contains a partial conversion.

**Nothing is cleared from the template except** `tabAccessPassword`,
`tabPINPasswordSetting`, `tabKVLUSBConnect`, `tabDispathCenterAck`,
`tabBleParamSetting` (DELETE then re-insert) and the template zones 1025
(free scan) / 1100 (channel setting) when functions 141 / 161 are off. All
other converter output is INSERTs into template tables that ship empty, or
UPDATEs of single-row settings tables.

### 2.2 Order of operations

`convertPublic` (`01P41b.cs:568-634`):

1. `convertZone`: zones, receiving modes, zone channel list, announcements.
2. `convertContactsAndAddress`: conventional groups (via groupings), P25C
   sub-groups, members including the pager itself, then per P25T knob: the
   system's individual IDs, its talkgroups selected on that knob, its
   sub-groups; finally the all-call rows.
3. `convertSelectiveCall`: single conventional knobs (channel bundle,
   signaling, receive list).
4. `convertMultiProtocolCH`: advanced (radio-compatible) conventional knobs.
5. `convertP25Trunking`: per P25T knob: system signaling (first time only),
   channel bundle + `tabP25TSetting`, receive list, call alert.
6. `convertMultiCHScan`, 7. `convertAllBandScan`.
8. `convertG4Function`: `tabDispatchChannel`.
9. `convertFunction`: device settings (section 3.6).
10. `convertDetailScreenItem`, 11. `convertDispatchCenterManualAck`,
12. `convertCustomAlertToneAndChannelAnnouncement`, 13. `convertEncrytionKeySetting`.

`convertPrivate` (`01P41b.cs:177-298`): `tabMyContact`, `RadioNo`, menus and
function lists, debug entries, zone 1025/1100 deletes, the pager's own
`tabRecvBehaviorByContactID` row, BLE settings.

Order matters for IDs: contact IDs depend on knob order, and the P25T SigID
depends on the order of `tabP25TrunkingSystemKnob` rows. Rows are read with
`select * from <table>` (rowid order) and filtered with `DataTable.Select`,
which keeps that order.

### 2.3 ID schemes

| Radio key | Rule (1.41a-d) | Evidence |
|---|---|---|
| zone ID | 1..N in `tabZone_New` row order (`ZoneNo` ignored) | `01P41b.cs:152-175` |
| `ZoneKnobID` (channel ID everywhere) | `zoneIndex * 100 + KnobIndex`; first knob table wins if a ChannelID repeats | `:111-150` |
| conventional SigID | = ZoneKnobID | `:2316` |
| P25T SigID | 1..N per distinct `(P25TrunkingID, ProtocolSettingID)` in knob order | `:3198-3204` |
| group contacts | from 10000. Conventional groups first (first-seen over groupings), then P25C sub-groups, then per P25T knob its TGs (key `sysGUID_GroupID`, so a TG used on two knobs of one system is one contact) and sub-groups. Before each P25T knob the counter is reset to `10000 + groups so far`, which makes the sequence contiguous. | `:771-950` |
| member contacts | counter from 1; the pager's own member row takes 99999 **and** consumes 1, so the first real member is 2. Conventional members, then each P25T system's individual IDs when the system is first met | `:1260-1373` |
| pager | 99999; all-call 65535 | base `:3022-3030` |
| custom alert tone | `100 + n` (n in first-use order); built-in IDs pass through raw (9999 default, 10000 chirp, 1..8) | `:654-673` |
| knob announcement | built-in 1..20 raw, custom `32 + n` | `:675-695` |
| encryption key | `tabEncryptionKeyList` 1..N; channel-key `AssignmentID = (zone-1)*16 + (knob-1)` | `:4285-4296,4453` |

Note how 1.41 differs from 2.06 on tones: 2.06 remaps built-ins (9999 -> 1,
1..8 -> 3..10, announcements 1..20 -> 50..69, custom announcements 164+);
1.41 does not.

### 2.4 Merging the per-system P25 DBs

The converter opens `<profile folder>\<P25TrunkingID>.db` (`base :86-90`)
only for systems referenced by a `tabP25TrunkingSystemKnob` row. The
`tabP25TrunkingSecurity` index is not consulted. System DBs in the folder that
no knob references are ignored (both units carry two such orphans). From each
system DB it reads:

- `tabP25TrunkingSystem` row 0: `WacnID`, `SystemID`, `SiteTrunkingFlag`,
  `Name`, full-spectrum scan fields. `SUID`, `ConverageType` and
  `ErrorRateThreshold` are ignored.
- `tabP25TrunkingControlCH`: `RxFreq` only, de-duplicated over the whole
  system in row order.
- `tabP25TrunkingSite`: `RFSSID`, `SiteID`, `Name`, de-duplicated on
  (RFSS, site).
- `tabGroup_New` / `tabSubGroup`: only the rows selected in the profile's
  `tabChannelGroup` / `tabChannelRevertGroup` / sub-group tables for some
  knob of that system.
- `tabMember_New`: every row with a non-null, non-`'0'` `P25TAddr`
  (individual IDs become contacts even if nothing uses them).

`tabP25TrunkingSiteControlCH` (site-to-CC links), CC bandwidth and Tx,
`tabEditPermission`, `tabSUID`, `tabRFBandTypeInfo` and `tabVersion` are
dropped. Protocol timers come from the **profile's** `tabP25TSignaling` row
named by the knob's `ProtocolSettingID`, not from the system DB's copy.

### 2.5 Conversions

- Frequencies stay integer Hz. Conventional `TXFrequency` = `RXFrequency`
  (the profile Tx is discarded). P25T channels get `RXFrequency = 0`.
- Group addresses for DMR (4), P25C (8) and P25T (9): `addr + 2147483648`
  (bit 31). Sub-group addresses: `0_<sub>_<grp>` (unit address type) or
  `<1|2>_<a>_<b>_<grp>` (`01P41b.cs:1235-1258`).
- RFSS ID: decimal if all digits, else parsed as hex, else 0 (`:3259-3274`).
  So an RFSS typed as `10` stays 10, while `0A` becomes 10.
- `AlertMode` everywhere = profile value - 1.
- Audio squelch 0..3 -> 20/40/60/100 % -> `CarrierSquelchSetting =
  255*p/100` (51/102/153/255), `SINADSquelchSetting = 20*p/100` (4/8/12/20)
  (`:2217-2230`).
- Receive mode `GetRecvMode` (base `:92-123`): 1->1, 33->32, 4/36/68->4,
  16->16, 48->64, 8->8, 128/129->128, 65->1, 97->97.
- CTCSS in 0.1 Hz (`ctFreq`, 670 when unused), DCS code word (`cdCodeWord`,
  19 when unused).
- Tone volume: `ToneVolume = (int)((5 - v) * 9.6)`; class volume
  `v == 0 ? 48 : (4 - (v - 1)) * 11` (base `:2612-2625`).
- DSP engineer params: 32 values, low two bytes of each, little-endian, hex
  (`base :2718-2736`).
- P25T full-spectrum ranges: same fill rule as 2.06 (data-model 2.4); with the
  flag off all three ranges are the first Rx range of the model's first band,
  truncated to whole MHz (763000000-776000000 for SX), step 12500.

## 3. Table-by-table mapping

"ZK" = ZoneKnobID. Profile tables are from `<GUID>.db` unless marked
"system DB". All 194 tables of the 1.41 schema are covered: section 3.1-3.7
lists every table the converter can write; section 3.8 lists the rest.

### 3.1 Zones and knobs

| Radio table | Source and rule |
|---|---|
| `tabCurrentZone` | `CurrentZoneID = 1` if any zone exists |
| `tabZone(ID, Name, ModifyByDevice, priorityToneID)` | one row per `tabZone_New` row: ID = zone index, `Name`; `ModifyByDevice = 1` when no knob of the zone has a non-zero receive mode, else 0; `priorityToneID` written 0, then UPDATEd on **all** rows (template 1025/1100 too) from `tabFunctionAlertTone` FuncID 1: 9999 -> 37, 10000 -> 39, 1..32 raw, custom -> ID + 100. Rows 1025/1100 deleted in `convertPrivate` if function 141/161 is disabled |
| `tabReceivingMode` | per knob with `CurrentMode != 0`: `(zone, KnobIndex, SupportedMode = CurrentMode = GetRecvMode, DualCHScanSettingID 0, MultiCHScanSettingID = FreeScanSettingID = ZK, CreateFlag 1, Name = ChannelName, CtCdRequest 1, ColorCodeRequest 1, NACRequest 1)`; scan knobs get `CtCdRequest = MatchCTCD` |
| `tabZoneChannelList` | per active knob except scan modes 4/36/68: `(zone, KnobIndex, ZK, 1)` |
| `tabChannelAnnouncement(ZoneID, ChannelID, FileName)` | per active knob: `ChannelID` holds the **knob index**, `FileName = G4-ChVoice-NNN.dat` with NNN = announcement ID (000 when none) |

### 3.2 Per-channel bundle

Written by `addTabChannelSql` (`01P41b.cs:2523-2704`, 1.41c `:603-785`) for
single conventional, advanced conventional and P25T knobs:

| Radio table | Rule |
|---|---|
| `tabChannel` | `(ZK, ChannelName, isSupportRepeater 0, RX = TX = RxFreq, RxBW = TxBW = RxBandWidth)`; P25T: freq 0, BW 1 |
| `tabChannelAdvInfo` | `ChannelType 0`, `SignallingSquechLogic` 1 for monitor mode (32) else 0, `AutoMute 1`, `SideTone 0`, `AudioSquelchType 1`, squelch pair, `Emphasis`/`Compander` from knob (P25T 0), `MsgRetries 2`, `RFActivityTimeout 500`, `UniLongDataEnable 0`, `ColorCode = RxColorCode`, `NAC = RxNAC` (P25T 0/0), `DefaultCallID` 65535 for CSQ/2-tone knobs and advanced knobs, else 0, `ATGID` (P25T knob value, else 0), TaTx fixed `0,670,1,19,1,0`, `SquelchOption` (P25C only), `PowerSavingRxOffDur` (CSQ/MDC/DVOA 100, 2-tone by FreqSetID 600/600/1600/350/1600/0, 5-tone/DMR/P25C/P25T 0) |
| `tabFunctionOnOffOfChan` | `(ZK, 0,0,0,0,0,0, talkAroundOn = TransmitMode (P25T 0), privacy, pushtolisten, PriorityCall = 1 if P25T, DelayPlayVoiceMsg = VoiceBuffering (0 when push-to-listen), MonitorModeOn, VoiceBufferPrompt, AlwaysUnreadVoiceMessage, AlwaysUnreadReminderAlert, TempRecvOn 0 [, ForceDrop in 1.41c+])` |
| `tabFunctionParamOfChan` | `(ZK, ZK, 3, ZK, 1, ResetModeSetID = ZK, or 0 for P25T)` |
| `tabVoiceCallSettings` | `(ZK, 0, 0, 0)` |
| `tabResetModeSetting` | `(ZK, ResetMode, DelayNTime, EnableCarrierOffTime (P25T 0), CarrierOffDelayTime (P25T 30), EnableTgidStayTime, TgidStayTime)` |
| `tabR2RModeSetting` | `(ZK, 0)` |
| `tabAlertModeSetting` | `(ZK, ZK, 1, AlertMode - 1, 2, 0, 2000, 2000, 2000, 0)`; plus device row `(0, -1, 1, 0, 2, 0, 2000, 2000, 2000, 0)`; afterwards **every** row is UPDATEd with `time`, 2-tone type/durations from `tabRingModeSettingPPS` |
| `tabRecvVoiceCallRecordByChannel` | `(zone, knob, RecordFlag)` |
| `tabRepeaterSettingOfChan` | per non-P25T protocol on the knob: `(ZK, protocol, AccessCodeType, RACID = ZK, 0,670,1,19,1,0,0,670,19,0)`; AccessCodeType CSQ/2-tone 0, MDC/DVOA 3, 5-tone 4, DMR 5, P25C 6 |
| `tabRecvSaList` | up to 5 rows from profile `tabRecvSaList`: `(ZK, SigType = protocol or 255 if SaType 0, SaType, ctFreq, 1, cdCodeWord, 1, cdModSense)`; advanced knobs merge identical tones across protocols |
| `tabDispatchChannel` | ZK of every single conventional and P25T knob whose `DispatchCHFlag = '0'` (literally zero, so every knob in practice) |

Conventional signaling by protocol (`convertSignaling`, `01P41b.cs:2108-2151`,
base `:1595-1866`), SigID = ZK: CSQ writes nothing; 2-tone
`tabTwoToneSignaling`; 5-tone `tabFiveToneSignaling` + `tabFiveToneFreqSets` +
`tab5ToneRepeaterSetting`; MDC `tabMdc1200Signaling` + `tabMDCRepeaterSetting`;
DVOA `tabUdc2400Signaling`; DMR `tabDMRSignaling` + `tabDMRRepeaterSetting`;
P25C `tabP25Signaling(sigid, txpretime, ..., rxnac = knob RxNAC, ...)` +
`tabP25RepeaterSetting(ZK, RxNAC, TxNAC)`.

### 3.3 P25 trunking

| Radio table | Rule |
|---|---|
| `tabP25TrunkingSignaling` | per SigID: `ACKWaitTimeout, RfssRspWaitTimeout, MFID, AlgId` (profile `tabP25TSignaling`), `SystemID, WacnID` (system DB), `WacnSystemIDCheckSkipFlag 0`, `FullSpectrumScanFlag`, `FullSpectrumScanTimer 0`, `FullSpectrumScanRanges` and 3 ranges, `SiteTrunkingFlag` (system DB), `OutOfRangeErrorRate 10` (`01P41b.cs:3187-3236`) |
| `tabSystemAlias` | `(9, SigID, system Name)` |
| `tabP25TrunkingControlChannels` | per distinct `RxFreq`: `(SigID, SeqNo = 256, 255, ..., RxChFreq Hz, RfssID 0, SiteID 0)` |
| `tabP25TrunkingSites` | per distinct (RFSS, site): `(SigID, RfssID, SiteID, SiteName, SeqNo = 256, 255, ...)` |
| `tabP25TSetting` | per P25T knob: `(ZK, MSELCallSamplingFlag, MaxMSELSamplingTime, TwoToneSubGroupOn = Enable2TSubGroup, MinShort/MinLong/MinToneAddrGap, OTAToneFlag 1, ShortA/ShortB/Long Tx durations, TxToneAddrGap, EncryptedCallFlag)` |
| `tabCallAlertGroup` | call-alert SUID x group rows (`:3299-3379`), plus `tabResetModeSetting.CarrierOffDelayTime = CallAlertTimeout` |

### 3.4 Contacts and receive lists

| Radio table | Rule |
|---|---|
| `tabContacts(ID, Name, IDType, Role, IsUnication, Note)` | groups `(cid, name, 2, 2, 0, '')`; members and the pager `(cid, name, 1, 0, 1, '')`; all-call `(65535, 'All', 0, 2, 0, '')` |
| `tabContactAddr` | groups: one row per protocol the group is used with, P25T `TGID + 2^31`; pager: `(99999, 9, tabPagerSUID.P25TAddr or 268435455)` then protocols 1, 2, 4, 5, 7, 8 from its member row (`''` default for 1/2, `'0'` for the rest); P25T individuals `(cid, 9, P25TAddr)` (no bit 31); all-call: 2 `AAAAA`, 4 `2164260863`, 5 `65535`, 7 `65535`, 8 `2147549183` (no P25T all-call) |
| `tabRadioGroup(PrivateId, GroupId)` | `(99999, cid)` per group contact |
| `tabRecvVoiceCallRecordByContactID` | groups: `(cid, RecordFlag)` |
| `tabContactOfRelayCtl` | groups: `(cid, 1, RelayCtrl, RelayOnTime)`; Relay 65535 -> 1/0, 1..65534 -> 1/n, 0 -> 0/0 |
| `tabContactOfBackground` | every contact `(cid, BackgroundColor)` (P25T individuals 0), all-call 0 |
| `tabRecvToneForContacts` | hard-coded rows for id 0 and 65535 (default tone 37); the pager always; groups with a tone: `toneID` 37/39/raw, `sourceID` 100 for 9999/10000 else ID + 100 for IDs below 100, `type` 1 for chirp else 2 |
| `tabContactAddrAndAlertTone` | for groups (and the pager) with a tone: `(cid, SigType, Address, sourceID, FileName)`, `FileName` = `AlertTone<id>.dat` or `G4-Tone-NNN` (custom, no `.dat`) |
| `tabSubgroupWithMultiSubAddr` | `(n, cid)` per sub-group contact |
| `tabMyContact` | `(99999)` (private) |
| `tabContactOfChan(ChannelID, ContactID, SignalingType)` | per knob per receivable contact. CSQ knob: `(ZK, 65535, 0)`. P25T knob: each selected TG `(ZK, cid, 9)` |
| `tabRecvGroupList(ZoneKnobID, SigType, SigID, GroupAddr, GroupId, SeqNo)` | CSQ: `(ZK, 0, ZK, '', 65535, 0)`. P25T: per `tabChannelGroup` row of the knob in row order `(ZK, 9, P25T SigID, TGID + 2^31, cid, SeqNo)` where SeqNo is a per-knob counter 1, 2, ... over rows whose profile `SeqNo > 0`, else 0. De-dup key is (ZK, SigType, SigID, cid); the 2.06 format-string bug is not present |
| `tabRecvBehaviorByContactID` | CSQ: `(ZK, 65535, 0, Duty 1, AlertMode 0, VoiceStorage 2, 0, acks 0)`. P25T TG: `(ZK, cid, Priority = tabChannelGroup.Priority, Duty 0, group AlertMode - 1, VoiceStorage 1 if group RecordFlag > 0 else 2, 9, Voice/Text/ImageRecvAck each copied into Recv/Read/Manual)`. Conventional groups: `Duty = OffDuty > 0 ? 0 : 1`. Private row `(0, 99999, 0, 0, pager AlertMode - 1, 1, -1)` |
| `tabRevertListGroup`, `tabP25TQCDelayRecordParm` | revert groups and 2-tone sub-groups (`01P41b.cs:1786-1987`). `addTabRevertListGroupSql` never records its de-dup key, so duplicates are possible |

Conventional selective-call knobs (anything but CSQ) take their groups from
`tabChannelGroupList` -> `tabGroupingOfGroup` (talkgroup "groupings"); a
non-CSQ conventional knob without a grouping makes `convertRecvGroupList`
return false and **programming fails** (`:1610-1643`).

### 3.5 Scan lists

| Radio table | Rule |
|---|---|
| `tabScanListSettings` | per `tabMultiChannelScan` row: `(ZK, PriorityScanType, SamplingFlag, PriorityZoneKnobID = "a,b" (type 1 only), ScanList = member ZKs comma-joined in tabMultiCHScanList row order (ScanOrder ignored) minus priority channels, RecvAllFlag = OpenSpeaker, StayTime, CallBackTime 0, RxPauseBetweenScanPaths 0, PriorityIntervalTime = DwellTime)` (`:3032-3104`) |
| (per scan knob) | `tabFunctionParamOfChan (ZK, 0, 0, 0, 1, ZK)`, `tabFunctionOnOffOfChan` (talkAround 1, privacy, PTL, unread flags), `tabResetModeSetting (.., 0, 30)`, `tabAlertModeSetting`, `tabRecvVoiceCallRecordByChannel`. No `tabChannel` row |
| `tabAllBandScanSettings`, `tabAllBandScanParam` | free scan knobs (`:3480-3632`): also a `tabChannel` row at the first band's start freq, `tabChannelAdvInfo` with NAC 3966 / ColorCode 255 / `RFActivityTimeout = StayTime`, and `tabRecvGroupList` rows with SigID 65535 per enabled protocol |

### 3.6 Device settings (`convertFunction`, `01P41b.cs:3674-4122`, 1.41c `:371-601`)

| Radio table | Source |
|---|---|
| `tabRadioBaseSetting` | INSERT `(OrgName, 'U3', '000000', '')`, then private UPDATE `RadioNo = pager member UserDeviceID` |
| `tabCallTimeLimit` | `tabVoiceCallSetting` (option 0 -> 4, user-defined limit only for option 3, else 90) |
| `tabCannedMessageMap` | `tabConnedMessage.Content`, IDs 1.. |
| `tabTextMessageSetting`, `tabFunctionsOnOff.showMsgContentOn` | `tabTextMessage` |
| `tabMenuUIType`, `tabTodaySettings`, `tabUnreadAlertSettings` | same-name profile tables |
| `tabHomeSettings` | `isFrequencyVisible`, `HomeStyle = tabHomeScreenSettings.HomeScreenStyle`; hard-coded `P25TSystemShowOption 0`, `P25TSystemName 'MPSCS'`, `NumberFormat 1`, timers 460/470/480/450, `IsRfssSiteVisible 1` |
| `tabUiNumberFormat` | `CallIdShowType`, `SysIdShowType`, `FrequencyUnit 0` |
| `tabDateTimeSetting`, `tabKeypadLockSetting`, `tabBacklightSettings`, `TabImageBoxSetting`, `tabSpecialKeyActionTime`, `tabRecvBackgroundColorSetting` | same-name profile tables |
| `tabOutRangeSetting` | `AlertTimes = Duration / Interval + 1` when repeating |
| `tabRingModeSetting` | `tabRingModeSettingPPS`, `isVoiceStartStopRing 1` |
| `tabReminderAlertSetting` | `tabReminderAlertSettingPPS` |
| `tabMemory`, `tabAudioRecordTimer`, `tabVoiceMemoSettings` | `tabVoiceMemoSetting` (function 310), `tabVoiceStorageSetting.MaxRecordTime` (function 201) |
| `tabDutySetting` | `tabDutyStatusSetting` |
| `tabFunctionsOnOff` | KeyToneOn, backlighton, ReminderAlertOn, UnreadAlertOn, AlertModeByCH, CradleType/PowerControlMode, BTHeadsetNotification, Duty/OOR/PowerOff ACK, TG hold/remove, SignalLevel (1.41c) |
| `tabFunctionsParam` | `TGHoldTimeout` |
| `tabBacklightModeSettings`, `tabBackLightModeSetting` | profile `tabBacklightModeSettings` |
| `tabAlertToneSettings`, `tabFuncClassToneVolume`, `tabFuncLEDSettings` | `tabAudioToneSetting`, `tabFuncClassToneVolume`, `tabFuncLEDSettings` (funId 3) |
| `tabGPRSAckSetting`, `tabGSMBaseSetting` | `tabGPRSAckSettingPPS`, `tabGSMBaseSettingPPS` |
| `tabDSPEngineerSettings` | `tabDSPEngineerSetting.Enable` + hex blob of `tabDSPEngineerParam` |
| `tabAccessPassword` | DELETE, then copy rows (32-hex MD5 values) |
| `tabPINPasswordSetting` | DELETE, then copy (`DefaultPINPwd = UserPINPwd`) |
| `tabDetailScreenItem` | copy of profile rows except Item 5 in RecvMode 3..6 |
| `tabDispathCenterManualAck`, `tabDispathCenterAck` | same-name profile tables |
| `tabTone`, `tabToneFileDetail` | per custom alert tone: `(100+n, 'G4-Tone-NNN', 2, 100+n)` and `/defpara/Applications/chvoicerecord/G4-Tone-NNN.dat`. Custom announcements get no table rows, only the file name in `tabChannelAnnouncement` |
| `tabEncryptionSetting` | type 3 if `EncryptionType == 3` else 1 |
| `tabKVLUSBConnect`, `tabKMFSystem`, `tabP25OTARFeatureControl`, `tabP25OTARFeatureControlByChannel`, `tabKMFSystemByChannel`, `tabEncryptionKeyList`, `tabCipherKeyAssignment` | key IDs, aliases, CKRs and per-knob assignments for P25T / P25C / encrypted DMR and DVOA knobs (CSQ knobs skipped). No key material |

### 3.7 Private DB (`convertPrivate`)

| Radio table | Source |
|---|---|
| `tabMenuLayout(PageNo, Position, ItemKey)`, `tabMenuListLayout` | `Data_G4A.tabMenuBase` filtered by Model (series `G5`), `HWBlock` contained in `tabMfgProductCode.ProductHardwareCode`, and the `FunctionID` expression (`,` = OR, `;` = AND) against enabled rows of the profile's `tabMemberFunctionList`; 9 per page |
| `tabFunctionStatus` | `tabFunctionStatusBase` top-level items, children appended with odd/even `DefaultOrder`; item 54 skipped |
| `tabFunctionSettingList` | `tabFunctionSettingBase`, de-duplicated by SettingID |
| `tabOtaMenuList` | `tabOtaMenuBase`, Model must match (no rows for G5) |
| `tabSpecialMenuLayout` | `tabSpecialMenuBase` (no rows here) |
| `tabEnablePassiveWorkStationFunctions` | appended to the template's rows from `tabEnablePassiveWorkStationFunctionsBase` |
| `tabBleParamSetting` | DELETE, copy of profile row |

### 3.8 Tables the converter never writes (template default, untouched)

Confirmed equal to the template in both uploaded DBs (87 tables):
`tabAllowAlterZoneChannelsByManualCHSetting`, `tabArmyRoleSettings`,
`tabAudioService`, `tabAudioSquelchSettings`, `tabAudioToneDetail`,
`tabBacklightManualModeSetting`, `tabBasicSignalingSetting`, `tabBattery`,
`tabBrightLevelToBrightness`, `tabCap`, `tabCapturePhotoSettings`,
`tabChannelLock`, `tabContactOfACK`, `tabCurrentHome`, `tabDeviceOnOff`,
`tabDispatchCenterSetting`, `tabEmgRevtChSettings`, `tabEmgSettings`,
`tabEnableReceivingFunctions`, `tabFixAlertToneVolume`,
`tabFuncAlertToneBaseInfo`, `tabFuncSettingPassword`, `tabFuncStatusPassword`,
`tabFunctionListOfUserSetting`, `tabFuncTone`, `tabFuncToneClass`,
`tabFuncToneVolume`, `tabFuncVibrate`, `tabFunLedMap`, `tabGPRSReceiveSetting`,
`tabGPSDisplaySetting`, `tabGPSEngSettings`, `tabGPSInformSettings`,
`tabGPSInformTarget`, `tabGPSLocationDisplaySetting`, `tabGsmBusinessSetting`,
`tabGSMCallContactList`, `tabHardKeyActionOnKeypadLock`, `tabHdPhotoProfile`,
`tabHomAndDetailItem`, `tabHomeAlarmLayout`, `tabHomeScreenItem`,
`TabInformation`, `tabKeyinNumberCallSetting`, `tabLedSetting`,
`tabLightSensorAndBackLight`, `tabLightSensorLevelToBrightness`,
`tabLimitFunctionsByProduct`, `tabLimitFunctionsByUser`, `tabM1DeviceType`,
`tabM1TransitWaitTime`, `tabMakePhotoSettings`, `tabManDownSettings`,
`tabManufactureInfo`, `tabMenuListUISpec`, `tabMenuOption`,
`tabMonitorModeSettings`, `tabMorseAudioToneDetail`, `tabOTAPassWord`,
`tabOtaSettings`, `tabOutputSettings`, `tabPagerMsgBoxSetting`,
`tabPagerMsgBoxSetting_New`, `tabPowerSavingRxOff`, `tabPowerSavingSetting`,
`tabRecordSettings`, `tabRecvImageForShowSetting`, `tabRemoteMonitorSettings`,
`tabRetryDonwloadSettings`, `tabRetryDownloadInfo`, `tabRFPreformance`,
`TabRRSettings`, `tabRssiRange`, `tabSameFreqChannelList`,
`tabSingleToneRepeaterSetting`, `tabSMSSetting`, `tabTimerSetting`,
`tabToneFreqDetail`, `tabTransmitPriority`, `tabTransmitSchedulerSettings`,
`tabUseDefineCHDetail`, `tabVibrate`, `tabVibrateDetail`,
`tabVoiceQualitySetting`, `tabVolumeLevelSetting`, `tabWatchDog`,
`tabZoneDetailSettings`.

The profile settings that look like they should feed some of these
(`tabEmgSettings`, `tabManDownSettings`, power saving, GPS) are simply not
converted for 1.41. The base class has `convertEmergencySetting`,
`convertManDownSetting` etc., but no 1.41 path calls them.

## 4. Empirical validation

### 4.1 Method and tools

All in `scratch/conv141/tools/` (run with `python3 -I`):

- `load2.py in.db out.sqlite3`: lossless SQLite 2 -> SQLite 3 copy. Every
  value is stored as TEXT and NULL stays NULL. Needed because `sqlite2 .dump`
  writes numeric-looking values unquoted: `dump_sqlite2.sh` turns
  `'0000000000000000'` into `0`, `'0000'` into `0`, and so on. That breaks
  the CSQ protocol ID, the "no key" ID `000000000000`, and any GUID that
  happens to be all digits. The `scratch/db3/` copies have this problem.
  (Also: SQLite 2 `PRAGMA table_info` default values can contain newlines.)
- `sim141.py <b|c> profile sysdir template Data_G4A out`: Python port of
  `convertPublic` + `convertPrivate` for 1.41b/1.41c. Paths not exercised by
  these profiles (non-CSQ conventional signaling, advanced knobs, sub-groups,
  revert groups, call alert, free scan, debug entries) print a warning
  instead of guessing. It applies the generated SQL to a copy of the template
  and stops on nothing; any SQL error is reported.
- `cmp.py template sim actual`: per table, compare the multiset of rows
  (integers normalised), report MATCH or MISMATCH with sample rows.
- `tabdiff.py template actual`: which tables differ from the template at all.

Inputs per unit: `defpara/Applications/PPS/u3db_PC.db` (byte-identical to the
`<GUID>.db` inside `u3db_PC.zip`), the zip's `<GUID>/` folder, the shipped
`DB_Base_G4A.DBV1.41b.db` / `DBV1.41c.db`, and the shipped `Data_G4A.db`.
Ground truth: `defpara/Applications/PPS/u3db.db`.

### 4.2 Results

Both runs: 0 SQL errors (A: 2092 public + 58 private statements, B: 2202 +
58), no warnings, so nothing outside the simulated paths was needed.

**Every table MATCHes for both units, and the rowid order matches too.** A
strict text comparison of the converter-written rows also matches; the only
strict differences were template rows damaged by the lossy `db3` template copy
(`tabGSMBaseSetting`, `tabEmgSettings`, password-placeholder tables), not
converter output.

| Group | Tables | Unit A (1.41b) | Unit B (1.41c) |
|---|---|---|---|
| Zones/knobs | `tabZone`, `tabReceivingMode`, `tabZoneChannelList`, `tabChannelAnnouncement` | MATCH (5/20/18/20 rows) | MATCH (5/22/20/22) |
| Channel bundle | `tabChannel`, `tabChannelAdvInfo`, `tabVoiceCallSettings`, `tabR2RModeSetting`, `tabDispatchChannel` | MATCH (18 each) | MATCH (20 each) |
| | `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabResetModeSetting`, `tabRecvVoiceCallRecordByChannel` | MATCH (20 each, incl. 2 scan knobs) | MATCH (22 each) |
| | `tabAlertModeSetting` | MATCH (21) | MATCH (23) |
| | `tabRepeaterSettingOfChan`, `tabRecvSaList` | MATCH (11, 3) | MATCH (11, 3) |
| P25T | `tabP25TrunkingSignaling`, `tabSystemAlias` | MATCH (7, 7) | MATCH (7, 7) |
| | `tabP25TrunkingControlChannels`, `tabP25TrunkingSites` | MATCH (47, 11) | MATCH (47, 11) |
| | `tabP25TSetting` | MATCH (7) | MATCH (9) |
| Contacts | `tabContacts`, `tabContactOfBackground` | MATCH (275) | MATCH (283) |
| | `tabContactAddr` | MATCH (285) | MATCH (293) |
| | `tabRadioGroup`, `tabRecvVoiceCallRecordByContactID`, `tabContactOfRelayCtl` | MATCH (120) | MATCH (125) |
| | `tabRecvToneForContacts`, `tabContactAddrAndAlertTone`, `tabMyContact` | MATCH (7, 11, 1) | MATCH (7, 11, 1) |
| Receive lists | `tabContactOfChan`, `tabRecvGroupList` | MATCH (131) | MATCH (145) |
| | `tabRecvBehaviorByContactID` | MATCH (132) | MATCH (146) |
| Scan | `tabScanListSettings` | MATCH (2) | MATCH (2) |
| Device settings | `tabRadioBaseSetting`, `tabCallTimeLimit`, `tabCannedMessageMap`, `tabHomeSettings`, `tabFunctionsOnOff`, `tabDateTimeSetting`, `tabOutRangeSetting`, `tabRingModeSetting`, `tabReminderAlertSetting`, `tabUnreadAlertSettings`, `tabAudioRecordTimer`, `tabAlertToneSettings`, `tabFuncClassToneVolume`, `tabBacklightModeSettings`, `tabBackLightModeSetting`, `TabImageBoxSetting`, `tabGPRSAckSetting`, `tabGSMBaseSetting`, `tabDSPEngineerSettings`, `tabAccessPassword`, `tabPINPasswordSetting`, `tabDetailScreenItem` (48), `tabDispathCenterManualAck` | MATCH | MATCH |
| Encryption | `tabEncryptionSetting`, `tabEncryptionKeyList`, `tabKMFSystem`, `tabKMFSystemByChannel`, `tabCipherKeyAssignment` | MATCH (1,1,1,7,7) | MATCH (1,1,1,9,9) |
| | `tabP25OTARFeatureControl`, `tabP25OTARFeatureControlByChannel` | MATCH (7, 7: per channel, the 1.41b rule) | MATCH (1, 9: per KMF, the 1.41c rule) |
| Menus | `tabMenuLayout`, `tabMenuListLayout`, `tabFunctionStatus`, `tabFunctionSettingList`, `tabEnablePassiveWorkStationFunctions` | MATCH (10, 10, 12, 11, 23+10) | MATCH (10, 10, 12, 11, 26+10) |
| Written, equal to template | `tabCurrentZone`, `tabBacklightSettings`, `tabBleParamSetting`, `tabDispathCenterACK`, `tabDutySetting`, `tabFuncLEDSettings`, `tabFunctionsParam`, `tabKeypadLockSetting`, `tabKVLUSBConnect`, `tabMemory`, `tabMenuUIType`, `tabRecvBackgroundColorSetting`, `tabSpecialKeyActionTime`, `tabTextMessageSetting`, `tabTodaySettings`, `tabUiNumberFormat`, `tabVoiceMemoSettings` | MATCH (no-op) | MATCH (no-op) |
| NOT EXERCISED (empty in both profiles) | `tabTwoToneSignaling`, `tabFiveToneSignaling`, `tabFiveToneFreqSets`, `tab5ToneRepeaterSetting`, `tabMdc1200Signaling`, `tabMDCRepeaterSetting`, `tabUdc2400Signaling`, `tabDMRSignaling`, `tabDMRRepeaterSetting`, `tabP25Signaling`, `tabP25RepeaterSetting`, `tabSubgroupWithMultiSubAddr`, `tabRevertListGroup`, `tabP25TQCDelayRecordParm`, `tabCallAlertGroup`, `tabAllBandScanSettings`, `tabAllBandScanParam`, `tabTone`, `tabToneFileDetail`, `tabOtaMenuList`, `tabSpecialMenuLayout` | | |
| Never written | the 87 tables in 3.8 | equal to template | equal to template |

Which PPS wrote them: both profiles have `tabDBVersion` 8.0 and
`tabPPSVersion` 2.31, the same as the V00-04-00-48 template; the radio DBs
have no version table. The zip timestamps say the units were last programmed
in April 2024 (A) and August 2024 (B). Since the V00-04-00-48 converter and
its `Data_G4A.db` menu tables reproduce both files exactly, the PPS used was
V00-04-00-48 or a release with identical 1.41b/1.41c logic. There's no
evidence of an older PPS.

Live vs uploaded: unit B's live `/root/Applications/u3db/u3db.db` is
identical to the upload. Unit A's differs in one row of
`tabBackLightModeSetting` (a backlight setting changed on the pager after programming). So the
pager edits its own DB in place, and readback from the live copy would show
user changes that `u3db_PC.zip` doesn't have.

### 4.3 DBV1.41d and DBV1.41e (2026-10-25)

Pager C (G5B67BF) was updated from V2.02 (DBV1.41c) to V2.05
(internal V1.35-T01S-08252026, DBV1.41e) with Unication's updater
([`firmware-update.md`](firmware-update.md)), then programmed once from PPS
V00-04-00-48 with its own profile, unchanged. Three backups bracket it:
before the update (1.41c), after the update, and after the PPS write.

- **Knobset's `Variant::E` regenerates the PPS-written radio DB with 194/194
  tables identical**, in rowid order. The profile has P25 trunking, analog,
  scan and DMR knobs; the golden test covers it from now on.
- Group contacts run from 1, members from 9000000 (the pager's own member
  row doesn't use one up), the pager is 9999999 and all call 9999998.
  Without P25 sub-groups (refused) the per-knob counter reset in section 2.3
  gives the same numbers as a plain running counter, so the converter keeps
  that code for both schemes.
- `Variant::D` is the 1.41c converter on the 1.41d template. The 1.41d
  converter's only change is two-tone detector thresholds, and Knobset
  refuses two-tone conventional channels, so for every profile Knobset
  accepts the 1.41d output equals the 1.41c output except
  `tabTwoToneSignaling`'s new columns (template only). No 1.41d pager has
  been seen; 1.41d and 1.41e templates are identical.
- The firmware migrates the radio DB in place: both `/defpara` and `/root`
  copies get the contact ID renames from the update's migration scripts
  (99999 -> 9999999, 65535 -> 9999998, in the 11 contact tables) and the
  rebuilt `tabTwoToneSignaling`. Talkgroup and member IDs keep their
  1.41c numbers until the next write. The `u3db.db.md5` sidecar is left
  holding the old file's MD5, so the first backup after an update fails its
  sidecar check. Knobset accepts that one mismatch when the radio DB is
  exactly what the backup read off the pager, and writes a correct sidecar.
  Such a backup isn't a converter reference, so the golden tests skip it.
- PPS's read of the updated pager also adds the 1.41d columns to the
  profile's own `tabTwoToneSignaling`; nothing else in the profile changed
  apart from the save timestamp.

## 5. What the real profiles contain (hobbyist view)

Both units carry the same profile lineage (same system GUIDs); B is a later
edit of A.

| Item | Unit A | Unit B |
|---|---|---|
| Zones | 3 (zone 1 has 16 knob rows, zones 2-3 have 8) | same |
| Active knobs | 20 | 22 |
| Receive modes | 4 TG scan (16), 3 TG monitor (48), 11 conventional monitor (33), 1 normal scan (4), 1 priority scan (36) | 6 TG scan, 3 TG monitor, 11 monitor, 1 scan, 1 priority scan |
| P25 system DBs in the folder | 9 (2 orphans not in `tabP25TrunkingSecurity`) | 9 |
| Systems used by knobs | 7, one P25T knob each | 7, on 9 knobs (two systems used twice), still 7 SigIDs |
| Protocol settings | 1 (P25T), shared by every P25T knob | same |
| Sites | 6 systems with 1 site, 1 system with 5 (11 total, RFSS IDs 1/3/4) | same |
| Control channels | 47 distinct (2 to 28 per system); only the RadioReference-imported system has site/CC links | same |
| Talkgroups in system DBs | 5, 10, 12, 35 and three full imports of ~1408 | similar (+2 and +2) |
| TGs selected on knobs | 120 rows (6, 1, 34, 11, 4, 1, 63), 120 contacts | 134 rows, 125 contacts (shared TGs collapse) |
| Priority TGs (`SeqNo > 0`) | 4 (three priority 1, one priority 2) | 7 |
| `ATGID`, `Priority`, `OffDuty` | 0 everywhere | same |
| Individual IDs | 154 in one system DB, all converted to contacts | 157 |
| Sub-groups, revert groups, call alert | none | none |
| Full-spectrum scan, site trunking | off in every system | same |
| Conventional channels | 11 single-channel knobs, all CSQ, VHF, bandwidth code 1, 3 with CTCSS | same |
| Scan lists | 2: a 7-channel normal scan and a priority scan (1 priority + 3 members) | same |
| Free scan, advanced conventional, custom tones/prompts, logo | none | none |
| Encryption | `EncryptionType` 3 with the single default key entry; P25T knobs get a channel key assignment | same |

## 6. Implications for implementation

### 6.1 What we must generate

Start from the shipped template for the reported version and write what PPS
writes. A truly minimal subset isn't safe, because several tables that the
pager clearly needs ship **empty** in the template and are only filled by the
converter: the menus (`tabMenuLayout`, `tabMenuListLayout`,
`tabFunctionStatus`, `tabFunctionSettingList`), `tabRadioBaseSetting`,
`tabMyContact`, `tabDetailScreenItem`, `tabDSPEngineerSettings`, the device
`tabAlertModeSetting` row, and the all-call contact. Ship those every time.

For a scanner profile (P25T + CSQ conventional + scan lists) the full set is
the 69 + 17 tables in 4.2. Grouped by what drives them:

1. Always: menus and function lists (needs `Data_G4A.db` and the profile's
   `tabMemberFunctionList` + `tabMfgProductCode`), pager contact, all-call,
   device settings, `tabCurrentZone`, `tabZone`.
2. Per active knob: `tabReceivingMode`, `tabChannelAnnouncement`,
   `tabZoneChannelList` (non-scan), and the channel bundle in 3.2.
3. Per P25 system/protocol pair: 3.3. Per P25T knob: `tabP25TSetting` and the
   receive-list triple (`tabContactOfChan`, `tabRecvGroupList`,
   `tabRecvBehaviorByContactID`).
4. Per scan knob: `tabScanListSettings` plus the reduced per-channel set.

### 6.2 Tricky rules

- The ID counters depend on row order of `tabZone_New`,
  `tabP25TrunkingSystemKnob`, `tabChannelGroup` and each system's
  `tabMember_New`. Our profile writer must keep a stable order, and the
  converter must iterate in rowid order.
- The pager's own member row consumes member ID 1 even though it gets 99999.
- P25T `tabRecvGroupList.SigID` is the system SigID, not the ZK. SeqNo is a
  renumbering of priority TGs, not a copy.
- `tabChannelAnnouncement.ChannelID` is the knob index.
- `ModifyByDevice` on `tabZone` is 1 for empty zones; it doesn't mean "edited
  on the device".
- `tabAlertModeSetting` rows all end up with the ring-mode timings.
- Individual IDs from the system DB all become contacts. A full
  RadioReference import with thousands of units would produce thousands of
  contacts; check pager limits before allowing that.
- Non-CSQ conventional knobs need a talkgroup grouping, or PPS fails. Our
  validator should enforce the same.
- `DispatchCHFlag = '0'` selects dispatch channels, the opposite of what the
  name suggests. Copy it, don't "fix" it.
- 1.41c+ needs the 18-column `tabFunctionOnOffOfChan`; a 1.41b template
  rejects the 18th value. Write column lists explicitly per version.
- Never move values through numeric literals. Keep everything TEXT, the way
  SQLite 2 stores it.

### 6.3 Test strategy

- **Golden tests from the two units.** Fixture = `u3db_PC.db` + the zip's
  `<GUID>/` folder + uploaded `u3db.db`, per unit. Run our converter against
  the shipped 1.41b / 1.41c template and assert, table by table, that the
  row multiset (as TEXT) equals the uploaded DB; also assert rowid order,
  since PPS's order is reproducible. Byte equality of the SQLite file is the
  wrong target (page layout differs); compare logical content.
- Run the same fixture through 1.41a/d/e as a smoke test: a and d should give
  the b/c result except for the documented deltas, e should differ only in
  contact IDs.
- Use `scratch/conv141/tools/sim141.py` as a reference oracle while the Rust
  port is young, and extend it (with code citations) before trusting any path
  it marks NOT EXERCISED. The next profile to capture should exercise
  conventional P25 with NAC, a DMR channel, a free-scan knob, a custom alert
  tone and a 2-tone sub-group.
- The fixtures are private: they contain agency names, the pager's unit ID,
  32-hex MD5 password hashes and an individual-ID list. Keep them out of the
  public repo (git-ignored, or synthesized and scrubbed copies that still pass
  the same golden comparison).
- Fix or replace `dump_sqlite2.sh` before building more fixtures on it; the
  loader in `load2.py` is the model to follow.
