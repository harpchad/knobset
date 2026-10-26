# Pager settings: talkgroup hold and lockout, display, alerts, backlight, voice storage, menu items

Spec for Knobset's Pager settings page: the device-wide settings of PPS
steps E8, E9, E11, E12, E14 and E16 (ui-inventory 2.6) that matter on a
scanner. Paths in `file:line` citations are relative to `decomp/`; `GS/`
means `View/View.GeneralSetting/` and `PV/` means `View/View.ProgramVersion/`.
Labels and control defaults come from the `.resx` files next to each
control (`View/View.GeneralSetting.<Control>.resx`, read with
`scratch/tools/resx_fields.py`); on-pager behaviour comes from PPS's help
file (`scratch/chm/topics/ParameterDescription/`).

## 0. Summary

- Every setting here lives in one small settings table of the profile
  (`<GUID>.db`), one row each except `tabFuncClassToneVolume` (one row per
  tone class) and `tabMemberFunctionList` (one row per function).
- **The converter already carries every one of them from the profile into
  the radio DB.** converter-1.41.md lists several of their radio tables as
  "written, equal to template" only because both real profiles hold the
  template values. No converter change is needed, and the golden test is
  untouched.
- One gap: the status-bar RSSI type (E16) only exists on DBV1.41c and later.
  The 1.41b converter never reads `tabHomeSettings.StatusBarRssiShowType`
  and the 1.41b radio template has no `tabFunctionsOnOff.SignalLevel`
  column. Knobset refuses that setting on a 1.41b pager instead of writing a
  change that can't reach it.
- Talkgroup hold and remove are two flags in `tabFunctionsOnOff`. **No
  function-list entry gates them**: the function catalog has no hold or
  remove item, PPS's E12 page never touches the function list, and the
  converter writes the flags unconditionally. The only related menu item is
  function 852 "TGID Hold Timeout", which adds the timeout to the pager's
  Function Setting menu so it can be changed on the pager. It's off in both
  real profiles and is offered under Pager menu items.
- **There's no lockout list to pre-load.** Neither the profile schema nor
  the 1.41 radio schema has a table of removed talkgroups, and PPS's help
  says a removal is temporary ("recovery by switch knob selector", up to
  64). The way to keep a talkgroup off a knob for good is to take it off the
  knob's list, which Knobset already does.
- One new edit kind, `Edits.settings`: a map from setting key to the new
  value in PPS's own encoding. Only keys whose value differs from the
  profile are applied. Each key has its PPS range and its own short list of
  radio tables, so a settings change can combine with any other edit.

## 1. How PPS saves these pages

Each page's `Save()` copies its controls into the row(s) of its
`DataTable` and calls `SaveDataTableEx` (e.g. `GS/TGHoldAndRemoveSettingControl.cs:58-66`).
`SaveDataTableEx` deletes the table and re-inserts every row with every
value quoted as text (`DataBase/Unication.SQLite/SQLiteDataBase.cs:598-649`,
`DataBase/Unication.SQLite/SQLiteSynchronise.cs:349-399`). Leaving the page
stamps `tabOrganization_New.FinishDate` (`View/View.BaseControl/BasePageControl.cs:873-890`).
The function list is saved the same way by
`U4PPSDataRepository.SaveMemberFunctionList`: delete `tabMemberFunctionList`,
then insert `(FunctionID, Enable)` for every function of the model's BSS
list in BSS order (`View/View.Profile/U4PPSDataRepository.cs:311-325`).

Knobset updates the changed column in place. For these single-row tables,
and for multi-row tables rewritten in their existing order, the result is
the same rows in the same order with the same text values. Values are
written as decimal integer text (`'1'`), as `Convert.ToInt32` produces.

PPS saves every control on the page, so opening and saving a page also
normalises values the controls can't show (a `FlashDuration` of 1 becomes
0, a `MaxRecordTime` of 90 becomes 60). Knobset writes only the keys the
user changed and leaves other values exactly as they are.

## 2. The settings

Columns: Knobset key; PPS label (page section); profile column; allowed
values in the profile's encoding (PPS control limits); PPS default
(control `DefaultText`, which matches the shipped `PPS_G4A.db` row unless
noted); converter rule; radio tables a change touches (measured, section 7).

### 2.1 Talkgroup hold and remove (E12)

Page: `GS/TGHoldAndRemoveSettingControl.cs`; profile table
`tabTGIDHoldAndRemoveSetting` (one row), read at `:50-56`, saved at
`:58-66`. Converter: 1.41b `PV/PPSU4ParamConverter_01P41b.cs:4053-4066`,
1.41c `PV/PPSU4ParamConverter_01P41c.cs:586-599`; Rust
`convert141.rs` `convert_other_function_setting`.

| Key | PPS label | Column | Values | Default | Converter | Radio tables |
|---|---|---|---|---|---|---|
| `hold.enabled` | TGID Hold Setting | `TGHoldFunc` | 0 disable, 1 enable | Disable | `tabFunctionsOnOff.TGHoldFunc` | `tabFunctionsOnOff` |
| `hold.timeout` | TGID Hold Timeout (min) | `TGHoldTimeout` | 0..99, step 1; 0 = no timeout (`:156-159`) | 0 | `tabFunctionsParam.TGHoldTimeout` | `tabFunctionsParam` |
| `hold.unhold_alert` | TGID Un-Hold Alert | `UnholdAlert` | 0/1 | Disable | `tabFunctionsOnOff.UnholdAlert` | `tabFunctionsOnOff` |
| `hold.remove` | TGID Remove Setting | `TGRemvFunc` | 0/1 | Disable | `tabFunctionsOnOff.TGRemvFunc` | `tabFunctionsOnOff` |

On the pager (help `12_TGHoldRemoveSetting/*.htm`): with hold enabled a
**hold** soft key keeps the pager on the current talkgroup's traffic and
ignores other talkgroups until un-hold is pressed; the timeout releases it
after N minutes (0 = never). Un-hold alert plays a short tone on un-hold.
With remove enabled a **block** soft key drops the current talkgroup from
the knob's list until the knob is turned; up to 64 can be blocked. Both
work only in Trunking TG-Scan and TG-Monitor modes.

#### 2.1.1 Does the function list have to enable them?

No, as far as PPS and the converter go:

- The function catalog `Data_G4A.tabU4Function` has no hold or remove
  function. Its only related entry is 852 "TGID Hold Timeout" (type 8,
  parent 831 Function Setting, `FunctionOnOff 2`).
- 852 maps to one row of `tabFunctionSettingBase` (`SettingID 26`, "TGID
  Hold Timeout"). Enabled, it adds that item to the pager's Function
  Setting menu (`tabFunctionSettingList`) so the user can change the
  timeout on the pager (help `8_PagerFunctionListSetting/TGIDHoldTimeout.htm`).
- No `tabMenuBase`, `tabFunctionStatusBase` or
  `tabEnablePassiveWorkStationFunctionsBase` row names a hold or remove
  function, `tabRelyFunction` and `tabPartnerFunction` are empty, and the
  E12 page doesn't touch `tabMemberFunctionList`.
- Both converters write `TGHoldFunc`, `TGRemvFunc` and `UnholdAlert`
  unconditionally from the E12 row (citations above).

So the soft keys depend only on the E12 flags. Knobset doesn't turn on
852 automatically when hold is enabled; the hold section points at the
menu item instead. Whether the pager shows the soft keys with 852 off has
to be confirmed on hardware (section 9, step 2).

#### 2.1.2 Pre-loaded lockouts

None. `notes/schema/PPS_G4A.sql` has no table of removed or locked-out
talkgroups (only the `TGRemvFunc` flag), `tabChannelGroup` has no lockout
column (`OffDuty`, `Priority`, `OutOfRange`, `OTA` and the ACK flags), and
the 1.41 radio schema has none either (`tabRecvGroupList`,
`tabRecvBehaviorByContactID` and `tabContactOfChan` carry priority, duty,
alert mode and voice storage only). The help text describes removals as
temporary and cleared by turning the knob, so the pager keeps them in
memory. To keep a talkgroup off a knob permanently, take it off the
knob's receive list (Zones & knobs).

### 2.2 Display format (E16)

Page: `GS/DisplayFormatSettingControl.cs:53-85`. Three profile tables.
Converter: 1.41b `01P41b.cs:3862-3886`, 1.41c `01P41c.cs:395-419`.

| Key | PPS label | Column | Values | Default | Converter | Radio tables |
|---|---|---|---|---|---|---|
| `display.system_id` | System ID Display Format | `tabUiNumberFormat.SysIdShowType` | 0 decimal, 1 hexadecimal | Decimal | `tabUiNumberFormat.SysIdShowType` | `tabUiNumberFormat` |
| `display.call_id` | Call ID Display Format | `tabUiNumberFormat.CallIdShowType` | 0 decimal, 1 hexadecimal | Decimal | `tabUiNumberFormat.CallIdShowType` | `tabUiNumberFormat` |
| `display.date` | Date Format | `tabDateTimeSetting.date_format` | 0 MM/DD/YYYY, 1 DD/MM/YYYY, 2 YYYY/MM/DD | DD/MM/YYYY (both real profiles: MM/DD/YYYY) | `tabDateTimeSetting.date_format` | `tabDateTimeSetting` |
| `display.hour` | Hour Format | `tabDateTimeSetting.hour_format` | 0 12 hour, 1 24 hour | 24 Hour (both real profiles: 12 hour) | `tabDateTimeSetting.hour_format` | `tabDateTimeSetting` |
| `display.rssi` | Status Bar RSSI Icon Type | `tabHomeSettings.StatusBarRssiShowType` | 0 RSSI icon, 1 RSSI dBm | RSSI Icon | **1.41c+ only**: `tabFunctionsOnOff.SignalLevel` (`01P41c.cs:397-409`); 1.41b ignores it | `tabFunctionsOnOff` |

System ID format covers WACN, System ID, RFSS, Site and NAC; call ID
format covers talkgroup and unit IDs in MDC1200, DVOA, DMR, P25C and P25T
(help `16_DisplayFormatSetting/*.htm`). Decimal matches RadioReference.

### 2.3 Alerts (E11, scanner-relevant parts)

Pages: Tone Volume `GS/ToneVolumeSettingControl.cs:60-93`, Out of Range
`GS/OutOfRangeAlertSettingControl.cs:61-86`, Unread Message Reminder
`GS/UnreadMsgReminderSettingControl.cs:46-63`. Converter: tone volume
1.41b `01P41b.cs:3823-3829` (volume via `ConvertToneNewVolume`,
`PV/PPSU4ParamConverterBase.cs:2618-2625`: 0 -> 48, else `(5 - v) * 11`);
unread 1.41b `:3854-3861`, 1.41c `:379-386`; out of range 1.41b
`:3887-3904`, 1.41c `:420-430`.

| Key | PPS label | Column | Values | Default | Converter | Radio tables |
|---|---|---|---|---|---|---|
| `alerts.receive_type` | Receiving Alert Tone Volume | `tabFuncClassToneVolume.type` where `funcClass 0` | 1 by volume knob, 2 fixed | Fixed Volume | same row, same value | `tabFuncClassToneVolume` |
| `alerts.receive_volume` | (volume slider) | `.volume` where `funcClass 0` | 0 mute, 1..5 (`GS/VolumeControl.cs:415-436,546`) | 5 | `volume = 48` for 0, else `(5 - v) * 11` | `tabFuncClassToneVolume` |
| `alerts.notify_type` | Notification Tone Volume | `.type` where `funcClass 1` | 1 by volume knob, 2 fixed | By Volume Knob | same | `tabFuncClassToneVolume` |
| `alerts.notify_volume` | (volume slider) | `.volume` where `funcClass 1` | 0..5 | 3 | as above | `tabFuncClassToneVolume` |
| `alerts.oor_alert` | In Range/Out Range Alert | `tabOutRangeSetting.InOutRangeAlert` | 0/1 | Enable | `InOutRangeAlert` | `tabOutRangeSetting` |
| `alerts.oor_timer` | Out Of Range Timer (s) | `.outRangeCheckTimer` | 0..999 (`:194-212`) | 20 | `outRangeCheckTimer` | `tabOutRangeSetting` |
| `alerts.in_range_timer` | In Range Timer (s) | `.inRangeCheckTime` | 0..999 (`:219-237`) | 5 | `inRangeCheckTime` | `tabOutRangeSetting` |
| `alerts.oor_repeat` | Alert Repeatedly | `.AlertRepeatedly` | 0/1 | Disable | gates the next two: off writes `AlertInterval 0, AlertTimes 0, AlertTimeout 0` | `tabOutRangeSetting` |
| `alerts.oor_interval` | Alert Interval (s) | `.AlertInterval` | 30..3600 (`:280-296`; the 10 s step is the spinner's, typed values pass) | 180 | `AlertInterval`, `AlertTimes = AlertDuration / AlertInterval + 1` | `tabOutRangeSetting`, only while repeat is on |
| `alerts.oor_timeout` | Alert Timeout (s) | `.AlertDuration` | 30..28800 (`:303-319`) | 900 | `AlertTimeout`, `AlertTimes` | `tabOutRangeSetting`, only while repeat is on |
| `alerts.unread` | Unread Msg Reminder Alert | `tabTodaySettings.isAlertToneOn` | 0/1 | Enable | `tabTodaySettings.isAlertToneOn`, `tabFunctionsOnOff.ReminderAlertOn` and `UnreadAlertOn` | `tabTodaySettings`, `tabFunctionsOnOff` |
| `alerts.unread_interval` | Remind Interval (s) | `tabTodaySettings.AlertToneInterval` | 3..3600 (`:133-149`) | 180 | `tabTodaySettings.AlertToneInterval`, `tabUnreadAlertSettings.AlertToneInterval` | `tabTodaySettings`, `tabUnreadAlertSettings` |

PPS greys out the volume slider unless the type is Fixed
(`ToneVolumeSettingControl.cs:42-50`), the interval and timeout unless
Alert Repeatedly is on (`OutOfRangeAlertSettingControl.cs:43-47`), and the
remind interval unless the reminder is on
(`UnreadMsgReminderSettingControl.cs:33-36`). The greyed values are still
saved and still converted. Knobset greys out the same controls.

Out of range alerts only apply to P25 trunking; they beep on every coverage
change, which is the scanner complaint (help `11_AlertToneSetting/OORAlert.htm`).
The unread reminder repeats a tone while any stored message is unread.

Not included: Receiving Alert Mode durations (2-tone, 5-tone/MDC, digital;
paging only), Priority Alert Tone (a tone picker that touches `tabZone` and
the custom tone catalog), BT Headset notification.

### 2.4 Backlight and screen (E14)

Pages: LCD Brightness `GS/LCDBrightnessSettingControl.cs:46-63`, Screen
Timeout `GS/ScreenTimeoutSettingControl.cs:42-56` (maximum 60 from the
resx), Receive Background `GS/ReceiveBackgroundSettingControl.cs:61-89`.
Converter: brightness 1.41b `01P41b.cs:3796-3803`; screen timeout
`:3995-4000` (1.41c `:528-533`); receive background `:4017-4029` (1.41c
`:550-560`).

| Key | PPS label | Column | Values | Default | Converter | Radio tables |
|---|---|---|---|---|---|---|
| `backlight.mode` | Brightness Mode | `tabBacklightModeSettings.Mode` | 0 automatic, 1 manual | Manual | `tabBacklightModeSettings.Mode`, `tabBackLightModeSetting.Mode`, `tabFunctionsOnOff.backlighton = Mode` | `tabBacklightModeSettings`, `tabBackLightModeSetting`, `tabFunctionsOnOff` |
| `backlight.level` | Brightness | `tabBacklightModeSettings.BrightLevel` | 1..10 (10 % .. 100 %) | 5 (50 %) | `tabBackLightModeSetting.BrightnessLevel` | `tabBackLightModeSetting` |
| `backlight.timeout` | Default Screen Timeout (s) | `tabBacklightSettings.DefaultTime` | 0..60; 0 = always on | 10 | `tabBacklightSettings.DefaultTime` | `tabBacklightSettings` |
| `backlight.flash_on` | Flash On Time (s) | `tabRecvBackgroundColorSetting.FlashOnTime` | 1..5 | 1 | same column | `tabRecvBackgroundColorSetting` |
| `backlight.flash_off` | Flash Off Time (s) | `.FlashOffTime` | 1..5 | 1 | same | `tabRecvBackgroundColorSetting` |
| `backlight.flash_duration` | Flash Duration / Duration Time | `.FlashDuration` | 0 always, or 3..60 s (`:69-71,82-83`) | Always | same | `tabRecvBackgroundColorSetting` |
| `backlight.flash_voice` | Voice Message | `.IsVoiceFlash` | 0 off, 1 on | Off | same | `tabRecvBackgroundColorSetting` |
| `backlight.flash_text` | Text Message | `.IsTextFlash` | 0/1 | Off | same | `tabRecvBackgroundColorSetting` |
| `backlight.flash_image` | Image Message | `.IsImageFlash` | 0/1 | Off | same | `tabRecvBackgroundColorSetting` |

The `Brightness` column (59 in every profile) has no control; the
converter copies it to the radio unchanged. PPS greys out Brightness in
automatic mode and Duration Time when Flash Duration is Always. Receive
flash blinks the talkgroup's background colour while a call or message
comes in (help `14_RecvBacklightSetting/*.htm`); with the per-talkgroup
colour off it has nothing to blink.

The pager edits brightness in its own live DB (converter-1.41 4.2: unit A's
live `tabBackLightModeSetting` differed from the upload). A write replaces
that with the profile's value, as PPS does.

### 2.5 Voice storage (E9)

Page: `GS/VoiceStorageSettingControl.cs:37-51`. Converter: 1.41b
`01P41b.cs:3958-3963`, 1.41c `01P41c.cs:491-496`.

| Key | PPS label | Column | Values | Default | Converter | Radio tables |
|---|---|---|---|---|---|---|
| `voice.record_limit` | Voice Message Recording Limit | `tabVoiceStorageSetting.MaxRecordTime` | seconds: 60, 120, ... 600 (combo of 1..10 minutes, index = s / 60 - 1) | 10 minutes | `tabAudioRecordTimer.second WHERE function = 201` | `tabAudioRecordTimer` |

It caps the length of one stored voice message. Text message, canned
message and voice memo settings on the same page don't matter on a scanner.

### 2.6 Pager menu items (E8)

Page: `View/View.FunctionListSetting/FunctionSelectionControl.cs`. Two
tabs: Function Selection (top-level functions, `:117-190`) and Device
Function Setting (`:368-495`), whose checkboxes are the items of the
pager's Function Status and Function Setting menus.

Function Selection lists the model's BSS functions with no parent and
`FunctionOnOff` other than 1 (`:143-187`); Device Function Setting lists the
`FunctionOnOff 2` functions named in `Data_G4A.tabFunctionForUser` whose
parents are all enabled (`:368-417,419-495`). Toggling a top-level function
sets every child to "all parents enabled" (`:206-249`). Unchecking 401 or
411 checks the other (`tabAOrBFunction`, `:311-329`); `tabRelyFunction`
and `tabPartnerFunction` are empty. Save writes the whole member list
(section 1). The converter turns enabled functions into the radio's menus
through the `FunctionID` expressions of `Data_G4A.tabMenuBase`,
`tabFunctionStatusBase`, `tabFunctionSettingBase`,
`tabEnablePassiveWorkStationFunctionsBase` (1.41b `01P41b.cs:177-298,409-440`;
`convert141.rs` `convert_private`); converter-1.41 4.2 confirms the menus
match on both pagers.

**Implemented: the Device Function Setting items.** Each is a leaf (no
children, no A-or-B partner) and appears in exactly one menu base row, so
enabling or disabling it adds or removes one item from one radio table and
nothing else. Key `menu.<FunctionID>`, column
`tabMemberFunctionList.Enable` of that row, values 0/1. Offered only when
the row exists and every parent is enabled (PPS hides it otherwise).

| Function | Pager item (`tabU4Function.FunctionName`) | Parent | Menu row | Radio table | A/B profiles |
|---|---|---|---|---|---|
| 810 | Select Zone | 801 | status 2 | `tabFunctionStatus` | on |
| 811 | Duty On/Off | 801 | status 61 | `tabFunctionStatus` | on |
| 812 | Bluetooth Settings | 801 | status 5 (+ children 6, 7; hardware block B) | `tabFunctionStatus` | on |
| 814 | In Range/Out Range Alert | 801 | status 23 | `tabFunctionStatus` | off |
| 815 | Alert Mode | 801 | status 22 | `tabFunctionStatus` | off |
| 816 | Log Adv. | 801 | status 220 | `tabFunctionStatus` | on |
| 817 | Rev. Encrypt Msg | 801 | status 222 | `tabFunctionStatus` | on |
| 820 | TEK Keyset | 801 | status 224 | `tabFunctionStatus` | on |
| 102 | Voice Buffering On/Off | 101 | status 217 | `tabFunctionStatus` | on |
| 103 | Fixed Alert | 101 | status 211 | `tabFunctionStatus` | on |
| 112 | Display Message Content | 111 | status 214 | `tabFunctionStatus` | hidden (111 off) |
| 522 | Auto Keypad Lock | 521 | status 213 | `tabFunctionStatus` | on |
| 841 | Zone | 831 | setting 5 | `tabFunctionSettingList` | on |
| 843 | LCD Brightness | 831 | setting 3 | `tabFunctionSettingList` | on |
| 844 | Screen Timeout | 831 | setting 4 | `tabFunctionSettingList` | on |
| 845 | Date & Time | 831 | setting 1 | `tabFunctionSettingList` | on |
| 846 | Audio Squelch | 831 | setting 17 | `tabFunctionSettingList` | on |
| 848 | Date Format | 831 | setting 21 | `tabFunctionSettingList` | on |
| 849 | Hour Format | 831 | setting 22 | `tabFunctionSettingList` | on |
| 850 | Unread Msg Alert | 831 | setting 25 | `tabFunctionSettingList` | on |
| 852 | TGID Hold Timeout | 831 | setting 26 | `tabFunctionSettingList` | off |
| 853 | Factory Setting | 831 | setting 27 | `tabFunctionSettingList` | on |
| 502 | Voice Memo Setting | 501 | setting 20 | `tabFunctionSettingList` | on |

The scanner-relevant ones are 852 (change the hold timeout on the pager),
814 and 815 (switch out-of-range alerts and alert mode on the pager) and
853 (hide the factory reset).

**Not implemented, documented only:**

- Top-level functions (Function Selection: 111 text, 121 image, 131 OTA,
  132 emergency, 141 All Band Scan, 151 Monitor Mode, 161 Channel Setting,
  171 Encrypt Key Program, 323 call alert, 401/411 message boxes, 501 voice
  memo, 511 alarm, 531-535 diagnostics, RF test, RSSI, CCH/TCH log, tone
  detect). Toggling one cascades into its children, can change
  `tabMenuLayout`, `tabMenuListLayout` and
  `tabEnablePassiveWorkStationFunctions`, and 141 / 161 delete or keep the
  radio's free-scan (1025) and channel-setting (1100) zones in `tabZone`.
  Each needs its own spec and hardware test.
- 511 Alarm: it's both a top-level function and a Function Setting item.
- 412 Call Alert Message Box (a box item with two parents, 323 and 411).
- 842 Alert Tone Volume, 847 Receive Alert Mode, 818 P25T TGID Stay, 819
  Temp Recv., 851: `FunctionOnOff 0` (forced off) or absent from the menu
  base tables, so they do nothing.

## 3. Converter

No change. Every key's value reaches the radio through the code cited in
section 2, which `convert141.rs` already implements (`convert_function`,
`convert_other_function_setting`, `convert_device_parameters`,
`convert_private`). The only version difference is `display.rssi` (2.2).
Tests change every key on both real backups and check the radio value
(section 7), which also proves the ported paths with non-template values.

## 4. Implementation spec

### 4.1 Edit

`Edits.settings: BTreeMap<String, i64>`: key -> new value in the profile
encoding of section 2. Applied by `g5_codeplug::settings::apply` before the
conversion, in one profile transaction:

1. Refuse an unknown key, a value outside the key's allowed set, and
   `display.rssi` on DBV1.41b ("this pager's radio DB has no RSSI display
   setting").
2. `menu.<id>`: refuse unless `id` is in the 2.6 list, its
   `tabMemberFunctionList` row exists, and every parent's row has
   `Enable 1`.
3. Read the current value; skip the key when it's equal (no-op).
4. `UPDATE <table> SET <column> = '<value>' [WHERE <row filter>]`; refuse
   unless exactly one row changes.
5. If anything changed, `tabOrganization_New.FinishDate = now`.

It returns the applied changes `(key, old, new)` for the write plan.
A batch whose only settings keys are no-ops and that has no other edit is
"no changes to write".

### 4.2 Write guard

Allowed radio tables = the union of the "Radio tables" column for the keys
that changed. They're listed per key in `settings::SETTINGS`, so a hold
change can only touch `tabFunctionsOnOff` / `tabFunctionsParam`, a voice
limit change only `tabAudioRecordTimer`, and so on. No other edit kind
writes these tables, and settings change no row counts, so a settings
edit combines with every other edit, zone reorders and deletes included.

### 4.3 Read

`Summary.settings`: the same map, read from the profile, holding every key
of section 2 (menu keys only for items PPS would show). `display.rssi` is
present on 1.41b profiles too; the UI disables it from the backup's DB
version.

## 5. UI

A Pager settings nav item next to Zones & knobs and P25 systems, with the
same profile header and Write button. One card per section in the order of
section 2, PPS's controls (switches for 0/1, selects for lists, number
fields with the PPS range), plain labels, and a one-line note on what the
setting does on the pager. A changed control shows the amber dot and a
"was ..." hint, a per-section Undo resets it, and each changed key counts
as one pending change. Controls PPS greys out are disabled under the same
conditions. The write dialog lists each changed setting as `label: old ->
new`.

## 6. Decisions

- **No automatic 852 with hold.** PPS doesn't link them; the menu item is
  offered separately, and the hold section's note points at it.
- **RSSI type refused on 1.41b** instead of a profile-only change that the
  pager would never see.
- **Only changed keys are written**, not the whole page, so out-of-range
  values PPS would normalise stay as they are.
- **Top-level function-list items left out** (2.6).
- **No lockout list** (2.1.2).

## 7. Tests

`crates/g5-codeplug/tests/settings.rs`, on the 1.41b backup and the newest
1.41c backup:

- every key, changed to another allowed value: the profile column holds the
  new value, the radio DB differs from the backup's in exactly the key's
  tables, and the radio row carries the converted value;
- writing the current value of every key changes nothing (profile or radio);
- out-of-range values, unknown keys, hidden menu items and 1.41b RSSI are
  refused;
- the out-of-range interval with Alert Repeatedly off changes the profile
  only.

`crates/g5-core/tests/edit_set.rs`: a settings edit plus a knob rename in one
batch builds a write set whose changed tables are the union, and the zip
carries the settings; a settings-only batch of no-ops is refused. The golden
test is unchanged and still passes.

## 8. Hardware checks

None of this is tested on a pager yet. Section 9 is the plan.

## 9. Pager test plan (lowest risk first)

1. Display: switch the date format to YYYY/MM/DD and the hour to 24 h, write,
   check the clock; switch back.
2. Hold: enable TGID Hold and Remove, timeout 0; write. On a TG-Scan knob
   during a call, look for the hold and block soft keys; hold, check the
   pager stays on the talkgroup, un-hold. Block a talkgroup, check it's
   skipped, turn the knob away and back, check it returns. If the soft keys
   don't appear, also enable menu item 852 and try again.
3. Hold timeout 1 min plus menu item 852: check the timeout appears in
   Function Setting and the hold releases after a minute.
4. Alerts: receiving alert tone by volume knob, out-of-range alert off,
   unread reminder off; write and confirm the beeps follow.
5. Backlight: brightness 30 %, screen timeout 30 s; voice limit 2 min.
6. RSSI dBm (unit B only).
