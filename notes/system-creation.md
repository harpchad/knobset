# Creating, deleting and assigning P25 trunking systems: PPS row-level behavior

Scope: what PPS V00-04-00-48 writes when the user creates a new blank P25
trunking system inside an open profile, deletes one, and puts a system on a
knob (an empty knob, a conventional knob, or a trunking knob that switches
systems). Sibling of [system-editing.md](system-editing.md), which covers
editing an existing system's properties, sites, control channels and
talkgroups; persistence mechanics described there (section 1) are not
repeated.

Evidence paths are relative to `decomp/`. Short forms: `P25T/` =
`View/View.P25TSystem/`, `ZONE/` = `View/View.Zone/`, `SQL/` =
`DataBase/Unication.SQLite/`, `CONV/` =
`View/View.ProgramVersion/PPSU4ParamConverter_01P41b.cs`. Real data: unit B's
earliest (PPS-written) backup and unit A's backup, both `u3db_PC.zip`
unpacked to `scratch/syscreate/unit{A,B}/`. Tools in
`scratch/syscreate_tools/`: `loadall.sh` (lossless SQLite 3 copies via
`conv141/tools/load2.py`), `knobs.py` (knob and index rows, structural
output), `sysdbs.py` (each system DB against the template and against
creation defaults), `exp.py` (applies the PPS row changes for each case to a
copy of unit B's profile, converts with Knobset's 1.41c converter and diffs
the radio DB against the unmodified conversion).

## 0. Summary

- **Creating a system is two writes at two times.** Choosing "Add <band>
  System" copies `DataBase\P25TSystem.db` to `<profile>\<guid>.db` and inserts
  a default system row, `Site 1`, one control channel at the band's lowest Rx
  frequency and the band row, **before** the editor dialog opens. OK rewrites
  the four system tables with the dialog's values; the index row in
  `tabP25TrunkingSecurity` is written only when the user leaves the page.
  **Cancel leaves the new file on disk.** Both real profiles carry two such
  orphans, byte-for-byte what `newP25Trunking` produces.
- GUIDs are .NET `Guid.NewGuid().ToString()`: lowercase, hyphenated. File
  name, `P25TrunkingID`, `DBFileName` and `tabP25TrunkingSystem.ID` are the
  same GUID.
- The band is picked from a menu with one entry per band of the profile's
  model (two on every G5); the system gets a single 2-letter `BandTypeCode`,
  never the 4-character code.
- The template already holds the "All" talkgroup (`GroupID` 16 zeros,
  TGID 65535), so a new system has `TGIDCount 1`. There is no "All Call"
  insert on this path (the RadioReference API importer adds one only when the
  imported list is empty).
- **Deleting a system never checks knobs.** The file is deleted immediately,
  the index row on page save. Knobs keep the dangling `P25TrunkingID`, the
  zone grid shows `[System Error]` and the pre-program check (C1) blocks
  programming. PPS also refuses to delete the last remaining system.
- **Knob assignment is a knob-editor save.** An empty knob gets a mode via
  `SelectKnobReceiveModeForm`; a conventional knob is switched with the
  "change receiving mode" dialog, which deletes all of the old knob's rows
  (including its membership in other scan lists) and builds a fresh trunking
  row with a **new ChannelID**. Trunking modes are 16 (TG scan), 48 (TG
  monitor) and 129 (radio-compatible trunking).
- The new `tabP25TrunkingSystemKnob` row is **appended**, gets the first
  index row as its system and the **first existing type-9 protocol setting**.
  The knob editor never creates a protocol setting; with none in the profile
  the knob cannot be saved.
- Every real P25T knob (16 rows over both units) has exactly the default
  values PPS's controls produce, so the defaults in section 3.2 are verified.
- **Switching a knob to another system keeps only talkgroups whose GroupID
  also exists in the new system**, in practice just the shared "All" row. The
  user must pick at least one talkgroup before the editor lets go.
- A trunking knob must have at least one talkgroup: enforced in the editor
  (`checkError`) and again before programming (B3, needs a TG with a TGID).
  Limit 64 talkgroups per knob list; no limit on trunking knobs beyond 64
  zones by 8 knobs.
- Conversion experiments: a created but unused system changes nothing in the
  radio DB; a new trunking knob touches 28 radio tables (32 when its system
  was not on any knob yet); switching an existing knob's system renumbers
  SigIDs and contact IDs of later knobs.

## 1. Creating a blank system

### 1.1 Entry point and band choice

`button_add` has no Click handler; its `MouseClick` (`P25T/P25TSyetemTotal.cs:2010`)
runs `button_add_MouseDown` (`:1347-1388`), which builds a context menu:

- one "Add `<Name>` System" item per band returned by `addBandTypeMenuData`
  (`:1563-1593`): for a profile, the model's first and second band
  (`ModelNumberInfo.FirstBandTypeInfo` / `SecondBandTypeInfo`); for the
  library (`P25TTotal.db`), every band in `Data_G4A.tabRFBandTypeInfo`. The
  item's `Tag` is the 2-letter code (`SX`, `VX`, ...), and `Name` is
  `name + " " + Comment`, e.g. "700/800 (763~776, 851~870 MHz)"
  (`View/View.Profile/RFBandTypeInfo.cs:21`);
- "Add System from Resources" (library copy, data-model 5.2 item 4);
- one RadioReference site-CSV import item per band, and "update selected
  item" when a row is selected (section 1.6).

`button_add_Click` (`:742-788`) reads the band from the clicked item's `Tag`;
called without a menu item (the grid's hidden Add entry, `:164,206-209`) the
code would be `''`. The add button is enabled while the list has fewer than
1024 rows (`MaxCount`, `:32,399`).

### 1.2 The new index row (in memory)

Built at `:746-763` before anything is written:

| Column | Value | Note |
|---|---|---|
| `P25TrunkingID` | new GUID, lowercase | `Guid.NewGuid().ToString()` |
| `DBFileName` | same GUID | file is `<profile>\<P25TrunkingID>.db` |
| `SystemName` | `"Trs Sys N"`, smallest N not already an exact `SystemName` | `CommonFunction.GetNextName` (`View/View.Common/CommonFunction.cs:637-673`) |
| `WacnID`, `SystemID` | `1`, `1` | |
| `SiteCount`, `ControlCHCount` | `0`, `0` | replaced by the dialog's counts on OK |
| `TGIDCount` | `1` | the template's "All" row |
| `SubGroupCount`, `IndividualCount` | `0`, `0` | |
| `SUID` | `''` | |
| `SystemCreateType` | `2` | every creation path writes 2 |
| `SystemSource` | `''` | |
| `BandTypeCode` | menu tag, 2 letters | |

Helper columns (`Flag`, `Select`, `Detail`, `TGIDDisplay`, ...) are dropped
on save (system-editing 1.2).

### 1.3 File creation (`newP25Trunking`, immediate)

`newP25Trunking(profilePath, row)` (`:970-1025`):

1. Folder = profile path minus `.db` (created if missing, i.e. the first
   system of a fresh profile creates `<GUID>\`); for the library, the folder
   of `P25TTotal.db`.
2. `File.Copy` of `PathsManagement.P25TSystemBaseDBPath()` =
   `%ProgramData%\Unication\<PpsDatabasePath>\DataBase\P25TSystem.db`
   (`View/View.ResourcesDefine/PathsManagement.cs:422-425`), i.e. the
   installed template, identical to
   `extract/msi/.../DataBase/P25TSystem.db` (MD5 `728b3062...`). A copy
   failure is logged and returns `''`.
3. One `ExeNonQuery` batch of up to four INSERTs (`:996-1022`):

| Table | Row written | Evidence |
|---|---|---|
| `tabP25TrunkingSystem` | `ID` GUID, `SystemCreateType '2'`, `SystemSource ''`, `Name` (quotes doubled, `View/View.Common/Common.cs:447-456`), `ConverageType '0'`, `WacnID '1'`, `SystemID '1'`, `FullSpectrumScanFlag '0'`, `FullSpectrumScanRanges '1'`, the nine range columns `''`, `SiteTrunkingFlag '0'`, `ErrorRateThreshold '5'`, `SUID ''` | `:998-1011` |
| `tabP25TrunkingSite` | `('1','Site 1','1','1','')` | `:1012` |
| `tabRFBandTypeInfo` | the `Data_G4A.tabRFBandTypeInfo` row whose `Code` equals `BandTypeCode`: `(ID, Code, Name, MinFreq, MaxFreq, Comment)`, e.g. `('8','SX','700/800','763000000','870000000','(763~776, 851~870 MHz)')`; skipped when no row matches (4-char code or `''`) | `:1014-1019` |
| `tabP25TrunkingControlCH` | `('1', RxMin, '1', RxMin, '1', '')` with `RxMin` = the band's Rx range 0 minimum (`RFBandTypeInfo.RxMinFreq`, `RFBandTypeInfo.cs:42`): 763000000 for SX, 136000000 for VX, `0` if no band matched | `:1020` |

Everything else stays as copied from the template: the "All" talkgroup
(`'0000000000000000','All','65535','','0','2','0','0','0','1','9999','0','000000000000','0','65535'`),
`tabP25TSignaling` (`ID '000000000000'`, 5000/1000/0/133/200/2500/0/1/1000/2000/5000/1500),
13 `tabEditPermission` rows, `tabVersion 'DBV2.01'`, empty
`tabP25TrunkingSiteControlCH`, `tabSubGroup`, `tabMember_New`, `tabSUID`.

### 1.4 The editor dialog

The four system tables are loaded back (`getP25TSystemTable`, `:1027-1040`)
and handed to `EditSystemForm` with `index -1` (`:765-769`), the same editor
system-editing section 2 describes. Defaults shown: alias `Trs Sys N`, WACN 1,
System ID 1 (hex spinners), site trunking off, SUID unchecked, 1 site, 1
control channel, full spectrum scan off.

- **OK** (`P25T/EditSystemForm.cs:46-56`): `NewSystemControl.checkData`
  (`P25T/NewSystemControl.cs:211-235`: alias non-blank, unique among index
  `SystemName` trimmed and case-sensitive; the control-channel rule is dead
  code, system-editing 2.1) then `Save` (`:243-268`), which fills the index
  row (`SystemName`, `WacnID`, `SystemID`, `SiteCount`, `ControlCHCount`,
  `SUID`) and copies `Name`, `WacnID`, `SystemID`, `SiteTrunkingFlag`, `SUID`
  into the system row. Back in `button_add_Click` (`:770-786`):
  `saveP25TTable` rewrites `tabP25TrunkingSystem`, `tabP25TrunkingSite`,
  `tabP25TrunkingControlCH`, `tabP25TrunkingSiteControlCH` (delete-all plus
  insert, system-editing 1.2), then the index row is **appended** to the
  in-memory index table.
- **Cancel** (`EditSystemForm.cs:58-61`): nothing else happens. The file from
  1.3 is not deleted and no index row exists for it.

If the user never touches the control-channel list, the creation row stays:
`RxFreq` = band minimum and `TxFreq` = band minimum (system-editing 4).

### 1.5 Profile writes

The index table is rewritten in full when the page is left or the toolbar
Save is pressed (`P25TSyetemTotal.Save`, `:591-607`, via
`BasePageControl.EnableExist`, `View/View.BaseControl/BasePageControl.cs:527-560`),
followed by `tabOrganization_New.FinishDate` = local `yyyy-MM-dd HH:mm:ss`
(`BasePageControl.cs:873-890`). The new row is the last row. No other
profile table changes: no zone, knob, protocol setting or talkgroup
selection is created.

`tabEditPermission` is copied from the template and never read
(system-editing 2.5).

### 1.6 Other ways a new system DB appears

| Path | Differences from 1.3/1.4 | Evidence |
|---|---|---|
| RadioReference site CSV, "add `<band>` System" | index row from `newAddP25TRow` (same defaults, alias "Trs Sys N" or the dialog's, WACN/SysID from the dialog), `newP25Trunking` with the creation rows, then `ImportCsvDataToSystem` **replaces** sites and CCs (CC bandwidth 0, `TxFreq 0`); `tabRFBandTypeInfo` and the "All" row stay; no editor dialog, no orphan on cancel (file made only after OK) | `P25T/P25TSyetemTotal.cs:1424-1447,1467-1487,1489-1561` |
| RadioReference SOAP API | `newP25Trunking(createSql: false)`: template copy only, then its own system row (`FullSpectrumScanRanges 0`), sites, CCs with links, and a **cleared** `tabGroup_New` refilled from RadioReference (plus "All Call" 65535 only if nothing was imported); no `tabRFBandTypeInfo` row; index `SystemSource` `RadioReference_<sid>`, system row `RadioReference-<sid>`; optional new zone with knob 1 in mode 16 | `View/View.RadioReference/RadioReferencehandler.cs:151-430` |
| Paste, resource add, export-zip import | `File.Copy` of an existing system DB under a new GUID; `SystemSource 'resource_<old id>'`, `SystemCreateType 2`; **talkgroup GroupIDs are copied unchanged** (matters for 3.5) | `:235-382,628-668` |

### 1.7 Limits and validation

| Rule | Value | Evidence |
|---|---|---|
| systems per profile (and library) | 1024 (add/paste disabled at 1024) | `:32,237,363,399` |
| alias | 1..14 chars, not whitespace-only, unique (trimmed, case-sensitive) | `NewSystemControl.cs:211-229,408`, `CommonFunction.cs:728-739` |
| WACN / System ID | 0..0xFFFFF / 0..0xFFF; **no uniqueness** across systems | `NewSystemControl.cs` designer; real data: one WACN/SysID pair is shared by four systems, another by three |
| SUID | `''` or 0..16777215 | system-editing 2.1 |
| at least one control channel | not enforced (dead code) | system-editing 2.1 |
| band | must be one of the model's bands (menu) | `:1563-1593` |

## 2. Deleting a system

`button_delete_Click` (`P25T/P25TSyetemTotal.cs:812-879`):

1. Requires a selection and a yes to "delete?" (`View/View.BaseControl/BaseControlDefine.cs:32-37`).
2. **Refuses when the list has exactly one row** (`:816`), and when every
   row is selected it keeps one (`:821-824`); the button is disabled at one
   row (`:400`). Once a profile has a system, PPS never lets it drop to zero.
3. If the system clipboard holds copied rows, the deleted IDs are removed
   from it (`:828-869`).
4. `deleteRows` → `deleteTrunking` (`:881-900`): `File.Delete` of
   `<profile>\<id>.db` **immediately**; errors are only logged.
5. `dgv.DeleteRows` removes the index rows from the bound table
   (`Control/Unication.Controls/PPSDataGridView.cs:883-904`); the table is
   rewritten on page save, then `FinishDate` (section 1.5).

What is **not** done: no look at `tabP25TrunkingSystemKnob`, no change to
any knob, talkgroup selection, zone or protocol row. Protocol settings, by
contrast, refuse deletion while a knob uses them
(`View/View.ProtocolSetting/ProtocolList.cs:710-760`).

Consequences for a knob that used the deleted system:

| Where | Effect | Evidence |
|---|---|---|
| zone grid | knob text gets `[System Error]` | `ZONE/ZoneListControl.cs:481-500` |
| knob editor | the system combo cannot select the dangling ID, so `SelectedValue` is null and `checkError` demands "Please select P25 Trunking System"; once a system is picked, only talkgroups whose GroupID exists there survive (3.5) | `ZONE/KnobTrunkingSystemControl.cs:134-170,211-229`, `View/View.Common/CommonFunction.cs:1162-1186` |
| pre-program check | C1 "system not found" (index row missing, or file missing) blocks programming | `View/View.ProfileCheck/ProfileCheckHandler.cs:714-730` |
| converter | would open the missing file and fail (never reached because of C1) | converter-1.41 2.4 |

The only cascade PPS has lives in the model-change path:
`ProgramController.updateTrunkingFreqList` deletes, for every index row whose
file is missing, the knobs using it and all their per-ChannelID rows and the
index row (`View/View.ProgramVersion/ProgramController.cs:770-873`). It runs
on the conversion copy when the profile model differs from the pager's, and
it does not reset `tabReceivingMode_NewG4`, so those knobs would keep mode 16
with no detail row.

## 3. Assigning a system to a knob

### 3.0 Receive modes and knob tables

`tabReceivingMode_NewG4.CurrentMode` (data-model 3.0): 0 none, 1 selective
call, 33 monitor, 128 radio-compatible, 4/36/68 scan, 8 free scan, 16
trunking TG scan, 48 trunking TG monitor, 129 radio-compatible trunking
(128 plus "System Type: Trunking", only offered when the model supports
P25T). The UI lists 16 and 48 only for models with `EnableP25T`
(`View/View.Common/NameValueVModel.cs:283-358`). The converter maps 16→16,
48→64, 129→128 (converter-1.41 2.5). Modes 16, 48 and 129 all use
`TrunkingKnobControl` and `tabP25TrunkingSystemKnob`
(`ZONE/ZoneListControl.cs:3483-3493`).

Mode 16 is what the RadioReference importer uses for its knob
(`RadioReferencehandler.cs:370`).

### 3.1 Knob editor mechanics

`ZoneListControl` loads 26 profile tables into memory (`:207-263`). Editors
change those DataTables; nothing reaches disk until `savePrivate`
(`:333-387`), which rewrites 20 tables in full (delete-all then insert, in
DataTable row order; batches of 5000, retried with 1000 and 300). It runs
when the user moves to another knob or zone (`ZoneKnobChangedEventHandler`,
`:3582-3608`: `checkError`, then the panel's `Save`, then `savePrivate`), on
page leave or toolbar Save (`Save`, `:389-401` via `EnableExist`), and
after a knob delete. **`checkError` gates every save** (`BasePageControl.cs:527-560`).
Cancel with a pending new knob deletes it again, or for a mode change
reloads every table from disk (`:309-331`).

Detail rows are created on demand by `Row_tabP25TrunkingSystemKnob`
(`:3687-3720`): if no row exists for (zone, knob) one is **appended** to the
DataTable, so after saving it is the last row of the table.

`TrunkingKnobControl.init` (`ZONE/TrunkingKnobControl.cs:177-191`)
initializes, in this order, knob info, Rx-group options, the talkgroup list,
the revert list, secure, function and binding settings, and last the
trunking-system control, whose initial `System_SelectedValueChanged` loads
the selected system's `tabGroup_New`/`tabSubGroup` and pushes them into both
talkgroup lists (`KnobTrunkingSystemControl.cs:81-104`, `TrunkingKnobControl.cs:74-78`).

### 3.2 Default trunking knob row (`tabP25TrunkingSystemKnob`)

Effective values after `Row_tabP25TrunkingSystemKnob` and the editor's
`Save` with untouched controls. The "code" column is the `setDefaut` or
`Save` that sets it; the last column is what all 16 real P25T knob rows
(7 in unit A, 9 in unit B) contain.

| Column | Value | Code | Real data |
|---|---|---|---|
| `ZoneID`, `KnobIndex` | the knob | `ZoneListControl.cs:3696-3697` | |
| `ChannelID` | new lowercase GUID | `:3698` | all lowercase GUIDs |
| `P25TrunkingID` | first row of the profile index (combo index 0), or the user's pick | `KnobTrunkingSystemControl.cs:158-165,236-241` | |
| `ProtocolSettingID` | first `tabProtocolSetting` row with `ProtocolType 9`, or the user's pick | `:177-198,236-241` | the profile's only type-9 row |
| `TxPowerLevel` 2, `AudioSquelch` 0, `OpenSpeakerLogic` 0, `DispatchCHFlag` 0, `HangTime` 50, `RFPerformance` 0, `DutyStatus` 0, `PriorityGroup` 1 | | `ZoneListControl.cs:3704-3718`, `ChannelFunctionSettingControl.cs:371-376` | same |
| `ATGID` 0, `MSELCallSamplingFlag` 0, `MaxMSELSamplingTime` 2, `ATGEncryptionKeyID '000000000000'` | | `TrunkingRxGroupOptionControl.cs:121-136` | same |
| `PushToListen` 0, `Privacy` 0, `RecordFlag` 1, `VoiceBuffering` 1, `VoiceBufferPrompt` 0, `VoiceBufferingForceDrop` 0, `AlertMode` 1, `AlwaysUnreadVoiceMessage` 0, `AlwaysUnreadReminderAlert` 1 | | `ChannelFunctionSettingControl.cs:371-418,445-491` | same |
| `ResetMode` 0, `DelayNTime` 30, `EnableTgidStayTime` 0, `TgidStayTime` 30, `DelayNFeature` 0, `DelayNTimeEx` 5, `EnableCarrierOffTime` 0, `CarrierOffDelayTime` 5 | `setDefaut` writes `DelayNFeature 1`; `Save` writes 0 when the checkbox is hidden | `BindingFeatureControl.cs:662-770` | same (0) |
| `EncryptedCallFlag` 0, `EncryptionBy` 0, `EncryptionKeyID`, `PrivateEncryptionKeyID`, `DynRegroupEncryptionKeyID`, `KMFSystemID` all `'000000000000'` | `setDefaut` writes `EncryptionBy 1`; `Save` writes 0 because "Encrypted" is unchecked | `ChannelSecureSettingControl.cs:269-408` | same (0) |
| `KeyStrapOption`, `PrivateKeyStrapOption`, `ATGKeyStrapOption`, `DynRegroupKeyStrapOption`, `DynRegroupATGKeyStrapOption` 0; `UserSelectableKeyset`, `UserSelectableKey` 0 | | `ZoneListControl.cs:3709-3717` | same |
| `DynRegroupATGEncryptionKeyID` | never set: `''` after save (not the DDL default) | | `''` |

`EncryptionBy` matters: 0 makes the converter write one channel key
assignment per knob; 1 means per-talkgroup keys (converter-1.41 3.6;
Knobset's converter refuses it).

### 3.3 Case (a): empty knob (mode 0) becomes trunking

Flow: double-click an empty cell → `SelectKnobReceiveModeForm`
(`ZoneListControl.cs:3432-3452`) → `ZoneKnobInfoControl.initForNew` and, on
OK, `Save` (`ZONE/SelectKnobReceiveModeForm.cs:45-49`,
`ZONE/ZoneKnobInfoControl.cs:221-251,272-283`) → the same double-click
handler opens `TrunkingKnobControl` on a new detail row → user picks system,
protocol and talkgroups → moving away or Save writes everything.

| Table | Change | Evidence |
|---|---|---|
| `tabReceivingMode_NewG4` (existing row) | `CurrentMode` 16 or 48 (129 via radio-compatible + Trunking); `ChannelName` = alias, prefilled `"Zone {row}-Knob {k}"` when empty, where `row` is the zone's grid position (1-based), not `ZoneNo`; required, max 14; `AlertToneID` = the knob voice prompt, defaulting to the mode's built-in prompt: **6** for 16, **11** for 48, 12 for 129; `AlertFileName` untouched (template `NULL`, so `''` after the rewrite) | `ZoneKnobInfoControl.cs:108-150,253-266,272-283,541`, `NameValueVModel.cs:47-63,230-263` |
| `tabP25TrunkingSystemKnob` | append the 3.2 row | `ZoneListControl.cs:3687-3720` |
| `tabChannelGroup` | append one row per selected TG, priority list first (`SeqNo` 1..n), then normal (`SeqNo 0`); `SigType ''`, `OffDuty`, `Priority`, `OutOfRange`, `OTA` and the nine ack columns `'0'` (ack values copied only when the ACK feature is visible and set) | `TrunkingRxGroupListControl.cs:580-609,654-715` |
| `tabP25TSubGroupEnable` | **mode 48: append `(ChannelID, '', '1')`**; mode 16/129: none unless the binding feature "2-tone sub-group" is chosen | `BindingFeatureControl.cs:662-685` |
| `tabP25TCallAlertEnable` | none unless binding feature "call alert" | `:662-690` |
| `tabP25TCallAlertSUID` | the talkgroup list's `init` appends `(ChannelID, SUID '', CallAlertTimeout 30, TGIDStayTime 30)` whenever the knob has none; **absent for every real knob**, see 5.3 | `TrunkingRxGroupListControl.cs:308-320` |
| `tabChannelRevertGroup`, `tabP25TChannelSubGroup`, `tabP25TChannelRevertSubGroup`, `tabP25TCallAlertSUIDGroup` | none with default settings (revert list hidden for `ResetMode 0`; sub-group and call-alert rows only with those features) | `TrunkingRevertGroupListControl.cs:473-480`, `TrunkingRxGroupListControl.cs:654-715` |
| `tabProtocolSetting`, `tabP25TSignaling` | **none**; the knob references an existing type-9 row | `KnobTrunkingSystemControl.cs:177-198` |
| system DB, `tabP25TrunkingSecurity` | none | |
| `tabOrganization_New` | `FinishDate` on page save | `BasePageControl.cs:873-890` |

If the profile has no type-9 protocol setting, `checkError` stops the save
with "Please select P25 Trunking Protocol Alias ... can be set in the
pervious step" [sic] (`KnobTrunkingSystemControl.cs:220-227`). The user must first
add one on the Protocol page, which writes
`tabProtocolSetting(ID GUID, ProtocolNo = smallest unused positive integer,
ProtocolName "Protocol N", ProtocolType '9')` and `tabP25TSignaling(ID, 5000,
2000, 0, 133, 200, 2500, 1000, 1, 1000, 2000, 5000, 1500)`, max 16 settings
(`View/View.ProtocolSetting/ProtocolList.cs:29,432-447,584-600`). The
RadioReference importer does the same automatically when no type-9 row
exists (`RadioReferencehandler.cs:282-320`).

### 3.4 Case (b): conventional knob switched to trunking

Flow: in the knob editor, the receive-mode button
(`ZoneKnobInfoControl.cs:399-422`) opens `ChangeRecvModeForm`; any move
between families is a `BigChange` and asks for confirmation
(`ZONE/ChangeRecvModeControl.cs:129-190`). `ZoneKnobInfoControl` writes the
new `CurrentMode` and the current voice prompt into the in-memory receive
row, then `ZoneListControl.RecvModeChanged` (`:3547-3580`) calls
`deleteKnob(zone, knob, clearChannelName: false)` and reopens the knob, which
creates a fresh trunking row as in 3.3.

`deleteKnob` (`ZoneListControl.cs:1309-1384`) collects the ChannelIDs of
the knob's rows in `tabSingleConvCHKnob`, `tabP25TrunkingSystemKnob`,
`tabAllBandScanCHKnob` and `tabAdvancedConvCHKnob`, and removes:

| Table | Rows removed |
|---|---|
| `tabSingleConvCHKnob`, `tabP25TrunkingSystemKnob`, `tabMultiChannelScan`, `tabMultiCHScanList`, `tabAllBandScanCHKnob`, `tabAdvancedConvCHKnob` | `ZoneID` and `KnobIndex` of the knob |
| `tabChannelGroup`, `tabRecvSaList`, `tabP25TSubGroupEnable`, `tabP25TChannelSubGroup`, `tabChannelRevertGroup`, `tabP25TCallAlertEnable`, `tabP25TCallAlertSUID`, `tabP25TCallAlertSUIDGroup`, `tabP25TChannelRevertSubGroup`, `tabAllBandScanBandParam`, `tabAllBandScanProtocolParam`, `tabChannelGroupList` | `ChannelID` in the collected list |
| `tabMultiCHScanList` | also `ChannelID` in the list: **the knob drops out of every scan list it was a member of** |

Not touched: the frequency in `tabConvFreqList` (stays in the pool), the
groupings in `tabGroupGrouping`/`tabGroupingOfGroup`, and
`tabMultiChannelScan.FirstPriorityCH`/`SecondPriorityCH` of a priority scan
that pointed at this knob (left dangling; the pre-program check then
rejects that scan knob, data-model 2.5 E1-E3).

Then 3.3 applies, with these differences:

| Table | Case (b) value |
|---|---|
| `tabReceivingMode_NewG4` | `ChannelName` **kept**; `CurrentMode` new; `AlertToneID` becomes `0` in practice: the old prompt (e.g. 2 for monitor) is not in the trunking prompt list, and `initRecvMode` then selects "None" (`ZoneKnobInfoControl.cs:171-219`) |
| `tabP25TrunkingSystemKnob` | appended with a **new** `ChannelID`; the conventional ChannelID is gone everywhere |

The same applies to scan, free-scan and radio-compatible knobs switched to
trunking. A trunking knob switched to another trunking mode (16↔48↔129) is a
`SlightChange`: only `CurrentMode`, the sub-group-enable row (48 always
writes it) and visibility-driven values change; the row and ChannelID stay
(`TrunkingKnobControl.cs:346-373`).

### 3.5 Case (c): trunking knob switched to another system

Flow: pick another entry in "P25 Trunking System" (`KnobTrunkingSystemControl.cs:81-104`).
`System_SelectedValueChanged` loads the new system's talkgroups and calls
`setGroup` on both lists.

`TrunkingRxGroupListControl.setGroup` (`:338-416`) keeps its priority and
normal **GroupID** lists from `init` and shows only those found in the new
system's `tabGroup_New`; everything else from the new system goes to the
"available" box. GroupIDs are per-system GUIDs except the template "All" row
(`0000000000000000`), and systems copied by paste, resource add or zip import
(their GroupIDs are the source's). Real data: no GroupID other than "All" is
shared by any two systems. So after a switch the list holds "All" if it was
selected, otherwise nothing, and `checkError` (`:461-473`) refuses to save
until the user picks at least one talkgroup.

On save:

| Table | Change | Evidence |
|---|---|---|
| `tabP25TrunkingSystemKnob` | `P25TrunkingID` (and `ProtocolSettingID` if changed) updated **in place**: row position and `ChannelID` unchanged; all other columns kept, including `ATGID` | `KnobTrunkingSystemControl.cs:236-241` |
| `tabChannelGroup` | the knob's rows deleted and the listed ones appended at the end of the table (priority first, `SeqNo` 1..n, then 0) | `TrunkingRxGroupListControl.cs:654-715` |
| `tabP25TChannelSubGroup`, `tabP25TCallAlertSUIDGroup` | rewritten for kept GroupIDs from the in-memory maps built from the **old** system (sub-group IDs are not re-validated against the new system) | same |
| `tabP25TCallAlertSUID` | `SUID` set to the new system's SUID only when call alert is visible | `:659-662` |
| `tabChannelRevertGroup`, `tabP25TChannelRevertSubGroup` | same keep-by-GroupID rule, only when the revert list is visible | `TrunkingRevertGroupListControl.cs:223-250,473-520` |
| `tabReceivingMode_NewG4` | unchanged | |

## 4. Validation and limits for trunking knobs

| Rule | Where | Evidence |
|---|---|---|
| knob alias non-blank, max 14 | editor | `ZoneKnobInfoControl.cs:253-266,541` |
| a system and a type-9 protocol selected | editor | `KnobTrunkingSystemControl.cs:211-229` |
| **at least one talkgroup** (priority or normal) | editor | `TrunkingRxGroupListControl.cs:465-472` |
| max 64 talkgroups per list (priority + normal) | editor (add buttons) | `:24,440-459` |
| `ATGID` not equal to any selected TG's TGID | editor | `:486-492,520-526` |
| no two selected TGs with the same TGID | editor and B3 | `:544-556`, `ProfileCheckHandler.cs:462-538` |
| mode 16/129 with 2-tone sub-group binding: at least one sub-group; with call alert: at least one bound TG | editor and B3 | `:558-571`, `ProfileCheckHandler.cs:757-796` |
| revert list (reset mode timeout/manual): at least one TG | editor; B3 only if the list has rows | `TrunkingRevertGroupListControl.cs:329-345`, `ProfileCheckHandler.cs:515-523` |
| knob row exists, protocol resolves | pre-program B2 | `ProfileCheckHandler.cs:700-712` |
| system in the index and its file exists | pre-program C1 | `:714-730` |
| at least one selected TG with a non-empty TGID | pre-program B3 | `:462-506` |
| zones 64, knobs 8 per zone; the check loop covers knobs 1..8 only | | `ZoneListControl.cs:78,82`, `ProfileCheckHandler.cs:263-292` |

There is no limit on the number of trunking knobs, systems used by knobs, or
SigIDs anywhere in the UI, the check or the converter.

## 5. Empirical checks

### 5.1 System DBs

Nine files per unit, seven indexed and used, two orphans (same GUIDs in
both units). Per file (`sysdbs.py`):

| Check | Result |
|---|---|
| orphans | both are exactly `newP25Trunking` output: name `Trs Sys N`, WACN 1, SysID 1, `Site 1` (1/1), one CC `('1', RxMin, '1', RxMin, '1', '')`, band row, untouched "All" row, template `tabEditPermission`/`tabP25TSignaling`/`tabVersion`. One is SX (CC 763000000), one VX (CC 136000000). Zip timestamps: 27648 bytes, the template size. This is the Cancel leftover from 1.4 |
| `ID` = file name | all nine |
| editor-made systems (6) | `SystemCreateType 2`, `SystemSource ''`, `ConverageType 0`, `FullSpectrumScanRanges 1`, ranges `''`, `ErrorRateThreshold 5`; CC 1 keeps `TxFreq` = band minimum; band row present (5 SX, 1 VX) |
| RadioReference-API system | `FullSpectrumScanRanges 0`, no band row, `BandTypeCode SXVX`, 28 site/CC links, no "All" row |
| "All" row | present in 4 of 6 editor systems (deleted in the 2 with full RR-CSV TG imports); `AlertToneID` is `9999` in some, `0` in others (normalized by the TG list dialog, system-editing 5.1) |
| GroupID sharing | only `0000000000000000` is shared between systems |
| `tabEditPermission`, `tabP25TSignaling`, `tabVersion` | equal to the template in all nine |

### 5.2 Index rows

7 per unit, all lowercase GUIDs, `P25TrunkingID = DBFileName`,
`SystemCreateType 2`, `SubGroupCount 0`, `SUID ''` except one system,
`BandTypeCode` `SX` x5, `VX` x1, `SXVX` x1 (the API import). Order: no
relation to creation date is recoverable, but new rows are appended by every
path.

### 5.3 Knob rows

| Check | Result |
|---|---|
| `tabP25TrunkingSystemKnob` | 7 (A) and 9 (B) rows; every non-key column equals the 3.2 defaults |
| `CurrentMode` | A: 4 x 16, 3 x 48; B: 6 x 16, 3 x 48 |
| `ChannelName` | none matches `Zone n-Knob k` (all renamed) |
| `AlertToneID` on trunking knobs | `0` on all 16 (PPS's default for a fresh mode-16/48 knob is 6/11; see 7.6) |
| `tabP25TSubGroupEnable` | exactly the three mode-48 knobs per unit, `GroupID ''`, `Enable2TSubGroup 1`; none for mode 16. Matches 3.3 |
| `tabChannelGroup` on trunking knobs | 120 (A), 134 (B); `SigType ''`, every flag and ack column `'0'` |
| `tabP25TCallAlertSUID` | one row per unit, `ChannelID ''`, `CallAlertTimeout ''`, `TGIDStayTime ''`, unrelated to any knob; **no row for any trunking knob**, although `TrunkingRxGroupListControl.init` appends one every time a knob without one is opened. Not explained by this build's code; possibly an older PPS wrote these knobs. No effect: the converter reads the table only when call alert is on (`CONV:3298-3320`) |
| `tabProtocolSetting` | one row, `ProtocolNo 1`, a 10-character name (consistent with "Protocol 1"), `ProtocolType 9`; its `tabP25TSignaling` row has `RfssRspWaitTimeout 2000` (the Protocol page / RR default, not the 1000 of the template's `'000000000000'` row) |
| knob row order | not zone order (zone 1 knobs 1, 2, 4, 3, then zone 2, then zone 1 knob 5, ...), consistent with append-on-create |
| mode-0 rows | template rows `('', '', '')` for (ChannelName, AlertToneID, AlertFileName); one row with `AlertToneID '0'`, the signature of `deleteKnob(clearChannelName: true)` |

### 5.4 Conversion experiments (`exp.py`)

Baseline: unit B's profile converted with Knobset's 1.41c converter equals
the uploaded `u3db.db` (194 tables identical). Each experiment applies the
rows PPS would write (in place, no table rewrite) and compares the new radio
DB to the baseline. "Changed" counts tables whose rows differ in content or
rowid order.

| Experiment | Radio tables changed |
|---|---|
| (d) create and index a system, no knob | **none** |
| (a) mode-0 knob → mode 16 on a system another knob already uses, 2 TGs, prompt 6 | 28: `tabReceivingMode`, `tabZoneChannelList`, `tabChannelAnnouncement`, `tabChannel`, `tabChannelAdvInfo`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabVoiceCallSettings`, `tabResetModeSetting`, `tabR2RModeSetting`, `tabAlertModeSetting`, `tabRecvVoiceCallRecordByChannel`, `tabDispatchChannel`, `tabP25TSetting`, `tabKMFSystemByChannel`, `tabCipherKeyAssignment`, `tabP25OTARFeatureControlByChannel`, and the 11 contact tables (`tabContacts`, `tabContactAddr`, `tabContactAddrAndAlertTone`, `tabContactOfBackground`, `tabContactOfChan`, `tabContactOfRelayCtl`, `tabRadioGroup`, `tabRecvBehaviorByContactID`, `tabRecvGroupList`, `tabRecvToneForContacts`, `tabRecvVoiceCallRecordByContactID`) |
| (a2) same knob, a system no knob used before ("All" only) | the 28 plus `tabP25TrunkingSignaling`, `tabSystemAlias`, `tabP25TrunkingSites`, `tabP25TrunkingControlChannels` (new SigID 8, appended) |
| (a3) as (a) with prompt 0 instead of 6 | only `tabChannelAnnouncement` (`G4-ChVoice-000.dat` instead of `-006.dat`) |
| (a4) as (a) with mode 48 and the sub-group-enable row | `tabReceivingMode` (mode 64), `tabChannelAnnouncement` (`-011`), `tabP25TSetting.TwoToneSubGroupOn 1` |
| (b) CSQ monitor knob → mode 16 (it was a member of a scan list) | 25: the channel tables of (a) except `tabZoneChannelList`, `tabDispatchChannel`, `tabChannelAnnouncement`, `tabR2RModeSetting`, `tabVoiceCallSettings` (same ZK already present), plus `tabRepeaterSettingOfChan` (row removed) and `tabScanListSettings` (member removed from `ScanList`) |
| (c) first trunking knob → a system used by a later knob | 14: the contact tables and `tabSystemAlias`, `tabP25TrunkingSites`, `tabP25TrunkingControlChannels`: SigIDs are assigned in knob-row order, so SigID 1 moves to the new system and every later knob's SigID and contact IDs shift. `tabP25TrunkingSignaling` stayed equal only because the two systems happen to share WACN, System ID and scan settings |
| (c2) same, to a system no knob used | 15: as (c) plus `tabP25TrunkingSignaling` |

The new knob's ZoneKnobID is `zone row * 100 + knob` whichever table holds
it, so (b) keeps the radio channel ID.

## 6. Contradictions with earlier notes

1. data-model 5.2 item 1 describes creation correctly but not its timing:
   the system DB exists before the dialog and survives Cancel, which is where
   the two orphan system files in both profiles come from.
2. data-model 5.3 lists "max 1024 systems"; there is also a floor: PPS never
   deletes the last system of a list.
3. data-model 3.4 gives `PriorityGroup 1` and `HangTime 50` as RadioReference
   defaults; the knob editor uses the same values (`ZoneListControl.cs:3704-3718`).
4. `ChannelSecureSettingControl.setDefaut` writes `EncryptionBy 1` and
   `BindingFeatureControl.setDefaut` writes `DelayNFeature 1`; neither value
   survives a save (both 0). A spec built from `setDefaut` alone would be
   wrong, and `EncryptionBy 1` would change the radio DB.
5. data-model 5 says the system template's `tabP25TSignaling` has
   `RfssRspWaitTimeout 1000`; true, but no knob uses it. Knobs reference the
   profile's own type-9 row (2000 in real data).

## 7. Implementation spec

### 7.1 Common rules

Everything in system-editing 9.1 applies (one transaction per DB, TEXT
values, `''` never NULL, lowercase hyphenated GUIDs, append new rows and
delete in place, `FinishDate` on save). In addition:

- **Zip contents change.** Create adds `<profile>/<id>.db`, delete removes
  it. `g5_codeplug::edit::repack_zip` (`crates/g5-codeplug/src/edit.rs:50-80`)
  only replaces entries that already exist; it needs add and remove support
  before either operation can be written to a pager.
- Never leave a system file without an index row, or an index row without a
  file. Write the file, the index row and `FinishDate` as one operation; on
  any failure delete the new file.
- Leave the existing orphan files alone. They are in the zip, unused, and
  harmless; deleting them is a separate, explicit cleanup.
- Profile row order is radio-visible (SigIDs and contact IDs follow
  `tabP25TrunkingSystemKnob` and `tabChannelGroup` row order): append exactly
  where PPS appends.

### 7.2 Create a system

Inputs: name, band code (one of the profile model's 2-letter codes; default
the first band), WACN, System ID, site trunking flag, optional SUID, sites,
control channels, optional talkgroups.

System DB `<profile folder>/<id>.db`, `id` = new lowercase GUID:

1. Copy `Templates.dir/P25TSystem.db` byte for byte (add a
   `Templates::system_template()` path).
2. `tabP25TrunkingSystem`: one row
   `(id, '2', '', name, '0', wacn, sysid, '0', '1', '', '', '', '', '', '', '', '', '', site_trunking, '5', suid_or_empty)`.
3. `tabP25TrunkingSite`: the user's sites as in system-editing 9.3; default
   `('1', 'Site 1', '1', '1', '')`.
4. `tabP25TrunkingControlCH`: the user's channels as `(n, rx_hz, '1', '', '1', note)`,
   IDs 1..n. Require at least one (the rule PPS intends, system-editing 2.1).
   Do not write PPS's band-minimum placeholder with `TxFreq = RxMin`.
5. `tabRFBandTypeInfo`: copy the `Data_G4A.tabRFBandTypeInfo` row with
   `Code = band` (all six columns, as text).
6. `tabGroup_New`: keep the template "All" row; append user talkgroups per
   system-editing 9.5.
7. Do not touch `tabEditPermission`, `tabP25TSignaling`, `tabVersion`,
   `tabP25TrunkingSiteControlCH`, `tabSubGroup`, `tabMember_New`, `tabSUID`.

Profile, one transaction:

- Append to `tabP25TrunkingSecurity`:
  `(id, id, name, wacn, sysid, site_count, cc_count, tg_count_including_All, '0', '0', suid_or_empty, '2', '', band)`.
- `tabOrganization_New.FinishDate`.

Validate: name 1..14, not blank, unique (trimmed, case-sensitive) among
index `SystemName`; WACN 0..0xFFFFF; System ID 0..0xFFF; SUID `''` or
0..16777215; band in the model's bands; fewer than 1024 index rows; sites,
CCs, TGs per system-editing 9.3-9.5. Duplicate WACN/System ID pairs are
allowed (real profiles have them).

Write guard: system DB = the new file only; profile =
`tabP25TrunkingSecurity`, `tabOrganization_New`; **radio DB: no table may
change** (experiment d).

### 7.3 Delete a system

1. **Refuse** if any `tabP25TrunkingSystemKnob` row has `P25TrunkingID = id`;
   name the zone/knob pairs. (PPS deletes anyway and blocks programming
   later.)
2. Otherwise, in one profile transaction: delete the index row in place,
   set `FinishDate`; then delete `<id>.db` and drop it from the zip.
3. Allow deleting the last system (PPS's one-row floor is a list-UI rule;
   the template profile has zero systems and the converter needs none).

Write guard: profile = `tabP25TrunkingSecurity`, `tabOrganization_New`; the
system file removed; radio DB: no change (an unused system is never read by
the converter).

### 7.4 Assign a system to a knob

Common rows:

- **Protocol setting**: use the first `tabProtocolSetting` row (row order)
  with `ProtocolType = '9'`. If none exists, append one first, as the Protocol
  page and the RadioReference importer do: `(guid, smallest unused positive
  ProtocolNo, 'Protocol N', '9')` plus `tabP25TSignaling(guid, '5000', '2000',
  '0', '133', '200', '2500', '1000', '1', '1000', '2000', '5000', '1500')`;
  refuse if that would exceed 16 settings.
- **Knob row** (cases a and b): append to `tabP25TrunkingSystemKnob` the 3.2
  row with a new lowercase GUID `ChannelID`, the chosen system and protocol,
  every other column exactly as in 3.2 (`EncryptionBy '0'`, `DelayNFeature '0'`,
  `DynRegroupATGEncryptionKeyID ''`).
- **Talkgroups**: delete the knob's `tabChannelGroup` rows and append one row
  per selected TG, priority TGs first with `SeqNo` 1..n in the user's order,
  then normal TGs with `SeqNo '0'`: `(ChannelID, GroupID, '', SeqNo, '0' x 13)`.
- **Mode 48**: also append `tabP25TSubGroupEnable(ChannelID, '', '1')`; mode
  16 writes none.
- Do not write `tabP25TCallAlertSUID` (see 7.6).

Case (a), empty knob: update the existing `tabReceivingMode_NewG4` row in
place: `CurrentMode` 16 or 48, `ChannelName` (default
`Zone {zone row}-Knob {k}`), `AlertToneID` (default `6` for 16, `11` for 48),
`AlertFileName ''` if NULL. Then the common rows.

Case (b), conventional (or scan/free-scan) knob: delete in place, for the
knob's old ChannelID, every row listed in 3.4 (both ZoneID/KnobIndex tables
and ChannelID tables, including its `tabMultiCHScanList` memberships in
other scan knobs). Before writing, refuse or require confirmation when the
old ChannelID is `FirstPriorityCH`/`SecondPriorityCH` of a scan knob, or
when removing it leaves a scan list with fewer than 2 members (both would
fail the pre-program check E1-E3). Update the receive row: `CurrentMode`
new, `ChannelName` kept, `AlertToneID '0'` (what PPS ends up with). Then the
common rows with a new ChannelID.

Case (c), trunking knob to another system: update `P25TrunkingID` (and
`ProtocolSettingID` if chosen) of the existing row in place; keep
`ChannelID`, row position and every other column. Rewrite `tabChannelGroup`
for the knob from the user's new selection (only GroupIDs that exist in the
new system; "All" may be kept as PPS keeps it). Delete the knob's
`tabChannelRevertGroup`, `tabP25TChannelSubGroup`,
`tabP25TChannelRevertSubGroup` and `tabP25TCallAlertSUIDGroup` rows whose
GroupID or SubGroupID is not in the new system (PPS leaves sub-group rows
from the old system; Knobset's converter refuses those features anyway).

Validate before writing: system in the index and its file present; a type-9
protocol resolvable; 1..64 talkgroups, each with a TGID; no duplicate TGIDs
on the knob; `ATGID` not equal to a selected TGID; alias 1..14; mode in
{16, 48} (129 is a radio-compatible variant Knobset does not need yet).

Radio write-guard allowlists (Knobset 1.41b/c converter, section 5.4):

```text
KNOB_CHANNEL_TABLES = tabReceivingMode, tabZoneChannelList,
  tabChannelAnnouncement, tabChannel, tabChannelAdvInfo,
  tabFunctionOnOffOfChan, tabFunctionParamOfChan, tabVoiceCallSettings,
  tabResetModeSetting, tabR2RModeSetting, tabAlertModeSetting,
  tabRecvVoiceCallRecordByChannel, tabDispatchChannel, tabP25TSetting,
  tabKMFSystemByChannel, tabCipherKeyAssignment,
  tabP25OTARFeatureControlByChannel, tabP25OTARFeatureControl (1.41b writes
  it per channel), tabZone (ModifyByDevice flips when the zone had no
  active knob, converter-1.41 3.1)
CONV_REMOVAL_TABLES = tabRepeaterSettingOfChan, tabRecvSaList,
  tabScanListSettings, tabAllBandScanSettings, tabAllBandScanParam
```

| Operation | Allowed radio tables |
|---|---|
| (a) empty knob | `KNOB_CHANNEL_TABLES` + `TALKGROUP_EDIT_TABLES` + `SYSTEM_EDIT_TABLES` (the last only needed when the (system, protocol) pair is new; rows are appended, existing SigIDs do not move) |
| (b) conventional knob | (a) + `CONV_REMOVAL_TABLES` |
| (c) system switch | `SYSTEM_EDIT_TABLES` + `TALKGROUP_EDIT_TABLES` (SigIDs and contact IDs of later knobs renumber; individual-ID contacts of a system no knob uses any more disappear) |
| create / delete system | none |

`TALKGROUP_EDIT_TABLES` and `SYSTEM_EDIT_TABLES` are the existing constants
in `crates/g5-codeplug/src/edit.rs:197-215`; `crates/g5-core/src/write.rs:191-203`
unions them per batch.

### 7.5 Open questions for an on-pager test

Verified 2026-10-18 on unit B (DBV1.41c) with Knobset: case (a), an empty
position (Metro knob 8) made a TG Scan knob on an existing system with
one talkgroup. The write passed read-back, and the pager showed the knob,
found the system and received traffic. The profile diff before and after
shows only that knob. Case (b) is deferred; case (c) is not yet written to a
pager.

1. Program case (a) and case (b) on unit B and confirm the new knob hunts and
   receives on the system, the knob name shows, and the scan list that lost
   the member still works.
2. Built-in knob announcement 6/11 (`G4-ChVoice-006.dat`): confirm the pager
   has the built-in file and speaks it, since every real knob uses 0.
3. Pager limits: how many trunking systems (SigIDs) and trunking knobs the
   firmware accepts; nothing in PPS caps them below 1024 systems and 512
   knobs.
4. `tabSystemAlias.SystemAlias` is `VARCHAR(11)` in the 1.41 schema while
   system names are up to 14: check whether the pager truncates or rejects
   long names (the current pagers already carry 14-character names).
5. Case (c): confirm a full reprogram with renumbered SigIDs and contacts
   behaves (it should, the DB is replaced whole).
6. Not a pager test, a PPS one: open a trunking knob in PPS on Windows,
   save, and check whether a `tabP25TCallAlertSUID` row appears (5.3).

### 7.6 PPS behavior Knobset should skip

| PPS behavior | Why skip |
|---|---|
| Creating the system file before the dialog and leaving it on Cancel | produces orphan files (two in each real profile) |
| Placeholder control channel at the band minimum with `TxFreq = RxMin` | a 763.000 or 136.000 MHz control channel is junk the pager would hunt; require real channels |
| Deleting a system that knobs use | leaves `[System Error]` knobs and blocks programming |
| Refusing to delete the last system | UI-only rule, no data reason |
| Full delete-and-reinsert of 20 zone tables on every knob save | in-place changes in one transaction give the same rows with less risk |
| `tabP25TCallAlertSUID` placeholder row per opened knob | absent from all real knobs, never converted without call alert |
| `setDefaut` values that `Save` overrides (`EncryptionBy 1`, `DelayNFeature 1`) | the saved values are 0; `EncryptionBy 1` changes the radio DB |
| Leaving priority-channel references and sub-group rows pointing at removed data (cases b and c) | they fail the pre-program check or the converter later |
| Defaulting a new knob to the first index row and first protocol silently | make the system choice explicit; keep the first-type-9 protocol rule |

One PPS default to keep but surface: the new-knob voice prompt (6 for TG
scan, 11 for TG monitor). It is what PPS writes, but every knob on Chad's
pagers has it off, so the UI should show it and let the user pick None.
