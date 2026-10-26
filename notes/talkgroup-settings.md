# Per-talkgroup settings: alert mode, alert tone, voice storage, colour

Scope: the settings PPS V00-04-00-48's "Add/Edit P25 Trunking Group" dialog
and P25 Trunking Group List grid carry for each talkgroup besides alias and
TGID, what each one stores, how the 1.41 converter carries it to the pager,
and Knobset's spec for editing them, singly and in bulk. Sibling of
[system-editing.md](system-editing.md) (its sections 5 and 9.5 cover the
talkgroup list as a whole) and [radioreference-import.md](radioreference-import.md).

Evidence paths are relative to `decomp/`. Short forms: `P25T/` =
`View/View.P25TSystem/`, `NVM` = `View/View.Common/NameValueVModel.cs`,
`CONV` = `View/View.ProgramVersion/PPSU4ParamConverter_01P41b.cs`, `RS/` =
`crates/g5-codeplug/src/`.

## 0. Summary

- Four settings are worth editing for a scanner: **alert mode**, **alert
  tone**, **voice storage** and **background colour**. Each is one column of
  the system DB's `tabGroup_New` row, and each lands in a different small set
  of radio tables. None of them renumbers contacts.
- Only talkgroups that some knob lists reach the radio DB. Changing a
  talkgroup no knob uses changes the profile and nothing on the pager.
- **Relay Control** (relay hardware on chargers and amplifiers) and **Key
  Alias** (encryption key) stay as they are: no G5 scanner use, and Key Alias
  only matters on knobs with encryption, which Knobset doesn't build.
- PPS has no bulk edit: "Multi-Select" only feeds Delete
  (`View/View.BaseControl/BaseControlDefine.cs:10-30`). After a RadioReference
  import leaves hundreds of talkgroups silent, PPS makes you change them one
  grid cell at a time. Knobset edits any selection at once.

## 1. Fields, values and PPS labels

The dialog is `P25T/NewP25TGroupControl.cs`; its lists come from
`P25T/P25TGroupListControl.cs:288-358` and NVM. Labels below are PPS's
(`P25T/NewP25TGroupControl.resx`, NVM) with Knobset's in brackets where they
differ.

| Field (dialog label) | Column | Values | Editor default | RR import default |
|---|---|---|---|---|
| Alert Mode | `AlertMode` | 1 Tone, 2 Vibrate, 3 None [Silent], 4 Tone & Vib [Tone and vibrate] (NVM:21-44) | 1 | 3 |
| Alert Tone | `AlertToneID` (text) | `0` None [No tone], `9999` Default Tone, `10000` Chirp Tone [Chirp], `1`..`8` Tone 1..8, then one entry per custom tone (`tabCustomAlertTone.ID`, a GUID, from the profile's `AlertToneTotal.db`) (NVM:175-222) | `9999` | `9999` |
| Voice Storage | `RecordFlag` | checkbox: 1 on, 0 off (`P25T/NewP25TGroupControl.cs:111,180`) | 1 | 0 |
| Background Color | `BackgroundColor` | 0 Off, 1 Green, 2 Blue, 3 Cyan, 4 Red, 5 Yellow, 6 Magenta, 7 White (NVM:65-108) | 0 | 0 |
| Relay Control, Relay On Time | `Relay` | 0 Off, 65535 Always On, 1..600 s Timeout (`P25T/NewP25TGroupControl.cs:112-128,181-191`, `View/View.GroupSetting/GroupList.cs:428-466`) | 0 | 0 |
| Key Alias | `EncryptionKeyID` | an ID of the profile's `tabEncryptionKeyList` (NVM:132-148) | first key | `000000000000` |
| (hidden) Key Strapping Option | `KeyStrapOption` | 0 Clear, 1 Secure, 2 User Select (NVM:111-129); grid column hidden | 2 | 1 |

Defaults: editor `P25T/P25TGroupListControl.cs:201-217`, paste `:179-189`,
RadioReference import `View/View.RadioReference/RadioReferencehandler.cs:553-562`.

Behavior worth copying or noting:

- The tone list is greyed out unless the alert mode is Tone or Tone & Vib
  (`P25T/NewP25TGroupControl.cs:64-74`, grid `P25T/P25TGroupListControl.cs:395-406`).
  The stored tone is kept either way, and the converter writes it either way.
- Opening the grid resets a tone it can't find in the list (a deleted custom
  tone) to `0` (`P25T/P25TGroupListControl.cs:408-412`). Knobset doesn't.
- Every PPS edit writes the whole row back; Save is
  `P25T/NewP25TGroupControl.cs:175-210` for the dialog, and the grid is bound
  straight to the columns (`P25T/P25TGroupListControl.cs:1202-1245`).

Values in the real profiles (all backups, all systems): `AlertMode` 1 and 3
(3 on every row a RadioReference import added), `AlertToneID` `0` on most
rows and `9999` on the rest, `RecordFlag` 0 and 1, `BackgroundColor` 0 and 4
(red, three rows), `Relay` always 0, `EncryptionKeyID` always the
default key, `KeyStrapOption` 0, 1 and 2. No custom tones.

## 2. How the converter carries each field

Group contacts are built once per (system, talkgroup) for talkgroups listed
on a P25 knob (`RS/convert141.rs:699-750`, CONV:1160-1233), and receive
behavior once per (knob, talkgroup) (`RS/convert141.rs:777-801`,
CONV:2058-2095).

| Field | Radio table and column | Mapping |
|---|---|---|
| `AlertMode` | `tabRecvBehaviorByContactID.AlertMode` | value minus 1 (0 tone .. 3 tone + vibrate), one row per knob listing the talkgroup |
| `AlertToneID` | `tabRecvToneForContacts` (`toneID`, `alias`, `sourceID`, `type`) and `tabContactAddrAndAlertTone` (`ToneId`, `FileName`) | `9999` -> toneID 37 "Default Tone", sourceID 100; `10000` -> 39 "Chirp Tone", type 1; `n` in 1..8 -> toneID n "Tone n", sourceID 100 + n, file `AlertTone<n>.dat`; `0` -> **no row in either table**; a custom GUID -> toneID 100 + first-use index (0 if its `.wav` is missing), plus `tabTone` and `tabToneFileDetail` rows (`RS/convert141.rs:157-165,407-424,525-555,1640-1649`, CONV:1180-1233) |
| `RecordFlag` | `tabRecvVoiceCallRecordByContactID.AllowRecord` and `tabRecvBehaviorByContactID.VoiceStorage` | `AllowRecord` = the value; `VoiceStorage` 1 if the value is > 0, else 2 |
| `BackgroundColor` | `tabContactOfBackground.BackgroundColor` | the value |
| `Relay` | `tabContactOfRelayCtl` (`RelayCtrl`, `RelayOnTime`) | 0 -> (0, 0); 65535 -> (1, 0); n -> (1, n) (`RS/convert141.rs:167-175`) |
| `EncryptionKeyID` | `tabCipherKeyAssignment` | only for knobs with `EncryptionBy 1` |
| `KeyStrapOption` | none | not converted |

The knob's own alert mode and voice storage (`tabP25TrunkingSystemKnob`,
`tabAlertModeSetting`, `tabRecvVoiceCallRecordByChannel`) are separate and
unchanged by any of this.

## 3. Interaction with the RadioReference import

Knobset's import (`notes/radioreference-import.md`) marks its new
talkgroups `imported`, and `apply_system_edit` gives such a new row PPS's
importer defaults: `AlertMode 3`, `RecordFlag 0`, `KeyStrapOption 1`
(`RS/system_edit.rs:538`). A hand-added talkgroup gets the editor's
defaults (`AlertMode 1`, `RecordFlag 1`, `KeyStrapOption 2`). An explicit
setting on a new row wins over either set of defaults; `KeyStrapOption`
always follows the `imported` flag. So the user can import, then turn alerts
on for the five talkgroups they care about in the same pending change,
before anything is written.

## 4. Knobset spec

### 4.1 Model

`TalkgroupEdit` gains four optional fields; `None` means "keep the stored
value" for an existing row and "use the defaults" for a new one:

| Field | Column | Accepted new values |
|---|---|---|
| `alert_mode` | `AlertMode` | 1..4 |
| `alert_tone` | `AlertToneID` | `0`, `9999`, `10000`, `1`..`8` (`ALERT_TONES`) |
| `voice_storage` | `RecordFlag` | true -> `1`, false -> `0` |
| `background` | `BackgroundColor` | 0..7 |

`current_state` fills all four from the row, so a client that loads a
system and sends it back unchanged sends the stored values. A value equal to
the stored one is always accepted, whatever it is: a custom tone GUID, an
`AlertMode 0`, a `RecordFlag 2` from some other tool. That keeps old
profiles round-tripping, and it's how custom tones stay untouched: Knobset
never writes a custom tone ID, and a row that has one keeps it unless the
user picks a built-in tone for that row explicitly in the dialog (the dialog
shows a custom tone read-only, so in practice it doesn't).

Comparison is by value, not by text: `RecordFlag` compares as on/off, the
numeric columns as integers, the tone as text. A row is rewritten only in
the columns whose value differs; everything else in the row is copied
through as stored. Relay, Key Alias and `KeyStrapOption` of existing rows are
never touched.

### 4.2 Change accounting

`SystemChange.tgs_settings_changed` counts existing talkgroups with at least
one changed setting. It's separate from `tgs_changed` (name, TGID, note),
because a settings change doesn't renumber contacts and needs a much
narrower write guard. A talkgroup whose name and settings both change counts
in both. `is_empty` includes the new counter.

### 4.3 Radio tables per change (write guard)

New allowlist `TALKGROUP_SETTINGS_TABLES` (`RS/edit.rs`), added by
`edit_set` whenever any system reports `tgs_settings_changed > 0`:

| Change (talkgroup on a knob) | Radio tables, measured on pager A (DBV1.41b) and B (DBV1.41c) backups |
|---|---|
| alert mode | `tabRecvBehaviorByContactID` |
| alert tone, between any two of default, chirp, Tone 1..8 | `tabContactAddrAndAlertTone`, `tabRecvToneForContacts` |
| alert tone to or from "No tone" (`0`) | the same two (rows appear or disappear) |
| voice storage | `tabRecvBehaviorByContactID`, `tabRecvVoiceCallRecordByContactID` |
| background colour | `tabContactOfBackground` |
| any of these on a talkgroup no knob lists | none |

A custom tone would add `tabTone` and `tabToneFileDetail`; neither is in the
allowlist, so a write that somehow set one would be refused. Name, TGID and
delete changes keep using `TALKGROUP_EDIT_TABLES` as before.

Profile side: only the system DB's `tabGroup_New`, plus the index row and
`tabOrganization_New.FinishDate` that every system edit already rewrites
(system-editing.md 9.6).

### 4.4 UI

- Talkgroup dialog (P25 systems): Alert, Alert tone (disabled unless Tone or
  Tone and vibrate, like PPS; a custom tone shows as "Custom tone from PPS
  (kept as is)"), Voice storage checkbox, Background with a colour swatch. A
  new talkgroup starts at the editor defaults.
- Talkgroup table: Alert column (icon, mode, tone when it isn't the
  default), Storage column (microphone icon when on), a colour dot after the
  alias when the background isn't Off. The search box matches the alert
  label, so "Silent" finds every silent talkgroup.
- Bulk edit: a checkbox per row (shift-click selects a range of the rows
  shown), a header checkbox that selects or clears every row the search
  shows, and a bar with Alert / Tone / Storage / Colour pickers (each
  defaulting to "keep") and "Apply to N". RadioReference categories aren't
  stored in the profile, but the import writes RadioReference's description
  into the note, and the search covers notes, so searching a description or
  name fragment and ticking the header selects a category's worth. A bulk
  tone change skips rows that have a custom tone and says how many it kept.
- The write dialog lists "settings changed on N talkgroups" per system.

## 5. Tests

`crates/g5-codeplug/tests/system_edit.rs`:

- `talkgroup_settings_change_only_their_radio_tables`, on one backup of each
  DB version: each field alone on a knob's talkgroup changes exactly the
  table set in 4.3 and only its own column of the row; "No tone" removes the
  tone rows; all four on a talkgroup no knob uses change no radio table.
- `bulk_talkgroup_settings`: all four on every talkgroup of a system; only
  settings tables change, names and order are kept, and re-applying the
  result is a no-op.
- `new_talkgroup_settings_override_defaults`: explicit values beat the
  importer's and editor's defaults; `KeyStrapOption` follows `imported`.
- `talkgroup_settings_validation_keeps_stored_values`: out-of-range and
  custom values are refused for new values, a stored custom tone and
  `AlertMode 0` survive an edit of another field.
- `unchanged_edit_is_a_no_op` still passes unchanged.

`crates/g5-core/tests/edit_set.rs` `talkgroup_settings_build_a_write_set`:
alert mode and voice storage on five knob talkgroups pass the write guard,
change only `tabRecvBehaviorByContactID` and
`tabRecvVoiceCallRecordByContactID`, and survive the zip round trip.

## 6. Pager test plan

Lowest risk first; each step is one write with a safety backup.

1. Change the background colour of one talkgroup on a knob. Check it on the
   pager during a call on that talkgroup.
2. Turn one silent imported talkgroup's alert to Tone (default tone) and
   voice storage on. Confirm it alerts and the call shows up in the voice
   log, and that a neighbouring silent talkgroup still doesn't alert.
3. Set one talkgroup's tone to Chirp and another to Tone 3; confirm they
   sound different.
4. Set one to "No tone" with alert mode Tone, and see what the pager does
   (PPS allows it; the radio DB then has no tone row for that contact).
5. Bulk: search a category's worth of imported talkgroups, set Vibrate on
   all of them, write, and spot-check a few.
