# Zones and scan lists: PPS row-level behavior

Scope: what PPS V00-04-00-48 writes when the user creates, renames, deletes
and reorders zones on the "Zone & Channel Setting" page, and (part two)
how it builds a Normal Scan (mode 4) or Priority Scan (mode 36) knob on an
empty position and edits its member list. Sibling of
[system-creation.md](system-creation.md) and
[conventional-knobs.md](conventional-knobs.md); the zone page mechanics in
system-creation 3.1 (in-memory DataTables, `savePrivate` rewriting 20
tables, `checkError` gating every save, `deleteKnob`) apply here and are
not repeated.

Evidence paths are relative to `decomp/`. Short forms: `ZONE/` =
`View/View.Zone/`, `ZLC` = `ZONE/ZoneListControl.cs`, `SLC` =
`ZONE/ScanListControl.cs`, `GRID` =
`Control/Unication.Controls/PPSDataGridView.cs`, `CHECK` =
`View/View.ProfileCheck/ProfileCheckHandler.cs`, `CONV` =
`View/View.ProgramVersion/PPSU4ParamConverter_01P41b.cs`, `RS` =
`crates/g5-codeplug/src/convert141.rs`. Real data: unit B's profile
`bac66824-...` (`scratch/conv-study/`, identical in `scratch/syscreate/unitB/`)
and unit A's profile `412cd30e-...` (`scratch/zones/unitA/`, unpacked from
the G5D66BE backup). Both profiles descend from one profile: same zone
GUIDs, same scan rows. Experiments: `scratch/zones_tools/exp.py` (log in
`scratch/zones/exp.log`, outputs in `scratch/zones/exp/`), the
conventional-knobs harness: apply the PPS rows to a copy of unit B's
profile, convert with Knobset's 1.41c converter, diff against the
unmodified conversion.

## 0. Summary

- **A new zone is one `tabZone_New` row and 8 `tabReceivingMode_NewG4`
  rows**, appended. Only `ID` (lowercase GUID) and `Name` (`Zone N`, the
  smallest N not already a zone name) are set; `ZoneNo`, `Note` and
  `ModifyByDevice` are written as `''`. The 8 receive rows are mode 0 with
  `ChannelName ''`, `AlertToneID '0'`, `AlertFileName ''`. Nothing else: no
  detail row, no `tabDefaultZone` change. Add writes to disk at once.
- 8 knob rows, never 16. Only the template's first zone has 16 (knobs 9..16
  are template rows no UI reaches). The grid, the RadioReference importers
  and the M5/M6 clone importers all write 8.
- Limits: 64 zones (Add disabled at 64), at least 1 (Delete disabled at 1).
  Zone name 1..14 characters, unique among zones (trimmed, case-sensitive);
  an empty or duplicate edit is silently reverted.
- **Zone order is `tabZone_New` row order, nothing else.** `ZoneNo` is never
  written by PPS (only the template zone has `1`) and no live code reads it;
  every converter numbers zones 1..N in row order. The radio zone ID, every
  channel ID (`zone * 100 + knob`), and the power-on zone (`CurrentZoneID 1`
  = first row) follow from it.
- **PPS can reorder zones**: drag a row by its number column, or sort by
  Zone Alias from the column header's right-click menu. Either reorders the
  grid's table; the next `savePrivate` rewrites `tabZone_New` in grid order.
  There is no Move Up/Down for zones (the move menu items are for knobs).
- **Delete cascades**: the zone row, all its receive rows, every knob detail
  row (by `ZoneID`), every per-channel row of its knobs (by `ChannelID`),
  its scan knobs' own member lists, **and its knobs' memberships in scan
  lists of other zones**. It does not clear `FirstPriorityCH` in other
  zones, does not touch frequencies, systems, groups, `tabDefaultZone`, and
  does not check the other scan lists still have 2 members.
- Radio impact (measured): create, rename, and delete of an empty last zone
  change **only `tabZone`**. Reorder, or delete/insert of a zone that is not
  last, renumbers every later knob: 19 to 24 tables. Deleting a zone that
  holds trunking knobs also renumbers P25 SigIDs and contacts: 36 tables.
- **Scan knobs** (`tabMultiChannelScan`, one row per knob, no ChannelID) list
  members in `tabMultiCHScanList` by the members' ChannelIDs. Candidates are
  conventional knobs only (`tabSingleConvCHKnob`, `tabAdvancedConvCHKnob`)
  in **any zone**, except radio-compatible mode 128. Never P25T, free scan
  or another scan knob. 2..16 members. A priority scan's priority channel
  must be one of the members and stays in the member list.
- The converter takes members in **row order**; `ScanOrder` is ignored
  (measured). It removes the priority channel from the radio `ScanList` and
  writes it to `PriorityZoneKnobID`.
- Measured: a new scan knob changes 8 radio tables (plus `tabZone` if the
  zone had no active knob); a member-list or priority edit changes only
  `tabScanListSettings`.
- Knobset's safe zone delete: **only zones whose knobs are all empty**. That
  removes rows nobody can reference, so no cascade is needed.

## 1. Creating a zone

### 1.1 Flow

`button_add` (or the grid's Add menu, `addZoneMenu_Click`, `ZLC:1488-1491`)
runs `button_add_Click` (`ZLC:1094-1123`):

1. New `tabZone_New` row in memory: `ID` = `Guid.NewGuid().ToString()`
   (lowercase, hyphenated), `Name` = `CommonFunction.GetNextName("Zone",
   grid table, "ZoneAlias")`, i.e. `"Zone N"` with the smallest N such that
   no zone has exactly that name (`View/View.Common/CommonFunction.cs:637-673`).
   Appended to the table.
2. For knob 1..`KnobCount` (8, `ZLC:78`): a new `tabReceivingMode_NewG4` row
   with `ZoneID`, `KnobIndex`, `CurrentMode 0`, `AlertToneID 0`; appended.
3. A grid row is appended and selected; the toolbar counts update.
4. **`savePrivate` runs immediately** (`ZLC:1122`): the 20 zone tables are
   rewritten (system-creation 3.1), `tabZone_New` in grid order
   (`GetTabZoneOrder`, `ZLC:403-426`). `tabOrganization_New.FinishDate` is
   written later, on page save (`View/View.BaseControl/BasePageControl.cs:873-890`).

Columns never set are absent from the DataRow and the full-row INSERT of
`SQLiteSynchronise.updateTable` writes `''` for them
(`DataBase/Unication.SQLite/SQLiteSynchronise.cs:349-399`).

### 1.2 Rows written

| Table | Row | Evidence |
|---|---|---|
| `tabZone_New` (appended) | `(ID = new GUID, ZoneNo '', Name 'Zone N', Note '', ModifyByDevice '')` | `ZLC:1096-1099`; unit B's Lake and NOAA rows are exactly this shape (`ZoneNo ''`), only the template's zone has `ZoneNo '1'` |
| `tabReceivingMode_NewG4` (8 appended) | `(ZoneID, k, '0', ChannelName '', AlertToneID '0', AlertFileName '')`, k = 1..8 | `ZLC:1100-1108`; unit B: Lake knob 7 and every NOAA/Lake row before it was edited |
| everything else | none: no detail row, no scan row, no `tabDefaultZone` change | |
| `tabOrganization_New` | `FinishDate` on page save | |

The DDL declares `ZoneNo INTEGER NOT NULL` and `ModifyByDevice INTEGER NOT
NULL`; SQLite 2 is typeless and PPS stores `''` in both for every zone it
creates. Note that unit B's `Lake knob 7` row (`ChannelName ''`,
`AlertToneID '0'`) is identical to a fresh add-zone row, so it is not proof
of a `deleteKnob` (see section 8).

### 1.3 Other zone-creating paths

| Path | Rows | Evidence |
|---|---|---|
| RadioReference P25 import with "Create new Zone/Knob" | same zone row (`Zone N`), knob 1 mode 16 `ChannelName "Zone N-Knob 1"`, prompt 6, knobs 2..8 mode 0 with `AlertToneID ''`; refuses at 64 zones | `View/View.RadioReference/RadioReferencehandler.cs:237-241,360-384` |
| RadioReference conventional import | one new zone per 8 frequencies, knobs mode 33; capacity `(ZoneLimit - zones) * 8`, `ZoneLimit` 64 from `Data_G4A.tabBusinessOption` | `:722-728,784-830` |
| M5 / M6 codeplug clone | same as the grid (8 rows, mode 0, `AlertToneID 0`) | `View/View.Clone/M5CloneControl.cs:642-656`, `M6CloneControl.cs:708-722` |
| Paste zone (onto an existing zone) | no new zone; see 3.3 | `ZLC:1534-1566` |

None of them sets `ZoneNo`, `Note` or `ModifyByDevice`.

### 1.4 Limits and validation

| Rule | Value | Evidence |
|---|---|---|
| zones per profile | 64: Add and its menu item disabled at 64 rows (`MaxCount`) | `ZLC:82,428-436` |
| minimum | 1: Delete disabled at 1 row, so the zone menu's Delete too | `ZLC:431,3355` |
| knob rows per new zone | 8 | `ZLC:78,1100` |
| name at creation | `Zone N`, unique by construction | `CommonFunction.cs:637-673` |
| `ChannelLimit 256` in `tabBusinessOption` | read into the profile, never enforced (64 x 8 = 512 knobs possible) | `View/View.Profile/BaseDataRepository.cs:516`, no other reader |

## 2. Renaming a zone

The Zone Alias cell is edited in the grid. Rules, all in the grid control:

| Step | Behavior | Evidence |
|---|---|---|
| input | `MaxInputLength 14`; no character filter | `ZLC:4079` |
| enter cell | old value remembered | `GRID:512-522`, column registered in `ZLC:192` |
| validate | if the new text is not blank and another row's alias equals it after trimming both (case-sensitive), PPS shows "alias exists" and keeps the old value; otherwise accepts | `GRID:524-557,1131-1146` |
| end edit | if the old value is still remembered (duplicate, or blank / whitespace-only input), the cell is reset to it | `GRID:499-510` |
| store | `dgv_CellValueChanged` copies the cell text, **untrimmed**, into `tabZone_New.Name` in memory | `ZLC:3759-3770` |
| write | next `savePrivate` (page leave, Save, or any knob or zone action) | `ZLC:333-401` |

Knob aliases are independent: a knob named `Zone 2-Knob 7` by the
new-knob prefill (`ZONE/ZoneKnobInfoControl.cs:117`, which uses the zone's
grid position, not its name) keeps that text after a zone rename or move.

Radio: `tabZone.Name` (`VARCHAR(30)` in 1.41) only (experiment z2).

## 3. Deleting a zone

### 3.1 Flow

`button_delete_Click` (`ZLC:1135-1157`) or the zone context menu
(`deleteZoneMenu_Click`, `ZLC:1493-1512`, offered on the number column,
`ZLC:3332-3365`): one zone (the first selected cell's row), confirmation
"delete zone?", then `deleteZone(id, deleteTabZone_New: true)`, the grid row
is removed, numbers are refreshed and `savePrivate` writes everything.
There is no check of what the zone contains.

### 3.2 What `deleteZone` removes (`ZLC:1170-1246`)

It first collects the ChannelIDs of the zone's knobs from
`tabSingleConvCHKnob`, `tabP25TrunkingSystemKnob`, `tabAllBandScanCHKnob`
and `tabAdvancedConvCHKnob` (scan knobs have no ChannelID), then:

| Table | Rows removed |
|---|---|
| `tabZone_New` | `ID = zone` |
| `tabReceivingMode_NewG4`, `tabSingleConvCHKnob`, `tabP25TrunkingSystemKnob`, `tabMultiChannelScan`, `tabMultiCHScanList`, `tabAllBandScanCHKnob`, `tabAdvancedConvCHKnob` | `ZoneID = zone` (for `tabMultiCHScanList`: the member lists **of this zone's scan knobs**) |
| `tabChannelGroup`, `tabRecvSaList`, `tabP25TSubGroupEnable`, `tabP25TChannelSubGroup`, `tabChannelRevertGroup`, `tabP25TCallAlertEnable`, `tabP25TCallAlertSUID`, `tabP25TCallAlertSUIDGroup`, `tabP25TChannelRevertSubGroup`, `tabAllBandScanBandParam`, `tabAllBandScanProtocolParam`, `tabChannelGroupList` | `ChannelID` in the collected list |
| `tabMultiCHScanList` (again) | `ChannelID` in the list: **this zone's knobs drop out of scan lists in every other zone** |

It is `deleteKnob` (system-creation 3.4) applied to all knobs at once, plus
the zone row and all receive rows. The copy/paste clipboard markers for the
zone are cleared (`ZLC:1172-1184`).

Not touched:

| Item | Consequence | Evidence |
|---|---|---|
| `tabMultiChannelScan.FirstPriorityCH` / `SecondPriorityCH` in **other** zones pointing at a deleted knob | dangling; pre-program E3 fails that scan knob. If programmed anyway, the converter writes a priority scan with an empty `PriorityZoneKnobID` (experiment s8) | `CHECK:694-697`, `CONV:3040-3062` |
| other zones' scan lists left with fewer than 2 members | pre-program E1 fails | `CHECK:690-693` |
| `tabConvFreqList`, P25 system DBs and index, protocol settings, groups, groupings | kept (orphan frequencies cost nothing, conventional-knobs 2.6) | |
| `tabDefaultZone` | may keep the deleted zone's GUID. No PPS code reads or writes this table (no reference anywhere in the decompiled sources); the converter always sets `CurrentZoneID 1` | grep; `CONV:1461` |

Unit B check (`exp.py`, z3/z4): deleting NOAA removes 1 zone row, 8 receive
rows, 7 `tabSingleConvCHKnob`, 1 `tabMultiChannelScan`, 7
`tabMultiCHScanList`; deleting Lake removes 1 + 8 + 4 conventional + 2
trunking knob rows, 1 scan row, 4 member rows, 64 `tabChannelGroup`, 3
`tabRecvSaList` and 2 `tabP25TSubGroupEnable` rows. In neither case does
another zone reference the deleted knobs, so nothing dangles.

### 3.3 Related zone operations

- **Paste zone** (`copyZone`, `ZLC:1248-1307`): empties the target with
  `deleteZone(target, false)` (zone row kept), then clones every knob table
  with new ChannelIDs. Scan rows and member lists are cloned with only the
  `ZoneID` replaced, so a pasted scan knob still scans the **source** zone's
  channels and keeps the source's priority ChannelID.
- Knob delete, copy, paste and move (`ZLC:1309-1486,3236-3322`) are knob
  operations and out of scope; moving a knob keeps its ChannelID, so scan
  memberships follow it.

## 4. Zone order

### 4.1 How order is stored and used

| Consumer | Zone number | Evidence |
|---|---|---|
| grid | grid row; loaded in `tabZone_New` row order | `ZLC:265-294` |
| save | `GetTabZoneOrder` rebuilds `tabZone_New` in grid order before every write | `ZLC:336,403-426` |
| 1.41a/b and 2.0x converters | `m_ZoneMap[ID] = position` in row order; ZK = position * 100 + knob | `CONV:152-175,111-150` |
| base-class maps using `ZoneNo` | overridden by every concrete converter; the `ZoneNo` reads in `updateDataToPPS` have no caller | `View/View.ProgramVersion/PPSU4ParamConverterBase.cs:374-440`, `CONV:4590-4768` |
| pre-program check, detail report | counter over rows | `CHECK:187-197`, `View/View.DetailReport/DetailReportControl.cs:794-845` |
| scan candidate list labels `Z{n}/P{k}` | counter over rows | `SLC:145-178` |
| new knob alias prefill `Zone {n}-Knob {k}` | grid position | `ZoneKnobInfoControl.cs:117` |
| Knobset | `load_table` orders by rowid; `build_maps` numbers zones in that order | `crates/g5-codeplug/src/row.rs:75-85`, `RS:383-399` |

So **row order is the order**. `ZoneNo` is decoration: the template ships
`1`, PPS writes `''` for every zone it adds, and changing it changes nothing
in the radio DB (experiment z5b).

How the order reaches the pager: `tabZone.ID` = 1..N in row order, every
knob's radio channel ID is `ID * 100 + knob`, and `tabCurrentZone` is set to
1, so the first row is the zone the pager selects after programming
(`CONV:1453-1527`, converter-1.41 3.1).

### 4.2 Reordering in PPS

There are two ways, both acting on the grid's DataTable:

| Gesture | Mechanism | Evidence |
|---|---|---|
| drag a row by its first visible column (the row number; Select, Flag and Zone ID columns are hidden) and drop it on another row | `setDragDrop` removes the row and inserts it at the target index; `dgv_DragDrop` renumbers | `ZLC:190`, `GRID:923-939,965-1005`, `ZLC:3829-3836`, resx `Column_Select/Flag/zoneID.Visible False` |
| right-click the "Zone Alias" header, "Sorted by Zone Alias ascending/descending" | `sortAndSelect` sorts by alias (string order) and copies the sorted rows back into the table in place | `ZLC:193`, `GRID:445-498,749-833`, `ZLC:3820-3827` |

Neither calls `savePrivate`; the new order is written at the next save of
the page (leaving it, toolbar Save, or any zone or knob action). The knob
"Move Up/Down" items swap one knob between adjacent zones; they don't move
zones (`ZLC:3236-3278`).

## 5. Radio tables per operation (experiments)

Baseline: unit B's profile converted by Knobset's 1.41c converter (equal to
the uploaded `u3db.db`, system-creation 5.4). Counts are tables whose rows
differ in content or order.

| Exp. | Change | Radio tables changed |
|---|---|---|
| z1 | add empty zone "Zone 1" at the end | **1**: `tabZone` (new row `4, 'Zone 1', ModifyByDevice 1`) |
| z1b | same with 16 receive rows | same as z1 |
| z2 | rename Lake to "Lake North" | **1**: `tabZone` |
| z7 | delete that empty last zone again | **1**: `tabZone` (back to baseline) |
| z5b | change `ZoneNo` values only | **none** |
| z3 | delete the last zone, NOAA (normal scan + 7 CSQ knobs) with the PPS cascade | **19**: rows removed from `tabAlertModeSetting`, `tabChannel`, `tabChannelAdvInfo`, `tabChannelAnnouncement`, `tabContactOfChan`, `tabDispatchChannel`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabR2RModeSetting`, `tabReceivingMode`, `tabRecvBehaviorByContactID`, `tabRecvGroupList`, `tabRecvVoiceCallRecordByChannel`, `tabRepeaterSettingOfChan`, `tabResetModeSetting`, `tabScanListSettings`, `tabVoiceCallSettings`, `tabZone`, `tabZoneChannelList` |
| z6 | empty zone inserted as 3rd of 4 (the reverse of deleting an empty zone that is not last) | **19**: the z3 list; NOAA's channels renumber 3xx to 4xx |
| z5 | reorder Metro, NOAA, Lake | **24**: the z3 list plus `tabCipherKeyAssignment`, `tabKMFSystemByChannel`, `tabP25OTARFeatureControlByChannel`, `tabP25TSetting` (Lake's trunking knobs move 2xx to 3xx) and `tabRecvSaList` (Lake's CTCSS knobs) |
| z4 | delete the middle zone Lake (2 TG monitor, priority scan, 4 CSQ) with the PPS cascade | **36**: the z5 list, plus the 11 contact tables (`tabContacts`, `tabContactAddr`, `tabContactAddrAndAlertTone`, `tabContactOfBackground`, `tabContactOfRelayCtl`, `tabRadioGroup`, `tabRecvToneForContacts`, `tabRecvVoiceCallRecordByContactID`, ...) and `tabSystemAlias`, `tabP25TrunkingSignaling`, `tabP25TrunkingSites`, `tabP25TrunkingControlChannels` (a system lost its last knob; later SigIDs and contact IDs shift) |

Contact IDs and P25 SigIDs follow knob-table row order, not zone order, so
a pure reorder leaves the contact list and the system tables alone (z5);
only tables keyed by ZK change.

The z5 set is exactly `KNOB_CHANNEL_TABLES` plus `CONV_KNOB_CHANNEL_TABLES`
(`crates/g5-codeplug/src/edit.rs:266-312`) plus `tabScanListSettings`, minus
`tabP25OTARFeatureControl` (a 1.41b-only per-channel table). That union is
the set of radio tables keyed by channel ID.

Special flag: radio `tabZone.ModifyByDevice` is computed by the converter,
1 when no knob of the zone has a non-zero mode, else 0 (`CONV:1469-1480,1514-1521`,
`RS:478-505`). The profile's `tabZone_New.ModifyByDevice` is never written
or read (`''` in every real and template row). An empty zone is therefore
the only zone with radio `ModifyByDevice 1` (z1), and the first active knob
put into it flips that to 0 (conventional-knobs 5.1).

## 6. Scan lists (Normal Scan 4, Priority Scan 36)

### 6.1 Tables

| Table | Key | Content |
|---|---|---|
| `tabReceivingMode_NewG4` | (zone, knob) | `CurrentMode` 4, 36 (or 68 Silent Scan), alias, voice prompt |
| `tabMultiChannelScan` | (zone, knob), **no ChannelID** | 23 columns: `StayTime`, `OpenSpeaker`, `PriorityScanType`, `FirstPriorityCH`, `SecondPriorityCH`, `SamplingFlag`, `DwellTime`, `PushToListen`, `Privacy`, `ResetMode`, `DelayNTime`, `EnableCarrierOffTime`, `CarrierOffDelayTime`, `AlertMode`, `RecordFlag`, `DutyStatus`, `MatchCTCD`, `MatchColorCode`, `MatchNAC`, `AlwaysUnreadVoiceMessage`, `AlwaysUnreadReminderAlert` |
| `tabMultiCHScanList` | (zone, knob) of the **scan knob** | one row per member: `ScanOrder` 1..n, `ChannelID` of the member knob |

### 6.2 Creating a scan knob on an empty position

Flow (`ZLC:3420-3504`): double-click the empty cell,
`SelectKnobReceiveModeForm` sets mode, alias (prefill `Zone {row}-Knob {k}`)
and prompt in the receive row (system-creation 3.3). For 4/36/68 the
handler calls `Row_tabMultiChannelScan` (`ZLC:3722-3741`), which appends a
row if none exists for (zone, knob), and opens `ScanKnobControl` with the
whole-profile `tabZone_New`, `tabReceivingMode_NewG4`, `tabSingleConvCHKnob`,
`tabAdvancedConvCHKnob` and the placeholder ChannelID `00000000000`
(`ZLC:3502`). Its `checkError` and `Save` run knob info, scan list, binding
feature and function setting in turn (`ZONE/ScanKnobControl.cs:122-182`; the
scan list `Save` runs twice, harmlessly). Cancel removes the pending knob
(system-creation 3.1). **A scan knob cannot be saved with fewer than 2
members**, so a profile with fewer than two candidate knobs cannot get one.

Default `tabMultiChannelScan` row (effective values after `setDefaut` and
the sub-controls' `Save` with untouched controls):

| Column | Value | Set by | Lake knob 2 "ElmwoodCo Fire" (36) | NOAA knob 1 "NOAA Scan" (4) |
|---|---|---|---|---|
| `StayTime` | `200` ms (spinner 100..1000, step 1) | `SLC:273,284`; designer `SLC:588-591`, resx `Maximum 1000` | 200 | 200 |
| `OpenSpeaker` | `1` for 4 and 36, `0` for 68 | `SLC:274`, `ZLC:3736` | 1 | 1 |
| `PriorityScanType` | `1` for 36, else `0` | `SLC:275` | 1 | 0 |
| `FirstPriorityCH` | the priority combo's ChannelID for 36 (defaults to the **first member**); `''` otherwise | `SLC:203-214,270,372-397` | `dd74db57...` (Lake knob 3 "Elmwood Fire") | `''` |
| `SecondPriorityCH` | never set: `''` | | `''` | `''` |
| `SamplingFlag` | `0` ("Priority Sampling" unchecked; only shown for 36) | `SLC:271,282` | 0 | 0 |
| `DwellTime` | `6` (0.6 s); when sampling is on, the interval x 10 (0.1..25.5 s) | `SLC:272,283`, designer `SLC:576-582`, resx `Maximum 25.5` | 6 | 6 |
| `PushToListen`, `Privacy` | `0`, `0` | `ZONE/ChannelFunctionSettingControl.cs:371-418,445-491` | 0, 0 | 0, 0 |
| `ResetMode`, `DelayNTime`, `EnableCarrierOffTime`, `CarrierOffDelayTime` | `0`, `30`, `0`, `30` | `ZONE/BindingFeatureControl.cs:662-771` | same | same |
| `AlertMode` | `1` (Tone) | `ChannelFunctionSettingControl.cs:445-491` | 1 | **3** (Silent, user edit) |
| `RecordFlag` | `1` (list Off / ID Match) | same | 1 | **0** (Off, user edit) |
| `DutyStatus`, `MatchColorCode`, `MatchNAC` | `0` | `ZLC:3737-3739` | 0 | 0 |
| `MatchCTCD` | `0` (CTCSS/DCS option combo) | `ChannelFunctionSettingControl.cs` Save/setDefaut | 0 | 0 |
| `AlwaysUnreadVoiceMessage`, `AlwaysUnreadReminderAlert` | `0`, `1` | same | 0, 1 | 0, 1 |

`exp.py` checks both real rows against this default: Lake knob 2 matches in
every column; NOAA knob 1 differs only in `AlertMode 3`, `RecordFlag 0`,
the same two user edits found on 8 conventional knobs (conventional-knobs
1.2).

Binding feature `Save` deletes `tabP25TSubGroupEnable` and
`tabP25TCallAlertEnable` rows for ChannelID `00000000000` and writes none
(the feature combo is hidden for scan modes), so a scan knob writes no
per-channel rows.

Receive row: `CurrentMode` 4/36, `ChannelName` alias, `AlertToneID` default
**3** ("Normal Scan") for 4, **4** for 36, 5 for 68
(`View/View.Common/NameValueVModel.cs:47-63`). Both real scan knobs use 0.

### 6.3 Member list editor (`ScanListControl`)

| Aspect | Behavior | Evidence |
|---|---|---|
| candidates | every (zone, knob) with a row in `tabSingleConvCHKnob` or `tabAdvancedConvCHKnob`, in zone row order then knob order, labelled `Z{n}/P{k} alias`; mode 128 (radio compatible) excluded. **Any zone.** Not checked: the receive mode being non-zero (a stale detail row on a mode-0 knob would be offered) | `SLC:99-186` |
| not candidates | P25T knobs, free-scan knobs, scan knobs (none has a row in those two tables) | same |
| initial selection | existing members in `tabMultiCHScanList` **row order** for this knob (`ScanOrder` not consulted) | `SLC:131-143,179-185` |
| add / remove | one at a time; Add disabled at 16 selected | `SLC:287-335` |
| reorder | Up / Down buttons | `SLC:337-370` |
| priority channel (36 only) | combo bound to the **selected** list; keeps its value if still listed, else jumps to the first member | `SLC:203-214,372-397` |
| checks | at least 2 members ("select channel"); for 36 a priority channel selected | `SLC:231-249` |
| save | delete the knob's member rows, append one per listed member with `ScanOrder` 1..n (so the list moves to the end of the table); set `FirstPriorityCH`, `SamplingFlag`, `DwellTime`, `StayTime`, `OpenSpeaker`, `PriorityScanType` | `SLC:256-277` |

Switching among 4, 36 and 68 is a "slight change" (row kept, fields
re-saved), `ZONE/ChangeRecvModeControl.cs:111,147,182`.

### 6.4 Real member lists

| Scan knob | `tabMultiCHScanList` (row order = `ScanOrder`) | Radio `tabScanListSettings` |
|---|---|---|
| Lake knob 2 "ElmwoodCo Fire", Priority Scan | Lake knob 3 Elmwood Fire (priority, CTCSS 127.3), knob 4 Elmwood FireGd, knob 5 Oak Ridge FG, knob 6 Ashford FG | `(202, PriorityScanType 1, SamplingFlag 0, PriorityZoneKnobID '203', ScanList '204,205,206', RecvAllFlag 1, StayTime 200, 0, 0, PriorityIntervalTime 6)` |
| NOAA knob 1 "NOAA Scan", Normal Scan | NOAA knobs 8, 7, 6, 5, 4, 3, 2 (WX7 first) | `(301, 0, 0, '', '308,307,306,305,304,303,302', 1, 200, 0, 0, 6)` |

The priority channel is stored as a regular member (first in the real
list) and the converter takes it out of `ScanList`. Both lists are
same-zone; nothing in the UI or converter requires that. Unit A holds the
same two lists.

### 6.5 Pre-program check (`CHECK:643-698`)

For knobs 1..8 with mode 4/36/68: B2 the `tabMultiChannelScan` row exists;
E2 every member ChannelID is an active conventional knob (single or
advanced, knobs 1..8, any non-zero mode including 128; `CHECK:198-240`),
and none has `tabP25TSubGroupEnable.Enable2TSubGroup 1`; E1 at least 2
valid members, **priority channel included**; E3 for 36 the
`FirstPriorityCH` is a valid conventional channel. A priority scan with the
priority channel plus one member passes and gives the radio a one-entry
`ScanList`.

### 6.6 Conversion (`CONV:3032-3103`, `RS:1174-1225`)

Per `tabMultiChannelScan` row, **whatever the knob's receive mode**:

| Radio table | Row |
|---|---|
| `tabScanListSettings` | `(ZK, StayTime, RecvAllFlag = OpenSpeaker, PriorityScanType, SamplingFlag, PriorityZoneKnobID = "first,second" ZKs for type 1, ScanList = member ZKs in row order minus the priority channels, CallBackTime 0, RxPauseBetweenScanPaths 0, PriorityIntervalTime = DwellTime)` |
| `tabFunctionParamOfChan` | `(ZK, 0, 0, 0, 1, ZK)` |
| `tabFunctionOnOffOfChan` | talk-around 1, privacy, push-to-listen, unread flags (`CONV:3639-3660`) |
| `tabResetModeSetting` | `(ZK, ResetMode, DelayNTime, EnableCarrierOffTime, CarrierOffDelayTime, 0, 30)` |
| `tabAlertModeSetting` | mode `AlertMode - 1` |
| `tabRecvVoiceCallRecordByChannel` | `(zone, knob, RecordFlag)` |
| `tabReceivingMode` | from `convertZone` (modes 4/36/68 all become radio 4, `ZK` in both setting-ID columns), then `CtCdRequest = MatchCTCD`; no `tabZoneChannelList` row and no `tabChannel` row |
| `tabChannelAnnouncement` | `G4-ChVoice-NNN.dat` from the prompt |

Members whose ChannelID is not in the channel map are skipped silently.
Experiments s7 and s9 show two converter behaviors Knobset must guard
against in the profile: a P25T knob's ChannelID in a member list is
emitted into `ScanList` (the UI never offers it), and a scan row left on a
mode-0 knob still emits its `tabScanListSettings`, function, reset, alert
and record rows (only `tabReceivingMode` and the announcement disappear).

### 6.7 Scan experiments

| Exp. | Change | Radio tables changed |
|---|---|---|
| s1 | Metro knob 8 (empty) becomes a Normal Scan of NOAA WX1 and WX2 (another zone), prompt 3 | **8**: `tabReceivingMode`, `tabChannelAnnouncement`, `tabScanListSettings` (`108, ..., '302,303'`), `tabFunctionParamOfChan`, `tabFunctionOnOffOfChan`, `tabResetModeSetting`, `tabAlertModeSetting`, `tabRecvVoiceCallRecordByChannel` |
| s1b | s1 with prompt 0 | vs s1: `tabChannelAnnouncement` only |
| s2 | Lake knob 7 becomes a Priority Scan: Elmwood Fire (priority), Elmwood FireGd, Oak Ridge, prompt 4 | the same 8 (`207, 1, ..., '203', '204,205'`) |
| s3 | NOAA Scan: swap two members | `tabScanListSettings` only |
| s4 | NOAA Scan: drop WX1 | `tabScanListSettings` only |
| s5 | `ScanOrder` values changed, rows untouched | **none** |
| s6 | ElmwoodCo Fire: priority moved to Elmwood FireGd | `tabScanListSettings` only (`'204'`, `'203,205,206'`) |
| s8 | ElmwoodCo Fire: `FirstPriorityCH` dangling | `tabScanListSettings` only: `PriorityZoneKnobID ''` and the old priority channel scanned as a plain member |

`tabZone` did not change in s1/s2 because both zones already had active
knobs; a scan knob in an empty zone also flips `ModifyByDevice`.

## 7. Empirical checks

| Check | Result |
|---|---|
| zone rows | 3 per unit; the template zone (`f19e48d6...`, the same GUID in every profile made from the template) with `ZoneNo '1'`; Lake and NOAA with `ZoneNo ''`; `Note ''` and `ModifyByDevice ''` everywhere |
| receive rows | template zone 16 (knobs 9..16 untouched template rows, `AlertToneID ''`), the two PPS zones 8 each |
| `tabDefaultZone` | the template zone's GUID in both units |
| scan rows | 2 per unit, equal to the 6.2 default except the NOAA alert and record edits |
| member lists | 7 and 4 rows, `ScanOrder` 1..n equal to row order, all members single-conventional knobs in the scan knob's own zone |
| converter vs uploaded radio DB | `tabZone`, `tabScanListSettings` and the scan knobs' function rows match (converter-1.41 4.2) |

## 8. Contradictions with earlier notes

1. data-model 3.1 gives `ZoneNo (1..)`. PPS never writes it after the
   template; real PPS-made zones hold `''`, and no converter reads it.
2. data-model 3.3 says `tabMultiCHScanList` holds `ScanOrder` as the order.
   The converter and the editor both use row order; `ScanOrder` is written
   (1..n) but never read.
3. conventional-knobs 1.4 and system-creation 5.3 call Lake knob 7's
   `AlertToneID '0'` "the `deleteKnob(clearChannelName: true)` signature".
   A fresh add-zone row is identical (`ChannelName ''`, `AlertToneID 0`), and
   Lake was made by Add, so the row may simply never have been used.
4. data-model 2.5 E1 reads "scan list has >= 2 valid conventional
   channels"; the priority channel counts toward the 2.
5. converter-1.41 6.2 lists `tabZone_New` row order as an ID counter
   input; this note adds that PPS itself reorders that table on drag and on
   alias sort, so a PPS round trip can renumber every channel.

## 9. Implementation spec

### 9.1 Common rules

Everything in system-creation 7.1 and conventional-knobs 8.1 applies: one
profile transaction, TEXT values, `''` never NULL, lowercase hyphenated
GUIDs, append new rows, update and delete in place, `FinishDate` on every
write. No zip entry is added or removed by any operation here. In addition:

- **Address zones by `ID`, not name**, inside a batch. Today
  `knob_system::zone_id`, `edit::rename_knob` and `conv_knob` resolve a
  zone by exact `Name` (`crates/g5-codeplug/src/knob_system.rs:58-70`); with
  zone creates and renames in the same batch, either apply zone edits first
  and resolve knob edits against the result, or switch edits to zone IDs.
  `Summary::Zone` needs the `ID` and the scan knobs' member lists.
- Show knobs 1..8 only. `Summary` reports the template zone's knobs 9..16 as
  empty; they are not usable positions.
- Zone names: 1..14 characters, printable ASCII (as `rename_knob`), stored
  **trimmed**, not blank, unique among `tabZone_New.Name` after trimming
  (PPS's rule is case-sensitive; Knobset should also refuse a
  case-insensitive duplicate, because Knobset resolves zones by name and a
  `Lake` / `lake` pair confuses users).
- Never write `ZoneNo`, `Note`, `ModifyByDevice` on existing rows; new rows
  get `''` in all three, as PPS writes.
- Leave `tabDefaultZone` alone in every operation (unread by PPS and the
  converter).

New write-guard constants next to the existing ones in
`crates/g5-codeplug/src/edit.rs`:

```text
ZONE_TABLES = tabZone
ZONE_RENUMBER_TABLES = KNOB_CHANNEL_TABLES + CONV_KNOB_CHANNEL_TABLES
  + tabScanListSettings            (every radio table keyed by channel ID;
                                    measured superset of z3, z5, z6)
SCAN_KNOB_TABLES = tabReceivingMode, tabChannelAnnouncement,
  tabScanListSettings, tabFunctionParamOfChan, tabFunctionOnOffOfChan,
  tabResetModeSetting, tabAlertModeSetting,
  tabRecvVoiceCallRecordByChannel,
  tabZone (only when the zone had no active knob)
SCAN_MEMBER_TABLES = tabScanListSettings
```

`ZONE_RENUMBER_TABLES` is wide. For reorder and middle-zone delete, also
run a stronger check: map every channel ID of the new radio DB back through
the old-to-new ZK permutation (and zone IDs through the zone permutation)
and require the result to equal the old radio DB, `tabZone` aside. Any
other difference means something other than renumbering changed.

### 9.2 Create a zone

Inputs: optional name (default `Zone N`, the smallest N that is not an
exact existing name, as `GetNextName`).

Validate: fewer than 64 `tabZone_New` rows; the name rules of 9.1.

Rows:

1. Append `tabZone_New (new GUID, '', name, '', '')`.
2. Append 8 rows `tabReceivingMode_NewG4 (id, k, '0', '', '0', '')`,
   k = 1..8, in knob order.
3. `tabOrganization_New.FinishDate`.

Always append (new zone last). Placing it elsewhere is a reorder (9.5) in
the same batch.

Write guard: profile `tabZone_New`, `tabReceivingMode_NewG4`,
`tabOrganization_New`; radio `ZONE_TABLES` (z1). An empty zone is valid for
programming as long as some other knob is active (A1).

### 9.3 Rename a zone

Inputs: zone ID, new name. Validate per 9.1 (excluding the zone itself from
the uniqueness test). Rows: update `tabZone_New.Name` in place,
`FinishDate`. Knob aliases are not changed, including PPS's
`Zone {n}-Knob {k}` defaults.

Write guard: profile `tabZone_New`, `tabOrganization_New`; radio
`ZONE_TABLES` (z2).

### 9.4 Delete a zone (Knobset's safe restriction)

Knobset deletes **only empty zones**. Refuse, naming the knobs, unless all
of these hold:

- the zone is not the only zone (PPS's floor; a profile with no zones has
  no knobs and fails A1 anyway);
- every `tabReceivingMode_NewG4` row of the zone has `CurrentMode 0`;
- no row with that `ZoneID` exists in `tabSingleConvCHKnob`,
  `tabAdvancedConvCHKnob`, `tabP25TrunkingSystemKnob`, `tabMultiChannelScan`,
  `tabAllBandScanCHKnob` or `tabMultiCHScanList` (stale detail rows; s9 shows
  a stale scan row still reaches the radio).

Why this is safe: an empty zone has no ChannelID, so no scan list,
priority reference, talkgroup selection, tone or sub-group row anywhere can
point into it, and no P25 SigID or contact ID depends on it. The PPS
cascade of 3.2 reduces to two deletes. The user empties knobs first (a knob
delete operation, which must clean scan memberships and priority references
in other zones: refuse when removing the knob would leave a scan list
below 2 members or orphan a `FirstPriorityCH`).

Rows:

1. Delete the `tabZone_New` row.
2. Delete all `tabReceivingMode_NewG4` rows of the zone (8, or 16 for the
   template zone).
3. `FinishDate`.

Write guard: profile `tabZone_New`, `tabReceivingMode_NewG4`,
`tabOrganization_New`. Radio: `ZONE_TABLES` when the zone is the last row
(z7); otherwise `ZONE_RENUMBER_TABLES` with the permutation check (z6), and
tell the user later zones' channels renumber.

Full cascade (deleting a zone with knobs) is deferred: it needs the knob
delete rules above for every knob, P25 SigID and contact renumbering (z4,
36 tables), and per-system cleanup, all of which the knob-delete path must
solve first.

### 9.5 Reorder zones

PPS supports it (4.2), so Knobset should too.

Inputs: the complete list of zone IDs in the new order. Validate: a
permutation of the existing IDs; something actually moves.

Rows: make `tabZone_New` row order equal the new order, every column value
unchanged (including `ZoneNo`): delete all rows and re-insert them in order
in one transaction, so rowids ascend in the new order. Leave
`tabReceivingMode_NewG4` and every knob table as they are (they are matched
by `ZoneID`). `FinishDate`.

Write guard: profile `tabZone_New`, `tabOrganization_New`; radio
`ZONE_RENUMBER_TABLES` plus the permutation check (z5). The new first zone
is the one the pager shows after programming.

### 9.6 Create a scan knob on an empty position

Inputs: zone ID, knob 1..8, alias (default `Zone {zone position}-Knob {k}`),
type Normal (4) or Priority (36), ordered members (ChannelIDs), priority
member (36), stay time (default 200 ms), priority sampling (default off) and
its interval (default 0.6 s), voice prompt (default 0; 3 for Normal and 4
for Priority are the built-in options), optional `AlertMode` 1..4 (1) and
`RecordFlag` 0/1 (1).

Validate:

| Rule | Source |
|---|---|
| receive row exists with `CurrentMode 0`; no row for (zone, knob) in the five knob tables or `tabMultiCHScanList` | conventional-knobs 8.2 |
| alias 1..14, printable ASCII | `ZoneKnobInfoControl.cs:541` |
| 2..16 members, all distinct | `SLC:231-249,289`; E1 |
| each member is a `tabSingleConvCHKnob` row in knobs 1..8 of an existing zone, receive mode 1 or 33, CSQ protocol (`0000000000000000`), no `tabP25TSubGroupEnable` row. Refuse P25T, free-scan, scan, radio-compatible (128) and advanced knobs | `SLC:99-186`; E2; Knobset's converter refuses non-CSQ conventional (conventional-knobs 5.3); s7 |
| Priority: the priority member is in the list. Warn when it leaves only one other member | `SLC:203-214`; E1, E3 |
| stay time 100..1000 ms; interval 0.1..25.5 s in 0.1 s steps | `SLC:576-591`, resx |
| Silent Scan (68): refuse until tested (it writes `OpenSpeaker 0`, `RecvAllFlag 0`) | `SLC:274` |

Rows (one transaction):

1. Update the receive row: `CurrentMode '4'` or `'36'`, `ChannelName`,
   `AlertToneID`, `AlertFileName ''` (if NULL).
2. Append `tabMultiChannelScan`, columns by name from the table's own
   column list (refuse unknown columns): the 6.2 default with
   `PriorityScanType` 0/1, `FirstPriorityCH` = priority ChannelID or `''`,
   `SecondPriorityCH ''`, `SamplingFlag` 0/1, `DwellTime` = interval x 10 if
   sampling else `6`, `StayTime`, `OpenSpeaker '1'`, the optional settings.
3. Append one `tabMultiCHScanList (zone, k, n, ChannelID)` per member,
   `n` = 1.. in list order. For Priority, put the priority member first, as
   in the real list.
4. `FinishDate`.

Write guard: profile `tabReceivingMode_NewG4`, `tabMultiChannelScan`,
`tabMultiCHScanList`, `tabOrganization_New`; radio `SCAN_KNOB_TABLES`
(s1, s2).

### 9.7 Edit a scan knob's members

Inputs: zone ID, knob, new ordered member list, and for Priority the
priority member.

Validate: the knob has mode 4 or 36 and a `tabMultiChannelScan` row; member
rules of 9.6. When the current priority member is dropped, require the
caller to pick a new one (PPS silently picks the first member).

Rows:

1. Delete the knob's `tabMultiCHScanList` rows; append the new list with
   `ScanOrder` 1..n (as PPS, the list moves to the end of the table; only
   relative order within a knob matters).
2. Priority changed: update `FirstPriorityCH` in place.
3. `FinishDate`.

Write guard: profile `tabMultiCHScanList`, `tabMultiChannelScan` (only when
the priority changed), `tabOrganization_New`; radio `SCAN_MEMBER_TABLES`
(s3, s4, s6).

### 9.8 On-pager tests and open risks

1. **Empty zone** (z1): program one and confirm the pager lists it, lets
   the user select it, and behaves sanely on its empty knob positions
   (radio `ModifyByDevice 1`, no `tabReceivingMode` rows).
2. **Zone rename** with a 14-character name, and one with an apostrophe.
3. **Reorder** (z5): confirm the pager shows the new order and starts on
   the new first zone. Check pager-side state keyed by channel ID (TGID
   hold and lockout lists, recorded voice, message history, last channel)
   for orphaned or misattributed entries after the renumber.
4. **Delete an empty middle zone** (z6 reversed): same renumbering checks.
5. **Cross-zone scan list** (s1): both real lists are same-zone; confirm a
   scan knob scans channels of another zone.
6. **Priority scan written by Knobset** (s2): confirm the priority channel
   is sampled, with and without Priority Sampling; and a priority scan with
   only one other member.
7. Built-in scan prompts 3 and 4 (`G4-ChVoice-003/004.dat`): every real
   knob uses 0.
8. Scan members with Rx CTCSS (Lake's list has three): already on a real
   pager via PPS, low risk; re-check after a Knobset write.
9. 16 members, and a scan knob in a zone with no other active knob
   (`ModifyByDevice` flip).
10. Pager limits: 64 zones and 512 knobs are possible in PPS while
    `tabBusinessOption.ChannelLimit` says 256; test before allowing more
    than about 30 zones of 8.
11. Not a pager test: open a Knobset-edited profile in PPS on Windows,
    save without changes, and confirm PPS keeps the zone order, the new
    zone's `''` columns and the scan rows.

### 9.9 PPS behavior Knobset should skip

| PPS behavior | Why skip |
|---|---|
| Full delete of a zone with knobs, leaving `FirstPriorityCH` in other zones dangling and scan lists below 2 | blocks programming (E1, E3); if forced, the radio gets a priority scan without a priority channel (s8) |
| Silent reorder when the user sorts the grid by alias | renumbers every channel without the user meaning to; Knobset reorders only on an explicit request |
| Storing the alias untrimmed | `" Lake"` and `"Lake"` pass PPS's trimmed uniqueness test only one way round; store trimmed |
| Paste zone keeping scan lists pointed at the source zone | surprising; out of scope, but don't copy it |
| Offering stale detail rows of mode-0 knobs as scan candidates | the member would fail E2 |
| Scan `checkError` picking the first member as priority when the old one is removed | make the user choose |
| Writing a `tabMultiChannelScan` row and then removing the knob without deleting it | the converter still emits the scan (s9) |
