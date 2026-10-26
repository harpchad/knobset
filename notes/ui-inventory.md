# G Series PPS V00-04-00-48: screen and workflow inventory, UX pain points

Scope: what the user sees and clicks, which of it matters to a hobbyist using a
G5 as a P25 trunking scanner, and what a replacement should do differently.
Storage, limits, enums and wire protocol are in `data-model.md`,
`operations.md` and `protocol.md`; this file links to them instead of
repeating them.

Sources:

- Help file `PPSHelper.chm`, extracted to `scratch/chm/` (topics under
  `scratch/chm/topics/`, flattened text in `scratch/chm_dedup.txt`).
- Decompiled UI under `decomp/View/`. Paths below are relative to `decomp/`.
- English labels pulled from every step's `.resx` into
  `scratch/resx_fields.txt` (script `scratch/tools/resx_fields.py`). Controls
  carry `ParameterText` (label), `DefaultText` (default column), `Items`
  (combo choices) and `Unit`. Some prompt strings live only in
  `PpsLanguage.Properties.LKeyU4.resx`, whose neutral culture is Chinese
  (`scratch/lkeyu4.txt`); English for those is paraphrased here.

Relevance scale used throughout (hobbyist = receive-only P25 trunking
scanning, maybe a few conventional channels):

- **High**: needed to get a working scanner, or touched on every edit.
- **Medium**: worth exposing; affects day-to-day behaviour.
- **Low**: rarely changed; sane defaults are fine.
- **Not relevant**: paging, dispatch, QCII/2-Tone/5-Tone/MDC/DVOA, encryption,
  BLE/PS&DRT, tow truck, engineer-only.

---

## 1. Screenshots in the help file

All under `../scratch/chm/img/`. The most useful for
the trunking flow are the `3_1_*` set (P25 trunking quick guide).

| File | Shows |
| --- | --- |
| `QuickGuide_1.jpg` | Edit window: step rail (E1..E7, scroll arrows, E19 Program), E1 Basic Information Setting, Save/Next toolbar |
| `E_EditProfile_1.jpg` | View/Edit Existing Profile list (No., Profile Name, Model Number, Band, Version, Date Created, Modified Time) and Edit/Program/Export/Delete/Read Device/Copy/Back |
| `D_CreateProfile_1.jpg` | Create New Profile model picker with filters and Search By Condition / Search By Connected Device |
| `3_1_GeneralReceptionSetting_1_1.jpg` | E2 P25 Trunking System List grid |
| `3_1_GeneralReceptionSetting_1_2.jpg` | Edit P25 Trunking System dialog (alias, WACN, System ID as dec + hex spinners, Site Trunking, Site List, Control Channel List, Full Spectrum Scan) |
| `3_1_GeneralReceptionSetting_1_3.jpg` | P25 Trunking System Site List dialog |
| `3_1_GeneralReceptionSetting_1_4.jpg` | Control Channel List dialog (states the band's Rx range, e.g. 763~776, 851~870 MHz) |
| `3_1_GeneralReceptionSetting_1_5.jpg` | Add P25 Trunking Group dialog (one talkgroup) |
| `3_1_GeneralReceptionSetting_1_6.jpg` | P25 Trunking Group List dialog (Max 8192) |
| `3_1_GeneralReceptionSetting_1_7.jpg`, `_1_8.jpg` | Sub-group and individual call lists |
| `3_1_GeneralReceptionSetting_2.jpg` | E4 Protocol Parameter Setting with a P25 Trunking protocol |
| `3_1_GeneralReceptionSetting_3_1.jpg` | E7 Zone & Channel grid with the Select Receiving Mode dialog |
| `3_1_GeneralReceptionSetting_3_2.jpg` | Trunking TG-Scan knob editor, top (Zone Knob Information, Trunking System Setting, Binding Feature) |
| `3_1_GeneralReceptionSetting_3_3.jpg` | Knob editor Talk Group List (Available / Priority / Non-Priority, Add >> / << Remove, Max 64) |
| `3_1_GeneralReceptionSetting_3_4.jpg`, `_3_5.jpg` | P25T Sub-Group Setting and Assign P25T Sub-Group dialogs (QCII paging) |
| `C_1_P25TSystemManager_1.jpg`, `_2.jpg` | C1 P25 Trunking System Manager (library) list and editor |
| `C_2_AlertToneManager*.jpg`, `C_3_VoicePromptManager*.jpg` | Custom alert tone and channel voice prompt libraries |
| `F_*`, `G_*`, `H_ProgramLog*.jpg`, `B_1_UserSetting*.jpg` | Import/export, M5/M6 clone, programming log, PPS user accounts |
| `1_*`, `2_*` | 2-Tone and P25 conventional QCII examples (paging; not relevant) |

The help file has no screenshot of Read Device, the Program step, the
RadioReference importer, or TG CSV import.

---

## 2. Navigation structure

### 2.1 Shell

1. **PPS Login** (`View/View.Login/Login.cs`): User Name / Password, local
   only (`operations.md` section 2, "PPS login"). Typing user `Unication`,
   password `Engineer` sets `UserType.Engineer` (`Login.cs:107-114`), which
   unlocks the engineer-only items marked below.
2. **Main window**: left rail of lettered buttons built by
   `View/View.MenuBar/MainMenuHandler.cs:44-60`; content pane on the right;
   bottom status bar (`View/View.StatusBar/Statusbar.cs`) with connected
   device count, user, version, Help, Detail, and an image-only
   **RadioReference** button (`Statusbar.cs:203-214`).
3. **Welcome** page: "Latest Programmed Profiles" list.
4. Each opened profile gets its **own top-level edit window**
   (`View/View.EditProfileUI/EditProfileController.cs:52-95`), titled
   `<entry> | <profile name> | <model> | <band>`. Several can be open at once.

### 2.2 Main menu (letters as shown in the UI)

| Letter | Item | Opens | Hobbyist |
| --- | --- | --- | --- |
| A | User Guide | CHM viewer | Low |
| B | Setting | B1 User Name and Password Setting (PPS login accounts); B2 PPS Option Setting (one field: "Unication PPS Improvement Program", Don't join / Join); Device Connection Setting (USB or IP, engineer only) (`StepMenuhandler.cs:91-101`) | Not relevant |
| C | Customer Public Resource Manager | C1 P25 Trunking System Manager, C2 Customer Alert Tone Manager, C3 Customer CH Voice Prompt Manager (`:150-157`) | C1 Medium (system library), C2/C3 Low |
| D | Create New Profile | model picker (`View/View.Profile/SelectNewModelNumberControl.cs`) | High |
| E | View/Edit Existing Profile | profile list (`View/View.Profile/ProfileListUI.cs`) | High |
| F | Profile Import/Export | F1 Import Single, F2 Import Multiple, F3 Export Single, F4 Export Multiple (`.unipps`), F5 Export Trace Log (not in manual) (`:112-121`) | Medium (backup/share) |
| G | Other Voice Pager Clone to Gx Pager | G1 M5 Clone, G2 M6 Clone (Motorola Minitor codeplug import) | Not relevant |
| H | Programming Log | searchable log: Profile Name, User Name, MSN, Radio No., Programming Time, DBVersion, PPSVersion | Low |
| I | Other Tools (engineer only) | Super Password, Default Profile Update, Log Level Setting (`:167-174`) | Not relevant |

An unused `getCreateProfileMenuList` (`:103-110`) still lists "Copy from the
Existing Profile", "Read Profile from Device" and "Import Data From Excel";
only the first two survive, as buttons on the profile list.

### 2.3 D. Create New Profile

Screen: **Filter Condition** (Voice Pager Model, First Band Type, Second Band
Type, Protocol checkboxes Analog / DMR / P25 Conventional / P25 Trunking),
**Filtered Model Number List** grid (Model Number, Band, Protocol, Hardware,
Software), buttons Search By Condition, **Search By Connected Device**, and
Select. Select asks to confirm the model, copies the template
(`ProfileManagerBiz.PrepareNewDB`), and opens the edit window at step D1
(`SelectNewModelNumberControl.cs:27-48`). Steps are numbered D1..D19 on this
path and E1..E19 when editing.

The new profile comes from the template `PPS_G4A.db`: one zone ("Zone 1",
all 8 knobs None), one conventional frequency, **no P25 systems and no
protocol settings** (checked in `scratch/db3/PPS_G4A.sqlite3`).

### 2.4 E. View/Edit Existing Profile

Grid columns: No., Profile Name, Model Number, Band, Version, Date Created,
Modified Time. Buttons (`View.Profile.ProfileListUI.resx`): **Edit**,
**Program**, **Export**, **Delete**, **Read Device**, **Copy**, Detail,
Convert New Model, Back.

- Edit: opens the edit window at E1 (`ProfileListUI.cs:296-308`).
- Program: opens the edit window and jumps straight to the Program step
  (`EditProfileController.ProgramView`, `:136-143`).
- Copy: confirm, creates `<name>_New`, opens it.
- Convert New Model: pick a different model number, `ChangeDB` rewrites the
  copy (frequencies clamped, unsupported protocols dropped), opens it.
- Read Device: section 5.3.

### 2.5 The profile edit window

Layout (`View/View.EditProfileUI/EditProfileControl.cs`, screenshot
`QuickGuide_1.jpg`):

- **Step rail** on the left: numbered buttons with up/down scroll arrows
  (only about 7 steps visible at once), and a pinned last button
  **Program** (`DetailReportAndProgram`, label "Program",
  `View.MenuBar.Menubar.resx`).
- **Content pane**: step title (`E<n> <title>`), a one-line note, and
  sub-sections. Many steps have a row of underlined anchor links at the top
  (`funcBar`) that scroll to each sub-section.
- **Parameter Description** pane on the right: an embedded browser showing the
  CHM topic for the focused field (`bindHelpKey`, `setWebPage`). View menu
  toggles it and the **Resource** side panel.
- **Resource** side panel: opened from the profile status bar buttons
  Frequency List, Trunking System List, Protocol List, Grouping List
  (`View.StatusBar.ProfileStatusbar.resx`; `setResourceWin`, `:207-238`).
  It lets you add a system or protocol while sitting in the Zone step.
- **Toolbar**: Save, Next, Cancel (only while a second-level editor such as a
  knob is open), Program (only on the Program step), Save as (export
  `.unipps`) (`updateToolBar`, `:1036-1050`).

Save model: leaving a step (rail click, Next, closing the window) runs that
step's `checkParam` + `checkError` and saves; any error blocks navigation
with a modal message (`BasePageControl.EnableExist`,
`View/View.BaseControl/BasePageControl.cs:526-560`;
`EditProfileForm.cs:113-115` saves on close). The Save button does the same
and shows "Save Success".

### 2.6 Step menu, in order

From `StepMenuhandler.getProfileStepList` (`View/View.MenuBar/StepMenuhandler.cs:123-148`).
Rail label first, page title second. "Sub-sections" are the anchor links or
group boxes on the page.

| # | Rail label / page title | Edits (see data-model.md) | Sub-sections and key English labels | Hobbyist |
| --- | --- | --- | --- | --- |
| E1 | Basic Information Setting | profile identity (3.12) | Profile Model Information (Model Number, RF Band, Protocol, Hardware: read-only); User Group Information Setting (User Group Name, shown as "Profile Name" in the manual; Introduction); PPS Admin Password Setting (Need Password to Read, Need Password to Program, Admin Password, 6-12 digits) | Low (name). Password: Low, but a footgun |
| E2 | P25 Trunking System Setting | per-system DBs (3.4, 5) | **P25 Trunking System List** grid: System Alias, WACN ID(HEX), System ID(HEX), System Detail (n Site(s), n Control CH(s)), TGID List, Sub Group List, SUID, Individual Call List; Total / Max 1024; Multi-Select, Add, Delete. Shows "This Function is Disabled" on models without P25T | **High** |
| E3 | Conventional Frequency Setting | `tabConvFreqList` (3.2) | Conventional Frequency List: Frequency Alias, Rx Frequency (MHz), Rx Bandwidth (KHz); Max 1024; Import, Export, Multi-Select, Add, Delete; note states the model's Rx range | Medium (conventional only) |
| E4 | Protocol Parameter Setting | `tabProtocolSetting` + signaling (3.6) | Protocol List (Protocol Alias, Protocol Type; Max 16) plus one parameter panel per type: 2-Tone, 5-Tone, MDC1200, DVOA, DMR (Repeater Slot, Repeater Timing, RC Rx Enable, Signaling Activity Timeout), P25 Conventional and P25 Trunking (Digital Tone System Setting: Tone A/B/C durations, tg2; Digital Tone Qualify Setting: minimum detect times; all QCII paging timings) | Low, but **a P25 Trunking entry must exist** before a trunking knob can save |
| E5 | Secure Setting | encryption (3.9) | General Setting (Display On Channel Change, Periodic Keyfail Alert Tone); Encryption Key Reference Resources (Key Alias, Common Key Reference (CKR), Import EKF); KMF & OTAR System List | Not relevant |
| E6 | Conventional Group ID Setting | conventional TGs, groupings, pager IDs (3.5) | Conventional TGID List (Group Alias, Alert Mode, Alert Tone, Voice Storage, Relay Control, Background Color, Key Alias, and per-protocol address columns 2-Tone, 5-Tone, MDC/DVOA TGID, DMR TGID, P25C TGID, P25C Sub-Group...); TGID Grouping List; Pager ID Setting; Member List | Low (P25C/DMR TGs on conventional channels); paging parts Not relevant |
| E7 | Zone & Channel Setting | zones, knobs (3.1-3.4) | **Zone & Channel List** grid: Zone Alias + Knob Position 1..8, Total / Max 64, Add / Delete; right-click Copy/Paste/Delete zone, Copy/Paste/Delete/Move Up/Down/Left/Right knob. Double-click a knob: Select Receiving Mode dialog, then the knob editor (section 3) | **High** |
| E8 | Pager Function List Setting | `tabFunction*` (3.10) | Function Selection (which items appear in the pager's menus: Paging, Dispatch Response, Pager Box, Other functions, e.g. Select Zone, Monitor Mode, TGID Hold Timeout, Diagnostics, All Band Scan, CCH/TCH Log, Log Adv., Alert Mode); Device Function Setting (Function Status Items, Function Setting Items, Box Items) | Medium |
| E9 | Voice & Data Message Setting | device settings (3.7) | Voice Storage Setting (Voice Message Recording Limit 1-10 min); Text Message Setting (Automatically Display After Receiving); Canned Message Setting; Voice Memo Setting (Play Memo After Alarm) | Medium (recording limit); rest Not relevant |
| E10 | PS & DRT Setting | dispatch ack / BLE | BLE Setting (BLE Name); PS & DRT ACK Function (IP, Port, Pager ID); Status Change ACK Setting; Manual ACK Setting. "This Function is Disabled" on models without it | Not relevant |
| E11 | Alert Tone Setting | device settings (3.7, 3.8) | Receiving Alert Mode Setting (2-Tone Alert Duration Type/Fixed Duration, 5-Tone/MDC Alert Duration, Digital Alert Duration); Priority Alert Setting (Priority Alert Tone); Tone Volume Setting (Receiving Alert Tone Volume, Notification Tone Volume: By Volume Knob / Fixed Volume); BT Headset Setting; Out of Range Alert Setting (Out Of Range Timer, In Range Timer, In Range/Out Range Alert, Alert Repeatedly, Alert Interval, Alert Timeout); Unread Message Reminder Setting (Unread Msg Reminder Alert, Remind Interval) | Medium (alert volume, out-of-range and unread reminders are noisy for scanning) |
| E12 | Talk Group Hold & Remove Setting | `tabTGIDHoldAndRemoveSetting` | TGID Hold Setting, TGID Hold Timeout (0-99 min), TGID Un-Hold Alert, TGID Remove Setting (on-pager "block" soft key, up to 64). **All default Disable** | **High** (hold and lockout are core scanner features) |
| E13 | Duty Status Setting | duty schedule | Default Duty Status (Duty Off / Duty On / Auto), Start Time, End Time, Duty Cycle (weekday checkboxes), Do Not Record Off Duty Message | Low |
| E14 | Receive Backlight Setting | display | Receive Background Setting (Flash On/Off Time, Flash Duration, Duration Time; flash on Voice/Text/Image Message); LCD Brightness Setting (Brightness Mode, Brightness); Screen Timeout Setting (Default Screen Timeout, 0 = always on) | Medium |
| E15 | Key & LED Setting | keys/LED | Auto Keypad Lock Setting; Play Key Setting (Click / Press 2 seconds); LED Setting (Enable Standby LED, Blinking Interval) | Low |
| E16 | Display Format Setting | display | System ID Display Format, Call ID Display Format (Decimal / Hexadecimal), Date Format, Hour Format, Status Bar RSSI Icon Type (RSSI Icon / RSSI dBm) | Medium (decimal TG IDs to match RadioReference; RSSI dBm) |
| E17 | Home Screen Setting | home screen, logo | Home Screen Style (Standard / Customized), Create / Edit Home Screen (Logo/Title Type, logo file 320x75 or 75x75, Title Lines 1-3); Home Screen Detail lists per mode family (Selective Call/Monitor, Scan, **Trunking TG-Scan / TG-Monitor**, Free Scan): pick which reception fields show | Medium (trunking detail fields), Low (logo) |
| (E18) | Engineer Parameter Setting (engineer only) | engineer | Engineer Setting, Debug Entry In Pager, Tow Truck Setting, DMR Vocoder Setting (MELPe/AMBE, Standard/Motorola), Power Saving Setting (Rx duration per protocol) | Not relevant |
| E18 | Detail Report | read-only | Printable report (`View/View.DetailReport/*.rdlc`: DetailReport, TrunkingSystemReport, ZoneKnobReport). Runs profile check | Medium (only overview of the whole codeplug) |
| E19 | Program | write to pager | Profile Information; **Programming Pager** grid (Pager Model Number, Pager MSN, DB Version); toolbar Program. Section 5.2 | **High** |

### 2.7 Second-level dialogs under E2 (and C1)

Each list is a modal dialog with the system's Alias / WACN ID / System ID
repeated read-only at the top, a grid, Total / Max, and Export / Import /
Multi-Select / Add / Delete. "Multi-Select" only toggles multi-row selection
for Delete; there's no bulk edit (`View/View.BaseControl/BaseControlDefine.cs:10-30`).

| Dialog | Fields | Hobbyist |
| --- | --- | --- |
| Add menu on the system list (`P25TSyetemTotal.cs:1347-1388`) | "Add <band> System" per band of the model; "Add System from Resources" (library copy); "Import .cvs from RadioReference.com and add <band> System" (typo in the UI); "Import .csv from RadioReference.com and update selected item" | High |
| Edit P25 Trunking System | System Alias, WACN ID, System ID (each as a decimal spinner plus a hex spinner), Site Trunking (Enable), SUID, buttons Site List / Control Channel List / Full Spectrum Scan | High |
| P25 Trunking System Site List (max 256) | Site Alias, RFSS ID(HEX), Site ID(HEX), Control CH of the Site (Optional) | High |
| Control Channel List (max 256) | Rx Frequency (MHz), Note; header shows allowed Rx range | High |
| P25 Trunking System Full Spectrum Scan | Enable Full Spectrum Scan, Full Spectrum Scan Range (1-3), Start/End freq (MHz), Step size (Hz) per range | Medium |
| P25 Trunking Group List (max 8192) | Group Alias, Alert Mode, Alert Tone, Voice Storage, Relay Control, Background Color, Key Alias, P25T TGID (shown "n (DEC) / n (HEX)"); Import menu: "Import .csv from RadioReference.com", "Import from Unication .csv file"; Export; Custom Tone Setting | **High** |
| Add P25 Trunking Group | Group Alias (14), Alert Mode (Tone / Vibrate / None / Tone & Vib), Alert Tone, Voice Storage, Relay Control, Background Color, Key Alias, Enable P25T TGID, P25T TGID (dec + hex) | High (alias, TGID, alert, storage, colour) |
| Import P25T TGID csv file from RadioReference.com | preview: Decimal, Hex, Alpha Tag, Mode, Description, Tag, Category, SubFleet; Select All / Unselect All; Selected / Max | High |
| Import P25 trunking system data (.csv) from RadioReference.com | preview: RFSS, Site Dec, Site Hex, Site NAC, Description, County Name, Lat, Lon, Range, Frequencies, Error; Selected Sites / Frequencies, Max 256 | High |
| P25 Trunking Sub Group List / New Sub Group | 2-Tone Phase 1/2, Digital Address (QCII) | Not relevant |
| P25 Trunking Individual Call List / Add Individual Call | Call Alias, P25T UID | Low |

### 2.8 Public resource manager (C)

- **C1 P25 Trunking System Manager** (`P25TSystemResourceFuncPage`): the same
  `P25TSyetemTotal` grid as E2, plus a **Band** column, bound to
  `P25TTotal.db` instead of a profile. Toolbar Save, Export (zip), Cancel.
  Systems go into a profile only by "Add System from Resources", which
  **copies** the system DB (`data-model.md` 5.2 item 4). Later library edits
  don't reach profiles.
- **C2 Customer Alert Tone Manager**: Default Alert Tone List (10) and Custom
  Alert Tone List (Tone Alias, Tone File, Play; Max 16).
- **C3 Customer CH Voice Prompt Manager**: same shape for knob voice prompts.

### 2.9 RadioReference importer (status bar button)

`decomp/RadioReference/RadioReference/RadioReferenceForm.cs` (labels set in
code, `:2050-2461`):

- "Login Radio Reference": User Name, Password, Remember me, Connect,
  Offline. Premium subscription required (`data-model.md` 5.2 item 3).
- "Import Radio Reference" window. Step 1: Select region (Country, State /
  Province, County, or Search by US zipcode / by sid). Step 2: Select System,
  with radio buttons Agencies / P25 Trunking Systems / Conventional Systems,
  Filter, Get Data. A **Data** tab shows a checkable tree (sites with
  frequencies, talkgroup categories with talkgroups), "Selected Site:{0}
  Channel:{1}", "Selected Talkgroup:{0}", option "Import all control
  channels" vs "Import primary & alternate control channels",
  checkbox **Create new Zone/Knob**, buttons **Import to Profile** and
  **Import to Public Resource**.
- Import to Profile needs an edit window open; with several open it asks
  which one (`EditProfileController.cs:173-198`).

---

## 3. Knob editor and receive modes

### 3.1 Opening a knob

Double-click a knob cell in E7. The **Select Receiving Mode** dialog shows
Zone #/Knob # with << >> arrows, Zone Alias (read-only), **Receiving Mode**
combo, **Channel Knob Alias** (auto-filled "Zone n-Knob m"), and, for Radio
Compatible Mode on P25T models only, **System Type** (Conventional /
Trunking) (`View/View.Zone/ZoneKnobInfoControl.cs:112-127`). OK opens the
mode's editor in the content pane; the toolbar switches to Save / Cancel.

Inside the editor the mode is read-only; a separate button opens
`ChangeRecvModeForm`. Changing within a family (selective-call family 1/33/128;
scan family 4/36/68; trunking family 16/48/129) keeps settings. Changing
across families deletes the knob's detail rows and reopens a blank editor
after one confirm (`ChangeRecvModeControl.cs:93-156`,
`ZoneListControl.cs:3547-3578`). Choosing None deletes the knob.

### 3.2 Every receive mode

Values are `tabReceivingMode_NewG4.CurrentMode`
(`View/View.Profile/ReceivingModeType.cs`; UI list
`View/View.Common/NameValueVModel.cs:283-358`). Manual text is from
`7_ZoneChannelSetting/Mode.htm`.

| Value | UI label | Who sees it | Manual meaning (paraphrased) | Editor sections | Hobbyist use |
| --- | --- | --- | --- | --- | --- |
| 0 | None | all | knob unused | none | n/a |
| 1 | Selective Call | all | conventional; only the listed talk group IDs open audio, with per-ID alert, voice storage, background colour | Zone Knob Information, Conventional Channel Setting, Channel Receive Option, Binding Feature, Function Setting, Secure Setting, Talk Group List, Revert - Talk Group List | Low (P25C/DMR channel restricted to listed TGs) |
| 33 | Monitor Mode | all | conventional; plays any signal matching the channel's squelch/NAC/colour code, plus per-ID alerts for listed IDs | same as 1 | **Medium**: the normal way to listen to an analog, P25C or DMR conventional channel. RadioReference agency import uses it (`data-model.md` 5.2) |
| 128 | Radio Compatible Mode | all | conventional; listed IDs only, **no** alert, voice storage or colour, audio plays directly | Zone Knob Information, Conventional Channel Setting, Channel Receive Option, Secure Setting, Talk Group List | Medium (scanner-like, no beeps) |
| 129 | Radio Compatible Mode with System Type = Trunking | P25T models | not documented; stored when System Type is Trunking (`ZoneKnobInfoControl.cs:276-279`). Uses the trunking editor; hides Alert Mode and unread-message options (`ChannelFunctionSettingControl.cs:245-256`); note text "Receive from regular talk groups specifically in Trunking system" (`TrunkingKnobControl.cs:195-205`) | trunking editor | **Medium/High**: trunked TG list with no per-call alert. Needs a pager test before we recommend it |
| 65 | Multi Selective Call | engineer only | several protocols on one conventional channel | Talk Group List - Tone / DMR / MDC | Not relevant |
| 97 | Multi Monitor Mode | engineer only | monitor variant of 65 | as 65 | Not relevant |
| 4 | Normal Scan | all | scan a list of up to 16 already-defined conventional knobs in order; stop on matching signal | Zone Knob Information, Scan List Setting (Stay Time, Scan List), Binding Feature, Function Setting | Medium (conventional scan list) |
| 36 | Priority Scan | all | Normal Scan plus a Priority Channel sampled more often (Priority Sampling, Priority Channel Check Interval) | as 4 | Medium |
| 68 | Silent Scan | all | scan, but only listed TG IDs open audio | as 4 | Low |
| 16 | Trunking TG-Scan | P25T models | trunked; **only** listed talkgroups are received; per-TG alert/storage/colour | Zone Knob Information, Trunking System Setting, Binding Feature, Function Setting, Secure Setting, **Talk Group List**, Talk Group Option, Revert - Talk Group List | **High**: the scan-list mode |
| 48 | Trunking TG-Monitor | P25T models | trunked; **all** talkgroups received; listed ones get their alert/storage/colour | as 16 minus Binding Feature and Revert list | **High**: "search / hear everything" mode |
| 8 | Free Scan | all | sweeps up to N ranges (Band Type, Start/End Frequency, Step Size, Bandwidth) decoding Analog, DMR, P25, MDC/DVOA (`View.Zone.FreeScanSettingControl` labels; `FreeScanKnobControl` note "Scan specific frequency range and receive from all talk groups") | Zone Knob Information, Scan Setting, Binding Feature, Function Setting | Medium (band search). The manual's "Only available in P25 Trunking" note for Free Scan is wrong |

The manual says "9 receiving modes"; customers see 9 plus None, and 129 hides
behind the System Type combo.

### 3.3 Trunking knob editor sections (16 / 48 / 129)

Anchor links: Zone Knob Information, Trunking System Setting, Binding Feature
Setting, Function Setting, Secure Setting, Talk Group List, Talk Group Option,
Revert - Talk Group List (screenshots `3_1_GeneralReceptionSetting_3_2.jpg`,
`_3_3.jpg`).

| Section | Labels | Hobbyist |
| --- | --- | --- |
| Zone Knob Information | Zone << >>, Knob << >>, Channel Knob Alias (14), Voice Prompt (default announces the mode, e.g. "Trunking TG Normal Scan") | High (alias), Medium (prompt) |
| Trunking System Setting | P25 Trunking System (combo "Name (WACN / SysID, Sites:n, Control CHs:n)"), P25 Trunking Protocol Alias; Site List / Control Channel List / Full Spectrum Scan shortcuts. Errors if either is missing: "can be set in the previous step" (`KnobTrunkingSystemControl.cs:211-229`) | High |
| Binding Feature Setting (16, 129) | Binding Feature (None / Sub-Group Feature / Call Alert Feature), Reset Mode, Timeout Reset Timer, Delay N Function/Timer, Record Delay Function/Timer, TGID Stay Function/Timer | Not relevant (QCII dispatch); leave None + Auto Reset |
| Function Setting | Audio Squelch, Push to Listen, Voice Storage, Voice Buffering, Voice Buffering Prompt Tone, Voice Buffering Force Drop, Alert Mode, Unread Msg Option, Unread Msg Alert Option | Medium (voice storage, buffering, alert) |
| Secure Setting | Secure Type, Receiving Encrypted Voice Messages, Private Call / Channel Secure / Dynamic Regrouping key aliases, KMF System | Not relevant (possibly "Receiving Encrypted Voice Messages" Off) |
| Talk Group List | Available list; **Priority Talk Group List** (ordered with up/down) and **Non-Priority Talk Group List**; Add >> / << Remove; "List Members - Total: n Max: 64" shared across both; Sub Group Setting, TGID Ack Setting, Call Alert Setting buttons | **High** |
| Talk Group Option | Priority Talk Group ID Option, Multi-Select Call Sampling, Maximum Sampling Time (1-5 s), Announcement Group ID (ATG, 0..FFFF), ATG Key Alias | Medium (ATG, sampling) |
| Revert - Talk Group List (16 only) | same shape as Talk Group List | Not relevant |

Talk Group List rules (`View/View.Zone/TrunkingRxGroupListControl.cs`): at
least one TG required to save (`:461-470`); max 64 across both lists (`:24`);
**Add >> moves one selected item per click** (`:754-774`); no multi-select,
double-click, search or sort.

---

## 4. Required ordering

Dependencies the UI enforces or implies for a trunking knob:

1. Model chosen at profile creation (band decides allowed frequencies).
2. **E2**: a system with at least one control channel (or full-spectrum scan
   on) and at least one talkgroup, all inside the model's band.
3. **E4**: a protocol of type **P25 Trunking** (a fresh profile has none).
   Default timings are fine; only QCII paging uses them.
4. **E7**: a zone (template has Zone 1), then the knob, then its talk group
   list.
5. E19 Program.

E2 and E4 can also be filled from the Resource side panel while on E7. The
RadioReference API import with "Create new Zone/Knob" does steps 2-4 in one
go (with a catch, section 5.1).

---

## 5. End-to-end hobbyist workflows today

Click counts are rough: one per button, menu item, list selection or dialog
OK; typing a field counts as one. "TG" = talkgroup.

### 5.1 (a) New pager, one P25 system, one scan list of N talkgroups

**Path 1, manual entry with RadioReference CSV exports** (free RR account
can download site and TG CSVs in a browser):

| Step | Actions | ~Clicks |
| --- | --- | --- |
| Start | Login (user, password, Login) | 3 |
| D | D. Create New Profile; Search By Connected Device (or filters); pick row; Select; confirm | 5 |
| D1 | type profile name; Next | 2 |
| D2 | Add; "Import .cvs from RadioReference.com and add 700/800 System"; file dialog; Select All in site preview; OK; type System Alias, WACN ID and System ID in the import dialog (editable in add mode, `ImportCsvForSiteControl.cs:134-136`; the site CSV has no WACN) | 8-12 |
| D2 TGs | click the TGID List cell; Import; "Import .csv from RadioReference.com"; file dialog; Select All (or tick N); OK; OK; then fix Alert Mode per TG inline if you don't want a tone before every call (CSV import sets Tone + Voice Storage on, `P25TGroupListControl.cs:761-766`) | 7, plus up to N |
| D3 | Next | 1 |
| D4 | Add; pick "P25 Trunking" from the type menu (`ProtocolList.cs:374-390`); Next | 3 |
| D5, D6 | Next, Next | 2 |
| D7 | double-click Zone 1 / Knob 1; Receiving Mode = Trunking TG-Scan; type alias; OK; (system and protocol auto-select the first entry); Talk Group List link; **select + Add >> for each TG**; Save | 6 + **2N** |
| D12 (optional) | enable TGID Hold and TGID Remove | 3 |
| D19 | click Program on the rail; Program; confirm; access password if set | 3-4 |

Total: about 45 clicks + 2N (+N if alerts are fixed by hand). N = 40 gives
about 125 clicks. Without CSVs, each site, control channel and TG is an
Add + dialog + OK (about 4 actions each), so a 3-site, 12-channel, 40-TG
system costs roughly 220 more.

**Path 2, RadioReference premium API**: open the profile (D or E), click the
status-bar RR button, log in, Step 1 region, Step 2 P25 Trunking Systems,
pick system, Get Data, tick sites and TG categories in the tree, tick
"Create new Zone/Knob", Import to Profile (about 15 clicks). The import
creates a new system, a P25T protocol, and a new zone with knob 1 in mode
16, but puts **only the first selected talkgroup** in the knob's list
(`View/View.RadioReference/RadioReferencehandler.cs:313-333,419`). You still
do the 2N Add >> clicks in E7. Talkgroups get Alert Mode 3 (none), unlike
the CSV path.

### 5.2 (b) Add talkgroups to an existing system

1. E. View/Edit Existing Profile, select, Edit (3).
2. E2 rail button (1); click the system's TGID List cell (1).
3. Either Add, fill alias + TGID (dec or hex), OK per TG (about 4 each), or
   Import from a fresh RR TG CSV, which appends and skips exact
   (alias, TGID) duplicates (`P25TGroupListControl.cs:706-770`) (about 6).
   OK (1).
4. E7 rail (1); double-click the knob (1); Talk Group List link (1); select +
   Add >> per new TG (2 each); Save (1). Repeat for every knob that should
   carry them; a knob caps at 64, so big systems need several TG-Scan knobs
   or a TG-Monitor knob.
5. Program on the rail, Program (2-3).

About 15 clicks + 4-6 per TG. The RR API path **can't update** an existing
system: every import makes a new system with a new GUID, renamed
"Trs Sys n" if the name collides (`RadioReferencehandler.cs:174,338-340`),
so you end up with duplicates and must re-point knobs. Editing the system in
C1 (library) doesn't touch profiles that already copied it.

### 5.3 (c) Read, edit, re-write a pager

1. Connect USB; E. View/Edit Existing Profile; **Read Device**; confirm
   "create a new profile from this device" (3).
2. Access password prompt if the profile set one (`operations.md` 2).
3. If a profile with the same GUID exists: Replace / New copy / Cancel
   (`View/View.ReadDevice/ReadProfileFromDeviceControl.cs:326-353`) (1).
4. Edit window opens at E1. Edit as in 5.2.
5. Program rail button; Program; confirm (2-3). If the attached pager's model
   differs, a "device does NOT support ..." dialog lists data that will be
   stripped (`ProgramQuestion`, `operations.md` 4).

About 8 clicks plus the edit. Caveats from `data-model.md` 0 and
`operations.md` 4b: Read only recovers the `u3db_PC.zip` that PPS itself
wrote, so on-pager changes (zone/knob edits made on the device) are lost;
the merge step is a stub.

---

## 6. UX pain points (inferred)

Grounded in the code and help file above.

1. **Paging-first information architecture.** 19 linear steps, of which a
   scanner user needs E2, E7, E12 and Program, plus a few settings in E11,
   E14, E16, E17. The rail shows about 7 steps at a time with scroll arrows,
   and the manual's quick guides are all QCII paging examples.
2. **Hidden prerequisite in E4.** A trunking knob can't save without a
   "P25 Trunking" protocol entry whose only fields are QCII tone timings. New
   profiles have none. The error points the user "to the previous step".
3. **One-at-a-time talkgroup assignment.** The knob's talk group list moves
   one item per Add >> click, with no search, filter, category, sort,
   multi-select or drag. The Available list can hold up to 8192 TGs. This is
   the single most repetitive task.
4. **64-TG ceiling per knob, invisible until hit.** Large systems force the
   user to split TGs across knobs or zones by hand; nothing helps plan that.
5. **TG definition and TG selection live in different steps.** Adding a TG
   means E2 (system dialog) then E7 (every knob that should hear it). There's
   no "add to scan list" from the TG list, and no view of which knobs carry a
   TG.
6. **RadioReference integration is half-done.** API import needs premium,
   always creates a new system, and fills the knob with only the first TG.
   CSV and API imports pick different defaults (Tone + Voice Storage vs
   silent). Aliases are truncated to 14 characters without a preview, so
   "FD Dispatch North" and "FD Dispatch South" can collide.
7. **Scanner features default off.** TGID Hold and TGID Remove (lockout) are
   disabled in E12; Alert Mode defaults to Tone so every call beeps; voice
   prompts announce the mode name on each knob change.
8. **Destructive mode changes.** Switching a knob between families wipes its
   configuration after one generic confirm.
9. **Modal validation, one error at a time.** Leaving a step validates and
   blocks with a message box; you fix errors serially. Errors aren't shown
   inline next to the field.
10. **Numbers in two bases everywhere.** WACN, System ID, RFSS, Site and TGID
    use paired decimal and hex spinners; RadioReference lists TGs in decimal
    and WACN/SysID in hex. Display Format (decimal/hex) is a separate step.
11. **Copy-not-link library.** C1 systems are copied into each profile, so a
    library update means re-adding the system and re-pointing knobs.
12. **Read loses device-side edits** and depends on a PPS-written zip.
13. **No codeplug overview while editing.** The zone grid shows only mode and
    alias per knob; the Detail Report is a separate printable step.
14. **Ceremony around the app itself.** Local login screen, improvement
    program prompt, per-profile admin password that blocks Read/Program,
    Windows only.
15. **Help text errors.** Free Scan is documented as trunking-only; the
    radio-compatible trunking variant isn't documented; the TG list
    screenshot predates the Import button; the quick guide tells you to click
    Save when navigation already saves.

Good parts worth keeping: the per-field Parameter Description pane, the
Default vs Current Setting columns on every form, Copy/Paste/Move on zones and
knobs, band-range hints on frequency dialogs, and append-with-dedupe on TG CSV
import.

---

## 7. Design recommendations for the new app

1. **Scanner-first navigation with four areas**: Systems (sites, control
   channels, talkgroups), Scan Lists, Knobs (zone x knob grid), Pager
   Settings (a curated subset of E8, E11, E12, E14, E16, E17). Put E4-E6, E9,
   E10, E13, E15, QCII, sub-groups, call alert and encryption under an
   "Advanced" area that is off by default.
2. **Create the P25 Trunking protocol entry silently** with PPS defaults
   (RadioReference import already does this) and never show it unless
   Advanced is on.
3. **Named scan lists as a first-class object.** A scan list is an ordered
   set of TGs from one system with a priority section; assigning it to a knob
   writes `tabChannelGroup` rows. Enforce the 64 cap with a live counter and
   offer "split into N knobs" when a list exceeds it. Edit a list once, all
   knobs using it update.
4. **Fast TG picker**: search box, RadioReference category and tag filters,
   checkbox multi-select, select-all-in-category, drag to reorder, and a
   column showing which scan lists already include each TG.
5. **Knob presets**: "Scan list" (16), "Hear everything on this system" (48),
   "Conventional channel" (33), "Band search" (8). Offer 129 (no-alert
   trunking) only after a hardware test confirms behaviour.
6. **Merge-aware import.** Import RR site and TG CSVs (free accounts) into an
   existing system matched by WACN/System ID; show a diff (new, changed alias,
   removed) before applying. Treat the premium API as optional. Show
   truncated 14-char aliases in the preview and let the user edit them.
7. **Hobbyist defaults**: TGID Hold and Remove on; TG Alert Mode None; voice
   prompt off or a short knob name; voice storage per user choice; keep the decimal
   Call ID / System ID display that PPS already defaults to. Make these a "Scanner defaults" profile option.
8. **Inline, non-blocking validation** using the rules in `data-model.md` 2.5
   and 5.3: errors listed in a panel and attached to fields; Program disabled
   until clean. Never block navigation.
9. **One number entry style**: accept decimal or `0x`/`h`-suffixed hex in a
   single field and show the other base beside it. Show the model's Rx ranges
   next to every frequency field.
10. **Safe mode changes**: switching mode families keeps the old config as a
    draft and offers undo.
11. **Read/Program as a sync screen**: one "Read pager", one "Write pager"
    with a diff of profile vs last read, an automatic backup of the pager's
    `u3db_PC.zip` before writing, and a plain warning that pager-side edits
    are not recoverable unless we reverse-map `u3db.db`.
12. **Live overview**: a zone x knob grid that shows system and TG count per
    knob, and a per-TG "where used" view, replacing the separate Detail
    Report.
13. **Shareable systems**: keep a local library, but link profiles to library
    systems with an explicit "update from library" action; import/export the
    PPS zip format (`data-model.md` 5.2 item 5) for compatibility.
14. **Keep contextual help**: write our own parameter descriptions (avoiding
    the errors in 6.15) in a side pane keyed by field.
15. **Drop the PPS login, improvement program and engineer gating.** Keep the
    pager access password only where the protocol requires it
    (`operations.md` 2).
