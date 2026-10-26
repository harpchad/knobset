# Copying and moving knobs: PPS row-level behavior

Scope: what PPS V00-04-00-48 writes when the user copies a knob and pastes
it on another position, or moves a knob with Move Up/Down/Left/Right, on the
E7 Zone & Channel List grid (ui-inventory 2.6). Sibling of
[zones-and-scan.md](zones-and-scan.md) (zone reorder and its renumbering
guard) and [knob-editing.md](knob-editing.md) (knob delete, classification);
their common rules apply here.

Evidence paths are relative to `decomp/`. Short forms as in zones-and-scan:
`ZLC` = `View/View.Zone/ZoneListControl.cs`, `CF` =
`View/View.Common/CommonFunction.cs`, `RS` =
`crates/g5-codeplug/src/convert141.rs`. Measurements: Knobset's 1.41b/1.41c
converter on unit A's backup and unit B's newest backup (trunked, analog,
DMR and scan knobs), in `crates/g5-codeplug/tests/knob_copy.rs`.

## 0. Summary

- **Paste within a profile clones the source knob.** The target's receive
  row takes every value of the source's, **name and voice prompt
  included**; each detail row is appended with the new zone, knob and a
  **new ChannelID** (`Guid.NewGuid`); every per-channel row (talkgroup
  list, tones, sub-group flag, ...) is appended under the new ChannelID.
  Nothing else is created: the copy **shares the source's frequency row**,
  protocol setting and talkgroup grouping.
- A pasted scan knob keeps the **same member list** and priority channel.
  A pasted conventional knob is not added to the scans that list the source.
- Paste onto an occupied position deletes that knob first (with a
  confirmation). Paste onto the source itself is disabled.
- **Move always swaps.** Up/Down swap the knob with the same position in
  the zone above/below; Left/Right with the neighbouring position. An empty
  target is swapped too (its empty row comes back). Rows are **re-keyed in
  place** (ZoneID, KnobIndex); the **ChannelID is kept**, so scan lists,
  priority channels and every per-channel row follow the knob without being
  touched.
- The radio numbers a channel `zone position x 100 + knob`, so a move
  renumbers the moved channels: 8 to 20 radio tables change in the moves
  measured and no table gains or loses a row. A copy adds one
  channel: 8 tables for a scan, 17 to 22 for the others.
- Limits: 64 zones and 8 positions (the grid), nothing else. No channel
  count is enforced (`ChannelLimit 256` is never read, zones-and-scan 1.4).
- Knobset: Copy to... and Move to... on a knob's panel, target picked by
  zone and position. Copy targets must be empty; moves to an occupied
  position are explicit swaps. A move's positions carry no other edit in
  the same write, and the move is checked on its own as a renumbering.

## 1. The knob context menu

Right-click on a knob cell (`dgv_CellMouseDown`, `ZLC:3324-3418`, knob
branch `ZLC:3366-3415`) shows Delete, Copy, Paste and Move Up/Down/Left/Right
with the cell's `zoneID|knob` as tag:

| Item | Enabled when | Handler |
|---|---|---|
| Copy | the knob is active (mode not 0) | `copyKnobMenu_Click`, `ZLC:1593-1600` |
| Paste | something was copied and the cell isn't the copied knob | `pasteKnobMenu_Click`, `ZLC:2142-2223` |
| Move Up / Down | the knob is active and a zone row exists above / below | `ZLC:3236-3256`, `ZLC:3258-3278` |
| Move Left / Right | the knob is active and it isn't position 1 / 8 | `ZLC:3280-3300`, `ZLC:3302-3322` |

Every handler ends with `savePrivate` (`ZLC:333-386`), which rewrites the
20 zone tables from the in-memory DataTables in their row order
(`DataBase/Unication.SQLite/SQLiteDataBase.cs:652-700`).

## 2. Copy and paste

### 2.1 Two paths

Copy stores `zoneID|knob` in `m_copyZoneKnob` and also serializes the knob
with its frequency, protocol settings, talkgroups, keys and tones to the
Windows clipboard (`KnobCopyToClipboard`, `ZLC:1602`). Paste picks:

- **same profile** and the copied zone still exists: `copyKnob(source,
  target)` (`ZLC:2211-2216`), the in-profile clone below;
- otherwise (another profile, or the source zone deleted): import the
  clipboard tables with fresh IDs (`ImprotClipboardDataToKnob`,
  `ZLC:2474`), followed by a "check the copied data" prompt
  (`ZLC:2218-2221`).

Knobset copies within a profile only. Cross-profile paste belongs to
profile cloning, a separate feature.

If the target is active, Paste asks "Are you sure you want to override
Z{n}/P{k}?" and `copyKnob` empties it with `deleteKnob` first
(`ZLC:2193-2201`, `ZLC:1388`; knob-editing 2.1 for what that removes).

### 2.2 What `copyKnob` writes (`ZLC:1386-1467`)

| Table | Rows | Evidence |
|---|---|---|
| `tabReceivingMode_NewG4` | the target row (in place) gets the source row's whole `ItemArray`, then its own ZoneID and KnobIndex back: `CurrentMode`, `ChannelName`, `AlertToneID` (voice prompt), `AlertFileName` all copied | `ZLC:1390-1400` |
| `tabSingleConvCHKnob`, `tabP25TrunkingSystemKnob`, `tabAllBandScanCHKnob`, `tabAdvancedConvCHKnob` | each source row appended with ZoneID, KnobIndex and ChannelID mapped (new ChannelID = `Guid.NewGuid()`); every other column, `ConvFreqID`, `ProtocolSettingID`, `P25TrunkingID` included, unchanged | `ZLC:1401-1453`, `CF:1003-1037` |
| `tabMultiChannelScan`, `tabMultiCHScanList` | the scan knob's own rows appended with ZoneID and KnobIndex mapped; **member ChannelIDs and `FirstPriorityCH` unchanged** | `ZLC:1450-1451` (`dictionary5` has no ChannelID map) |
| `tabChannelGroupList`, `tabChannelGroup`, `tabRecvSaList`, `tabP25TSubGroupEnable`, `tabP25TChannelSubGroup`, `tabChannelRevertGroup`, `tabP25TCallAlertEnable`, `tabP25TCallAlertSUID`, `tabP25TCallAlertSUIDGroup`, `tabP25TChannelRevertSubGroup`, `tabAllBandScanBandParam`, `tabAllBandScanProtocolParam` | every row of the source ChannelID appended under the new one, in table order | `ZLC:1454-1465`, `CF:934-968` |
| `tabConvFreqList`, `tabProtocolSetting`, `tabGroup_New`, `tabGroupGrouping`, systems | **nothing**: the copy points at the source's rows | absent from `copyKnob` |
| `tabEmergencySetting`, `tabGPSInformTarget` | nothing: a copy is never the emergency or GPS channel | absent |
| other scans | nothing: the copy isn't added to the source's scan memberships | absent |

So a pasted analog knob **shares its frequency row** with the source,
which PPS allows anywhere (knob-editing 3.2: any number of knobs can point
at one row; editing it retunes all of them). A pasted DMR or P25
conventional knob shares the protocol setting and the talkgroup grouping
(`tabChannelGroupList` is cloned with the same `GroupingID`).

**Name:** the source's, unchanged, so two knobs carry the same alias until
the user renames one. PPS has no uniqueness rule for knob aliases.

### 2.3 Row order

Detail and per-channel rows are appended. Contact and P25 SigIDs follow
knob-table row order (zones-and-scan 5), so the copy's new rows come last
and existing IDs stay put; measured, no copy changes `tabContacts`.

## 3. Move Up, Down, Left, Right

### 3.1 Mechanics

Each handler swaps two positions through a placeholder zone ID
`"00000000000"` (`ZLC:3245-3247`, `3267-3269`, `3289-3291`, `3311-3313`):
target to placeholder, source to target, placeholder to source.
`moveKnob` (`ZLC:1469-1486`) re-keys ZoneID and KnobIndex in place
(`CF:970-1001`) in seven tables: `tabReceivingMode_NewG4`,
`tabSingleConvCHKnob`, `tabP25TrunkingSystemKnob`, `tabMultiChannelScan`,
`tabMultiCHScanList` (the scan knob's own member rows),
`tabAllBandScanCHKnob`, `tabAdvancedConvCHKnob`.

Consequences:

- **ChannelIDs never change.** Scans listing the knob, `FirstPriorityCH`,
  `SecondPriorityCH`, `tabEmergencySetting.ChannelID`, talkgroup lists,
  tones, sub-group flags: all keyed by ChannelID, all follow.
- A moved scan knob takes its member list along (its rows are keyed by its
  own position).
- An empty target is swapped like any other: its empty receive row moves to
  the source position. There is no "move to empty" distinct from a swap.
- Row order is kept, so contact IDs and SigIDs don't shift. But the receive
  rows swap keys, so a zone's `tabReceivingMode_NewG4` rows end up out of
  knob order (Lake knob 3 moved to Test knob 4 leaves Test's rows ordered
  4, 1, 2, 3, 5, ...). The converter emits a zone's radio `tabReceivingMode`,
  `tabZoneChannelList` and `tabChannelAnnouncement` rows in receive-row
  order (`RS:475-514`), so the radio DB gets them out of knob order too.
- `m_copyZoneKnob` follows a moved knob (`ZLC:3251-3254`).
- Four older tables keyed by (ZoneID, KnobIndex) are left alone by every
  knob operation: `tabAllBandScan`, `tabP25TKnobGroup`, `tabReceivingMode`,
  `tabSingleChannel`. All four are empty in every real profile.

### 3.2 What a move does to IDs and references

| Item | Effect |
|---|---|
| radio channel ID (ZK) | moved knob: old ZK to new ZK; a swapped knob the reverse |
| scans listing a moved channel (any zone) | member row untouched; the radio `ScanList` and `PriorityZoneKnobID` show the new ZK |
| a moved scan knob | its `tabScanListSettings` row moves to the new ZK with its members |
| `tabZone.ModifyByDevice` | flips when a zone gains its first or loses its last active knob (zones-and-scan 5) |
| contacts, SigIDs, systems | unchanged (table order kept) |
| per-channel radio rows (P25T settings, encryption, DMR/P25C signaling, alert, reset, record) | renumbered with the channel |
| pager-side state keyed by channel (TG hold and lockout, recorded voice, last channel) | not in the programmed files; unknown, a pager test (as for a zone reorder, zones-and-scan 9.8 item 3) |

The backed-up live radio DB (`root/Applications/u3db/u3db.db`) differs
from the programmed one only in `tabCurrentZone` or a setting changed on the
pager, so hold, lockout and recordings aren't kept in any file Knobset
writes.

## 4. Limits

| Rule | Value | Evidence |
|---|---|---|
| positions | knobs 1..8 per zone (the grid's 8 columns) | `ZLC:78` |
| zones | 64 | zones-and-scan 1.4 |
| channel count | none enforced (`ChannelLimit` unread) | zones-and-scan 1.4 |
| frequencies | a copy adds none (shared row), so the 1024 cap doesn't bite | 2.2 |
| what can be copied or moved | any active knob; Move needs a neighbour position or zone | 1 |

## 5. Radio tables per operation (measured)

Baseline: the backup's own radio DB (the converter reproduces it exactly,
golden test). Counts are tables whose rows differ.

| Operation | Radio tables changed |
|---|---|
| copy a trunked knob (TG Scan or TG Monitor, same or other zone) | **20**: `tabAlertModeSetting`, `tabChannel`, `tabChannelAdvInfo`, `tabChannelAnnouncement`, `tabCipherKeyAssignment`, `tabContactOfChan`, `tabDispatchChannel`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabKMFSystemByChannel`, `tabP25OTARFeatureControlByChannel`, `tabP25TSetting`, `tabR2RModeSetting`, `tabReceivingMode`, `tabRecvBehaviorByContactID`, `tabRecvGroupList`, `tabRecvVoiceCallRecordByChannel`, `tabResetModeSetting`, `tabVoiceCallSettings`, `tabZoneChannelList`; plus `tabP25OTARFeatureControl` on 1.41b |
| copy an analog knob | **17**: the 16 common channel tables (the trunked list minus `tabCipherKeyAssignment`, `tabKMFSystemByChannel`, `tabP25OTARFeatureControlByChannel`, `tabP25TSetting`) plus `tabRepeaterSettingOfChan`; plus `tabRecvSaList` with a tone (18) |
| copy a DMR knob | **19**: analog plus `tabDMRSignaling`, `tabDMRRepeaterSetting` |
| copy a P25 conventional knob | **22**: analog plus `tabP25Signaling`, `tabP25RepeaterSetting` and the three per-channel encryption tables |
| copy a scan knob (normal or priority) | **8**: `tabAlertModeSetting`, `tabChannelAnnouncement`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabReceivingMode`, `tabRecvVoiceCallRecordByChannel`, `tabResetModeSetting`, `tabScanListSettings` |
| any copy into a zone with no active knob | the above plus `tabZone` |
| move Lake 3 (priority channel of the Lake 2 scan) to Test 4 | 19, row counts equal; `PriorityZoneKnobID` `203` to `304` |
| move a trunked knob within its zone | 20, row counts equal |
| move a DMR knob within its zone | 19, including `tabDMRSignaling`, `tabDMRRepeaterSetting` |
| move a scan knob to another zone | 8, row counts equal |
| swap two analog knobs in one zone | 11 |
| swap two trunked knobs across zones | 10 |
| move into a new empty zone | the move's tables plus `tabZone` |
| any move, then the reverse move (or a swap twice) | **none** |

No copy changes `tabContacts` or any system table: the copy lists the
same talkgroups, which are already contacts. Every copy only adds rows.

## 6. Implementation spec

### 6.1 Common rules

As knob-editing 8.1: one transaction per operation, positions by zone name
and knob 1..8, `FinishDate` on every write, classification from
`CurrentMode` with exactly one matching detail row (1/33 conventional,
16/48 trunked, 4/36 scan; anything else refused). In addition, refuse a
position with rows in any of the four untouched tables of 3.1.

### 6.2 Copy a knob to an empty position

Inputs: source zone and knob, target zone and knob, the copy's name (the UI
fills in the source's, as PPS does; the user can change it), a new
lowercase GUID ChannelID (not for scan knobs).

Validate: source active and classified; `refuse_sub_features` on its
ChannelID (sub-groups, call-alert groups, revert talkgroups: Knobset
models none of them); target empty with no leftover detail or scan rows;
name 1..14 printable ASCII; ChannelID a new GUID; target not the source.

Rows, as `copyKnob` (2.2): the target receive row takes the source's
values with the new name; detail and scan rows appended with the new
position and ChannelID (scan member IDs kept); per-channel rows appended
under the new ChannelID. Frequency, protocol setting, grouping and systems
are shared.

**Decision: the copy shares the frequency row**, as PPS's paste does. The
radio DB is the same either way (frequencies are per channel on the
radio), and Knobset already handles shared rows: editing one knob's
frequency gives it its own row (`update_conv_knob` takes a new `freq_id`
when the row is shared), removing one keeps the row while the other uses
it.

**Decision: the target must be empty.** PPS's paste-over is a delete plus
a copy; in Knobset the user removes the knob first and writes, so the
removal's scan checks (knob-editing 8.2) run on their own.

Write guard: trunked `KNOB_CHANNEL_TABLES` plus `TRUNKED_COPY_TABLES`
(`tabContactOfChan`, `tabRecvGroupList`, `tabRecvBehaviorByContactID`, not
the contact list); analog `CONV_KNOB_CHANNEL_TABLES`; digital
`CONV_KNOB_CHANNEL_TABLES` plus `DMR_KNOB_TABLES` and `P25C_KNOB_TABLES`;
scan `SCAN_KNOB_TABLES`.

Copies apply after other new knobs and before scan edits, so a scan edit
in the same write can list a copied analog channel.

### 6.3 Move a knob, or swap two

Inputs: source and target positions, and `swap`. An empty target with
`swap` off is a move; an active target needs `swap` on (Knobset never
swaps by accident); `swap` on an empty target is refused.

Rows: detail and scan rows re-keyed in place through PPS's placeholder,
as `moveKnob` (3.1). **Decision: the two receive rows swap their values
instead of their keys**, so every zone's receive rows stay in knob order
and the radio DB matches one where the knob was created at its new place.
PPS's key swap leaves rows out of knob order (3.1), a shape no real pager
backup has; the profile content is the same either way (PPS reads receive
rows by ZoneID and KnobIndex). Contact and SigID order is unaffected,
because the detail rows keep their places.

Report the scans that list a moved channel; they follow it.

Write guard, reusing the zone reorder's renumbering check: every changed
table must be keyed by channel ID (`ZONE_RENUMBER_TABLES` plus
`DIGITAL_RENUMBER_TABLES`, the DMR and P25 conventional signaling tables)
and keep its row count, `tabZone` aside.

### 6.4 Conflicts with other edits in one write

Positions in a write are named as they are before it.

| Rule | Why |
|---|---|
| a copy's source carries no other knob edit (rename, talkgroups, system, conventional edit, removal, scan edit) | what's copied is the stored knob; the user writes the change first |
| a copy's target carries no other edit and no second copy, and isn't itself a copy source | it's an empty position until the copy lands |
| a moved or swapped position (either end) carries no other knob edit, isn't copied or a copy target, and is in only one move | a moved knob's edits would have to name it by its new position; refusing is simpler than proving the order safe |
| moves and copies don't combine with a zone reorder or zone delete | those renumber every channel and are written alone already |

**Decision: moves are applied last and checked separately.** `edit_set`
converts after all other edits, applies the moves, converts again, checks
the first result against the other edits' allowed tables and the second
against the first with the renumbering check. So a move can share a write
with unrelated edits (other knobs, systems, new zones) and each part is
still proven: the other edits add or change only what they may, the moves
only renumber. Moving into a zone created in the same write works (zone
creates come first).

### 6.5 Write-guard constants

```text
TRUNKED_COPY_TABLES = tabContactOfChan, tabRecvGroupList, tabRecvBehaviorByContactID
DIGITAL_RENUMBER_TABLES = tabDMRSignaling, tabDMRRepeaterSetting,
  tabP25Signaling, tabP25RepeaterSetting
```

`ZONE_RENUMBER_TABLES` doesn't list the digital signaling tables, so a zone
reorder that renumbers a DMR or P25 conventional knob is refused today
(measured with the DMR knob in unit B's Test zone). Adding
`DIGITAL_RENUMBER_TABLES` to the reorder guard is the fix; it's left to
the zone code.

### 6.6 On-pager tests and open risks

1. **Copy an analog knob** to an empty position in another zone, rename it,
   write: both knobs receive; then edit the copy's frequency in a second
   write and confirm the original keeps its own.
2. **Copy a trunked knob** within its zone: the copy scans the same
   talkgroups; TG Monitor copies too.
3. **Copy a scan knob**: the copy scans the same members.
4. **Move a scan member** (Lake 3, the priority channel) to another zone:
   the priority scan still samples it at its new place.
5. **Swap** two knobs in one zone, then two across zones.
6. Pager state keyed by channel after a move: TG hold and lockout lists,
   recorded voice and message history, the last selected channel.
7. Copy and move of DMR and P25 conventional knobs (experimental like the
   knobs themselves).

### 6.7 PPS behavior Knobset skips

| PPS behavior | Why skip |
|---|---|
| paste over an active knob in one step | the removal half has its own scan checks; do it as a separate write |
| cross-profile paste through the clipboard | profile cloning is a separate feature |
| receive rows swapped by key | out-of-order radio rows; swap values instead (6.3) |
| implicit swap on every move | Knobset asks for a swap explicitly |
| copying knobs with sub-groups, call-alert groups or revert talkgroups | Knobset doesn't model them (refused as for every knob edit) |
