# Editing an existing P25 trunking system: PPS row-level behavior

Scope: what PPS V00-04-00-48 writes when the user edits a system that is
already attached to a profile (system properties, sites, control channels,
talkgroups), how the profile index row is kept in sync, and what Knobset must
do to produce the same rows. Creating systems, sub-groups and individual IDs
are out of scope except where they share code.

Evidence paths are relative to `decomp/`. Short forms: `P25T/` =
`View/View.P25TSystem/`, `SQL/` = `DataBase/Unication.SQLite/`, `CONV/` =
`View/View.ProgramVersion/PPSU4ParamConverter_01P41b.cs`. Real-data checks use
the earliest backup's `u3db_PC.zip` (PPS-written; later backups differ only in
the profile DB, the nine system DBs are byte-identical). Tools:
`scratch/sysedit_tools/analyze.py`, `details.py` (structural output only).

## 0. Summary

- **Persistence is a full-table rewrite.** Every editor dialog works on
  in-memory `DataTable`s; on OK, `SQLiteDataBase.SaveDataTableEx` runs
  `delete from <t>;` for each table, then one `insert into <t> values('..',..);`
  per DataTable row, in DataTable row order, every value single-quoted text
  (`SQL/SQLiteDataBase.cs:652-707`, `SQL/SQLiteSynchronise.cs:25-28,349-399`).
  Deletes and inserts run as two separate `begin;..commit;` batches of at most
  5000 statements (`SQL/SQLiteDataBase.cs:178-226`), so a failing INSERT leaves
  the table empty.
- **NULL never survives a save.** The reader stores empty strings and NULLs as
  `DBNull` (`SQL/SQLiteDB.cs:152-179`), and the writer emits `''` for both.
  Real data: zero NULLs in any system DB or index column.
- **Row order on disk = DataTable order**, which the grid's Sort and
  drag-and-drop physically rewrite (`Control/Unication.Controls/PPSDataGridView.cs:749-801,965-1005`).
  Exception: `tabP25TrunkingControlCH.ID` is an `INTEGER` single-column primary
  key, i.e. the rowid, so control channels always read back in **ID order**
  whatever the insert order (verified with the SQLite 2.8.17 CLI). Sites and
  talkgroups have no integer PK and read back in insert order.
- **Order that reaches the radio:** sites (row order) and control channels (ID
  order) give `SeqNo` 256, 255, ... in the radio DB. `tabGroup_New` row order
  has **no** radio effect: contact numbering follows the profile's
  `tabChannelGroup` row order per knob (`CONV:952-1007`).
- **Talkgroup delete or TGID change never touches the profile.** Knob
  references (`tabChannelGroup.GroupID` etc.) are GUIDs; a TGID change flows
  through on the next conversion, a delete leaves orphan rows. The converter
  skips orphans in receive lists but **fails** on an orphan in a revert list
  (`CONV:1765-1770`), and the pre-program check blocks a knob left with no
  valid talkgroup.
- **`tabEditPermission` is never read** by any code. Read-only mode exists
  only when the system list is opened as a picker/resource browser.
- **Index row** (`tabP25TrunkingSecurity`) is updated in memory by the dialogs
  and rewritten when the user leaves the page or presses Save, together with
  `tabOrganization_New.FinishDate`. `SystemSource` and `SystemCreateType` are
  never changed by an edit.
- Several validations the notes previously listed as enforced are dead code:
  control-channel duplicate check, "at least one control channel" rule.
  Talkgroup names and TGIDs are not unique-checked.

## 1. Persistence mechanics

### 1.1 Load

`P25TSyetemTotal.getP25TSystemTable` (`P25T/P25TSyetemTotal.cs:1027-1040`)
loads `tabP25TrunkingSystem`, `tabP25TrunkingSite`, `tabP25TrunkingControlCH`,
`tabP25TrunkingSiteControlCH` with `select * from <t>;`. Talkgroups load
separately with `GetTableByName("tabGroup_New")` (`:1055-1060`). All columns
are untyped strings; NULL and `''` both become `DBNull` (`SQL/SQLiteDB.cs:152-179`).

### 1.2 Save

`SaveDataTableEx(List<DataTable>)` (`SQL/SQLiteDataBase.cs:652-707`):

1. `AcceptChanges()` on each table (Deleted rows vanish, order kept).
2. Per table: `delete from <t>;` into a delete list; one INSERT per row into
   an insert list (`SQLiteSynchronise.updateTable`, `:349-399`). Columns are
   taken in CREATE TABLE order from the parsed schema; DataTable columns not
   in the schema (UI helpers such as `Flag`, `Select`, `Detail`,
   `RxFreqDisplay`, `RelayDisplay`, `P25T`) are ignored; schema columns
   missing from the DataTable get the DDL default with quotes stripped
   (`SQL/SqlParser.cs:165-171`). Values: `'` + `value.ToString()` with `'`
   doubled + `'`.
3. Execute the delete list, then the insert list, each wrapped in
   `begin;`/`commit;` every 5000 statements.

`SaveDataTableEx(DataTable)` (`:598-650`) is the single-table form (delete
and insert in one list). The DataRow-state path `SaveDataTable` (per-row
INSERT/UPDATE/DELETE, `:366-475`) is not used by any P25 editor.

Inline grid edits on the system list bypass DataTables and run one UPDATE
directly (section 2.3).

### 1.3 SQLite 2 key behavior (verified on a template copy)

| Table | Key | Effect |
|---|---|---|
| `tabP25TrunkingControlCH` | `ID INTEGER NOT NULL, Primary key(ID)` | rowid alias: `select *` returns ID order; ID must be an integer (`'x'` or `''` gives "datatype mismatch"), unique ("PRIMARY KEY must be unique") |
| `tabP25TrunkingSystem` | `ID VARCHAR(64)`, PK | unique index only ("column ID is not unique") |
| `tabEditPermission` | `PermissionID INTEGER`, PK | rowid alias (never edited) |
| `tabP25TrunkingSite`, `tabP25TrunkingSiteControlCH`, `tabGroup_New`, `tabSubGroup`, `tabMember_New` | none | insert order; delete-all then re-insert yields rowids 1..n in the new order |
| profile `tabP25TrunkingSecurity` | `P25TrunkingID VARCHAR(64)`, PK | unique index, insert order |

### 1.4 When each dialog writes

| Trigger | Writes | Evidence |
|---|---|---|
| System "Detail" cell, EditSystemForm OK | system DB: all four of `tabP25TrunkingSystem`, `tabP25TrunkingSite`, `tabP25TrunkingControlCH`, `tabP25TrunkingSiteControlCH`, only if the system row changed or any of the other three differs (set compare, `CommonFunction.CompareDataTable`) | `P25T/P25TSyetemTotal.cs:1214-1275`, `View/View.Common/CommonFunction.cs:1289-1331` |
| TGID cell, P25TGroupListForm OK | system DB: `tabGroup_New` | `P25T/P25TSyetemTotal.cs:1134-1159` |
| Inline Alias / WACN / System ID cell | system DB: one `update tabP25TrunkingSystem set <col> = '<v>';` | `:1328-1345` |
| SUID cell, DigitAddrForm OK | system DB: `update tabP25TrunkingSystem set SUID = '<v>';` | `:1111-1133` |
| RR site CSV "update selected item" | system DB: `tabP25TrunkingSite`, `tabP25TrunkingControlCH` (separate single-table saves) | `:1390-1465,1489-1561` |
| Leave page / toolbar Save | profile: `tabP25TrunkingSecurity` (full rewrite, only if changed) then `tabOrganization_New.FinishDate` | `:591-607`, `View/View.BaseControl/BasePageControl.cs:494-560,873-890`, `View/View.EditProfileUI/EditProfileControl.cs:362-378,1016-1030` |

Change detection quirks: the sub-dialogs add helper columns to the tables they
return, and `DataRowComparer` treats rows with different column counts as
unequal. So opening the Site list, the Control-channel list or the TGID list
and pressing OK always counts as a change and triggers the rewrite, even if
nothing was edited. A pure reorder with no helper columns (not reachable in
practice) would not be saved because the compare is set-based.

## 2. System properties

### 2.1 Fields

Editor: `NewSystemControl` inside `EditSystemForm`
(`P25T/NewSystemControl.cs`, `P25T/EditSystemForm.cs:47-56`).

| Field (system DB / index) | UI | Range, rule | Stored as | Evidence |
|---|---|---|---|---|
| `Name` / `SystemName` | text box | max 14 chars; not blank/whitespace ("Alias can't be left blank."); unique among index `SystemName`, compared trimmed and case-sensitive, own row excluded ("The alias exist!") | text, quotes doubled only in raw-SQL paths (`convertToDBName`) | `NewSystemControl.cs:211-229,249,262,408`, `CommonFunction.cs:728-739` |
| `WacnID` | hex spinner | 0..0xFFFFF | decimal integer text | `:250,263,420` |
| `SystemID` | hex spinner | 0..0xFFF | decimal integer text | `:251,264,450` |
| `SiteTrunkingFlag` | checkbox | 0/1 | `'0'`/`'1'` | `:182,265` |
| `SUID` (both) | checkbox + spinner | unchecked: `''`; checked: 0..16777215, UI default 1 | decimal text or `''` | `:183-185,254,266,570` |
| `FullSpectrumScanFlag`, `FullSpectrumScanRanges`, `Start/End/StepSizeOfRange1..3` | separate dialog | section 2.2 | ints | `P25T/FullSpectrumScanControl.cs` |
| `ID`, `SystemCreateType`, `SystemSource`, `ConverageType`, `ErrorRateThreshold` | none | never written by edits | | |

`NewSystemControl.Save` (`:243-268`) writes the index row first
(`SystemName`, `WacnID`, `SystemID`, `SiteCount` = site rows, `ControlCHCount`
= CC rows, `SUID`, plus display-only columns) and then copies `Name`,
`WacnID`, `SystemID`, `SUID` and `SiteTrunkingFlag` into
`tabP25TrunkingSystem` row 0.

Dead validation: the "set at least one control channel or enable full
spectrum scan" check sits in an `else if` that only runs when the index table
is null (`:221-235`). Both callers pass the index table, so a system with zero
control channels and scan off saves without complaint. data-model 5.3 lists
this rule as enforced; it is not.

### 2.2 Full spectrum scan dialog

`FullSpectrumScanForm` edits `tabP25TrunkingSystem.Rows[0]` in place
(`P25T/NewSystemControl.cs:116-125`, `P25T/FullSpectrumScanForm.cs:41-51`).
On OK, `checkData` then `Save` (`P25T/FullSpectrumScanControl.cs:271-324`):

- Writes `FullSpectrumScanFlag` (0/1), `FullSpectrumScanRanges` (1..3) and
  **all nine** range columns, even with the flag off or fewer ranges in use:
  `Start/EndFreqOfRangeN = Convert.ToInt32(MHz * 1e6)`, `StepSizeOfRangeN` Hz.
- Defaults when a column is empty: start = first Rx range start of the first
  band, end = last Rx range end of the first band (both whole MHz), step 12500
  (`:170-210`).
- Checks only when the flag is on, per used range: start and end in the same
  band ("RxTx should same band"), `start + step <= end`. Step 1..1,000,000 by
  spinner limits.

Real data: no system has had this dialog confirmed (all range columns `''`;
`FullSpectrumScanRanges` is `'1'` from creation, `'0'` on the
RadioReference-API system).

### 2.3 Inline edits in the system list

`Column_Alias` (unique, non-blank via `CharacterManageColumnIndexList`),
`Column_wacnID` and `Column_SystemID` are editable in the grid. A change
updates the bound index row and runs one UPDATE on the system DB
(`P25T/P25TSyetemTotal.cs:1328-1345`). The SUID cell opens a `DigitAddrForm`
(0..16777215 or unset) and updates both (`:1111-1133`). These paths do not
touch counts or other tables.

### 2.4 Band of the system

On opening the Detail editor, an empty `BandTypeCode` is filled with the
profile model's band codes (`getCurrenProfileBandTypeCode`, e.g. `SXVX`)
(`P25T/P25TSyetemTotal.cs:1221-1224,790-810`); this persists with the next
index save. Rx ranges come from `findBandTypeRxFeqRange` per 2-letter code
(`:909-946`): `Data_G4A.tabRFBandRanges` rows with `RxTx != 1`, bounds
truncated by integer division to whole MHz. Shipped data: `VX` 136-174 MHz;
`SX` 763-776 and 851-870 MHz; `UX` 330-520 MHz. A 4-char code gives two maps
(first and second band); the CC editor uses their union. An empty code falls
back to 136-870 MHz.

### 2.5 `tabEditPermission`

No code reads or writes it (no match for `tabEditPermission` or
`EditPermission` anywhere in the decompiled sources). The DDL comment says
`Permission` 0 = no edit, 1 = allow; the template has 13 rows, all
`Permission = 1`, `Enable = 0` only for "Group Number". All nine real system
DBs match the template. Read-only editing is driven solely by
`!button_add.Visible`, which is false when the list runs as a library picker
or resource page (`P25T/P25TSyetemTotal.cs:192-199,428-469,1244`).

## 3. Sites

Editor: `SiteListControl` in `SiteListForm`, opened from the system editor
with a **copy** of the site, CC and link tables
(`P25T/NewSystemControl.cs:82-98`).

| Operation | Rows written (in the in-memory site table) | Evidence |
|---|---|---|
| Add | append `ID` = smallest unused positive integer (string compare over existing IDs, fills gaps), `Name` = `"Site N"` (smallest unused N), `RFSSID`/`SiteID` = `1`/`1`, or if that pair is taken the first free pair scanning RFSS 1..255 then Site 1..255, `Note` `''` | `P25T/SiteListControl.cs:258-304`, `CommonFunction.cs:637-673` |
| Edit | grid: `Name` (max 14; blank or duplicate reverts to the old value, duplicate also prompts "The alias exist!"), `RFSSID`, `SiteID` (0..255, hex display and entry, stored decimal) | `:96-113,836`, `PPSDataGridView.cs:499-560` |
| Delete | remove selected rows; the last remaining site cannot be deleted; delete link rows `SiteID = <site row ID>` | `:306-343` |
| Paste | append copies with new ID, name de-duplicated, `Note ''`, free RFSS/Site pair | `:145-180` |
| Import (Unication CSV) | RFSS/Site parsed as **hex** text, 0..255; names 1-14, unique; duplicate pairs rejected; "delete all" option replaces the table | `:472-571` |
| Sort / drag | reorders rows, persisted | |

OK button (`P25T/SiteListForm.cs` `btnOk_Click`) runs `checkDataValid`
(`SiteListControl.cs:345-373`): exact-duplicate `Name` ("Site Alias ({0}) can
not be same.") and duplicate `RFSSID`/`SiteID` pair ("Not allow to configure
duplicated combinations of RFSS ID "{0:X}" / Site ID "{1:X}"."). Count limit
256 (`:26`), via the Add button enable state.

Storage: `RFSSID` and `SiteID` are decimal integer text (NumericUpDown cell
values; the hex is display-only, the CSV export converts to hex,
`:573-595`). The `Note` column is hidden; only the RadioReference CSV import
fills it.

### 3.1 Site-to-control-channel links

`tabP25TrunkingSiteControlCH(SiteID, ControlChannelID)` holds the **row IDs**
of `tabP25TrunkingSite.ID` and `tabP25TrunkingControlCH.ID`, not the P25 site
number. The per-site selection UI ("Control CH of the Site (Optional)") is
hidden (`SiteListControl.cs:88`, resx `Column_ControlCH`), so users can only
lose links: site delete and CC delete remove matching rows
(`SiteListControl.cs:336-343`, `ControlCHControl.cs:446-453`). Nothing happens
to a site left without channels. The converter ignores this table entirely
(converter-1.41 2.4), so links have no radio effect. Real data: only the
RadioReference-API system has links (28, none dangling); editor-made systems
have none.

## 4. Control channels

Editor: `ControlCHControl.editInit` in `ControlCHForm`, one shared pool per
system (`P25T/ControlCHControl.cs:197-249`).

| Operation | Rows written | Evidence |
|---|---|---|
| Add | append `ID` = smallest unused positive integer, `RxFreq` = first range start (whole MHz × 1e6, e.g. 763000000 for SX), `RxBandWidth` `1`, `TxFreq` `''`, `TxBandWidth` `1`, `Note` `''` | `:346-374` |
| Edit | `RxFreq` from the MHz cell (5 decimals, spinner step 0.0125, limited to the band ranges inclusive): `Convert.ToInt32(MHz * 1e6)`. `Note` max 64. Bandwidth column hidden and read-only. `TxFreq` never touched | `:484-490,228-231,962`, resx |
| Delete | remove selected rows, may go to zero; delete link rows `ControlChannelID = <ID>` | `:426-453` |
| Paste | new row; frequency copied only if in range, else keeps the Add default | `:165-195` |
| Import (Unication CSV) | `ID` = max existing + 1 onward; freq must be in range; note cut to 64 | `:536-595` |
| Sort / drag | reorders the DataTable, but IDs travel with the rows and the table reads back in ID order, so the change does not persist | section 1.3 |

No duplicate check: `hadSameFreqAndValid` and `newFreq` exist but are never
called (`:376-424`), and the form's OK just calls `Save` (`ControlCHForm.cs`
`btnOk_Click`). Pressing Add twice yields two rows at the band start. The
converter later de-duplicates by `RxFreq` (`CONV:3238-3253`). data-model 5.3
and limits-and-misc list "no duplicates" as enforced; it is not.

Real data: in every editor-made system the first CC (created by
`newP25Trunking` as `('1', RxMin, '1', RxMin, '1', '')`,
`P25T/P25TSyetemTotal.cs:1020`) still has `TxFreq` = band minimum while its
`RxFreq` was edited; later CCs have `TxFreq ''`. The RadioReference-API system
has `TxFreq = RxFreq`. CSV import writes `RxBandWidth 0`, `TxFreq 0`
(`:1546-1553`). The pager is receive-only and the converter reads only
`RxFreq`.

## 5. Talkgroups (`tabGroup_New`)

Editor: `P25TGroupListControl` in `P25TGroupListForm`, given the live table
(`P25T/P25TSyetemTotal.cs:1134-1159`); add dialog `NewP25TGroupControl`.

### 5.1 Side effects of merely opening the list

`init` (`P25T/P25TGroupListControl.cs:288-358`) and the first paint
(`:384-474`) rewrite values in the bound rows, and OK always saves:

- `RecordFlag` reformatted through `Convert.ToInt32` (an empty value throws).
- `EncryptionKeyID` replaced by the first key of the profile's
  `tabEncryptionKeyList` (or `'000000000000'` "Default key" when the list is
  empty) if not found in that list (`P25T/P25TSyetemTotal.cs:1298-1326`).
- `AlertToneID` replaced by `'0'` if not in the tone list (0 none, 9999
  default, 10000 chirp, 1..8, plus custom `tabCustomAlertTone.ID`s of the
  profile's `AlertToneTotal.db`, `View/View.Common/NameValueVModel.cs:175-222`).

### 5.2 Operations

| Operation | Rows written | Evidence |
|---|---|---|
| Add (dialog) | append: `GroupID` new GUID (lowercase `D` format), `GroupName` user text (default `"Group N"`), `GroupNo` = smallest unused positive integer, `Note ''`, `RecordFlag` checkbox (default `1`), `Type '0'`, `Relay` (0 off, 65535 always, 1..600 s; default 0), `Priority '0'`, `DutyStatus '0'`, `AlertMode` (1 tone, 2 vibrate, 3 none, 4 tone+vibrate; default `'1'`), `AlertToneID` (default `'9999'`), `BackgroundColor` 0..7 (default `'0'`), `EncryptionKeyID` first key of the list, `KeyStrapOption '2'`, `P25TAddr` 1..65535, default = lowest unused TGID from 1, or `''` if "Enable P25T TGID" is unticked | `P25TGroupListControl.cs:196-230`, `NewP25TGroupControl.cs:97-211`, `View/View.AddrBaseControl/DigitAddrControl.cs:243-306` |
| Edit (grid) | `GroupName` (max 14, blank reverts, **duplicates allowed**: `OnlyCheckChracterEmpty = true`), `AlertMode`, `AlertToneID` (read-only unless AlertMode 1 or 4), `RecordFlag`, `Relay` (RelayControlForm), `BackgroundColor`, `EncryptionKeyID`, `P25TAddr` (DigitAddrForm: 1..65535 or unset, **no duplicate check**) | `:129-130,476-511`, `View/View.GroupSetting/DigitAddrForm.cs:42-75,188-200` |
| Delete | remove selected rows; at least one row always kept; nothing else touched (`CheckIfCanDeleteGroup` returns true) | `:232-264` |
| Paste | append with new GUID, de-duplicated name, new GroupNo, `Type '0'`, `Priority '0'`, `DutyStatus '0'`, `KeyStrapOption 2`, invalid list values reset | `:149-194` |
| Import RR CSV | needs `Decimal`, `Alpha Tag`; TGID 1..65535; skips rows whose (name cut to 14, TGID) already exist; appended rows get the Add defaults but `Note ''`, and `GroupNo` computed against the **original** table so every imported row gets the same number | `:668-783`, `P25T/ImportCsvForTGIDControl.cs:111-124` |
| Import Unication CSV | `GroupNo` = max existing (< 65535) + 1 onward; "delete all" replaces every row with new GUIDs | `:546-614,643-666` |
| Sort / drag | reorders rows, persisted, no radio effect | |

Hidden, never user-editable: `GroupID`, `GroupNo`, `Note`, `Type`,
`Priority`, `DutyStatus`, `KeyStrapOption` (resx `Visible = False`). The
template "All" row (`GroupID '0000000000000000'`, `GroupNo 65535`, `Type 2`,
`P25TAddr 65535`, `KeyStrapOption 0`) is an ordinary deletable row, and its
GroupID is the same in every system DB.

On OK the list's row count goes to the index `TGIDCount` (all rows, "All"
included) and `ZoneListControl.clearCopyP25TIdMap` clears an in-memory knob
copy/paste map (`View/View.Zone/ZoneListControl.cs:2758-2786`). Nothing else.

### 5.3 What happens to knob references

Profile tables that point at talkgroups by `GroupID`: `tabChannelGroup`,
`tabChannelRevertGroup`, `tabP25TChannelSubGroup`,
`tabP25TCallAlertSUIDGroup` (all keyed by knob `ChannelID`).
`tabP25TrunkingSystemKnob.ATGID` holds a TGID number, not a GroupID.

- **TGID change:** no profile write. The GUID link survives and the converter
  picks up the new `P25TAddr`. `ATGID` is not updated if it held the old TGID.
- **Delete:** no profile write; the knob rows become orphans.
  - Knob editor: `TrunkingRxGroupListControl.setGroup` lists only GroupIDs
    found in the system table, and `Save` deletes the knob's rows and
    re-appends only listed ones (priority first with `SeqNo` 1..n, then
    `SeqNo 0`), so orphans disappear only when that knob is re-saved, and its
    rows move to the end of the table (`View/View.Zone/TrunkingRxGroupListControl.cs:248-416,580-716`).
  - Pre-program check: a knob whose list has no valid TG with a TGID gets
    error B3 ("Group on Z/P has N invalid, reselect"); same for a revert list
    with rows but none valid; duplicate TGIDs among a knob's TGs also error.
    Any error blocks programming (`View/View.ProfileCheck/ProfileCheckHandler.cs:462-538,700-755`,
    `View/View.ProgramVersion/ProgramController.cs:30-66`).
  - Converter: receive-list orphans are skipped without consuming a contact
    ID (`CONV:963-966,1739-1744`); a revert-list orphan returns false and
    aborts programming (`CONV:1765-1770`, same in 1.41a/c); call-alert
    orphans are skipped (`CONV:3333-3338`).
- **Empty TGID on a selected TG** passes the profile check but
  `long.Parse("")` in `convertAddress` aborts conversion (`CONV:1235-1242`).
- **Duplicate GroupID** in one system DB makes the converter's
  `Dictionary.Add` throw (`CONV:918-925`).

Radio-relevant TG columns (converter): `GroupName`, `AlertMode`,
`AlertToneID`, `RecordFlag`, `Relay`, `BackgroundColor`, `P25TAddr`;
`EncryptionKeyID` only for knobs with `EncryptionBy = 1`
(`CONV:4381-4570`). `GroupNo`, `Note`, `Type`, `Priority`, `DutyStatus`,
`KeyStrapOption` are not converted. The real profile's knobs all have
`EncryptionBy 0`, and its radio `tabCipherKeyAssignment` holds only channel
keys.

## 6. Dirty, version and source fields

| Field | Behavior on edit | Evidence |
|---|---|---|
| profile `tabOrganization_New.FinishDate` | set to local `yyyy-MM-dd HH:mm:ss` (whole table rewritten) when the page saves successfully | `BasePageControl.cs:873-890` |
| system `tabVersion` (`DBV2.01`) | untouched; only `synchronizeDB` (paste, resource import) rewrites it | `SQL/SQLiteSynchronise.cs:241-347` |
| profile `tabDBVersion` | untouched | |
| `SystemSource`, `SystemCreateType` (both DBs) | untouched by edits; copy/paste/resource-add set `resource_<id>` and `2` | `P25T/P25TSyetemTotal.cs:272-273,643` |
| `DBFileName`, `P25TrunkingID` | untouched | |
| `BandTypeCode` | filled from the profile model if empty (section 2.4); RR CSV update fills only the first 2 chars | `:1221-1224,1424-1429` |
| `SiteCount`, `ControlCHCount` | system dialog OK, RR CSV update | `NewSystemControl.cs:252-253`, `:1452-1453` |
| `TGIDCount`, `SubGroupCount`, `IndividualCount` | list dialogs' OK | `:1147,1173,1199` |

There is no modification flag, hash or timestamp inside the system DB.

## 7. RadioReference site CSV "update selected item"

Reachable from the Add menu with a system selected
(`P25T/P25TSyetemTotal.cs:1347-1465`). It is a replace, not a merge:
`ImportCsvDataToSystem` clears both tables, then writes one site per distinct
(`RFSS`, `Site Dec`) (ID 1..n, `Name` = Description cut to 14, `RFSSID` = raw
CSV text, `SiteID` = `Site Dec` or `Site Hex` parsed when > 255, `Note` =
Description) and one CC per distinct non-zero frequency (ID 1..n, `RxFreq` =
MHz × 1e6, bandwidth 0, Tx 0, `Note` = Description) (`:1489-1561`). Links are
not cleared, so old link rows may now point at unrelated IDs. Name, WACN and
System ID typed in the dialog are ignored in update mode. The limit check uses
`256 - current count` although the import replaces everything (`:1433-1434`).

## 8. Empirical checks on the real profile

One profile, 7 index rows, 9 system DB files (2 not indexed and not on any
knob), 9 P25T knobs over 7 systems, 134 `tabChannelGroup` rows on those
knobs, 0 orphan references.

| Check | Result |
|---|---|
| GUID case | profile, system file names, `P25TrunkingID`, `DBFileName`, `tabP25TrunkingSystem.ID`, every `GroupID`, every `tabChannelGroup.ChannelID/GroupID`: lowercase hyphenated. Template "All" row `GroupID` is `'0000000000000000'` |
| NULL vs `''` | zero NULLs in any column of any system table or the index. Unset values are `''` (`SUID`, FSS ranges, notes, `SystemSource`, `tabChannelGroup.SigType`) |
| Index vs system DB | `SystemName`, `WacnID`, `SystemID`, `SystemCreateType`, `SUID` and all five counts match for every indexed system; `TGIDCount` counts the "All" row |
| `SystemSource` | 6 of 7 `''`; the RadioReference-API system has `RadioReference-<sid>` in the system DB but `RadioReference_<sid>` in the index (two format strings, `View/View.RadioReference/RadioReferencehandler.cs:257,352`) |
| `SystemCreateType` | `2` everywhere |
| Name lengths | max 14 for system, site and TG names |
| `RFSSID`, `SiteID` | all-digit integer text; values above 9 occur and none contain A-F, consistent with decimal storage |
| Site IDs | 1..n in row order in every system; no duplicate (RFSS, Site) pairs |
| CC IDs | 1..n; no duplicate `RxFreq`; two systems not in ascending frequency order (ID order is the user's add order); `RxBandWidth`/`TxBandWidth` `1` everywhere |
| `TxFreq` | creation row keeps the band minimum (section 4); editor-added `''`; RR-API system = `RxFreq` |
| TG names | duplicates exist (1408 rows, 1406 distinct names) |
| TGIDs | one system pair has TGID 65535 twice; no empty TGIDs |
| `GroupNo` | two RR-CSV-imported systems have 34 and 11 rows sharing one `GroupNo` (the import bug in 5.2) |
| TG defaults | `KeyStrapOption 2`, `DutyStatus 0`, `Priority 0`, `Type 0` on non-"All" rows; "All" row `Type 2`, `KeyStrapOption 0` |
| `tabEditPermission` | identical to the template in all 9 systems |
| `tabRFBandTypeInfo` | one row per single-band system (`SX` 763000000-870000000 or `VX` 136000000-174000000); none for the `SXVX` RR-API system (`newP25Trunking` cannot match a 4-char code, `:1014-1019`) |
| `tabVersion` | `DBV2.01` everywhere |
| `tabOrganization_New.FinishDate` | 19-char `yyyy-MM-dd HH:mm:ss` |

Contradictions with earlier notes or a plain reading of the UI:

1. data-model 5.3 / limits-and-misc: "no duplicate control-channel
   frequencies" and "must have >= 1 control channel unless full-spectrum scan"
   are not enforced (dead code). Real data has no duplicates only because
   users never created any.
2. data-model 5.1 gives `SystemSource` as `resource_<id>` or
   `RadioReference-<sid>`; the index row of an API import uses an underscore.
3. data-model 5.3 says the editor writes `TxFreq ''`; true for added rows,
   but the creation row keeps `TxFreq` = band minimum forever.
4. TG `DutyStatus` defaults to `'0'` from the UI, not the DDL default `1`.
5. `tabChannelGroup.SigType` is `''`, never 9 (the knob editor never sets it).

## 9. Implementation spec

### 9.1 Common rules for Knobset

- Work on a copy of the profile DB and the one system DB; commit both or
  neither.
- Rewrite each touched table the PPS way, but inside **one** transaction per
  DB: `BEGIN; DELETE FROM <t>; INSERT INTO <t> VALUES('v1','v2',...); ... COMMIT;`
  with columns in CREATE TABLE order, every value single-quoted text with `'`
  doubled, and `''` for any absent or NULL value. Preserve existing row order;
  append new rows; remove deleted rows in place.
- Never write NULL. Integers as plain decimal text (`'851012500'`, `'0'`).
- New GUIDs: lowercase, hyphenated, 36 chars (.NET `Guid.ToString()`).
- New integer IDs (site, CC, `GroupNo`): smallest unused positive integer,
  compared as strings over existing values (PPS fills gaps).
- CC IDs must be unique integers (rowid); rows come back in ID order, so to
  move a CC you must renumber IDs.
- On save, update the index row (9.6) and set
  `tabOrganization_New.FinishDate` to local `yyyy-MM-dd HH:mm:ss`.
- Write guard (system DB): only the tables named per operation below. Never
  touch `tabEditPermission`, `tabVersion`, `tabP25TSignaling`,
  `tabRFBandTypeInfo`, `tabSUID`, `tabSubGroup`, `tabMember_New`.

### 9.2 System properties

SQL: rewrite `tabP25TrunkingSystem` (one row) changing only `Name`,
`WacnID`, `SystemID`, `SiteTrunkingFlag`, `SUID`, and when FSS is edited
`FullSpectrumScanFlag`, `FullSpectrumScanRanges` and all nine range columns
(write all nine, as PPS does). Equivalent single-row form:
`UPDATE tabP25TrunkingSystem SET Name='..', WacnID='..', ...;`. Keep `ID`,
`SystemCreateType`, `SystemSource`, `ConverageType`, `ErrorRateThreshold`.

Validate: `Name` 1..14 chars, not whitespace-only, unique (trimmed,
case-sensitive) among other index rows; WACN 0..0xFFFFF; System ID 0..0xFFF;
flag 0/1; SUID `''` or 0..16777215; FSS when on: ranges 1..3, each start and
end inside one Rx range of the same band of the system's `BandTypeCode`, step
1..1,000,000, start + step <= end. Also enforce what PPS intends but skips: at
least one CC unless FSS is on.

Profile writes: index row `SystemName`, `WacnID`, `SystemID`, `SUID`.

Radio tables after conversion (only if a knob uses the system):
`tabSystemAlias` (Name); `tabP25TrunkingSignaling` (WACN, System ID,
SiteTrunkingFlag, FSS fields). SUID: none (it reaches `tabCallAlertGroup`
only after the knob is re-saved; PPS does not cascade it, neither should we).

### 9.3 Sites

SQL: rewrite `tabP25TrunkingSite` (`ID, Name, RFSSID, SiteID, Note`); on
delete also rewrite `tabP25TrunkingSiteControlCH` minus rows whose `SiteID`
equals the deleted site's `ID`.

- Add: append `(nextId, 'Site N' or user name, rfss, site, '')`; default pair
  1/1 or first free pair.
- Edit: change `Name`, `RFSSID`, `SiteID` in place; keep `ID`, `Note`.
- Delete: remove the row; refuse to delete the last site.
- Reorder: allowed; order persists and changes radio `SeqNo`.

Validate: 1..256 rows; `Name` 1..14, not blank, unique; RFSS 0..255, Site
0..255, stored decimal; (RFSS, Site) unique; `ID` unique positive integer.

Radio tables: `tabP25TrunkingSites` (rows, names, `SeqNo` 256 down in row
order).

### 9.4 Control channels

SQL: rewrite `tabP25TrunkingControlCH` (`ID, RxFreq, RxBandWidth, TxFreq,
TxBandWidth, Note`); on delete also rewrite `tabP25TrunkingSiteControlCH`
minus rows whose `ControlChannelID` is the deleted ID.

- Add: `(nextId, rxHz, '1', '', '1', '')`.
- Edit: change `RxFreq` (and `Note`) only; leave `TxFreq`, bandwidths as
  they are (matches PPS, including the stale creation-row `TxFreq`).
- Delete: remove; zero rows allowed by PPS but blocked by our 9.2 rule
  unless FSS is on.
- Reorder: renumber IDs 1..n in the new order and remap
  `tabP25TrunkingSiteControlCH.ControlChannelID`; PPS cannot do this, so treat
  it as a Knobset extension.

Validate: 0..256 rows; `RxFreq` integer Hz inside the system band's Rx
ranges (whole-MHz bounds, inclusive; union of both codes for a 4-char
`BandTypeCode`); reject duplicates (PPS does not, the converter would drop
them); `Note` <= 64 chars; `ID` unique integer.

Radio tables: `tabP25TrunkingControlChannels` (distinct `RxFreq` in ID order,
`SeqNo` 256 down, `RfssID 0`, `SiteID 0`). Links, bandwidth, Tx and notes: no
radio change.

### 9.5 Talkgroups

SQL (system DB): rewrite `tabGroup_New` (`GroupID, GroupName, GroupNo, Note,
RecordFlag, Type, Relay, Priority, DutyStatus, AlertMode, AlertToneID,
BackgroundColor, EncryptionKeyID, KeyStrapOption, P25TAddr`).

- Add: append `(newGuid, name, nextGroupNo, '', recordFlag(1), '0',
  relay(0), '0', '0', alertMode('1'), alertTone('9999'), bg('0'),
  firstKeyId, '2', tgid)`.
- Edit: change only `GroupName`, `AlertMode`, `AlertToneID`, `RecordFlag`,
  `Relay`, `BackgroundColor`, `EncryptionKeyID`, `P25TAddr`; keep the rest.
  Do not reproduce PPS's open-time normalizations (5.1) except as explicit
  repairs; they rewrite every row.
- Delete: remove the row; keep at least one row. Then, in the **profile**,
  for `ChannelID` in `(SELECT ChannelID FROM tabP25TrunkingSystemKnob WHERE
  P25TrunkingID = '<sys>')`, delete rows with that `GroupID` from
  `tabChannelGroup`, `tabChannelRevertGroup`, `tabP25TChannelSubGroup`,
  `tabP25TCallAlertSUIDGroup`, removing them in place (other rows keep their
  order), and renumber the remaining `SeqNo > 0` rows of each affected knob
  1..n in their current `SeqNo` order. Scope by ChannelID is mandatory: the
  "All" GroupID `'0000000000000000'` is shared by every system. This goes
  beyond PPS, which leaves orphans, but gives the same radio output for
  receive lists and avoids the revert-list conversion failure. After the
  cascade, flag any knob left with no talkgroup (PPS would block programming).
- TGID change: no profile write. Warn if a knob's `ATGID` equals the old
  TGID.

Validate: 1..8192 rows; `GroupID` unique lowercase GUID (or the 16-zero
"All" id); `GroupName` 1..14, not blank (duplicates allowed, warn);
`P25TAddr` 1..65535 and required (PPS allows `''`, which breaks conversion);
warn on duplicate TGIDs in the system, error when two TGs with one TGID are
on the same knob; `AlertMode` 1..4; `AlertToneID` in {0, 9999, 10000, 1..8,
profile custom tone IDs}; `RecordFlag` 0/1; `Relay` 0, 65535 or 1..600;
`BackgroundColor` 0..7; `EncryptionKeyID` an ID in profile
`tabEncryptionKeyList`.

Radio tables (only for TGs selected on some knob of the system; the
alert, tone, storage and colour rows are measured and implemented in
[talkgroup-settings.md](talkgroup-settings.md)):

| Change | Radio tables |
|---|---|
| name | `tabContacts` |
| alert mode | `tabRecvBehaviorByContactID` |
| alert tone | `tabRecvToneForContacts`, `tabContactAddrAndAlertTone`; with custom tones also `tabTone`, `tabToneFileDetail` (custom tone numbering is first-use order) |
| record flag | `tabRecvVoiceCallRecordByContactID`, `tabRecvBehaviorByContactID` |
| relay | `tabContactOfRelayCtl` |
| background colour | `tabContactOfBackground` |
| TGID | `tabContactAddr`, `tabRecvGroupList`, `tabContactAddrAndAlertTone`, `tabCallAlertGroup`, `tabSubgroupWithMultiSubAddr` and sub-group address rows, `tabCipherKeyAssignment` (knobs with `EncryptionBy 1` only) |
| key alias | `tabCipherKeyAssignment` (knobs with `EncryptionBy 1` only) |
| delete of a selected TG, or the cascade | every contact-keyed table, because later group contact IDs shift down: `tabContacts`, `tabContactAddr`, `tabRadioGroup`, `tabRecvVoiceCallRecordByContactID`, `tabContactOfRelayCtl`, `tabContactOfBackground`, `tabRecvToneForContacts`, `tabContactAddrAndAlertTone`, `tabSubgroupWithMultiSubAddr`, `tabContactOfChan`, `tabRecvGroupList`, `tabRecvBehaviorByContactID`, `tabRevertListGroup`, `tabP25TQCDelayRecordParm`, `tabCallAlertGroup`, `tabCipherKeyAssignment` |
| add, or edit of an unselected TG | none |
| reorder `tabGroup_New` | none |

### 9.6 Index row sync (profile `tabP25TrunkingSecurity`)

After any system-DB write, update the row with `P25TrunkingID = '<sys>'`:
`SystemName` = `tabP25TrunkingSystem.Name`, `WacnID`, `SystemID`, `SUID`
mirrored; `SiteCount`, `ControlCHCount`, `TGIDCount` (all `tabGroup_New`
rows), `SubGroupCount`, `IndividualCount` = row counts; `BandTypeCode` = the
profile model's codes if it was empty (PPS behavior), else unchanged. Leave
`P25TrunkingID`, `DBFileName`, `SystemCreateType`, `SystemSource` alone, and
keep row order. The converter never reads this table, but the knob page and
the pre-program check do.

### 9.7 Write-guard allowlist

| Operation | System DB tables | Profile tables | Radio tables (expected diff) |
|---|---|---|---|
| System properties | `tabP25TrunkingSystem` | `tabP25TrunkingSecurity`, `tabOrganization_New` | `tabSystemAlias`, `tabP25TrunkingSignaling` |
| Sites | `tabP25TrunkingSite`, `tabP25TrunkingSiteControlCH` (delete only) | same two | `tabP25TrunkingSites` |
| Control channels | `tabP25TrunkingControlCH`, `tabP25TrunkingSiteControlCH` (delete, reorder) | same two | `tabP25TrunkingControlChannels` |
| Talkgroup add / edit | `tabGroup_New` | same two | contact tables per 9.5 |
| Talkgroup delete | `tabGroup_New` | same two plus `tabChannelGroup`, `tabChannelRevertGroup`, `tabP25TChannelSubGroup`, `tabP25TCallAlertSUIDGroup` | all contact-keyed tables per 9.5 |

A system referenced by no knob produces no radio diff for any operation.
