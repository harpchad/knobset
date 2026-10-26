# Removing, editing, switching and converting knobs: PPS row-level behavior

Scope: what PPS V00-04-00-48 writes when the user removes a knob (makes the
position empty, `CurrentMode 0`), edits an existing analog conventional knob
in place, switches a trunked knob between TG Scan (16) and TG Monitor (48),
and converts a knob from one family to another (conventional to trunked and
back). Siblings: [zones-and-scan.md](zones-and-scan.md) (zones, scan-knob
creation and member editing), [system-creation.md](system-creation.md) (trunked knobs on
empty or conventional positions; its section 3.1 on knob-editor mechanics,
`savePrivate` and `checkError` gating applies here unchanged) and
[conventional-knobs.md](conventional-knobs.md) (analog knob creation,
frequency pool, squelch encoding).

Evidence paths are relative to `decomp/`. Short forms: `ZONE/` =
`View/View.Zone/`, `CHECK/` = `View/View.ProfileCheck/ProfileCheckHandler.cs`,
`CONV/` = `View/View.ProgramVersion/PPSU4ParamConverter_01P41b.cs`, `RS/` =
`crates/g5-codeplug/src/convert141.rs`, `COMMON/` =
`View/View.Common/CommonFunction.cs`. Real data: both units' PPS-written
profiles as extracted to `scratch/syscreate/unit{A,B}/` (SQLite 3 copies
`*.s3`). Both carry the same zone, scan and conventional rows (identical
`tabMultiChannelScan`, `tabMultiCHScanList`, `tabSingleConvCHKnob`,
`tabRecvSaList`). Experiments: `scratch/knobedit_tools/exp.py` writes
`scratch/knobedit/<case>/` and the full log `scratch/knobedit/results.txt`;
it applies each case's rows to a copy of unit B's profile, converts with
Knobset's 1.41c converter and diffs the radio DB against the unmodified
conversion (which equals unit B's uploaded `u3db.db`, 194 of 194 tables).

## 0. Summary

- **Removal is one function, `deleteKnob`.** The Delete context-menu item,
  the receive-mode dialog's "None" and Cancel on a pending new knob all call
  it, then save. It resets the receive row (`CurrentMode 0`, `AlertToneID 0`,
  `ChannelName ''`), deletes the knob's row in each of the five detail tables
  by zone and knob, and deletes every row keyed by the knob's `ChannelID` in
  13 child tables, **including its memberships in other knobs' scan lists**.
- **What removal leaves behind:** the knob's frequency row, a scan knob's
  `FirstPriorityCH` pointing at the removed knob, scan lists with fewer than
  two members, gaps in `ScanOrder`. The last three fail the pre-program
  check (E1, E3) and PPS shows no warning when deleting. The converter then
  silently drops the dangling priority channel (experiment r4: a priority
  scan with an empty `PriorityZoneKnobID`).
- **Detail rows matter more than the mode.** Both converters convert every
  detail row whatever `CurrentMode` says: a knob set to mode 0 with its
  detail rows kept still produces a nameless radio channel (experiment r7).
  Removing a knob means deleting its rows, not only resetting the mode.
- **Real scan knobs:** NOAA knob 1 "NOAA Scan" (Normal Scan, 7 members, WX7
  down to WX1) and Lake knob 2 "ElmwoodCo Fire" (Priority Scan, members Lake
  knobs 3 to 6, priority channel Lake knob 3 "Elmwood Fire", which is also
  member 1). All members are CSQ Monitor knobs. Every frequency row is used
  by exactly one knob; no leftover or orphan rows exist in either unit.
- **Editing a conventional knob in place is a set of single-column changes.**
  Frequency and bandwidth live on the frequency row (shared by every knob
  that points at it), squelch in `tabRecvSaList`, the other options in the
  knob row. Each option reaches exactly one radio table: frequency and
  bandwidth `tabChannel`, tone `tabRecvSaList`, `AlertMode`
  `tabAlertModeSetting`, `RecordFlag` `tabRecvVoiceCallRecordByChannel`,
  `AudioSquelch`/`Emphasis`/`Compander` `tabChannelAdvInfo`, prompt
  `tabChannelAnnouncement`. `TxFreq`, `TxBandWidth` and the frequency name
  reach nothing.
- **Frequencies are referenced only by `ConvFreqID`** in
  `tabSingleConvCHKnob` and `tabAdvancedConvCHKnob`. Safe edit: update the
  row in place when only this knob uses it, else append a new row and
  repoint the knob. Both give the same radio DB (e1, e1b).
- **16 to 48 and back is a "slight change":** the knob row and its
  `ChannelID` stay. Rows that change: `CurrentMode`; `AlertToneID` (a
  built-in 6/8/11 prompt becomes 0, 0 and custom prompts stay); the knob's
  `tabP25TSubGroupEnable` row (48 always writes `(ChannelID, '', '1')`; 16
  must not have one, or the pre-program check B3 fails); the binding-feature
  columns, which a 48 knob saves at their hidden defaults. Radio: 2 tables
  (`tabReceivingMode` mode 16/64, `tabP25TSetting.TwoToneSubGroupOn`), 3 with
  a prompt change.
- **Changing family (conventional, scan, trunked, free scan) is a "big
  change": `deleteKnob` without clearing the alias, then a fresh detail row
  with a new `ChannelID`.** Conventional to trunked drops the knob from every
  scan list. Trunked to conventional defaults the new knob to the **first
  frequency row of the profile**, which in unit B belongs to Elmwood Fire.
- **Write-guard sets** (section 6): trunked removal = `KNOB_CHANNEL_TABLES` +
  `TALKGROUP_EDIT_TABLES` + `SYSTEM_EDIT_TABLES`; conventional removal =
  `CONV_KNOB_CHANNEL_TABLES` + `tabScanListSettings`; scan removal = 8
  tables; conventional edit = 6 tables; 16/48 switch = 3 tables;
  conversions = the union of the removal and creation sets of both kinds.

## 1. Entry points and change classes

### 1.1 The three ways a knob is removed

| Path | Flow | Evidence |
|---|---|---|
| Zone grid, right-click a knob cell, "Delete" | item enabled only when `CurrentMode != 0`; asks "Are you sure to delete this Z{row}/P{k}?"; `deleteKnob(zone, k)` (clear alias), refresh the cell, `savePrivate` | `ZONE/ZoneListControl.cs:3379-3387,1568-1591` |
| Knob editor, receive-mode button, choose "None" | `ChangeRecvModeControl.save` asks "The knob will be deleted if Receiving Mode None selected, continue?"; `getRecvModeChangeAction` returns Delete; `RecvModeChanged` calls `deleteKnob(zone, k, clearChannelName: true)`, closes the editor (`base.Cancel`) and runs `savePrivate` | `ZONE/ChangeRecvModeControl.cs:129-133,158-169`, `ZONE/ZoneKnobInfoControl.cs:399-422`, `ZONE/ZoneListControl.cs:3547-3573` |
| Cancel while a new knob is pending | `m_NewChannel` holds `row:zone:knob`; `deleteKnob` and refresh | `ZONE/ZoneListControl.cs:309-331` |

Zone deletion does the same per zone (`deleteZone`, `:1170-1240`).

### 1.2 Change classes (`getRecvModeChangeAction`)

`ZONE/ChangeRecvModeControl.cs:129-156`:

| New mode | Old and new in the same family | Action |
|---|---|---|
| 0 | | Delete |
| same as old | | None |
| any | selective-call family {1 Selective Call, 33 Monitor, 128 Radio Compatible}, multi family {65, 97}, scan family {4, 36, 68}, trunking family {16, 48, 129} | SlightChange: same detail row, same `ChannelID`; the open editor re-runs its visibility logic (`notifyControlPanelChanged`) |
| any | different families, or free scan (8) on either side | BigChange: confirmation "Do you want to change the Receiving Mode from ... to ...?" (`:190`), then `deleteKnob(clearChannelName: false)` and the knob is reopened, which creates a fresh detail row (`ZoneListControl.cs:3574-3578`) |

Before either change runs, `button_recvMode_Click` writes the new
`CurrentMode` and the **currently selected voice prompt** into the in-memory
receive row (`ZoneKnobInfoControl.cs:414-415`). The prompt is then
re-resolved against the new mode's list (section 4.3).

## 2. Removing a knob

### 2.1 `deleteKnob` (`ZONE/ZoneListControl.cs:1309-1384`)

1. Clears the knob clipboard if it held this knob (`:1311-1315`).
2. With `clearChannelName` (every removal path): the receive row gets
   `CurrentMode 0`, `AlertToneID 0`, `ChannelName ''` (`:1317-1327`).
   `AlertFileName` is not touched. After the full-table rewrite these are
   the strings `'0'`, `'0'`, `''`: the signature of Lake knob 7 in both
   units (conventional-knobs 1.4).
3. Collects the `ChannelID` of the knob's row in `tabSingleConvCHKnob`,
   `tabP25TrunkingSystemKnob`, `tabAllBandScanCHKnob` and
   `tabAdvancedConvCHKnob` (`:1328-1364`). A scan knob has no `ChannelID`
   (`ScanKnobControl` is opened with the placeholder `00000000000`,
   `:3502`), so for scan knobs the list is empty.
4. Deletes:

| Table | Key | Effect |
|---|---|---|
| `tabSingleConvCHKnob`, `tabP25TrunkingSystemKnob`, `tabMultiChannelScan`, `tabAllBandScanCHKnob`, `tabAdvancedConvCHKnob` | `ZoneID`, `KnobIndex` | the knob's detail row, whatever its kind |
| `tabMultiCHScanList` | `ZoneID`, `KnobIndex` | a scan knob's own member list |
| `tabMultiCHScanList` | `ChannelID` in the list | **the knob's membership in every scan list** |
| `tabChannelGroup`, `tabChannelGroupList`, `tabRecvSaList`, `tabP25TSubGroupEnable`, `tabP25TChannelSubGroup`, `tabChannelRevertGroup`, `tabP25TCallAlertEnable`, `tabP25TCallAlertSUID`, `tabP25TCallAlertSUIDGroup`, `tabP25TChannelRevertSubGroup`, `tabAllBandScanBandParam`, `tabAllBandScanProtocolParam` | `ChannelID` in the list | talkgroup list, grouping link, tones, sub-group and call-alert rows, revert list, free-scan ranges and protocols |

`deleteTable` with an empty value list returns at once
(`COMMON:835-840`), so a scan knob's removal touches only the zone/knob
tables. All deletions happen in the in-memory DataTables; `savePrivate`
rewrites the 20 zone tables in full (system-creation 3.1).

### 2.2 What removal does not touch

| Item | State after removal | Consequence |
|---|---|---|
| `tabConvFreqList` row of a conventional knob | kept (the zone page never writes frequencies, conventional-knobs 1.1) | none on the radio (experiment r3f); costs one of 1024 rows |
| `tabMultiChannelScan.FirstPriorityCH` / `SecondPriorityCH` of a scan knob that pointed at the removed knob | kept, dangling | E3 at program time; the converter drops the dangling ID, leaving `PriorityScanType 1` with no priority channel (r4) |
| scan list left with 0 or 1 member | kept | E1 at program time |
| `ScanOrder` of the remaining members | gaps (1, 3, 4, ...) | none: neither the scan editor (`ZONE/ScanListControl.cs:128-139`, row order) nor the converters (`CONV:3054-3065`, `RS:1189-1197`) read `ScanOrder` for ordering |
| system DB, `tabP25TrunkingSecurity`, `tabProtocolSetting` | kept | an unused system costs nothing (system-creation 5.4 d); the radio loses that SigID when no other knob uses it (r2) |
| `tabGroupGrouping`, `tabGroupingOfGroup`, `tabGroup_New` | kept | |
| custom announcement in `VoicePromptTotal.db` | kept | |
| `tabAdvancedChannelGroup` (advanced knobs' groups) | not even loaded by the zone page (`ZoneListControl.cs:207-263`) | a removed mode 65/97 knob would leave its rows; Knobset's converter refuses advanced knobs anyway (`RS:362-364`) |
| `tabEmergencySetting.ChannelID` | not checked | `''` in both units; only the pre-1.41 base converter reads it (`View/View.ProgramVersion/PPSU4ParamConverterBase.cs:1905-1930`) |
| `tabZone_New`, `tabDefaultZone` | kept | a zone with no active knob converts with `ModifyByDevice 1` (r8) |

Nothing else in the profile references a knob: a search of the decompiled
UI for `KnobIndex`, `FirstPriorityCH` and `ConvFreqID` outside the zone
page finds only the frequency page, the detail report, the pre-program
check, clone pages, the RadioReference importer, protocol-setting deletion
and schema upgraders. `tabFunctionAlertTone` has `ZoneID ''`, `KnobIndex 0`
(the global priority alert tone), `tabGPSInform*` are empty.

### 2.3 Per receive mode

| Kind (mode) | Detail rows a knob has | Rows `deleteKnob` removes | Real examples (unit B) |
|---|---|---|---|
| TG Scan (16) | `tabP25TrunkingSystemKnob`; `tabChannelGroup` (1..64); optional sub-group, call-alert, revert rows | all of them | Metro 1, 3, 4, 5, 6, 7: 2 to 34 TGs, no sub-group or call-alert rows |
| TG Monitor (48) | same plus `tabP25TSubGroupEnable (ChannelID, '', '1')` | all of them | Metro 2 (1 TG), Lake 1 (1 TG), Lake 8 (63 TGs); each has the sub-group-enable row |
| Monitor (33) / Selective Call (1) | `tabSingleConvCHKnob`; 0..5 `tabRecvSaList`; for non-CSQ protocols `tabChannelGroupList` and group rows; memberships in `tabMultiCHScanList` | all of them, memberships included; the frequency row stays | 11 Monitor knobs, 3 with one CTCSS row, all 11 are scan members; no mode-1 knob exists |
| Normal Scan (4) / Priority Scan (36) | `tabMultiChannelScan`; its own `tabMultiCHScanList` rows | both; members are untouched | NOAA 1, Lake 2 (section 2.4) |
| Free Scan (8) | `tabAllBandScanCHKnob`; `tabAllBandScanBandParam`, `tabAllBandScanProtocolParam` | all three | none |

### 2.4 The real scan knobs

`tabMultiChannelScan` (identical in both units):

| Knob | Mode | `PriorityScanType` | `FirstPriorityCH` | `SecondPriorityCH` | `StayTime` | `OpenSpeaker` | `SamplingFlag` | `DwellTime` | `AlertMode` | `RecordFlag` | other columns |
|---|---|---|---|---|---|---|---|---|---|---|---|
| NOAA 1 "NOAA Scan" | 4 | 0 | `''` | `''` | 200 | 1 | 0 | 6 | 3 | 0 | `ResetMode 0`, `DelayNTime 30`, `CarrierOffDelayTime 30`, match flags 0, `AlwaysUnreadReminderAlert 1` |
| Lake 2 "ElmwoodCo Fire" | 36 | 1 | `dd74db57...` (Lake 3 "Elmwood Fire") | `''` | 200 | 1 | 0 | 6 | 1 | 1 | same |

`tabMultiCHScanList`, in row order:

| Rows | Scan knob | `ScanOrder` | Members |
|---|---|---|---|
| 1-7 | NOAA 1 | 1..7 | NOAA 8 (WX7), 7, 6, 5, 4, 3, 2 (WX1) |
| 8-11 | Lake 2 | 1..4 | Lake 3 (Elmwood Fire, also the priority channel), Lake 4, 5, 6 |

The priority channel is always a member: the priority combo lists only the
selected members and defaults to the first (`ZONE/ScanListControl.cs:205-216,372-397`).
Members can be any single or advanced conventional knob in any zone except
mode 128 (`:99-186`); trunked knobs are never offered. Limits: 2..16
members (`:231-240,289-294`), priority required for mode 36 (`:241-247`).

Radio rows (base conversion), `tabScanListSettings`
(`ScanListSettingID, PriorityScanType, SamplingFlag, PriorityZoneKnobID, ScanList, RecvAllFlag, StayTime, CallBackTime, RxPauseBetweenScanPaths, PriorityIntervalTime`):

```text
202|1|0|203|204,205,206|1|200|0|0|6
301|0|0||308,307,306,305,304,303,302|1|200|0|0|6
```

The priority channel is moved out of `ScanList` into `PriorityZoneKnobID`
(`CONV:3040-3066`, `RS:1176-1197`). Both scan knobs convert to radio mode 4
(`tabReceivingMode ... 4|4 ... CtCdRequest 0` from `MatchCTCD`); a scan
knob gets no `tabZoneChannelList` and no `tabChannel` row.

### 2.5 Pre-program checks that removal can trip

| Code | Rule | Evidence |
|---|---|---|
| A1 | at least one knob with `CurrentMode != 0` in knobs 1..8 of some zone | `CHECK:263-299` |
| E1 | each scan knob has at least 2 members that are active single or advanced conventional knobs | `CHECK:643-692`, scan map `:199-240` |
| E2 | no member that is not such a knob; no member with the sub-group feature | `CHECK:655-684` |
| E3 | a priority scan's `FirstPriorityCH` is an active conventional knob | `CHECK:694-697` |

The C# converter drops a dangling priority channel or member silently
(`CONV:3040-3066`); so does Knobset's (`RS:1179-1197`). Without the check, a
hand-written profile would program a priority scan with no priority channel.

### 2.6 Experiments

"Changed" = table content or row order differs from the base conversion.

| Case | Profile change | Radio tables changed |
|---|---|---|
| r1 | Metro 1 (TG Scan, first `tabP25TrunkingSystemKnob` row) removed, PPS rows | 30: the per-channel bundle (`tabReceivingMode`, `tabZoneChannelList`, `tabChannelAnnouncement`, `tabChannel`, `tabChannelAdvInfo`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabVoiceCallSettings`, `tabResetModeSetting`, `tabR2RModeSetting`, `tabAlertModeSetting`, `tabRecvVoiceCallRecordByChannel`, `tabDispatchChannel`), the P25 per-channel tables (`tabP25TSetting`, `tabKMFSystemByChannel`, `tabCipherKeyAssignment`, `tabP25OTARFeatureControlByChannel`), 9 contact tables, and the 4 system tables: **SigIDs renumber** (the "Metro South" system moves from SigID 1 to 7 because Metro 6 uses it later) |
| r1b | Metro 7 (TG Scan, last trunk row) removed | 26: the same minus the 4 system tables, `tabContactAddrAndAlertTone` and `tabRecvToneForContacts`, plus `tabRadioGroup` and `tabContactOfRelayCtl` |
| r2 | Lake 8 (TG Monitor, 63 TGs, only knob on "Statewide") removed | 32: as r1; SigID 6 disappears, SigID 7 becomes 6 |
| r3 | NOAA 2 (CSQ Monitor, member of NOAA Scan) removed | 18: the 17 tables of a CSQ knob (conventional-knobs 5.2 a1) plus `tabScanListSettings` (member 302 removed) |
| r3b, r3c | Lake 4 (no tone), Lake 6 (CTCSS) removed | 18 / 19 (`tabRecvSaList` row 206 removed) |
| r4 | Lake 3 (the priority channel) removed, PPS rows | 19; `tabScanListSettings` row 202 becomes `1|0||204,205,206`: priority scan with no priority channel |
| r4k | as r4, `FirstPriorityCH` moved to Lake 4 | 19; row 202 becomes `1|0|204|205,206` |
| r5, r5b | NOAA 1 (Normal Scan), Lake 2 (Priority Scan) removed | 8: `tabReceivingMode`, `tabChannelAnnouncement`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabResetModeSetting`, `tabAlertModeSetting`, `tabRecvVoiceCallRecordByChannel`, `tabScanListSettings` |
| r6a / r6b | Lake 5 made Selective Call (1), then removed | r6a: 3 (`tabReceivingMode` 32 to 1, `tabChannelAdvInfo.SignallingSquechLogic` 1 to 0, `tabFunctionOnOffOfChan.MonitorModeOn` 1 to 0); r6b vs r6a: the same 19 tables as r3c |
| r7 | Lake 4 receive row reset to mode 0, detail rows **kept** | 6: `tabChannel` keeps row 204 **with an empty name**, `tabChannelAdvInfo`, `tabFunctionOnOffOfChan` keep it too; only `tabReceivingMode`, `tabZoneChannelList`, `tabChannelAnnouncement` lose it |
| r8 | all 8 NOAA knobs removed | 19: r3's set plus `tabZone` (`NOAA` `ModifyByDevice` 0 to 1) |
| r3f vs r3 | also delete NOAA 2's now unused frequency row | none |

## 3. Editing an analog conventional knob in place

### 3.1 Where each visible option lives

The knob editor for a CSQ Monitor knob is `SelectiveCallControl`; each
sub-control's `Save` writes into the in-memory rows (conventional-knobs
1.1). For an existing knob the rows are reused (`Row_tabSingleConvCHKnob`
returns the existing row, `ZONE/ZoneListControl.cs:3622-3628`), so an edit
never changes `ChannelID` or row position.

| Option (UI) | Stored in | PPS save rule | Radio table and value | Experiment |
|---|---|---|---|---|
| Frequency (combo of pool rows) | `tabSingleConvCHKnob.ConvFreqID` | the selected `FreqID` (`ZONE/ConventionalSettingControl.cs:389`); `checkError` demands a selection (`:342-351`) | `tabChannel` Rx = Tx = the row's `RxFreq` | e1, e1b |
| Frequency value | `tabConvFreqList.RxFreq` (Frequency page grid; Rx only) | `View/View.FrequencySetting/FreqSettingControl.cs:464-470` | same | e1 |
| Bandwidth | `tabConvFreqList.RxBandWidth` | grid column bound to Rx only (conventional-knobs 2.1) | `tabChannel` both bandwidth columns | e2 |
| CTCSS/DCS | `tabRecvSaList` rows of the `ChannelID` | delete the knob's rows, append one per distinct non-None slot (`ConventionalSettingControl.cs:393-427`) | `tabRecvSaList` (first 5 rows) | e3, e3b, e3c, e3d |
| Audio squelch Low/Mid/High | `AudioSquelch` 0/1/2 | visible for CSQ (`ZONE/ConventionalReceiveOptionControl.cs:263,277-287`) | `tabChannelAdvInfo` `CarrierSquelchSetting`/`SINADSquelchSetting` 51/4, 102/8, 153/12 | e6 |
| Rx de-emphasis | `Emphasis` 0/1 | visible for CSQ (`:261`) | `tabChannelAdvInfo.Emphasis` | e7 |
| Compander | `Compander` 0/1 | visible for CSQ (`:262`) | `tabChannelAdvInfo.Compander` | e8 |
| Alert mode Tone/Vibrate/Silent/Tone and Vibrate | `AlertMode` 1..4 | always visible except mode 129 (`ZONE/ChannelFunctionSettingControl.cs:56-77,254,405-408`) | `tabAlertModeSetting.mode = AlertMode - 1` | e4 |
| Voice storage Off/ID Match/All Voice | `RecordFlag` 0/1/2 (Monitor list) | visible for Monitor; for Selective Call with CSQ it is hidden and saved as **1** (`:24-54,236-240,389-392,425-436`) | `tabRecvVoiceCallRecordByChannel.AllowRecord` | e5 |
| Voice prompt | `tabReceivingMode_NewG4.AlertToneID` (0 or 2 for Monitor, plus custom) | `ZONE/ZoneKnobInfoControl.cs:272-283` | `tabChannelAnnouncement` `G4-ChVoice-NNN.dat` | e9 |
| Alias | `tabReceivingMode_NewG4.ChannelName` | same | `tabChannel`, `tabReceivingMode` (`RENAME_TABLES`) | existing rename path |

Hidden for CSQ Monitor and written at fixed values on every save (so an
in-place edit should leave them alone): `SquelchOption 3`, `TransmitMode 1`,
`VoiceBuffering 1`, `PushToListen 0`, `Privacy 0`, the binding-feature
columns, `EncryptionBy 0` (conventional-knobs 1.2).

### 3.2 Frequency rows and sharing

- `FreqID` is referenced by `tabSingleConvCHKnob.ConvFreqID` and
  `tabAdvancedConvCHKnob.ConvFreqID` only (`ConventionalSettingControl.cs:389`,
  `ZONE/MultiSelectiveCallProtocolSettingControl.cs`; a search for `FreqID`
  finds no other writer outside clone, report, import and check code).
- The knob combo lists every pool row (`ConventionalSettingControl.cs:238-250`),
  so any number of knobs can point at one row. Editing a row on the
  Frequency page retunes every knob on it, without warning.
- Unit B: 11 rows, each used by exactly one knob.
- Both ways to retune one knob give the same radio DB: updating its own row
  in place (e1) or appending a new row and repointing `ConvFreqID` (e1b).
  Only `tabChannel` changes.
- `TxFreq`, `TxBandWidth` and `Name` reach no radio table (e1c, e10;
  conventional-knobs 2.2). Setting `TxFreq = RxFreq` while editing is a
  free cleanup of the stale band-minimum values.
- A dangling `ConvFreqID` makes the editor show no frequency and fail
  `checkError`, and fails B1 (`CHECK:559-563`).

### 3.3 Squelch rows

`Save` always deletes the knob's `tabRecvSaList` rows and appends the new
set (`ConventionalSettingControl.cs:393-427`), so after an edit in PPS the
knob's rows sit at the end of the table. Row position across knobs does not
reach the radio: moving Elmwood Fire's row to the end changes nothing
(e11). Carrier to tone, tone to tone and tone to carrier each change only
`tabRecvSaList` (e3, e3b, e3c); `MonitorModeOn` stays 1 on the radio
(conventional-knobs 3.3).

### 3.4 Experiments (Lake 4 unless noted; all vs base)

| Case | Change | Radio tables changed |
|---|---|---|
| e1 | own frequency row `RxFreq` (and `TxFreq`) 154.19 to 154.25 MHz | `tabChannel` |
| e1b | new frequency row 154.25 MHz, `ConvFreqID` repointed, old row orphaned | `tabChannel` (identical to e1) |
| e1c | `TxFreq` only | none |
| e2 | `RxBandWidth`/`TxBandWidth` 1 to 2 | `tabChannel` (`1|1` to `2|2`) |
| e3 | carrier to CTCSS 100.0 (row appended) | `tabRecvSaList` (`204|0|1|1000|1|19|1|0` added) |
| e3b | Lake 6 CTCSS 79.7 to 100.0 in place | `tabRecvSaList` |
| e3c | Lake 6 CTCSS to carrier (row deleted) | `tabRecvSaList` |
| e3d | Lake 6 CTCSS to DCS 754 inverted | `tabRecvSaList` (`206|0|2|670|1|492|1|1`) |
| e4 | `AlertMode` 1 to 3 | `tabAlertModeSetting` (`mode` 0 to 2) |
| e5 | `RecordFlag` 1 to 2 | `tabRecvVoiceCallRecordByChannel` |
| e6 | `AudioSquelch` 0 to 2 | `tabChannelAdvInfo` (51/4 to 153/12) |
| e7, e8 | `Emphasis` 1 to 0, `Compander` 0 to 1 | `tabChannelAdvInfo` |
| e9 | prompt 0 to 2 | `tabChannelAnnouncement` |
| e10 | frequency `Name` | none |
| e11 | Lake 3's tone row moved to the table end | none |

## 4. Switching a trunked knob between TG Scan (16) and TG Monitor (48)

### 4.1 Flow

The knob editor's receive-mode button offers the mode list; 16 to 48 or 48
to 16 is a SlightChange (section 1.2). `ZoneKnobInfoControl` writes the new
`CurrentMode` and the current prompt into the receive row and re-runs
`initRecvMode` (`ZONE/ZoneKnobInfoControl.cs:399-422`); `TrunkingKnobControl`
runs `SlightChangeRecvMode` (`ZONE/TrunkingKnobControl.cs:346-373`), which
updates titles and visibility and calls `changeRecvMode` on the talkgroup
list, secure, function and binding controls. The detail row, `ChannelID`,
system, protocol, talkgroups and options are kept. The next knob move or
Save runs `checkError` and every sub-control's `Save` (system-creation 3.1).

### 4.2 Rows written on save

| Table | 16 to 48 | 48 to 16 | Evidence |
|---|---|---|---|
| `tabReceivingMode_NewG4` | `CurrentMode 48`; `AlertToneID` per 4.3 | `CurrentMode 16`; `AlertToneID` per 4.3 | `ZoneKnobInfoControl.cs:272-283` |
| `tabP25TSubGroupEnable` | knob's row deleted, then `(ChannelID, '' , '1')` appended: mode 48 forces it | knob's row deleted; re-added only if the binding feature "Sub-Group Feature" is still selected (4.4) | `ZONE/BindingFeatureControl.cs:662-687` |
| `tabP25TCallAlertEnable` | deleted (binding control hidden) | written only if "Call Alert Feature" is chosen | same |
| `tabP25TrunkingSystemKnob` binding columns | saved at the hidden values `ResetMode 0`, `DelayNTime 30`, `EnableTgidStayTime 0`, `TgidStayTime 30`, `DelayNFeature 0`, `DelayNTimeEx 5`, `EnableCarrierOffTime 0`, `CarrierOffDelayTime 5` (the whole control is hidden for 48, `TrunkingKnobControl.cs:208-227`) | the values the now visible controls hold (the unchanged column values) | `BindingFeatureControl.cs:688-727` |
| `tabP25TrunkingSystemKnob` other columns | unchanged: function settings use the same visibility for 16 and 48 (`ChannelFunctionSettingControl.cs:200-245`, `Save :371-418`), secure and Rx-option controls ignore the mode | unchanged | |
| `tabChannelGroup` | the knob's rows deleted and re-appended in list order (every save does this, `ZONE/TrunkingRxGroupListControl.cs:654-715`); same content | same | |
| `tabP25TChannelSubGroup`, `tabP25TCallAlertSUIDGroup` | rewritten from the in-memory maps; empty for every real knob | same | `:654-715` |
| `tabChannelRevertGroup` | **kept if the revert list was visible** (it is hidden for 48 only on first load, `TrunkingKnobControl.cs:217-219`), while `ResetMode` is saved as 0: inconsistent | | |
| `tabOrganization_New` | `FinishDate` | same | |

`tabChannelGroup` semantics are the same for both modes: at least one
talkgroup is required (`TrunkingRxGroupListControl.cs:465-472`, mode
independent) and the converter writes the list identically (s1: no contact
table changes). The UI note reads "Receive from regular talk groups" for 16
and "Receive from all talk groups" for 48 (`TrunkingKnobControl.cs:195-206`);
the real 48 knobs hold 1, 1 and 63 talkgroups.

### 4.3 Voice prompt

`VoicePromptData(mode)` (`View/View.Common/NameValueVModel.cs:230-281`)
offers None (0), the mode's built-ins and the profile's custom prompts:
mode 16 gets 6 (TG Normal Scan) and 8 (TG Priority Scan), mode 48 gets 11
(TG Monitor) (`:47-63`, `View/View.VoicePrompt/DefaultVoicePromptListControl.cs:21-80`).
`initRecvMode` keeps the stored value when it is in the new list, else picks
the list's first entry, None (`ZoneKnobInfoControl.cs:187-207`). So:

| Before | After 16 to 48 | After 48 to 16 |
|---|---|---|
| 0 | 0 | 0 |
| 6 or 8 | 0 | |
| 11 | | 0 |
| custom GUID | kept | kept |

Every real trunked knob has 0, so a PPS switch leaves the prompt at 0.

### 4.4 Binding-feature quirk on 48 to 16

`BindingFeatureControl.init` picks the combo list from the mode at load
time: P25T list (None, Sub-Group, Call Alert) for 16/129, the two-entry
list for anything else, 48 included (`:589-607`), and selects index 1
because the 48 knob has the sub-group-enable row (`:608-619`).
`changeRecvMode` does not reload the list (`:798-804`). After 48 to 16 in
the same session the combo becomes visible with "Sub-Group Feature"
selected, the talkgroup list's sub-group button turns on
(`TrunkingKnobControl.cs:85-93`), and `checkError` refuses to save with "At
least one group should be assigned the sub-group."
(`TrunkingRxGroupListControl.cs:558-563`). The user must set the binding
feature to None, which deletes the row on save. Had the row survived, the
pre-program check would fail with B3 (`CHECK:755-796`; the check skips mode
48 at `:757-760` but not 16) and the radio would get
`TwoToneSubGroupOn 1` on a TG Scan knob (s2b vs s2). Knobset should delete
the row directly.

### 4.5 Real data

The 6 TG Scan and 3 TG Monitor knobs of unit B have identical
`tabP25TrunkingSystemKnob` values in every non-key column, binding columns
included (`ResetMode 0`, `DelayNTime 30`, `EnableTgidStayTime 0`,
`TgidStayTime 30`, `DelayNFeature 0`, `DelayNTimeEx 5`,
`EnableCarrierOffTime 0`, `CarrierOffDelayTime 5`, `AudioSquelch 0`,
`RecordFlag 1`, `AlertMode 1`). The only per-mode difference in the profile
is the sub-group-enable row on each 48 knob. So a switch on these knobs is
exactly: `CurrentMode`, the sub-group-enable row, and possibly the prompt.

### 4.6 Experiments

| Case | Change | Radio tables changed |
|---|---|---|
| s1 | Metro 1: `CurrentMode 48`, sub-group-enable row appended | `tabReceivingMode` (16 to 64), `tabP25TSetting` (`TwoToneSubGroupOn` 0 to 1) |
| s1b | as s1 without the row | `tabReceivingMode` only |
| s2 | Lake 8: `CurrentMode 16`, row deleted | `tabReceivingMode` (64 to 16), `tabP25TSetting` (1 to 0) |
| s2b | as s2, row kept | `tabReceivingMode` only (and B3 would fail in PPS) |
| s3 | s1 with prompt 11 | s1 plus `tabChannelAnnouncement` (`-011`) |
| s4 | s1 with Metro 1's `tabChannelGroup` rows moved to the table end, as PPS's save does | same 2 tables as s1 |

## 5. Converting a knob's type

### 5.1 PPS: delete, then create

Any change between families is a BigChange (section 1.2):
`RecvModeChanged` calls `deleteKnob(zone, k, clearChannelName: false)` and
reopens the knob (`ZONE/ZoneListControl.cs:3565-3578`). With
`clearChannelName` false the receive row keeps `ChannelName`, the new
`CurrentMode` and the old prompt; every detail and child row of the old
`ChannelID` is deleted as in 2.1, scan memberships included. The reopen
creates a fresh detail row with a **new `ChannelID`** and the kind's
defaults (`Row_tabSingleConvCHKnob`, `Row_tabP25TrunkingSystemKnob`,
`Row_tabMultiChannelScan`, `Row_tabAllBandScanCHKnob`, `:3622-3757`); the
new editor resolves the prompt against the new mode (4.3), which turns any
built-in prompt into 0. Cancel reloads every table from disk (`:322-328`).

| Direction | New rows on save | PPS traps |
|---|---|---|
| conventional or scan to trunked | system-creation 3.3/3.4: appended `tabP25TrunkingSystemKnob` default row (first index system, first type-9 protocol), `tabChannelGroup` rows, sub-group-enable row for 48 | a scan knob that used the knob as priority channel is left dangling; a scan list can drop below 2 members |
| trunked or scan to conventional | appended `tabSingleConvCHKnob` default row (conventional-knobs 1.2) with `ConvFreqID` = **the first `tabConvFreqList` row** (`ConventionalSettingControl.cs:444-450`); `tabRecvSaList` rows for tones | in unit B the first row is Elmwood Fire's 154.04 MHz: the converted knob silently shares it until the user picks another; no new frequency row is created |
| trunked to trunked on another system | not a conversion: in place (system-creation 3.5) | |
| conventional to scan | `tabMultiChannelScan` default row; members chosen in the editor | the converted knob also loses its own memberships in other scan lists |

The radio channel ID stays the same in every direction: it is
`zone row x 100 + knob` whichever detail table holds the knob
(system-creation 5.4).

### 5.2 Experiments

| Case | Change | Radio tables changed |
|---|---|---|
| c1 | Lake 4 (CSQ Monitor, scan member) to TG Scan on Metro 1's system with 2 TGs: PPS removal rows without clearing the alias, `CurrentMode 16`, prompt 0, default trunk row, 2 `tabChannelGroup` rows | 16: `tabChannel` (frequency 0), `tabChannelAdvInfo`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabResetModeSetting`, `tabRecvVoiceCallRecordByChannel` (row order), `tabP25TSetting`, `tabKMFSystemByChannel`, `tabCipherKeyAssignment`, `tabP25OTARFeatureControlByChannel`, `tabContactOfChan`, `tabRecvGroupList`, `tabRecvBehaviorByContactID`, `tabReceivingMode` (32 to 16), `tabRepeaterSettingOfChan` (row removed), `tabScanListSettings` (204 removed from the Lake list) |
| c2 | Metro 7 (TG Scan, last trunk row) to CSQ Monitor on a new 155.475 MHz row | 21: 7 bundle tables (`tabChannel`, `tabChannelAdvInfo`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabReceivingMode` 16 to 32, `tabRecvVoiceCallRecordByChannel`, `tabResetModeSetting`), the 4 P25 per-channel tables (rows removed), 9 contact tables (the knob's talkgroup contacts go), `tabRepeaterSettingOfChan` (row added) |
| c2b | Metro 1 (first trunk row) to CSQ Monitor | 23: as c2 plus the 4 system tables (SigIDs renumber) |

### 5.3 Other slight changes (for reference)

| Change | Rows | Radio |
|---|---|---|
| Monitor 33 to Selective Call 1 (CSQ) | `CurrentMode`; `SquelchOption` saved 0 instead of 3 (`ConventionalReceiveOptionControl.cs:265`); voice storage hidden, `RecordFlag` forced 1 | r6a: `tabReceivingMode`, `tabChannelAdvInfo`, `tabFunctionOnOffOfChan` |
| Normal 4 to Priority 36 | `CurrentMode`; `PriorityScanType 1`; `FirstPriorityCH` = the selected member, the first by default (`ScanListControl.cs:256-277,437-441`) | not measured |
| Priority 36 to Normal 4 | `PriorityScanType 0`, `FirstPriorityCH ''`, `SamplingFlag 0`, `DwellTime 6` | not measured |

## 6. Radio tables per operation

Measured unions over the experiments above, against the `*_TABLES`
constants in `crates/g5-codeplug/src/edit.rs`:

| Operation | Tables seen | Covered by |
|---|---|---|
| remove a trunked knob (r1, r1b, r2) | 32 | `KNOB_CHANNEL_TABLES` + `TALKGROUP_EDIT_TABLES` + `SYSTEM_EDIT_TABLES` (exactly; `tabP25OTARFeatureControl` and `tabZone` allowed but not seen) |
| remove a conventional knob (r3, r3b, r3c, r4k, r6b, r8) | 20 | `CONV_KNOB_CHANNEL_TABLES` + `tabScanListSettings` (exactly) |
| remove a scan knob (r5, r5b) | 8 | new `SCAN_KNOB_TABLES` (below), plus `tabZone` when the zone empties |
| edit a conventional knob (e1-e11) | 6 | new `CONV_EDIT_TABLES`; plus `RENAME_TABLES` when renamed |
| switch 16/48 (s1-s4) | 3 | new `MODE_SWITCH_TABLES` |
| conventional to trunked (c1) | 16 | `KNOB_CHANNEL_TABLES` + `TALKGROUP_EDIT_TABLES` + `SYSTEM_EDIT_TABLES` + `tabRepeaterSettingOfChan`, `tabRecvSaList`, `tabScanListSettings` |
| trunked to conventional (c2, c2b) | 27 | `CONV_KNOB_CHANNEL_TABLES` + `KNOB_CHANNEL_TABLES` + `TALKGROUP_EDIT_TABLES` + `SYSTEM_EDIT_TABLES` |

Several "changed" tables in the conversions are row-order changes only: the
converter writes conventional knobs before trunked ones, so a knob that
changes family moves within `tabResetModeSetting`,
`tabRecvVoiceCallRecordByChannel` and similar tables.

## 7. Contradictions with earlier notes

1. system-creation 7.1 says contact IDs follow `tabChannelGroup` row order.
   Experiment s4 moved the first knob's whole block to the end of the table
   with no radio change: what matters is knob row order and the order of a
   knob's own rows, not the block's position in the table.
2. system-creation 3.4 calls 16/48 a slight change where "48 always writes"
   the sub-group-enable row. True; the reverse direction does not delete it
   by itself: PPS blocks the save until the user clears the binding feature
   (section 4.4).
3. conventional-knobs 8.2 and system-creation 7.4 describe the empty-knob
   precondition as "no detail row"; section 2.6 r7 shows why: a leftover
   detail row on a mode-0 knob is still converted into a radio channel.
4. data-model 3.3 says the second priority channel is "not set by the UI";
   confirmed, and both real priority-scan rows have `SecondPriorityCH ''`.
   It also implies `ScanOrder` orders the list; nothing reads it (2.2).

## 8. Implementation spec

### 8.1 Common rules

Everything in system-editing 9.1, system-creation 7.1 and conventional-knobs
8.1 applies: one profile transaction per batch, TEXT values, `''` never
NULL, lowercase hyphenated GUIDs, append new rows, update and delete in
place (no full-table rewrites), `tabOrganization_New.FinishDate` on save, no
zip entries added or removed. In addition:

- Identify the knob by zone name (exactly one `tabZone_New` row) and knob
  1..8 whose `tabReceivingMode_NewG4` row exists.
- **Classify** the knob from `CurrentMode` and require exactly one detail
  row of the matching kind and none of the others: 1/33 `tabSingleConvCHKnob`;
  16/48 `tabP25TrunkingSystemKnob`; 4/36 `tabMultiChannelScan`. Refuse 8,
  65, 68, 97, 128, 129 (Knobset's converter refuses free scan and advanced
  knobs; silent scan and radio-compatible modes have no verification).
- For conventional knobs require `ProtocolSettingID '0000000000000000'`
  (CSQ); the converter refuses everything else (`RS:899-901`).
- Refuse when the knob uses features the converter refuses or Knobset does
  not model: `refuse_sub_features` (sub-groups, call-alert groups, revert
  talkgroups, in `crates/g5-codeplug/src/edit.rs`), plus
  `tabP25TCallAlertEnable` with `EnableCallAlert 1` and any
  `tabChannelGroupList` row.
- Applying order in `edit_set` (`crates/g5-core/src/write.rs`):
  system edits, then removals, then mode switches and conventional edits,
  then new knobs, then system deletes, then renames and talkgroup edits. A
  removal and an add on the same knob in one batch is the conversion of
  8.5; the "another change in the same batch" refusal (the `touched` check)
  must allow exactly that pair.

### 8.2 Remove a knob

Inputs: zone, knob. Optional, for scan side effects: per affected priority
scan, the replacement priority member.

Validation (refuse otherwise):

1. Section 8.1 classification; the knob is active (`CurrentMode != 0`).
2. Not the last active knob in knobs 1..8 of the whole profile (A1).
3. For a conventional knob, for every scan knob whose `tabMultiCHScanList`
   holds its `ChannelID` (list them by zone and knob in the message):
   - at least 2 members must remain (E1); otherwise refuse and name the scan
     knob (the user removes or edits the scan knob first);
   - if it is that scan's `FirstPriorityCH` (or `SecondPriorityCH`), a
     replacement must be given or defaulted: default to the first remaining
     member in row order, which is what PPS's scan editor does when the
     priority member is removed (`ScanListControl.cs:372-397`); report the
     choice. The replacement must be a remaining member.
4. `tabEmergencySetting.ChannelID` is not the knob's `ChannelID` (it is
   `''` in real data; refuse rather than leave a dangling reference).

Rows, one transaction (`C` = the knob's `ChannelID`, absent for scan knobs):

1. `UPDATE tabReceivingMode_NewG4 SET CurrentMode '0', AlertToneID '0',
   ChannelName ''` for the knob. Leave `AlertFileName`.
2. `DELETE` by `ZoneID`/`KnobIndex` from `tabSingleConvCHKnob`,
   `tabP25TrunkingSystemKnob`, `tabMultiChannelScan`, `tabMultiCHScanList`,
   `tabAllBandScanCHKnob`, `tabAdvancedConvCHKnob`.
3. If `C`: `DELETE ... WHERE ChannelID = C` from `tabChannelGroup`,
   `tabChannelGroupList`, `tabRecvSaList`, `tabP25TSubGroupEnable`,
   `tabP25TChannelSubGroup`, `tabChannelRevertGroup`,
   `tabP25TCallAlertEnable`, `tabP25TCallAlertSUID`,
   `tabP25TCallAlertSUIDGroup`, `tabP25TChannelRevertSubGroup`,
   `tabMultiCHScanList`, `tabAllBandScanBandParam`,
   `tabAllBandScanProtocolParam`.
4. For each priority scan from validation step 3:
   `UPDATE tabMultiChannelScan SET FirstPriorityCH = <replacement>` (or
   `SecondPriorityCH ''`). Leave the other members' `ScanOrder` as is
   (gaps are harmless, 2.2).
5. Conventional knob: delete its `tabConvFreqList` row when no remaining
   `tabSingleConvCHKnob` or `tabAdvancedConvCHKnob` row references it and
   it is not the table's only row (PPS keeps one row, conventional-knobs
   2.6). Radio: no change (r3f). Knobset creates one row per knob
   (conventional-knobs 8.6), so the row is effectively the knob's.
6. `FinishDate`.

Leave the system, protocol setting and talkgroup tables alone. An unused
system stays in the index; the user deletes it separately (system-creation
7.3).

Radio write guard:

| Kind | Allowed |
|---|---|
| trunked (16/48) | `KNOB_CHANNEL_TABLES` + `TALKGROUP_EDIT_TABLES` + `SYSTEM_EDIT_TABLES` |
| conventional (1/33) | `CONV_KNOB_CHANNEL_TABLES` + `tabScanListSettings` |
| scan (4/36) | `SCAN_KNOB_TABLES` |

### 8.3 Edit an analog conventional knob in place

Inputs (each optional; absent = unchanged): `rx_hz`, `bandwidth`,
`squelch` (`carrier` / `ctcss` / `dcs`, as `conv_knob::Squelch`),
`audio_squelch` 0..2, `emphasis` 0/1, `compander` 0/1, `alert_mode` 1..4,
`record_flag` 0..2, `voice_prompt` 0/2, `freq_id` (a new GUID, needed only
when a new frequency row is written).

Validation: knob mode 33 with CSQ (mode 1 is out of scope: its
`RecordFlag` and `SquelchOption` rules differ, 5.3); frequency and
bandwidth rules of conventional-knobs 8.2 (Rx inside a model Rx range,
10 Hz steps, bandwidth 1 or 2, 3 held back); CTCSS 670..2550 tenths, DCS
from the 104-code list; refuse a squelch edit when the knob has more than
one `tabRecvSaList` row (no real knob has; Knobset models one tone).

Rows:

1. **Frequency or bandwidth.** Let `F` = the knob's `ConvFreqID`. Count
   references to `F` in `tabSingleConvCHKnob` and `tabAdvancedConvCHKnob`.
   - Exactly 1 and the row exists: `UPDATE tabConvFreqList SET RxFreq = rx,
     TxFreq = rx, RxBandWidth = bw, TxBandWidth = bw WHERE FreqID = F`
     (keep `Name`, `TransmitMode`).
   - Shared, or `F` dangles: append `(freq_id, name, '1', rx, rx, bw, bw)`
     with the naming rule of conventional-knobs 8.3 step 1, and `UPDATE
     tabSingleConvCHKnob SET ConvFreqID = freq_id` for this knob only. Refuse
     at 1024 rows. The other knobs keep `F`.
2. **Squelch.** `DELETE FROM tabRecvSaList WHERE ChannelID = C`, then
   append at most one row as in conventional-knobs 8.3 step 4. (Equivalent
   to PPS's delete and append; position is radio-neutral, e11.)
3. **Knob options.** `UPDATE tabSingleConvCHKnob SET` only the given
   columns among `AudioSquelch`, `Emphasis`, `Compander`, `AlertMode`,
   `RecordFlag`. Do not rewrite hidden columns.
4. **Prompt.** `UPDATE tabReceivingMode_NewG4 SET AlertToneID`.
5. `FinishDate`.

Radio write guard, a new constant:

```text
CONV_EDIT_TABLES = tabChannel, tabChannelAdvInfo, tabRecvSaList,
  tabAlertModeSetting, tabRecvVoiceCallRecordByChannel,
  tabChannelAnnouncement
```

Union only the tables the edit's fields can reach if a tighter guard is
wanted: frequency/bandwidth `tabChannel`; squelch `tabRecvSaList`;
`AudioSquelch`/`Emphasis`/`Compander` `tabChannelAdvInfo`; `AlertMode`
`tabAlertModeSetting`; `RecordFlag` `tabRecvVoiceCallRecordByChannel`;
prompt `tabChannelAnnouncement`.

### 8.4 Switch TG Scan 16 and TG Monitor 48

Inputs: zone, knob, new mode (16 or 48), optional voice prompt (0 or the new
mode's built-in: 6 or 8 for 16, 11 for 48; default 0). May be combined with
a system change and talkgroup list in the same `KnobSystemEdit`
(`assign_knob_system` currently refuses `m != e.mode`,
`crates/g5-codeplug/src/knob_system.rs:143-145`).

Validation: knob is 16 or 48 with its trunk row; `refuse_sub_features` and
no call-alert enable row; for 16 to 48, the binding columns are at their
hidden-save values (`ResetMode 0`, `DelayNTime 30`, `EnableTgidStayTime 0`,
`TgidStayTime 30`, `DelayNFeature 0`, `DelayNTimeEx 5`,
`EnableCarrierOffTime 0`, `CarrierOffDelayTime 5`), else refuse naming the
column (PPS would reset them, which would also change
`tabResetModeSetting`; no real knob has other values); the talkgroup list
still passes `check_knob_list`.

Rows (`C` = the knob's `ChannelID`, unchanged):

1. `UPDATE tabReceivingMode_NewG4 SET CurrentMode = new, AlertToneID =
   prompt`. A custom GUID prompt is kept when the caller passes none.
2. `DELETE FROM tabP25TSubGroupEnable WHERE ChannelID = C`; for 48 then
   append `(C, '', '1')`.
3. Leave `tabP25TrunkingSystemKnob` and `tabChannelGroup` in place (do not
   re-append the talkgroup rows; s4 shows PPS's re-append is radio-neutral,
   but it is churn).
4. `FinishDate`.

Radio write guard, a new constant:

```text
MODE_SWITCH_TABLES = tabReceivingMode, tabP25TSetting,
  tabChannelAnnouncement
```

plus `SYSTEM_EDIT_TABLES` + `TALKGROUP_EDIT_TABLES` when the same edit
changes the system or talkgroups.

### 8.5 Convert a knob's type

Do it as PPS does: removal of the old kind without clearing the alias, then
creation of the new kind on the now empty position, in one transaction.

Inputs: zone, knob, the full creation inputs of the new kind
(`KnobSystemEdit` for trunked, `ConvKnobEdit` for analog conventional), and
the scan replacement choices of 8.2.

Validation: 8.2 steps 1, 3 and 4 for the old knob (step 2 does not apply:
the knob stays active); the creation validation of the new kind, except
that the position's precondition "mode 0 and no detail rows" is checked
after the removal rows, inside the transaction. Refuse a conversion to the
same family (use 8.3 or 8.4).

Rows:

1. 8.2 steps 2 to 5 (detail and child rows of the old `ChannelID`, scan
   memberships and priority fix-ups, the orphaned frequency row).
2. Receive row: `CurrentMode` = new mode, `ChannelName` kept unless a new
   alias is given, `AlertToneID` = the new kind's prompt input (default 0;
   PPS ends at 0 for built-ins).
3. The creation rows of the new kind with a **new `ChannelID`**:
   system-creation 7.4 (trunk row, `tabChannelGroup`, sub-group-enable for
   48) or conventional-knobs 8.3 (new frequency row, knob row, tone row).
   Never default to the profile's first frequency row.
4. `FinishDate`.

Radio write guard:

| Direction | Allowed |
|---|---|
| conventional to trunked | `KNOB_CHANNEL_TABLES` + `TALKGROUP_EDIT_TABLES` + `SYSTEM_EDIT_TABLES` + `tabRepeaterSettingOfChan`, `tabRecvSaList`, `tabScanListSettings` (system-creation's `CONV_REMOVAL_TABLES` without the free-scan tables) |
| trunked to conventional | `CONV_KNOB_CHANNEL_TABLES` + `KNOB_CHANNEL_TABLES` + `TALKGROUP_EDIT_TABLES` + `SYSTEM_EDIT_TABLES` |

Scan to anything and anything to scan are out of scope until Knobset can
create scan knobs (zones-and-scan.md 9.6).

### 8.6 New write-guard constants

Next to the existing `*_TABLES` constants in `crates/g5-codeplug/src/edit.rs`:

```text
SCAN_KNOB_TABLES = tabReceivingMode, tabChannelAnnouncement,
  tabFunctionOnOffOfChan, tabFunctionParamOfChan, tabResetModeSetting,
  tabAlertModeSetting, tabRecvVoiceCallRecordByChannel,
  tabScanListSettings, tabZone (only when the zone empties)
CONV_EDIT_TABLES = tabChannel, tabChannelAdvInfo, tabRecvSaList,
  tabAlertModeSetting, tabRecvVoiceCallRecordByChannel,
  tabChannelAnnouncement
MODE_SWITCH_TABLES = tabReceivingMode, tabP25TSetting,
  tabChannelAnnouncement
SCAN_MEMBER_TABLES = tabScanListSettings (added to every operation that
  drops a conventional knob from a scan list or moves a priority channel)
```

`edit_set` unions the sets per batch as today. Removing a knob
that empties its zone also needs `tabZone`; it is already in
`KNOB_CHANNEL_TABLES` and `CONV_KNOB_CHANNEL_TABLES`.

### 8.7 On-pager tests and open risks

1. **Removal** (r1, r3): program unit B with one trunked and one
   conventional knob removed. Check the empty position on the pager (what it
   shows and announces), and that the pager does not get stuck when it was
   sitting on that position at write time.
2. **Scan side effects** (r3, r4k): NOAA Scan with 6 members, and the Lake
   priority scan with its priority moved to Lake 4. Confirm both scan and
   that the priority channel is sampled.
3. **SigID renumbering** (r1, r2, c2b): removing the first trunk row or a
   system's only knob shifts SigIDs of later knobs. Same open question as
   system-creation 7.5 item 5; confirm every remaining trunked knob still
   affiliates and receives.
4. **A zone with no active knob** (r8): `tabZone.ModifyByDevice 1`. Confirm
   the pager skips or shows the zone sensibly. Until then, consider
   refusing removals that empty a zone.
5. **16 to 48 and 48 to 16** (s1, s2): confirm a switched knob behaves like
   a native one: 48 receives all talkgroups of the system, 16 only the
   list; `TwoToneSubGroupOn` follows the mode as on the real knobs.
6. **Conversions** (c1, c2): conventional to trunked is system-creation's
   deferred case (b); trunked to conventional has not been on a pager.
7. **In-place frequency edit** of a scan member (e1 on Lake 4): confirm the
   scan uses the new frequency (the ZK is unchanged, so it should).
8. Not a pager test: open a Knobset-edited profile in PPS on Windows (a
   removal with a priority fix-up, a 48 to 16 switch, a conversion), save,
   and run PPS's pre-program check. It should report nothing and rewrite
   the same rows.
9. Free scan, advanced and silent-scan knobs: no real data and no
   converter support; removal of such knobs stays refused.

### 8.8 PPS behavior Knobset should skip

| PPS behavior | Why skip |
|---|---|
| Removing a knob that is a scan's priority channel without fixing `FirstPriorityCH` | E3 blocks programming; the radio would get a priority scan with no priority channel (r4) |
| Removing a member that leaves a scan list below 2 members, without warning | E1 blocks programming |
| Removing a knob by resetting the mode only (never done by PPS, easy to do by hand) | the detail rows still convert to a nameless radio channel (r7) |
| Leaving the removed knob's frequency row in the pool | harmless but uses one of 1024 rows; Knobset's rows are one per knob |
| Defaulting a converted conventional knob to the profile's first frequency row | silently shares another knob's frequency (Elmwood Fire's in unit B) |
| Keeping the sub-group-enable row after 48 to 16, or blocking the save until the user clears it | B3 failure and `TwoToneSubGroupOn 1` on a TG Scan knob; delete the row |
| Keeping revert rows on 16 to 48 while saving `ResetMode 0` | inconsistent data; Knobset refuses knobs with revert rows |
| Resetting binding columns silently on 16 to 48 | changes `tabResetModeSetting`; refuse non-default values instead |
| Deleting and re-appending a knob's `tabChannelGroup` and `tabRecvSaList` rows on every save | radio-neutral (s4, e11) but needless churn; update in place |
| Editing a shared frequency row (Frequency page) | retunes every knob on it; write a new row for the edited knob |
| Leaving `TxFreq` at the band minimum after an Rx edit | meaningless; write Tx = Rx |
| Not removing `tabAdvancedChannelGroup` rows with an advanced knob | dangling rows; out of scope while advanced knobs are refused |
