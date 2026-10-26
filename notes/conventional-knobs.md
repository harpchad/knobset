# Analog conventional knobs: PPS row-level behavior for an empty knob becoming a Monitor-mode channel

Scope: what PPS V00-04-00-48 writes when the user puts an analog
conventional channel (carrier squelch, optionally with Rx CTCSS or DCS) on
an empty knob position (`CurrentMode 0`) in Monitor mode (`CurrentMode 33`),
how conventional frequencies are created, how squelch and sub-audio are
stored, what validation applies and what the 1.41 converter makes of it.
P25 conventional (NAC) and DMR (color code, timeslot) are covered only where
they differ (section 6). Sibling of [system-creation.md](system-creation.md)
(trunking knobs); its section 3.1 (knob editor mechanics, `savePrivate`,
`checkError` gating, `deleteKnob`) applies here unchanged and is not
repeated.

Evidence paths are relative to `decomp/`. Short forms: `ZONE/` =
`View/View.Zone/`, `FREQ/` = `View/View.FrequencySetting/FreqSettingControl.cs`,
`CHECK/` = `View/View.ProfileCheck/ProfileCheckHandler.cs`, `RR/` =
`View/View.RadioReference/RadioReferencehandler.cs`, `CONV/` =
`View/View.ProgramVersion/PPSU4ParamConverter_01P41b.cs`, `RS/` =
`crates/g5-codeplug/src/convert141.rs`. Real data: unit B's profile
`bac66824-47f8-48d2-8c33-1d7ff7a5718a` (model `G5B67BF-SXVXEN1401UNI`, first
band SX, second VX), as extracted to `scratch/conv-study/` and, identical in
every table used here, `scratch/syscreate/unitB/`. Experiments:
`scratch/convknob_tools/exp.py` (writes `scratch/convknob/exp/`), built on
the system-creation experiment harness: it applies the rows of section 8 to
a copy of the profile, converts with Knobset's 1.41c converter and diffs the
radio DB against the unmodified conversion.

## 0. Summary

- **Two pages, two moments.** The knob editor never creates a frequency.
  Frequencies live in `tabConvFreqList` and are made on the "Conventional
  Frequency Setting" page; the knob only picks one by `ConvFreqID`, and a
  new knob defaults to the **first row of the table**. The profile template
  ships one frequency row with the all-zero FreqID `0000000000000000`, so a
  knob whose user never picked another frequency points at that row. That
  is exactly the Elmwood Fire knob.
- **The new knob is one appended `tabSingleConvCHKnob` row plus an update
  of the existing `tabReceivingMode_NewG4` row.** For carrier squelch
  nothing else per knob is written: no `tabChannelGroup`, no
  `tabChannelGroupList`, no announcement, alert or function rows. With Rx
  CTCSS/DCS, one `tabRecvSaList` row per tone (max 5).
- **Rx tones are stored only in `tabRecvSaList`.** The knob's
  `RxSaType`/`RxCTFreq`/`RxCDCodeWord`/`RxCDModSense` and the four `Tx*`
  sub-audio columns are never written by the knob editor (they are `''` in
  every real row) and no converter reads them. Only the RadioReference
  importer fills them.
- **Carrier squelch = `ProtocolSettingID '0000000000000000'` and no
  `tabRecvSaList` row.** CTCSS and DCS are not protocols: they keep the CSQ
  protocol ID and add sub-audio rows with `SigType '0'`.
- Encodings: CTCSS in tenths of Hz (`1273` = 127.3 Hz, 67.0..255.0 Hz);
  DCS as the decimal value of the octal code (`754` -> 492), one of 104
  listed codes; `cdModSense 1` = inverted.
- **TxFreq is not RxFreq.** A new frequency row gets Rx = Tx = the lowest Rx
  band edge of the model (136000000 on an SX/VX pager); editing the Rx cell
  afterwards changes `RxFreq` only, so Tx stays at the creation value. The
  template row gets Rx = first band's Rx minimum and Tx = that + 12500
  (763012500 on SX) when the profile is created, which is where Elmwood
  Fire's `TxFreq 763012500` comes from. The converter ignores Tx entirely.
- Bandwidth codes 1 = 12.5 kHz, 2 = 25 kHz, 3 = 20 kHz. 3 is offered on
  **every** G-series model (the model's project is always `Gx`).
- The default knob row (section 1.2) matches all 11 real conventional
  knobs except `AlertMode` and `RecordFlag` on 8 of them, which are user
  edits (Silent, voice storage Off).
- Validation is light: Rx inside one of the model's Rx ranges (inclusive),
  alias 1..14 characters, bandwidth non-zero, frequency must exist. Nothing
  checks CTCSS/DCS at program time.
- Conversion: a new CSQ Monitor knob changes **17 radio tables**, plus
  `tabRecvSaList` when it has a tone and `tabZone` when the zone had no
  active knob. No contact is created (CSQ uses the always-present all-call
  65535). An unused frequency changes nothing. Knobset's converter already
  handles CSQ with CTCSS/DCS and all bandwidth codes; it refuses every
  other conventional protocol.

## 1. Empty knob to Monitor mode: flow and rows

### 1.1 Flow

1. Double-click an empty cell (`ZONE/ZoneListControl.cs:3420-3452`):
   `SelectKnobReceiveModeForm` runs `ZoneKnobInfoControl.initForNew` and,
   on OK, `Save` (`ZONE/SelectKnobReceiveModeForm.cs:35,47-48`), which writes
   `CurrentMode`, `ChannelName` and `AlertToneID` into the in-memory receive
   row (`ZONE/ZoneKnobInfoControl.cs:272-283`). Choosing a mode prefills an
   empty alias with `Zone {row}-Knob {k}` (`row` = the zone's grid
   position, 1-based) and sets the voice prompt to the mode's built-in
   prompt (`:108-150`), **2** ("Monitor Mode") for 33
   (`View/View.Common/NameValueVModel.cs:47-63`,
   `View/View.VoicePrompt/DefaultVoicePromptListControl.cs:21-40`).
2. The same handler sees mode 33 and calls `Row_tabSingleConvCHKnob`
   (`ZONE/ZoneListControl.cs:3454-3464,3622-3653`): if no row exists for
   (zone, knob) it **appends** a new one with a new lowercase GUID
   `ChannelID` and the defaults of section 1.2. If a row already exists it
   is reused as is.
3. `SelectiveCallControl.init` (`ZONE/SelectiveCallControl.cs:172-195`)
   initializes the sub-controls. For CSQ, `ProtocolChangedEventHandler`
   (`:79-94`) hides the talkgroup list, the secure settings and the binding
   features; the function settings and the receive options stay visible.
4. Moving to another knob or zone, leaving the page or Save runs
   `checkError` then `Save` of every sub-control in this order: knob info,
   conventional setting, receive option, binding feature, function, secure,
   Rx talkgroup list, revert list (`:214-305`), then `savePrivate` rewrites
   the zone tables (system-creation 3.1). `tabConvFreqList` is loaded by
   the zone page (`ZoneListControl.cs:231,258`) but **is not in the save
   list** (`:336-357`): the zone page never writes frequencies (the one
   exception is pasting knobs from another profile, `:2272-2290,2931-2957`).

### 1.2 Default `tabSingleConvCHKnob` row

Effective values after `Row_tabSingleConvCHKnob` and the sub-controls'
`Save` with untouched controls, CSQ, Monitor mode. A `Save` overrides a
`setDefault` value where they differ. "Never set" columns are absent from
the DataRow, so PPS's full-row INSERT writes `''` (not the DDL default).
Columns in DDL order (`notes/schema/PPS_G4A.sql:1185-1243`, 56 columns).

| Column | Value | Set by | Real knobs |
|---|---|---|---|
| `ZoneID`, `KnobIndex` | the knob | `ZoneListControl.cs:3631-3632` | |
| `ChannelID` | new lowercase GUID | `:3633` | all lowercase GUIDs |
| `ConvFreqID` | first `tabConvFreqList` row's `FreqID` (`''` if none), or the user's pick | `ZONE/ConventionalSettingControl.cs:444-450,389` | Elmwood Fire: `0000000000000000` (the default first row); others: picked |
| `ProtocolSettingID` | `0000000000000000` (CSQ) | `:27,447,390` (Save writes CSQ when the alias combo is hidden) | all 11 |
| `RxSaType`, `RxCTFreq`, `RxCDCodeWord`, `RxCDModSense`, `TxSaType`, `TxCTFreq`, `TxCDCodeWord`, `TxCDModSense` | `''` | never set by the editor | `''` in all 11, including the 3 with CTCSS |
| `RxNAC` | `0` | `:448,391` (hidden -> 0) | `0` |
| `TxNAC` | `''` | never set | `''` |
| `RxColorCode` | `0` | `:449,392` | `0` |
| `TxColorCode` | `''` | never set | `''` |
| `Emphasis` | `1` (Rx de-emphasis Yes) | `ZONE/ConventionalReceiveOptionControl.cs:303,261` | `1` |
| `Compander` | `0` | `:304,262` | `0` |
| `TxPowerLevel` | `2` | `ZoneListControl.cs:3641` | `2` |
| `AudioSquelch` | `0` (Low) | `ConventionalReceiveOptionControl.cs:305,263`; also `ChannelFunctionSettingControl.cs:449` (its `Save` writes it only for trunking, `:373-376`) | `0` |
| `OpenSpeakerLogic` | `0` | `ZoneListControl.cs:3639` | `0` |
| `HangTime` | `50` | `:3640` | `50` |
| `DefaultGroupID` | `''` | `:3642` | `''` |
| `DispatchCHFlag` | `0` | `:3643` | `0` |
| `Encryption` | `''` | never set (DDL default `'0'` does not apply) | `''` |
| `RFPerformance` | `0` | `:3644` (DDL default is 1) | `0` |
| `PushToListen` | `0` | `ZONE/ChannelFunctionSettingControl.cs:461,387` (hidden for CSQ Monitor) | `0` |
| `Privacy` | `0` | `:453,379` (hidden) | `0` |
| `SquelchOption` | `3` | `ConventionalReceiveOptionControl.cs:307,265` (combo hidden unless P25C; Save writes 3 for Monitor mode) | `3` |
| `VoiceBuffering` | `1` | `ChannelFunctionSettingControl.cs:469,395` (checkbox hidden for CSQ Monitor -> 1) | `1` |
| `VoiceBufferPrompt`, `VoiceBufferingForceDrop` | `0`, `0` | `:473,477,399,403` | `0`, `0` |
| `ResetMode`, `DelayNTime` | `0`, `30` | `ZONE/BindingFeatureControl.cs:734-738,690-697` (whole control hidden for CSQ) | `0`, `30` |
| `EnableCarrierOffTime`, `CarrierOffDelayTime` | `0`, `30` | `:758-768,714-727` | `0`, `30` |
| `AlertMode` | `1` (Tone) | `ChannelFunctionSettingControl.cs:481,407` | **3** on 8 knobs, 1 on 3 |
| `RecordFlag` | `1` ("ID Match"; list Off/ID Match/All Voice for Monitor) | `:465,391`, list `:38-54` | **0** on 8 knobs, 1 on 3 |
| `DutyStatus` | `0` | `ZoneListControl.cs:3645` | `0` |
| `TransmitMode` | `1` | `ConventionalReceiveOptionControl.cs:306,264` ("Repeater Mode (Digital)" hidden -> 1) | `1` |
| `AlwaysUnreadVoiceMessage`, `AlwaysUnreadReminderAlert` | `0`, `1` | `ChannelFunctionSettingControl.cs:485,489,411,415` | `0`, `1` |
| `DelayNFeature`, `DelayNTimeEx` | `0`, `5` | `setDefaut` writes 1; `Save` writes 0 when hidden (`BindingFeatureControl.cs:750,706`) | `0`, `5` |
| `EnableTgidStayTime`, `TgidStayTime` | `0`, `30` | `:742-746,698-705` | `0`, `30` |
| `EncryptionBy` | `0` | `setDefaut` writes 1; `Save` writes 0 because "Encrypted" is unchecked (`ZONE/ChannelSecureSettingControl.cs:395,349-366`) | `0` |
| `EncryptionKeyID`, `PrivateEncryptionKeyID` | `000000000000` | `:399,403,368-375` | same |
| `KeyStrapOption`, `PrivateKeyStrapOption` | `0`, `0` | `ZoneListControl.cs:3646,3648` (DDL default is 1) | `0` |
| `KMFSystemID` | `000000000000` | `:3647`, `ChannelSecureSettingControl.cs:380-383` | same |
| `UserSelectableKeyset`, `UserSelectableKey`, `EncryptedCallFlag` | `0` | `ZoneListControl.cs:3649-3651`, `ChannelSecureSettingControl.cs:345-348` | `0` |

Check (`exp.py`, first block): Lake knobs 4, 5 and 6 equal this row in every
non-key column; the 7 NOAA knobs and Lake knob 3 (Elmwood Fire) differ only
in `AlertMode 3` and `RecordFlag 0`, two visible combos on the Function
Setting page. Those are user edits; the default is Tone and ID Match.

### 1.3 Every profile table touched

| Table | Change | Evidence |
|---|---|---|
| `tabReceivingMode_NewG4` (existing row) | `CurrentMode` 33; `ChannelName` = alias (prefilled `Zone {row}-Knob {k}`, required, max 14); `AlertToneID` = voice prompt, default **2**; `AlertFileName` untouched (template `NULL`, `''` after the full-table rewrite) | `ZoneKnobInfoControl.cs:108-150,253-283,541` |
| `tabSingleConvCHKnob` | append the 1.2 row | `ZoneListControl.cs:3622-3653` |
| `tabRecvSaList` | delete the knob's rows, then append one per configured tone: `(ChannelID, SigType '0', SaType, ctFreq, ctSte '1', cdCodeWord, cdSte '1', cdModSense)`; none for carrier squelch | `ConventionalSettingControl.cs:393-427` |
| `tabChannelGroup`, `tabChannelGroupList` (isRevert 0) | the knob's rows deleted; nothing added for CSQ | `ZONE/ConventionalRxGroupListControl.cs:669-676,769-778` |
| `tabChannelRevertGroup`, `tabChannelGroupList` (isRevert 1), `tabP25TChannelRevertSubGroup` | deleted; nothing added (only P25C/P25T write revert lists) | `ZONE/ConventionalRevertGroupListControl.cs:507-520` |
| `tabP25TChannelSubGroup` | deleted; nothing added | `ConventionalRxGroupListControl.cs:772-775` |
| `tabP25TSubGroupEnable`, `tabP25TCallAlertEnable` | the knob's rows deleted; nothing added (binding feature hidden for CSQ, and the sub-group flag is forced only for mode 48) | `BindingFeatureControl.cs:662-690` |
| `tabConvFreqList` | **none** from the knob editor | section 1.1 step 4 |
| `tabProtocolSetting`, signaling tables, groupings, groups | none | |
| announcement, alert, function, scan tables | none; a new knob is not added to any scan list | |
| `tabOrganization_New` | `FinishDate` on page save | `View/View.BaseControl/BasePageControl.cs:873-890` |

The deletions are no-ops for a fresh ChannelID. Real data agrees: no
conventional knob has a `tabChannelGroup`, `tabChannelGroupList`,
sub-group or call-alert row; the 3 CTCSS knobs have exactly one
`tabRecvSaList` row each.

Cancel on the knob editor with a pending new knob removes it again
(system-creation 3.1).

### 1.4 The empty positions in unit B

| Position | Receive row before | Note |
|---|---|---|
| Metro knob 8 | `CurrentMode 0`, `ChannelName ''`, `AlertToneID ''`, `AlertFileName ''` | template-style empty row |
| Lake knob 7 | `CurrentMode 0`, `ChannelName ''`, `AlertToneID '0'` | the `deleteKnob(clearChannelName: true)` signature (system-creation 5.3) |
| Metro knobs 9-16 | template rows | beyond the 8-column grid; never offered |

Neither has a detail row in any knob table.

## 2. Conventional frequencies (`tabConvFreqList`)

### 2.1 Where rows come from

| Path | FreqID | Name | RxFreq / TxFreq | Bandwidth | Evidence |
|---|---|---|---|---|---|
| Profile template | `0000000000000000` | `Frequency 1` | `''` / `''` | 1 / 1 | `PPS_G4A.db` |
| New profile (`CreateProfile`) | unchanged | unchanged | **UPDATE of every row**: Rx = first band's range-0 Rx minimum, Tx = that + 12500 (SX: 763000000 / 763012500; VX first: 136000000 / 136012500) | unchanged | `View/View.Profile/ProfileManagerBiz.cs:183-195`, `View/View.Profile/U4PPSDataRepository.cs:343-358`, `View/View.Profile/RFBandTypeInfo.cs:42` |
| Frequency page, Add | new lowercase GUID | `Frequency N`, smallest N not already an exact Name | Rx = Tx = `m_minFreq`, the lowest Rx range minimum over both bands (136000000 on SX/VX) | 1 / 1 | `FREQ:172-194,339-369`, `View/View.Common/CommonFunction.cs:637-673` |
| Grid edit of Rx | | | **`RxFreq` only** | | `FREQ:464-470` |
| Grid edit of bandwidth | | | | **`RxBandWidth` only** (column bound to it, `:1247`) | |
| Paste (within the page) | new GUID | source name if unused, else `Frequency N` | Rx = Tx = source Rx if in range, else the Add default | Rx = Tx = source if offered | `FREQ:113-155` |
| Unication CSV import | new GUID (or the existing one when "delete all" and the name matches) | CSV name, max 14 | Rx = Tx = CSV Rx if in range | from CSV, Tx = Rx | `FREQ:646-726` |
| RadioReference CSV import | new GUID | alpha tag, shortened to 14 and de-duplicated | Rx = Tx | 1 / 1 | `FREQ:863-985` |
| RadioReference agency import (knobs too) | new GUID, or **reuses** a row with the same Name and RxFreq | alpha tag (14) | Rx = Tx | mode "1" -> 2, else 1; Tx = Rx | `RR:833-851` |
| Model change on the conversion copy | | | Rx moved into the new model's ranges; Tx untouched | | `View/View.ProgramVersion/ProgramController.cs:663-672` |

`TransmitMode` is `1` on every path. The page saves with
`SaveDataTableEx` (full rewrite) when anything changed (`FREQ:449-462`).

### 2.2 The TxFreq values in unit B, explained

| Row | Rx | Tx | How |
|---|---|---|---|
| `0000000000000000` "Elmwood FirDis" | 154040000 | 763012500 | the template row: profile creation set Rx 763000000 / Tx 763012500 (first band SX); the user renamed it and edited Rx in the grid; Tx kept |
| the other 10 | 154.19 to 162.55 MHz | 136000000 | each made with Add (Rx = Tx = 136000000, the VX minimum, lower than SX's 763000000), then Rx edited in the grid |

So data-model 3.2's "`TxFreq` always = `RxFreq`" holds only at creation.
After any grid edit Tx is stale, and the Tx band check (`CheckRxFreqRange`
with a Tx value, `FREQ:532-601`) has no caller that passes one. **No
converter reads `TxFreq`**: `tabChannel.TXFrequency` = `RxFreq`
(`CONV:2526-2535`, `RS:798-803`). `TxBandWidth` is likewise ignored
(`RxChanBandWidth = TxChanBandWidth = RxBandWidth`).

### 2.3 Bandwidth

| Code | Meaning | Offered |
|---|---|---|
| 1 | 12.5 kHz | always |
| 2 | 25 kHz | always |
| 3 | 20 kHz | when `isSupportBandWidth20KHz()` (`View/View.Profile/ModelNumberInfo.cs:333-344`): project `Gx` or a 7X/8X/SX band. `Project` is the model number's first two characters, or its product code, and **any value starting with `G` becomes `Gx`** (`:186-207`), so every G5 offers 20 kHz, VHF-only models included |
| 0 / other | invalid | pre-program check D2 rejects 0 for non-DMR/P25C knobs |

Display names `12.5`, `25`, `20` (`FREQ:298-307`, `View/View.Common/Common.cs:419-428`).
Unit B: all 11 rows are 1/1, so codes 2 and 3 are not exercised in real data.

### 2.4 Rx range check

The Rx grid column gets a range map built from `Data_G4A.tabRFBandRanges`
rows with `RxTx = 0` for the model's first and second band (`FREQ:309-381`).
The spinner (`Control/Unication.Controls/PPSNumericUpDown.cs:143-160,323-348`)
accepts a value inside any range, **edges included**; otherwise it shows
"The value ... is not in the valid range ..." and restores the previous
value. Paste and import use the same inclusive test (`FREQ:135,705`). The
spinner step is 0.0125 MHz and 5 decimals are kept, so stored values are
multiples of 10 Hz; typed values need not be on any channel grid (unit B
has 155002500).

Rx ranges for unit B's model (`tabRFBandRanges`): SX 763000000-776000000
and 851000000-870000000; VX 136000000-174000000. Tx ranges (SX also
793-806, 806-824 MHz) are irrelevant for a receive-only pager.

### 2.5 Name

`Name` max 14 characters (grid `MaxInputLength 14`, `FREQ:1232`; imports
`:674`). `checkParam` (`FREQ:425-447`) rejects a blank name; its
duplicate-name test never fires because the dictionary is never filled, so
PPS does not actually enforce unique frequency names. The name never
reaches the pager: the radio channel name is the knob alias.

### 2.6 Limits, sharing and deletion

| Rule | Value | Evidence |
|---|---|---|
| rows per profile | 1024 (Add disabled at 1024) | `FREQ:29,126,260-268` |
| last row | cannot be deleted; when all are selected one is kept | `FREQ:196-223,263` |
| sharing | any number of knobs may reference one FreqID (the knob combo lists every row, `ConventionalSettingControl.cs:238-250`); unit B shares none | |
| delete a frequency a knob uses | **allowed**: `CheckIfCanDelete` (`FREQ:225-258`) has no caller. The knob keeps a dangling `ConvFreqID`; the editor then shows no selection and its `checkError` demands a frequency; the pre-program check fails with B1 | `CHECK/ProfileCheckHandler.cs:559-563` |
| knob removed | `deleteKnob` leaves the frequency in the pool (system-creation 3.4); orphan frequencies cost nothing (experiment f) | `ZoneListControl.cs:1309-1384` |

The all-zero FreqID is not a sentinel. It is the template's first row and a
normal frequency; data-model 3.0's "16 zeros = CSQ/none" applies to
protocol-setting IDs only.

## 3. Squelch and sub-audio

### 3.1 Where Rx tones are stored

The "CTCSS / CDCSS for Analog" block (count spinner 0..5, resx
`numbericUpDownControl_RxSaCount.Maximum = 5`, five `SaControl`s) is
visible for every protocol except DMR and P25C
(`ConventionalSettingControl.cs:324-328`). `init` loads the knob's
`tabRecvSaList` rows (`:253-259`); `Save` (`:393-427`) deletes them and
appends one row per SA slot 1..count whose type is not None and whose
setting string differs from the earlier ones ("Same or empty SA settings
will NOT be saved"). The knob row's Rx/Tx sub-audio columns are not
touched by any knob editor path. Only the RadioReference agency importer
sets `RxSaType`/`RxCTFreq`/`RxCDCodeWord` and the Tx twins
(`RR:1195-1223`), alongside the `tabRecvSaList` row it also writes.
Neither the 1.41 nor the 2.06 converter reads the knob columns.

### 3.2 Encodings (`ZONE/SaControl.cs`)

| Column | CTCSS | DCS | Evidence |
|---|---|---|---|
| `SigType` | `'0'` (the knob's protocol type; CSQ) | `'0'` | `ConventionalSettingControl.cs:415` |
| `SaType` | 1 | 2 (0 = None, never stored by the editor) | `SaControl.cs:29`, resx Type items None/CTCSS/CDCSS |
| `ctFreq` | tenths of Hz, `Convert.ToInt32(text * 10)` | 0 | `:31-41` |
| `cdCodeWord` | 0 | decimal value of the octal code, `Convert.ToInt32(text, 8)` | `:43-53` |
| `cdModSense` | 0 | 1 = Invert checked, else 0 | `:55-65` |
| `ctSte`, `cdSte` | `'1'` | `'1'` | `ConventionalSettingControl.cs:418,421` |

CTCSS: an editable combo (`DropDownStyle 1`, `SaControl.cs:396`) offering 42
standard tones 67.0 ... 254.1 (`:173-235`); free entry is allowed, clamped
to 67.0..255.0 with a message and cut (not rounded) to one decimal on leave
(`:266-305`). DCS: a fixed list (`DropDownStyle 2`, `:404`) of 104 codes
(resx `comboBoxControl_CDCSS.Items`..`Items103`): 023 025 026 031 032 036
043 047 051 053 054 065 071 072 073 074 114 115 116 122 125 131 132 134 143
152 155 156 162 165 172 174 205 212 223 225 226 243 244 245 246 251 252 261
263 265 266 271 306 311 315 325 331 343 346 351 364 365 371 411 412 413 423
425 431 432 445 446 452 455 464 465 466 503 506 516 521 525 532 546 552 564
565 606 612 624 627 631 632 645 652 654 662 664 703 712 723 725 726 731 732
734 743 754.

Real rows (unit B): Elmwood Fire `(SigType 0, SaType 1, ctFreq 1273, ctSte 1,
cdCodeWord 0, cdSte 1, cdModSense 0)` (127.3 Hz), Oak Ridge FG 1567
(156.7 Hz), Ashford FG 797 (79.7 Hz). Byte for byte what `Save` writes.
No real DCS row exists.

The global `tabCTCSSSignaling` / `tabCDCSSSignaling` rows
(`ID '000000000000'`, `Ste 0`, `ModSense 0`) are read by neither converter.

### 3.3 What "carrier squelch" means in stored values

- `ProtocolSettingID = '0000000000000000'`: protocol type CSQ (the
  protocol-type combo always offers CSQ = 0, plus the type of each existing
  non-trunking protocol setting, `ConventionalSettingControl.cs:260-295`).
- Zero `tabRecvSaList` rows for the ChannelID: pure carrier squelch.
- One or more rows: CSQ plus Rx tone squelch. The converter sets
  `MonitorModeOn` 2 without and 1 with sub-audio (`CONV:2472-2484`), but
  writes it to the radio as `MonitorModeOn > 0 ? 1 : 0`, so the only radio
  difference between carrier and tone squelch is the `tabRecvSaList` rows
  (experiment a3).

"None" in an SA slot is not a stored value: an SA count of 1 with type None
saves nothing. The RadioReference importer is the exception: a tone field of
`CSQ` gives a `tabRecvSaList` row with `SaType 0` (`RR:1173-1190`), which
converts to a radio row with `SigType 255`. Knobset must not copy that.

### 3.4 The other receive-option values for analog Monitor mode

| Column | Analog Monitor | Evidence |
|---|---|---|
| `SquelchOption` | combo hidden (shown for P25C only); stored **3**. The converter writes `SquelchOption 0` to the radio for every non-P25C knob, so the value is inert | `ConventionalReceiveOptionControl.cs:35-47,265,285`, `CONV:2491-2507` |
| `AudioSquelch` | combo visible (hidden only for DMR): 0 Low, 1 Mid, 2 High (resx). Converter: 0..3 -> 20/40/60/100 % -> `CarrierSquelchSetting` 51/102/153/255, `SINADSquelchSetting` 4/8/12/20 | `:263,283`, `CONV:2217-2230`, `RS:174-183` |
| `Emphasis` | visible: 0 No, 1 Yes (default) | `:261,281` |
| `Compander` | visible: 0 No (default), 1 Yes | `:262,282` |
| `TransmitMode` | "Repeater Mode (Digital)" hidden for analog -> 1 | `:264,284` |

## 4. Validation

### 4.1 Knob editor (`checkError`, gates every save)

| Rule | Evidence |
|---|---|
| knob alias not blank or whitespace; max 14 (text box limit) | `ZoneKnobInfoControl.cs:253-266,541` |
| a frequency selected (fails when `ConvFreqID` dangles) | `ConventionalSettingControl.cs:345-351` |
| a protocol type selected; a protocol alias for non-CSQ | `:352-366` |
| DMR and P25C: the frequency's `RxBandWidth` must be `1` | `:367-378` |
| CTCSS 67.0..255.0 Hz (on leaving the field) | `SaControl.cs:266-293` |
| at most 5 SA entries; empty and duplicate ones dropped silently except for a notice | resx, `ConventionalSettingControl.cs:396-427` |
| receive option, function, secure, binding, CSQ talkgroup lists | no checks for analog (`checkError` returns true; the CSQ group list returns before any check) |

### 4.2 Frequency page

Rx in the model's Rx ranges, inclusive (spinner); name non-blank, 14
characters; 1024 rows; at least one row (section 2).

### 4.3 Pre-program check (`CheckProfileData`)

For modes 1, 33 and 128 `checkSelectiveCall` (`CHECK:538-578`):

| Code | Rule | Lines |
|---|---|---|
| B2 | a `tabSingleConvCHKnob` row exists for (zone, knob); a non-CSQ `ProtocolSettingID` resolves | `:540-554` |
| B1 | `ConvFreqID` exists in `tabConvFreqList` | `:559-563` |
| D2 | non-DMR/P25C: `RxBandWidth != 0` | `:314-327` |
| D1 | DMR/P25C: `RxBandWidth == 1` | `:317-322` |
| B3 | any non-CSQ protocol, **Monitor mode included**: a grouping of that protocol with at least one group that has an address, no duplicate addresses | `:570-577,359-460` |
| A1 | at least one active knob in knobs 1..8 of some zone | `:263-299` |

Nothing re-checks the Rx range, CTCSS/DCS values, the alias length or
`SquelchOption` at program time; a hand-written row goes straight to the
converter.

## 5. Conversion (1.41b/c)

### 5.1 Rules for a single conventional knob

`convertSingleCHKnobInfo` (`CONV:2284-2520`), `addTabChannelSql`
(`CONV:2523-2704`, 1.41c `01P41c.cs:603-785`); Knobset `RS:886-969,798-868`.
ZK = zone row index x 100 + knob; the conventional SigID is the ZK.

| Radio table | Row for a CSQ Monitor knob |
|---|---|
| `tabReceivingMode` | `(zone, knob, 32, 32, 0, ZK, ZK, 1, ChannelName, 1, 1, 1)` (33 -> 32) |
| `tabZoneChannelList` | `(zone, knob, ZK, 1)` |
| `tabChannelAnnouncement` | `G4-ChVoice-NNN.dat`, NNN = `AlertToneID` (002 default, 000 none) |
| `tabChannel` | `(ZK, ChannelName, 0, BW, BW, RxFreq, RxFreq)` |
| `tabChannelAdvInfo` | `SignallingSquechLogic 1` (Monitor), `AudioSquelchType 1`, squelch pair, `Emphasis`, `Compander`, `ColorCode 0`, `NAC 0`, `DefaultCallID 65535`, `ATGID 0`, TaTx `0,670,1,19,1,0`, `SquelchOption 0`, `PowerSavingRxOffDur 100` |
| `tabFunctionOnOffOfChan` | `talkAroundOn = TransmitMode (1)`, `DelayPlayVoiceMsg = VoiceBuffering (1)`, `MonitorModeOn 1`, unread flags, `ForceDrop 0` (1.41c) |
| `tabFunctionParamOfChan` | `(ZK, ZK, 3, ZK, 1, ZK)` |
| `tabVoiceCallSettings`, `tabR2RModeSetting` | `(ZK, 0, 0, 0)`, `(ZK, 0)` |
| `tabResetModeSetting` | `(ZK, 0, 30, 0, 30, 0, 30)` |
| `tabAlertModeSetting` | `(ZK, ZK, 1, AlertMode - 1, 2, 0, 2000, 2000, 2000, 0)`, later UPDATEd with the ring settings like every row |
| `tabRecvVoiceCallRecordByChannel` | `(zone, knob, RecordFlag)` |
| `tabRepeaterSettingOfChan` | `(ZK, 0, 0, ZK, 0, 670, 1, 19, 1, 0, 0, 670, 19, 0)` (protocol 0, access code type 0) |
| `tabRecvSaList` | per profile row (first 5): `(ZK, SigType = 0, or 255 if SaType 0, SaType, ctFreq if CTCSS else 670, 1, cdCodeWord if DCS else 19, 1, cdModSense)` |
| `tabDispatchChannel` | `(ZK)` (`DispatchCHFlag '0'`) |
| `tabContactOfChan`, `tabRecvGroupList`, `tabRecvBehaviorByContactID` | `(ZK, 65535, 0)`; `(ZK, 0, ZK, '', 65535, 0)`; `(ZK, 65535, 0, Duty 1, 0, VoiceStorage 2, 0, acks 0)` |
| `tabZone` | `ModifyByDevice` 1 -> 0 if this is the zone's first active knob (converter-1.41 3.1) |

Notes:

- The **unused filler for DCS is 19, which is also code 023**. A DCS 023
  channel and a CTCSS channel differ only in `SaType` and `ctFreq`; test
  DCS with another code (the experiment uses 754).
- `MonitorModeOn` counts all `tabRecvSaList` rows, including `SaType 0`
  ones; only the first 5 are emitted (`CONV:2405-2431`).
- `AlertMode` reaches `tabAlertModeSetting.mode` as `AlertMode - 1`;
  `RecordFlag` only `tabRecvVoiceCallRecordByChannel`.
- The frequency name, `TxFreq`, `TxBandWidth`, the knob's Rx/Tx sub-audio
  columns, `SquelchOption` (non-P25C), `TxNAC`, `TxColorCode`, `Encryption`
  and `DutyStatus` reach no radio table.
- CSQ knobs get no encryption rows: `convert_encryption` skips a knob whose
  protocol ID is not a real setting (`RS:1584-1592`).

### 5.2 Experiments (`scratch/convknob_tools/exp.py`)

Baseline: unit B's profile converted with Knobset's 1.41c converter (equal
to the uploaded radio DB, system-creation 5.4). "Changed" = table content or
row order differs.

| Experiment | Radio tables changed |
|---|---|
| (a1) Metro knob 8 (mode 0) -> CSQ Monitor, new frequency 155.475 MHz code 1, prompt 2 | **17**: `tabReceivingMode`, `tabZoneChannelList`, `tabChannelAnnouncement`, `tabChannel`, `tabChannelAdvInfo`, `tabFunctionOnOffOfChan`, `tabFunctionParamOfChan`, `tabVoiceCallSettings`, `tabResetModeSetting`, `tabR2RModeSetting`, `tabAlertModeSetting`, `tabRecvVoiceCallRecordByChannel`, `tabRepeaterSettingOfChan`, `tabDispatchChannel`, `tabContactOfChan`, `tabRecvGroupList`, `tabRecvBehaviorByContactID` |
| (a2) as a1, prompt 0 | vs a1: `tabChannelAnnouncement` only (`-000` instead of `-002`) |
| (a3) as a1 with Rx CTCSS 100.0 | vs a1: `tabRecvSaList` only, `(108, 0, 1, 1000, 1, 19, 1, 0)` |
| (a4) as a1 with Rx DCS 754 inverted | vs a1: `tabRecvSaList` only, `(108, 0, 2, 670, 1, 492, 1, 1)` |
| (a5) as a1, 25 kHz | vs a1: `tabChannel` only (bandwidth 2/2) |
| (a6) as a1, reusing the NOAA 1 frequency row instead of a new one | vs a1: `tabChannel` only (the frequency) |
| (a7) Lake knob 7 (the deleteKnob signature) | the same 17 tables, ZK 207 |
| (f) a frequency row added, no knob | **none** |

`tabZone` did not change because every zone of unit B already had an
active knob. No contact, P25 or encryption table changed: the CSQ knob uses
the all-call contact, which the converter writes unconditionally
(`RS:748-753`).

### 5.3 Knobset's converter

`convert141.rs` already covers this case: CSQ only (any other protocol type
is `Error::Unsupported`, `RS:899-901`), frequency lookup, `RxBandWidth` for
both bandwidth columns (`RS:908-911,801`), all `tabRecvSaList` row types
(`RS:926-935,857-866`) with the same filler values, `MonitorModeOn` and the
voice-buffering rules. The CTCSS path matches PPS on the 3 real rows
(converter-1.41 4.2). The DCS path and bandwidth codes 2 and 3 are straight
copies with no real-data or pager confirmation.

## 6. Where P25 conventional and DMR differ

| Aspect | P25 conventional (type 8) | DMR (type 4) |
|---|---|---|
| protocol setting | knob references a `tabProtocolSetting` row of the type (plus its `tabP25Signaling` / `tabDMRSignaling` and `tabDMRRepeaterSetting`); the knob editor never creates one | same; **the timeslot lives in the protocol setting** (`tabDMRRepeaterSetting.TimeSlot` 0 = slot 1, 1 = slot 2, keyed by the protocol ID, `RR:878-912`), not in the knob |
| knob value | `RxNAC` 0..4095 (`ConventionalSettingControl.cs:187,391,627`); RR default 0xF7E | `RxColorCode` 0..15 (`:188,392,655`) |
| bandwidth | must be code 1 (editor and D1) | same |
| sub-audio | SA block hidden, no `tabRecvSaList` | same |
| `SquelchOption` | visible: Monitor offers Normal 1 / Monitor 3; converter maps Monitor 0/2 -> 3 | hidden, 0 in radio |
| receive options | Emphasis hidden (saved 1), Compander hidden (0), repeater checkbox visible (`TransmitMode` 0/1) | as P25C, AudioSquelch hidden (0) |
| talkgroups | **a grouping is required even in Monitor mode** (B3); otherwise the C# converter fails (converter-1.41 3.4); rows in `tabChannelGroupList`, `tabChannelGroup` | same |
| other | binding features (sub-group, revert list), voice buffering, secure settings visible | secure settings visible |
| Knobset converter | refused (`RS:900`) | refused |

## 7. Contradictions with earlier notes

1. data-model 3.2: "`TxFreq` always = `RxFreq`". Only at creation. Grid
   edits change Rx only; profile creation sets the template row's Tx to the
   first band's Rx minimum + 12.5 kHz (section 2.2).
2. data-model 3.2: 20 kHz "only if project Gx or band 7X/8X/SX". Every
   G-series model number makes the project `Gx`, so 20 kHz is always
   offered on a G5.
3. data-model 3.2 lists the knob's Rx/Tx `SaType`/`CTFreq`/`CDCodeWord`/
   `CDModSense` as the channel's squelch. The knob editor never writes them
   and no converter reads them; Rx tones are only `tabRecvSaList`.
4. data-model 3.0: the 16-zero ID means "CSQ / none" for protocol settings,
   but the same literal is also the template's first, ordinary FreqID.
5. data-model 3.2 implies frequency names are unique; PPS's check for it is
   dead (section 2.5).
6. `ChannelSecureSettingControl.setDefaut` (`EncryptionBy 1`) and
   `BindingFeatureControl.setDefaut` (`DelayNFeature 1`) are overridden on
   save here as on trunking knobs (system-creation 6.4).

## 8. Implementation spec

### 8.1 Common rules

Everything in system-creation 7.1 and system-editing 9.1 applies: one
profile transaction, TEXT values, `''` never NULL, lowercase hyphenated
GUIDs, append new rows, update in place, `FinishDate` on save. Write
`tabSingleConvCHKnob` by column name from the table's own column list and
refuse unknown columns, as `knob_system::default_knob_row` does
(`crates/g5-codeplug/src/knob_system.rs:80-108`). No zip entry is added or
removed.

### 8.2 Inputs and validation

| Input | Rule (refuse otherwise) |
|---|---|
| zone | exactly one `tabZone_New` row with that name (as `knob_system::zone_id`) |
| knob | 1..8; its `tabReceivingMode_NewG4` row exists with `CurrentMode 0`; **no row** for (zone, knob) in `tabSingleConvCHKnob`, `tabAdvancedConvCHKnob`, `tabP25TrunkingSystemKnob`, `tabMultiChannelScan`, `tabAllBandScanCHKnob` (PPS would silently reuse a stale single-conv row) |
| alias | 1..14 characters, not whitespace-only (`system_edit::check_name`); default `Zone {zone row}-Knob {k}` |
| Rx frequency | integer Hz, multiple of 10; inside a `tabRFBandRanges` row with `RxTx 0` of the model's first or second band, edges included (band codes from `tabModel` via the model-number decoding, data-model 4.2). Warn, do not refuse, when not a multiple of 2500 Hz |
| bandwidth | 1 (12.5), 2 (25) or 3 (20). Accept 3 (PPS offers it on every G5) but keep it out of the UI until the on-pager test in 8.5 passes |
| squelch | `carrier`; or `ctcss` with tenths 670..2550 (UI offers the 42 standard tones); or `dcs` with a code from the 104-code list (octal text in the UI, stored as its decimal value) and an invert flag. Up to 5 tones are allowed by the schema, distinct by (type, value, invert); Knobset needs one |
| voice prompt | 0 (none) or 2 (built-in Monitor Mode); default 0, as for trunking knobs (system-creation 7.6) |
| frequency choice | default: create a new row. Optional: an existing `FreqID` the caller names explicitly; it must exist |
| limits | fewer than 1024 `tabConvFreqList` rows when creating one |

Optional knob settings may be exposed with PPS's defaults: `AlertMode`
1..4 (default 1), `RecordFlag` 0..2 (default 1; real knobs use 0),
`AudioSquelch` 0..2 (default 0), `Emphasis` 0/1 (1), `Compander` 0/1 (0).

### 8.3 Rows to write (one profile transaction)

1. **Frequency** (unless an existing FreqID was given): append to
   `tabConvFreqList`
   `(FreqID = new GUID, Name, '1', rx, rx, bw, bw)`. Name = the knob alias
   if no row already has exactly that Name, else `Frequency N` with the
   smallest unused N (PPS's `GetNextName`). Tx = Rx and TxBandWidth =
   RxBandWidth, as every PPS creation path writes; do not reproduce the
   band-minimum Tx that grid edits leave behind.
2. **Receive row**: update the existing `tabReceivingMode_NewG4` row in
   place: `CurrentMode '33'`, `ChannelName` = alias, `AlertToneID` = `'0'`
   or `'2'`, `AlertFileName ''` (only if it is NULL).
3. **Knob row**: append to `tabSingleConvCHKnob` the section 1.2 row with
   `ChannelID` = new GUID, `ConvFreqID` = the frequency, `ProtocolSettingID
   '0000000000000000'`, the eight Rx/Tx sub-audio columns, `TxNAC`,
   `TxColorCode` and `Encryption` as `''`, and the optional settings of 8.2
   (defaults otherwise).
4. **Tones**: for each tone, append to `tabRecvSaList`
   `(ChannelID, '0', SaType, ctFreq, '1', cdCodeWord, '1', cdModSense)`:
   CTCSS `('1', tenths, '1', '0', '1', '0')`, DCS `('2', '0', '1',
   decimal_code, '1', invert)`. None for carrier squelch; never a
   `SaType 0` row.
5. `tabOrganization_New.FinishDate`.

Nothing else: no protocol setting, grouping, `tabChannelGroup`,
`tabChannelGroupList`, sub-group, call-alert or scan row.

### 8.4 Write guard

Profile tables allowed to change: `tabReceivingMode_NewG4` (one row,
updated), `tabSingleConvCHKnob` (one row appended), `tabConvFreqList` (one
row appended, only when a frequency was created), `tabRecvSaList` (rows
appended, only with tones), `tabOrganization_New`.

Radio tables (section 5.2), as a new constant next to `KNOB_CHANNEL_TABLES`
(`crates/g5-codeplug/src/edit.rs:266-286`):

```text
CONV_KNOB_CHANNEL_TABLES = tabReceivingMode, tabZoneChannelList,
  tabChannelAnnouncement, tabChannel, tabChannelAdvInfo,
  tabFunctionOnOffOfChan, tabFunctionParamOfChan, tabVoiceCallSettings,
  tabResetModeSetting, tabR2RModeSetting, tabAlertModeSetting,
  tabRecvVoiceCallRecordByChannel, tabRepeaterSettingOfChan,
  tabDispatchChannel, tabContactOfChan, tabRecvGroupList,
  tabRecvBehaviorByContactID,
  tabRecvSaList (only when a tone is set),
  tabZone (only when the zone had no active knob)
```

Do not reuse `KNOB_CHANNEL_TABLES` + `TALKGROUP_EDIT_TABLES`: that would
allow the P25 and encryption tables (`tabP25TSetting`,
`tabKMFSystemByChannel`, `tabCipherKeyAssignment`,
`tabP25OTARFeatureControl*`) and the contact list (`tabContacts`,
`tabContactAddr`, ...), none of which a CSQ knob may change. A change to
any of them means the edit did something it should not.

### 8.5 On-pager tests and open risks

1. Program case a1 on unit B (Metro knob 8, a local analog or NOAA
   frequency) and confirm the knob shows its alias, opens on carrier and
   receives audio. Repeat with prompt 2 to confirm the built-in Monitor
   Mode announcement exists (every real knob uses 0).
2. Rx CTCSS written by Knobset: same as PPS's real rows, low risk; confirm
   the pager stays muted on a carrier without the tone.
3. **Rx DCS** has no real-data or pager evidence: test a normal and an
   inverted code other than 023.
4. **25 kHz and 20 kHz** (codes 2, 3) are not in any real profile; confirm
   the pager accepts both and that 20 kHz is meaningful on a VHF channel.
5. Several tones on one knob (up to 5): confirm the pager opens on any of
   them. Until then Knobset should write one.
6. A knob in a zone with no active knob flips `tabZone.ModifyByDevice`
   (rule known, not yet exercised by Knobset on a pager).
7. Frequencies off a 2.5 kHz grid: PPS stores 10 Hz steps; whether the
   synthesizer honors them is unknown (unit B's 155.0025 MHz knob is on the
   2.5 kHz grid).
8. Not a pager test: open a Knobset-made knob in PPS on Windows, save, and
   confirm PPS rewrites the same rows (it should; the row equals PPS's own
   default).

### 8.6 PPS behavior Knobset should skip

| PPS behavior | Why skip |
|---|---|
| Defaulting a new knob to the first frequency row | silently tunes the knob to whatever the template row holds (763.000 MHz on a fresh SX profile) |
| Tx left at the band minimum after an Rx edit | meaningless and confusing; write Tx = Rx |
| Deleting a frequency a knob uses | leaves a dangling `ConvFreqID` that blocks programming; refuse instead |
| Sharing one frequency row across knobs by default | a later frequency edit would retune every knob on it; create one row per knob unless the caller asks |
| Writing knob `Rx*/Tx*` sub-audio columns (RR importer) | unread by every converter; the editor leaves them `''` |
| `SaType 0` rows for "CSQ" tones (RR importer) | produce `SigType 255` radio rows for nothing |
| Built-in voice prompt 2 as the default | every real knob has none; surface it as an option |
| `setDefaut` values that `Save` overrides (`EncryptionBy 1`, `DelayNFeature 1`) | the saved values are 0 |
