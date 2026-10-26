# DMR and P25 conventional knobs: protocol settings, talkgroups, conversion

Scope: what PPS V00-04-00-48 writes when the user adds a DMR (protocol
type 4) or P25 conventional (type 8) channel to an empty knob in Monitor
mode (`CurrentMode 33`) or Selective Call (`CurrentMode 1`): the protocol
setting, the talkgroups and grouping the knob needs, the knob row, what
PPS validates and what its 1.41 converter writes to the radio DB. Sibling
of [conventional-knobs.md](conventional-knobs.md) (analog knobs); its
sections 1 to 3 (knob editor flow, frequencies, receive-row update) apply
here unchanged and are not repeated. Section 6 of that note is superseded
by this one.

Evidence paths are relative to `decomp/`. Short forms: `PL/` =
`View/View.ProtocolSetting/`, `ZONE/` = `View/View.Zone/`, `GRP/` =
`View/View.GroupSetting/`, `CHECK/` =
`View/View.ProfileCheck/ProfileCheckHandler.cs`, `RR/` =
`View/View.RadioReference/RadioReferencehandler.cs`, `CONV/` =
`View/View.ProgramVersion/PPSU4ParamConverter_01P41b.cs`, `CONVC/` = the
`_01P41c.cs` sibling, `BASE/` = `PPSU4ParamConverterBase.cs` in the same
folder, `RS/` = `crates/g5-codeplug/src/convert141.rs`, `HELP` =
`scratch/chm_dedup.txt` (text of the PPS help file; quoted topics are
`ParameterDescription/...` pages). Real data: unit B's profile
(`G5B67BF-SXVXEN1401UNI`, radio DB 1.41c) at `scratch/conv-study/` and
`scratch/syscreate/unitB/`; unit A's profile (`G5D66BE`, 1.41b) from its
backup. Neither contains a DMR or P25C channel, a grouping or a
`tabChannelGroupList` row, so every rule below comes from PPS code, the
shipped templates and the help text. Experiments:
`scratch/digconv_tools/exp.py` (writes `scratch/digconv/exp/`).

## 0. Summary

- **Three profile objects per digital knob, all created on other pages
  first.** A protocol setting (`tabProtocolSetting` plus its signaling
  rows), at least one talkgroup (`tabGroup_New` with a `DMRAddr` or
  `P25Addr`) and a grouping of that protocol (`tabGroupGrouping` +
  `tabGroupingOfGroup`). The knob editor only picks them. Its protocol-type
  combo lists a type only when a protocol setting of that type exists.
- **The DMR timeslot is in the protocol setting**, not the knob:
  `tabDMRRepeaterSetting.TimeSlot` (0 = slot 1, 1 = slot 2), keyed by the
  protocol setting's ID. Two DMR knobs on different slots need two DMR
  protocol settings. Color code (`RxColorCode`) and NAC (`RxNAC`) are on
  the knob.
- **A grouping is required in Monitor mode too.** Three independent gates:
  the knob editor's Talk Group List refuses to save a non-CSQ knob without
  a grouping that has at least one group; the pre-program check raises B3;
  the 1.41 converter returns failure for a non-CSQ knob without a
  `tabChannelGroupList` row, which aborts programming. The minimal set is
  one group and one grouping per protocol. PPS's own RadioReference
  importer creates exactly that for a Monitor knob with no known
  talkgroup (DMR address 16776415, P25C address 65535).
- **Knob row = the analog default with four columns changed**:
  `ProtocolSettingID`, `RxColorCode` (DMR) or `RxNAC` (P25C),
  `SquelchOption` (P25C only; 3 = Monitor, which per the help ignores the
  NAC, 1 = Normal, which matches it) and optionally `TransmitMode`. Plus
  one `tabChannelGroupList` row and one `tabChannelGroup` row per group.
  Bandwidth must be code 1 (12.5 kHz).
- **Conversion adds**, beyond the 17 analog channel tables:
  `tabDMRSignaling` + `tabDMRRepeaterSetting` (DMR) or `tabP25Signaling` +
  `tabP25RepeaterSetting` (P25C), one group contact per grouping member
  (address + 2^31) and a receive-list triple per member instead of the
  all-call one. Conventional group contacts take IDs from 10000 before the
  P25 trunking talkgroups, so **one new conventional group renumbers every
  trunking talkgroup contact** (experiment g1: 11 contact tables change).
  On a profile with encryption type 3 (both units) a P25C knob also gets
  the per-channel key rows that P25T knobs get.
- **DMR voice settings are device-wide**: the converter takes
  `DMRVocoder` and `DMRCompatibility` from the profile's
  `tabVoiceCallVocoderSetting` (1 = AMBE, 1 = Motorola on every G5 model),
  not from the protocol setting's own `DMRCompatibility` column.
- **Knobset's converter refuses both protocols on purpose**: its policy is
  to refuse every path no real profile exercised (`RS:9-11`), and
  `convert_single_ch_knob` and `convert_recv_group_list` stop on any
  conventional type other than CSQ (`RS:899-901`, `RS:991-993`). The
  contact and encryption code already handles types 4 and 8. Section 6
  lists what is missing.
- **Biggest open risk for DMR on unit B**: the model code (67) and the
  profile (`DMRC 1`) say DMR, but the pager's handshake reports protocol
  mode `0x00010517`, which is the analog + P25C + P25T mask without the
  DMR bit (section 7). Unit A has no DMR at all (`DMRC 0`). P25C is in
  both.

## 1. Protocol settings

### 1.1 Creating one

Page "Protocol Parameter Setting" (`PL/ProtocolControl.cs`,
`PL/ProtocolList.cs`). Add opens a menu of the model's protocols without
CSQ (`PL/ProtocolList.cs:395-413`, from the profile's `tabProtocolMode`
flags via `View/View.Profile/ModelNumberInfo.cs:266-300`). Choosing one
(`PL/ProtocolList.cs:432-447`) appends a `tabProtocolSetting` row and the
type's parameter rows (`:449-627`). Paste (`:257-341`) copies a row and
then resets the hidden parameters with the type's `set_default`.

| Rule | Value | Evidence |
|---|---|---|
| max protocol settings | 16, all types together (unit B already uses 1 for P25T) | `PL/ProtocolList.cs:29,770-779` (`updateDgvToolBar`) |
| alias | `Protocol N`, smallest unused N; grid max 14 characters; blank rejected; the duplicate check is dead (its dictionary is never filled) | `:438,1195,845-866` |
| `ProtocolNo` | smallest unused integer as text | `:439`, `View/View.Common/CommonFunction.cs:636-673` |
| `ProtocolType` | combo is read-only after creation | `:1203` |
| delete | removes the parameter rows (`:659-710`); `CheckIfCanDelete` (`:712-776`) has no caller, so a knob can be left pointing at a deleted setting (pre-program B2) | |

### 1.2 Rows for a new DMR setting (type 4)

| Table | Column | New value | UI | Range / meaning | Evidence |
|---|---|---|---|---|---|
| `tabProtocolSetting` | `ID`, `ProtocolNo`, `ProtocolName`, `ProtocolType` | new GUID, next number, `Protocol N`, `4` | | | `PL/ProtocolList.cs:436-440` |
| `tabDMRSignaling` | `ID` | the setting's ID | | | `:543-555` |
| | `TXPreFrames` | 4 | none | Tx only | |
| | `RCRxEnable` | 0 | combo, **disabled** | help: "currently not supported" | `PL/DMRSetting.cs:33-38`, HELP `4_ProtocolSetting/DMR/RCRxEnable` |
| | `NoSigActTimeout` | 600 | spinner 60..15360 ms | "Signaling Activity Timeout"; help: used for channel scan, should exceed the preamble by 60 ms | `PL/DMRSetting.cs` designer `Maximum 15360`, `Minimum 60` |
| | `ACKWaitTimeout` | 5000 | none | | |
| | `MFID` | 0 | none | manufacturer ID | |
| | `SAP4NRS`, `SAP4RS` | 4, 5 | none | data service access points | |
| | `DMRCompatibility` | 2 | none | **never read by the 1.41 or 2.06 converter** (section 1.5) | |
| `tabDMRRepeaterSetting` | `ID` | the setting's ID | | | `PL/ProtocolList.cs:556-560` |
| | `TimeSlot` | 0 | combo "Repeater Slot": Slot 1 / Slot 2 | stored as the combo index: **0 = slot 1, 1 = slot 2** | `PL/DMRSetting.cs:50-58,77-84`, resx `comboBoxControl_repeaterSlot.Items/Items1` |
| | `TimingAlign` | 0 | combo "Repeater Timing", **disabled** | 0 Aligned, 1 Offset ("not supported" per help) | `PL/DMRSetting.cs:33-38`, HELP `4_ProtocolSetting/DMR/RepeaterTiming` |

`DMRSetting.Save` writes only the four UI fields (`PL/DMRSetting.cs:77-84`).
The RadioReference importer writes the same values, with `TimeSlot` from
the agency data (`RR:899-923`), and reuses an existing DMR setting whose
slot matches instead of creating a duplicate (`RR:875-897,924-937`).

The help on slots (HELP `4_ProtocolSetting/DMR/RepeaterSlot`): the slot
applies in repeater mode; a pager set to slot 1 or 2 always also hears
talkaround (direct) transmissions; a pager in talkaround mode hears
talkaround calls "as well as repeated calls on the configured repeater
slot". So the knob's default talkaround setting (section 3.2,
`TransmitMode 1`) still receives a repeater on the configured slot.

### 1.3 Rows for a new P25 conventional setting (type 8)

| Column | New | Paste default | UI (all ms) | Evidence |
|---|---|---|---|---|
| `ID` | setting's ID | | | `PL/ProtocolList.cs:563-581` |
| `TXPreTime` | 20 | 10 | none | `PL/P25CSetting.cs:120-131` |
| `NoSigActTimeout` | 600 | 600 | none (the help lists it, the panel has no control) | |
| `ACKWaitTimeout` | 5000 | 5000 | none | |
| `MFID`, `AlgId` | 0, 133 | same | none | |
| `MinShortDigitalToneDuration` | 200 | kept | 0..1000 | data-model 3.6 |
| `MinLongDigitalToneDuration` | 2500 | kept | 200..5000 | |
| `MinToneAddrGap` | 1000 | kept | 20..5000; `checkParam` rejects it above `TxToneAddrGap` | `PL/P25CSetting.cs:145-155` |
| `OTAToneFlag` | 1 | 1 | none | |
| `ShortDigitalToneATxDuration`, `ShortDigitalToneBTxDuration`, `LongDigitalToneTxDuration`, `TxToneAddrGap` | 1000, 2000, 5000, 1500 | kept | Tx timings, 200..33000 / 500..5000 | |

The tone fields serve QCII two-tone paging over P25 (sub-group binding);
for plain voice monitoring every value can stay at its default. There is
no repeater-setting row in the profile for P25C; the radio's
`tabP25RepeaterSetting` is built from the knob (section 5.2). The
RadioReference importer reuses the first existing P25C setting and only
creates one if none exists, with `TXPreTime 10` (`RR:1029-1068`).

### 1.4 Template rows that are not settings

Both the profile template (`DataBase/PPS_G4A.db`) and unit B's profile hold
one row each in `tabDMRSignaling`, `tabDMRRepeaterSetting`, `tabP25Signaling`
and `tabP25TSignaling` with `ID '000000000000'` (12 zeros), and no
`tabProtocolSetting` row points at them. They are inert defaults (the
importer explicitly skips them, `RR:878,1029`). Knobset must never reference
or edit them. The DMR one equals a new setting; the P25 one has
`TXPreTime 10`.

### 1.5 Device-wide DMR vocoder and compatibility

`tabVoiceCallVocoderSetting` (one row: `DMRVocoder`, `DMRCompatibility`) is
copied from the model's `BSS_G4A.db` when the profile is created
(`View/View.Profile/U4PPSDataRepository.cs:129-140`) and is editable only
on the Engineer-login page "DMR Vocoder Setting"
(`View/View.GeneralSetting/DMRVocoderControl.cs:35-50`; combos MELPe/AMBE
and Standard/Motorola). Every one of the 140 G5 BSS databases in the
installer ships `DMRVocoder 1` (AMBE) and `DMRCompatibility 1` (Motorola);
both real profiles have that row.

The 1.41 converter reads this row through `BbsDataRepository`
(`View/View.Profile/BbsDataRepository.cs:93-108`) and writes its two values
into every radio `tabDMRSignaling` row (`BASE:1784-1828`); the 2.06
converter does the same (`PPSU4ParamConverter_02P01.cs:1661-1671`). If the
row is missing, PPS dereferences a null object and programming fails. The
protocol setting's `DMRCompatibility 2` is dead data.

### 1.6 Model support

| Unit | Profile `tabProtocolMode` | Model protocol code | Pager handshake protocol mode |
|---|---|---|---|
| A (`G5D66BE`) | `P25C 1`, `DMRC 0` | 66 = `0x10517` | `0x00010517` |
| B (`G5B67BF`) | `P25C 1`, `DMRC 1` | 67 = `0x13717` | `0x00010517` |

PPS only offers DMR when the profile's flag allows it, and it compares
protocol codes of the profile model and the connected pager's model
number when programming (`View/View.ProgramVersion/ProgramController.cs:179-203`),
never the handshake value. The handshake value is decoded with the same
bit table (`View/View.Profile/ProtocolMode.cs:87-137`: `0x200` DMR,
`0x400` P25C, `0x10000` P25T) and lacks the DMR bit on unit B.

## 2. Talkgroups and groupings

### 2.1 Groups (`tabGroup_New`), page "Conventional Group ID Setting"

`GroupList.addNewGroup` (`GRP/GroupList.cs:723-772`) prepares the row and
`NewGroupSetting` edits it (`GRP/NewGroupSetting.cs:355-475`):

| Column | Editor default | Notes |
|---|---|---|
| `GroupID` | new GUID | |
| `GroupName` | `Group N`, max 14 (`GRP/NewGroupSetting.cs:596`) | the contact name on the pager |
| `GroupNo` | next integer | |
| `Note` | `''` | |
| `RecordFlag` | 1 | "Voice Storage" checkbox |
| `Type`, `Relay`, `Priority`, `DutyStatus` | 0, 0, 0, 0 | |
| `AlertMode` | 1 (Tone) | 1..4 |
| `AlertToneID` | 9999 (default tone) | 0 = none |
| `BackgroundColor` | 0 | |
| `EncryptionKeyID` | first `tabEncryptionKeyList` ID (`000000000000` on both units) | |
| `KeyStrapOption` | 2 | |
| `TwoToneAddr` .. `P25Sub3Addr` | `''`; the protocol's address when its checkbox is set | `UDCAddr` is always copied from `MDCAddr` (`:412`) |

Addresses are stored as decimal text. Ranges in the group dialog
(`View/View.AddrBaseControl/ParameterDigitAddrControl.cs:614-645`): DMR
1..16776415 **or** 16777200..16777215; P25C 1..65535. Max 8192 groups
(`GRP/GroupList.cs:33`).

The template ships a group "All" (`GroupNo 2`, `RecordFlag 1`,
`AlertMode 1`, `AlertToneID 0`, `KeyStrapOption 2`, `MDCAddr`/`UDCAddr`
65535, `DMRAddr 16777215`, `P25Addr 65535`), each protocol's all-call ID.
Unit B's user deleted it; only the empty `Group 1` remains.

### 2.2 Groupings (`tabGroupGrouping`, `tabGroupingOfGroup`)

Page "TGID Grouping List" (`GRP/GroupingList.cs`). Add asks for the
protocol, then opens the member picker (`:806-829`):

| Item | Rule | Evidence |
|---|---|---|
| `tabGroupGrouping` row | `(GroupingID new GUID, GroupingAlias "TGID List N", Type = protocol, Note '')` | `GRP/GroupingList.cs:812-816` |
| alias | max 14, non-blank; duplicate check dead like the protocol one | `:1066,522-541` |
| max groupings | 1024 | `:22` |
| members offered | only groups whose address column for that protocol is non-empty | `GRP/GroupList.cs:1133` |
| members per grouping | picker: 16 for DMR, 20 for P25C (`GRP/GroupList.cs:1086-1100`; the DMR 15 branch there is unreachable). The paste helper uses 15 for DMR (`GRP/GroupingList.cs:250-262`) | |
| duplicate addresses | rejected on save | `GRP/GroupList.cs:500-562` |
| `tabGroupingOfGroup` | `(GroupingID, GroupID)` per selected group, in `tabGroup_New` row order | `:566-575` |

A grouping is reusable by any number of knobs. A knob always receives on
**all** members of its grouping; per-knob options (None / Off Duty /
Priority) are a radio-button choice per member
(`ZONE/KnobGroupOptionControl.cs:49-51`).

### 2.3 Required even for Monitor mode

| Gate | What it demands | Evidence |
|---|---|---|
| knob editor | the "Talk Group List" control is shown for every non-CSQ protocol in both modes (`ZONE/SelectiveCallControl.cs:79-94`); its `checkError` refuses a missing grouping and a grouping with no members | `ZONE/ConventionalRxGroupListControl.cs:485-503` |
| pre-program check B3 | a grouping of the knob's protocol with at least one member whose address column is not NULL (an empty string passes); no duplicate addresses | `CHECK:359-460,570-577` |
| 1.41 converter | no `isRevert 0` row in `tabChannelGroupList` for the knob, or no grouping of the knob's type among several, or an empty grouping: return failure | `CONV:1608-1643` |

What the pager does with the list in Monitor mode is not in the code. The
help says Monitor mode receives "from all talk groups in single frequency"
(`ZONE/SelectiveCallControl.cs:197-206`), the P25C "Monitor" squelch option
"will receive any group call and ignore network access code checking"
(HELP `7_1_SelectiveCall/3_SquelchOption`), and listed IDs drive alerts and
"ID Match" voice storage (HELP `7_1_SelectiveCall/5_VoiceStorage`).

**Minimal set PPS itself creates.** The RadioReference agency importer
builds, for a DMR or P25C agency frequency with no talkgroup, a Monitor
knob plus one placeholder group (DMR address 16776415, `RR:966-985`; P25C
address 65535, `RR:1097-1115`), a one-member grouping named
`TGID List N` of that type (`RR:986-994,1118-1126`), a `tabChannelGroup`
row and a `tabChannelGroupList (ChannelID, GroupingID, 0)` row
(`RR:1235-1244`). Its group rows differ from the editor's (`RecordFlag 0`,
`AlertMode 3` silent, `KeyStrapOption 1`, `RR:1320-1345`), it writes the
legacy `tabGroupAddress` and a `tabGroupMembers_New` link per member
(`RR:1346-1373`), and its `tabChannelGroup` row has `SeqNo 0` and no
`SigType` (`RR:458-478`). The importer reuses one placeholder group per
protocol across knobs but creates a new grouping per knob.

## 3. The knob

### 3.1 What the editor does differently from analog

`SelectiveCallControl.ProtocolChangedEventHandler`
(`ZONE/SelectiveCallControl.cs:79-94`) and each sub-control react to the
protocol:

| Control | DMR | P25C | Evidence |
|---|---|---|---|
| Conventional Channel Setting | protocol alias combo (settings of that type); color code spinner 0..15; SA block hidden | alias combo; NAC spinner 0..4095 (hex entry); SA block hidden | `ZONE/ConventionalSettingControl.cs:180-226,264,324-328,627,655` |
| Channel Receive Option | Emphasis, Compander, Audio Squelch hidden; "Repeater Mode (Digital)" checkbox shown | Emphasis, Compander hidden; Audio Squelch shown; repeater checkbox shown; Squelch Option combo shown unless the sub-group binding is on | `ZONE/ConventionalReceiveOptionControl.cs:269-288` |
| Binding Feature | hidden | shown (None / Sub-Group); with None every reset and delay control stays hidden | `ZONE/SelectiveCallControl.cs:90`, `ZONE/BindingFeatureControl.cs:382-560` |
| Function Setting | Monitor: voice buffering checkbox shown (default on), prompt tone and force drop shown under it. Selective: push-to-listen, privacy, voice buffering | same | `ZONE/ChannelFunctionSettingControl.cs:200-260` |
| Secure Setting | shown; "Encrypted" unchecked | shown | `ZONE/SelectiveCallControl.cs:86` |
| Talk Group List | shown, grouping combo of type 4 | type 8 | section 3.3 |
| Revert Talk Group List | hidden (needs P25 sub-group binding plus a revert reset mode) | hidden by default | `ZONE/BindingFeatureControl.cs:289-317` |

Hidden controls save fixed values, so with untouched controls every
column outside the table in 3.2 keeps the analog default of
conventional-knobs 1.2 (`Emphasis 1`, `Compander 0`, `AudioSquelch 0`,
`VoiceBuffering 1`, `ResetMode 0`, `DelayNTime 30`, `DelayNFeature 0`,
`EncryptionBy 0`, `EncryptedCallFlag 0`, `KMFSystemID 000000000000`, ...).
Hidden Emphasis saves 1 and hidden Compander 0
(`ZONE/ConventionalReceiveOptionControl.cs:259-266`); the visible voice
buffering checkbox starts checked from `setDefaut`
(`ZONE/ChannelFunctionSettingControl.cs:445-468`) and saves 1.

### 3.2 Knob row: columns that differ from the analog Monitor default

| Column | DMR Monitor | P25C Monitor | Evidence |
|---|---|---|---|
| `ProtocolSettingID` | the DMR setting's ID | the P25C setting's ID | `ZONE/ConventionalSettingControl.cs:390` |
| `RxColorCode` | 0..15, default 0 | 0 | `:392` |
| `RxNAC` | 0 | 0..4095, default 0 (editor) | `:391` |
| `SquelchOption` | 3 (combo hidden, Monitor writes 3; converter writes 0) | combo Normal (1) / Monitor (3), default 3 | `ZONE/ConventionalReceiveOptionControl.cs:35-47,126-185,264` |
| `TransmitMode` | 1 = talkaround (default), 0 = "Repeater Mode (Digital)" checked | same | `:133,263` |
| `TxColorCode`, `TxNAC` | `''` (never set by the editor) | `''` | the RR importer sets both to the Rx value (`RR:996-1005,1131-1165`) |

NAC semantics from the help (HELP `7_1_SelectiveCall/2_P25NAC`): entered
in hex; **0xF7E "indicates to receive any NAC and any call"**; 0xF7F is
reserved for repeaters "and is not allowed by radio". The editor does not
block 0xF7F. The RR importer uses 0xF7E (3966) when the NAC is unknown
(`RR:1144,1163`).

P25C squelch option semantics (same help page, section 0 quote):
Selective (0) = only listed talkgroups; Normal (1) = any group call with a
matching NAC; Don't Care NAC (2) = listed talkgroups, NAC ignored; Monitor
(3) = any group call, NAC ignored. Monitor mode offers 1 and 3; Selective
Call offers 0, 1 and 2. **So a P25C Monitor knob left at its default
ignores the NAC; to monitor one NAC, choose Normal (1).** DMR has no such
option; whether a DMR Monitor knob gates on the color code is not stated
anywhere.

### 3.3 Receive-list rows written by the knob

`ConventionalRxGroupListControl.Save` (`ZONE/ConventionalRxGroupListControl.cs:769-778,669-767`)
deletes the knob's `tabChannelGroup`, its `isRevert 0`
`tabChannelGroupList` row and its sub-group rows, then:

1. `tabChannelGroupList (ChannelID, GroupingID, isRevert 0)`, one row.
2. One `tabChannelGroup` row per grouping member, with `ChannelID`,
   `GroupID`, `SigType` = 4 or 8, `SeqNo`, `OffDuty` (0/1), `Priority`
   (0/1), `OutOfRange 0`, `OTA 0` and the nine ack columns (0, or the ACK
   dialog's values when the model supports acks).

Row order and `SeqNo`: the panel builds its member controls in reverse
grouping order (`:196-247`) and `Save` walks them in that order with a
descending counter, so rows are appended last member first and the member
at grouping position i (1-based) gets `SeqNo i`. The converter only reads
`SeqNo` for P25C (section 5.2).

The knob editor also saves `tabP25TSubGroupEnable` and
`tabP25TCallAlertEnable` rows only when a binding feature is chosen
(`ZONE/BindingFeatureControl.cs:662-690`), so nothing for the default.

### 3.4 Selective Call (mode 1)

Same tables. Differences from Monitor:

| Item | Selective Call | Evidence |
|---|---|---|
| `tabReceivingMode_NewG4.AlertToneID` default | 1 (built-in "Selective Call" prompt) | `View/View.Common/NameValueVModel.cs:47-63` |
| `SquelchOption` | P25C combo Selective (0, default) / Normal (1) / Don't Care NAC (2); DMR hidden, saves 0 | `ZONE/ConventionalReceiveOptionControl.cs:16-34,141-151,264` |
| `RecordFlag` | Off (0) / ID Match (1, default); no All Voice | `ZONE/ChannelFunctionSettingControl.cs:24-37` |
| `PushToListen`, `Privacy` | visible, default 0 | `:200-260` |
| talkgroup list | exactly as 3.3 | |

The receive list is the grouping. Limits: one non-revert grouping per
knob; 16 (DMR, 15 to be safe) or 20 (P25C) members; groups and groupings
as in section 2. The knob cannot subset a grouping. Changing a grouping's
members changes every knob that uses it on the next conversion (the
converter reads `tabGroupingOfGroup`, section 5.1), while that knob's
`tabChannelGroup` rows stay stale until the knob is saved again (stale rows
only cost their Off Duty / Priority options). With the P25C sub-group
binding, the knob also allows at most 32 sub-groups
(`ZONE/ConventionalRxGroupListControl.cs:53,527-534`); Knobset does not
need that path.

## 4. Validation

### 4.1 Knob editor (`checkError`, gates every save)

| Rule | Evidence |
|---|---|
| a frequency, a protocol type and, for non-CSQ, a protocol alias | `ZONE/ConventionalSettingControl.cs:342-366` |
| DMR and P25C: the frequency's `RxBandWidth` must be `1` | `:367-378` |
| color code 0..15, NAC 0..4095 (spinner limits) | `:627,655` |
| a grouping is selected and has at least one member | `ZONE/ConventionalRxGroupListControl.cs:485-503` |
| duplicate addresses in the grouping: warning | `:553-600` |
| P25C with voice buffering off and alert Tone or Tone & Vibrate: warning that the start of calls is cut | `ZONE/ChannelFunctionSettingControl.cs:359-370` |

### 4.2 Other pages

Protocol alias non-blank; P25C `MinToneAddrGap <= TxToneAddrGap`; group
alias 14; group address ranges (2.1); grouping alias non-blank, 14, members
with an address for the grouping's protocol, member caps and no duplicate
addresses (2.2).

### 4.3 Pre-program check (`CheckProfileData`)

For modes 1, 33 and 128 (`CHECK:538-578`): B2 the knob row exists and its
non-CSQ protocol setting resolves; B1 the frequency exists; D1 DMR/P25C
need `RxBandWidth 1` (`CHECK:314-327`); B3 for any non-CSQ protocol a
grouping of that type with at least one member whose address is not NULL,
no duplicate addresses; for P25C the same duplicate check on the revert
list and, with the sub-group binding, at least one sub-group
(`CHECK:359-460`). Nothing checks the color code, NAC, slot, vocoder row or
signaling rows; the converter fails on several of those instead.

### 4.4 Converter-time failures (programming aborts)

| Condition | Evidence |
|---|---|
| `ProtocolSettingID` not in `tabProtocolSetting` | `CONV:2302-2312,1560-1564` |
| no `tabChannelGroupList` row, no grouping of the knob's type, or an empty grouping | `CONV:1608-1643` |
| a grouping member with an empty address for the protocol: `long.Parse('')` throws while building contacts | `CONV:870-876,1235-1242` |
| no `tabVoiceCallVocoderSetting` row (DMR) | `BASE:1786-1795` |

A missing `tabDMRSignaling` or `tabP25Signaling` row for the setting does
**not** fail: the signaling insert is skipped (for P25C the
`tabP25RepeaterSetting` row is still written), and the pager gets a DMR or
P25C channel with no signaling row. Knobset should refuse instead.

## 5. Conversion (1.41b/c)

Per single conventional knob, `convertSelectiveCall` runs
`convertSingleCHKnobInfo`, then `convertSignaling`, then
`convertRecvGroupList` (`CONV:2232-2282`). ZK = zone row index x 100 +
knob; the knob's SigID is the ZK. 1.41c only changes the channel bundle
(`ForceDrop`) and drops one early return in the receive list that matters
only for revert lists (`CONVC:603-785,822-1274`).

### 5.1 DMR knob

| Radio table | Row (Monitor; values for untouched controls) | Evidence |
|---|---|---|
| `tabReceivingMode`, `tabZoneChannelList`, `tabChannelAnnouncement` | as analog (conventional-knobs 5.1) | |
| `tabChannel` | `(ZK, alias, isSupportRepeater 0, Rx, Rx, 1, 1)` | `CONV:2330`, `CONVC:606-615` |
| `tabChannelAdvInfo` | as analog except `ColorCode = RxColorCode`, `NAC = RxNAC` (0), `DefaultCallID 0`, `SquelchOption 0`, `PowerSavingRxOffDur 0`; `Emphasis`/`Compander` and the squelch pair still come from the knob (1, 0, 51, 4) | `CONV:2324-2410,2492-2513`, `CONVC:617-622` |
| `tabFunctionOnOffOfChan` | `talkAroundOn = TransmitMode` (1 talkaround, 0 repeater), `DelayPlayVoiceMsg 1`, `MonitorModeOn 1` (no SA rows, internal value 2) | `CONV:2472-2484`, `CONVC:623-643` |
| `tabFunctionParamOfChan`, `tabVoiceCallSettings`, `tabResetModeSetting`, `tabR2RModeSetting`, `tabAlertModeSetting`, `tabRecvVoiceCallRecordByChannel`, `tabDispatchChannel` | as analog | |
| `tabRepeaterSettingOfChan` | `(ZK, 4, AccessCodeType 5, RACID ZK, 0, 670, 1, 19, 1, 0, 0, 670, 19, 0)` | `CONVC:666-688` |
| `tabRecvSaList` | none (the editor stores no tones for DMR) | |
| `tabDMRSignaling` | `(SigID ZK, TXPreFrames, RxColorCode = knob RxColorCode, RCRxEnable, NoSigActTimeout, ACKWaitTimeout, MFID, SAP4NRS, SAP4RS, DMRCompatibility = vocoder row, DMRVocoder = vocoder row)`; with defaults `(ZK, 4, cc, 0, 600, 5000, 0, 4, 5, 1, 1)`. Values are inserted unquoted | `BASE:1784-1810` |
| `tabDMRRepeaterSetting` | `(SACID ZK, TimeSlot, TimingAlign, RxColorCode = knob RxColorCode, TxColorCode = knob TxColorCode)`; editor default `(ZK, slot, 0, cc, '')` | `BASE:1811-1826` |
| `tabContactOfChan` | per grouping member: `(ZK, contact, 4)`. **No all-call row**: the call that would add it discards its result | `CONV:1574-1577,1653` |
| `tabRecvGroupList` | per member: `(ZK, 4, SigID ZK, DMRAddr + 2147483648, contact, SeqNo 0)` | `CONV:1669-1673` |
| `tabRecvBehaviorByContactID` | per member: `(ZK, contact, Priority = tabChannelGroup.Priority, Duty = 1 unless OffDuty, AlertMode = group AlertMode minus 1, VoiceStorage = 1 if group RecordFlag > 0 else 2, 4, acks from tabChannelGroup)`; defaults to 0/1 when the knob has no `tabChannelGroup` row | `CONV:1674-1682,2058-2095` |

Members are taken from `tabGroupingOfGroup` (row order) for the grouping
chosen as follows: the knob's single `isRevert 0` row, or with several rows
the first whose grouping `Type` equals the knob's protocol
(`CONV:1608-1643`). `tabChannelGroup` is consulted only for the per-member
options. A member missing from the contact map is skipped silently
(`CONV:1648-1651`; cannot happen when both come from the same groupings).

The 1.41 radio schema has no slot column in `tabDMRSignaling`
(`notes/schema/DB_Base_G4A.DBV1.41e.sql:342-355`); the slot lives in
`tabDMRRepeaterSetting(SACID, TimeSlot, TimingAlign, TxColorCode,
RxColorCode)` (`:334-341`). The 2.06 converter moves it into
`tabDMRSignaling.RptSlot` with the same 0/1 meaning
(`PPSU4ParamConverter_02P01.cs:1653-1690`). The 1.41 converter hard-codes
`isSupportRepeater 0` (`CONV:2330`); 2.06 derives it from `TransmitMode`.

### 5.2 P25C knob

Same channel bundle as DMR with `ColorCode 0`, `NAC = RxNAC`, and:

| Radio table / column | Row | Evidence |
|---|---|---|
| `tabChannelAdvInfo.SquelchOption` | knob value; Monitor mode turns 0 and 2 into 3; Selective (and radio-compatible) mode turns 3 into 0 | `CONV:2492-2502` |
| `tabRepeaterSettingOfChan` | `(ZK, 8, AccessCodeType 6, ZK, ...)` | |
| `tabP25Signaling` | `(SigID ZK, txpretime, nosigacttimeout, ackwaittimeout, rxnac = knob RxNAC, mfid, algid, MinShort, MinLong, MinToneAddrGap, EncryptedCallFlag = knob, ShortA, ShortB, LongDigitalToneBTxDuration = profile LongDigitalToneTxDuration, TwoDigitalToneAddrTxGap = profile TxToneAddrGap)`; defaults `(ZK, 20, 600, 5000, nac, 0, 133, 200, 2500, 1000, 0, 1000, 2000, 5000, 1500)` | `BASE:1830-1862` |
| `tabP25RepeaterSetting` | `(SACID ZK, RxNAC = knob RxNAC, TxNAC = knob TxNAC)`; editor default `(ZK, nac, '')` | `BASE:1863` |
| `tabContactOfChan` | per member `(ZK, contact, 8)`; no all-call row | `CONV:1653` |
| `tabRecvGroupList` | per member, when its sub-group `Alt` is above 0 (always, without the sub-group binding: `getAlt` returns 1, `CONV:1994-2009`): `(ZK, 8, ZK, P25Addr + 2147483648, contact, SeqNo)`, where SeqNo counts 1, 2, ... over members whose `tabChannelGroup.SeqNo > 0` and is 0 otherwise. With editor-written rows every member gets a number: 1..N in grouping order | `CONV:1654-1668` |
| `tabRecvBehaviorByContactID` | as DMR with `SigType 8` | |
| revert list | only with an `isRevert 1` grouping (sub-group binding with a revert reset mode): extra contact-of-chan and behavior rows, and `tabRevertListGroup` pairs | `CONV:1689-1725,1788-1830` |

### 5.3 Contacts

`convertContacts` (`CONV:771-950`) walks **every** `tabChannelGroupList`
row (any knob, revert rows included, even rows whose ChannelID is no
knob), takes each grouping once, and gives every member group one contact
ID from 10000 in first-seen order, with one `tabContactAddr` row per
protocol the group is used with: `(contact, 4, DMRAddr + 2^31)` and/or
`(contact, 8, P25Addr + 2^31)`. The group contact bundle is
`tabRadioGroup (99999, contact)`, `tabContacts (contact, name, 2, 2, 0,
'')`, `tabRecvVoiceCallRecordByContactID (contact, RecordFlag)`,
`tabContactOfRelayCtl (contact, 1, relay...)`, `tabContactOfBackground`,
and, only when `AlertToneID > 0`, `tabRecvToneForContacts` and
`tabContactAddrAndAlertTone` (`CONV:1160-1233`). P25 trunking talkgroups
start after the conventional groups (`CONV:929-932`), so adding N
conventional groups shifts every trunking talkgroup contact by N. Members
other than the pager also get an address row for each protocol in use
when they have one (`CONV:1285-1373`); the pager's own rows are fixed.

The all-call contact 65535 already carries a DMR address `2164260863`
(2^31 + 16777215) and a P25C address `2147549183` (2^31 + 65535) on every
conversion. A group with DMR address 16777215 or P25 address 65535 (the
template "All" group, or the importer's P25C placeholder) therefore
duplicates an all-call address under a second contact ID; PPS does not
care.

### 5.4 Encryption side effects

`convertEncrytionKeySettingByKnob` (`CONV:4381-4578`, 1.41c
`CONVC:187-370`) treats single conventional knobs of type 8 like P25T knobs
and type 4 only when `EncryptedCallFlag` is set. With
`tabEncryptionSetting.EncryptionType 3` and at least one key (both units),
a P25C knob gets `tabKMFSystemByChannel (ZK, 1)`, a
`tabCipherKeyAssignment` channel-key row (`AssignmentID = (zone-1) x 16 +
(knob-1)`), and either a per-channel `tabP25OTARFeatureControl` row plus
`tabP25OTARFeatureControlByChannel (ZK, ZK)` (1.41b, unit A) or
`tabP25OTARFeatureControlByChannel (ZK, 1)` (1.41c, unit B). A default DMR
knob gets none. Conventional knobs are processed before P25T knobs
(`CONV:4300-4345`), so the new rows land before the trunking ones.

### 5.5 Selective Call conversion differences

`tabReceivingMode` supported/current mode 1, `tabChannelAdvInfo.SignallingSquechLogic 0`,
`MonitorModeOn 0`, the announcement file of prompt 1, P25C `SquelchOption`
3 mapped to 0. Signaling, contacts and receive lists are identical.

### 5.6 Experiments (`scratch/digconv_tools/exp.py`, Knobset's converter)

| Case | Result |
|---|---|
| (g1) one DMR group (16776415) in a type-4 grouping, referenced by a `tabChannelGroupList` row whose ChannelID is no knob | converts; **11 radio tables change**, exactly `TALKGROUP_EDIT_TABLES` (`crates/g5-codeplug/src/edit.rs:245-257`): contact 10000 is the new group (`tabContactAddr (10000, 4, 2164260063)`), the 125 trunking talkgroups move from 10000..10124 to 10001..10125 |
| (k1) DMR Monitor knob on Metro knob 8 with setting, group, grouping and receive-list rows | refused: "conventional channels with protocol type 4 (only carrier squelch is verified)" |
| (k2) same for P25C | refused, type 8 |

g1 also confirms that an orphan `tabChannelGroupList` row still costs a
contact; PPS's `deleteKnob` removes the knob's `tabChannelGroupList` rows
(`ZONE/ZoneListControl.cs:1309-1384`), and Knobset must too.

## 6. Knobset's converter: the gap

Why it refuses: `convert141.rs` validates only paths exercised by the two
real pagers and turns everything else into `Error::Unsupported`
(`RS:9-11`). Neither profile has a non-CSQ conventional knob, so
`convert_single_ch_knob` refuses any protocol type other than 0
(`RS:899-901`) and `convert_recv_group_list` refuses any non-CSQ type other
than 9 (`RS:991-993`). There is no port of `convertSignaling` at all.

Already in place and matching PPS: grouping walk and group contacts for
types 4 and 8 including the 2^31 address rule (`RS:584-643,511-517`),
member address rows per protocol in use (`RS:681-691`), contact
renumbering of trunking talkgroups (`RS:726`), the receive-list helpers
with PPS's de-dup keys (`RS:760-795`), the channel bundle with
`ColorCode`/`NAC`/`TransmitMode` passed through and power saving 0 for
types 4 and 8 (`RS:797-868,922-925,957-967`), access code types 5 and 6
(`RS:185-193`), and P25C/DMR in the encryption pass (`RS:1583-1636`).

Missing, in PPS order per knob:

1. **Knob info for types 4 and 8** in `convert_single_ch_knob`: accept them
   (keep refusing 1, 2, 5, 7 and anything else); set
   `squelch_option` for type 8 with the Monitor/Selective mapping of 5.2
   (0 for every other type, as now); refuse when the knob has any
   `tabRecvSaList` row or its frequency's `RxBandWidth` is not 1 (PPS would
   emit tone rows with SigType 4/8, and fails D1).
2. **Signaling** (new step between knob info and receive list), keyed by the
   knob's protocol setting ID:
   - DMR: the profile `tabDMRSignaling` row and `tabDMRRepeaterSetting` row
     of that ID and the first `tabVoiceCallVocoderSetting` row; refuse when
     any is missing. Emit the two rows of 5.1 with the column lists PPS
     names, `RxColorCode` from the knob in both, `TxColorCode` copied from
     the knob as stored (empty string for editor-made knobs), vocoder
     values from the device row, the profile `DMRCompatibility` ignored.
   - P25C: the profile `tabP25Signaling` row (refuse when missing); emit
     `tabP25Signaling` with the column mapping of 5.2 (note the two renamed
     Tx columns) and `tabP25RepeaterSetting (ZK, RxNAC, TxNAC)` from the
     knob.
3. **Receive list for types 4 and 8** in `convert_recv_group_list`: pick the
   grouping as in 5.1 and refuse when there is none or it is empty (PPS
   fails programming here too); for P25C refuse an enabled sub-group
   binding and any `isRevert 1` row (unverified paths, like the trunking
   revert list now); for each member in `tabGroupingOfGroup` order emit
   contact-of-chan, receive-group-list and receive-behavior rows (in that
   order), with the DMR or P25C address from the contact pass, SeqNo 0 for
   DMR and PPS's counter for P25C, the knob's `tabChannelGroup` row for
   options; refuse a member without a contact. Do not emit the all-call
   contact-of-chan row.
4. **Report**: add "DMR conventional channel" / "P25 conventional channel"
   to `Report.unverified` until a pager test passes (section 9.9).
5. **Tests**: a synthetic profile per protocol asserting the exact rows of
   9.4 and 9.5; the g1 renumbering case; the refusal cases (missing
   grouping, missing signaling row, missing vocoder row, tones on a digital
   knob, bandwidth not 1, sub-group binding).

Nothing changes in `convert_contacts` or `convert_encryption`, but both
paths become live for the first time on a pager.

## 7. Risks (no Windows, no golden capture)

| # | Risk | How it shows | How to detect / fall back |
|---|---|---|---|
| 1 | Unit B's firmware lacks DMR: the handshake mode `0x00010517` has no DMR bit although model and profile say DMR | knob shows but stays silent on a busy repeater, or the pager rejects the DB | test P25C first (its bit is present); then DMR on a repeater with known steady traffic. Read the pager's live `u3db.db` back and confirm the rows survived |
| 2 | Monitor mode still gates on the talkgroup list for DMR (the help only promises it for P25C squelch) | hears calls on the placeholder only, or nothing | replace the placeholder with the repeater's real talkgroups (16 max); compare with a Selective Call knob listing the same talkgroups |
| 3 | Color code or slot wrong, or talkaround mode not hearing the repeater despite the help | silence | try `TransmitMode 0` (Repeater Mode); flip the slot; try another color code; listen on an analog-capable scanner to confirm activity |
| 4 | Vocoder mismatch (`DMRCompatibility 1` Motorola on a non-Motorola system) | carrier and maybe a call indication, garbled or no audio | engineer setting only; leave at the BSS default unless audio is garbled, then try 0 (Standard) |
| 5 | P25C NAC handling: default Monitor (3) ignores the NAC; 0xF7E means any NAC; 0xF7F is forbidden | hears neighbouring systems on the same frequency, or nothing with 0xF7F | use Normal (1) to filter one NAC; never write 0xF7F |
| 6 | P25C `SeqNo` 1..N acts as a priority order on the pager (as for trunking) | odd preemption between talkgroups | only visible with several members; start with one |
| 7 | Empty `TxColorCode` / `TxNAC` columns (the editor's values) misread as garbage | none expected for receive; worst case a rejected DB | if a DB is rejected, retry with Tx = Rx (the importer's values) |
| 8 | Encryption rows for a P25C knob on an encryption-type-3 profile | none expected (every P25T knob already has them) | none needed; compare with a P25T knob's rows |
| 9 | Contact renumbering breaks something keyed by contact ID on the pager (call history, per-talkgroup state) | trunking talkgroup names or alerts wrong after the update | check two or three trunking talkgroups by name and alert after programming |
| 10 | Duplicate addresses with the all-call contact when using the template "All" addresses | none expected | use 16776415 (the importer's DMR placeholder) if all-call traffic misbehaves |
| 11 | Band: unit B covers 136-174, 763-776 and 851-870 MHz only | UHF repeaters cannot be entered | Knobset refuses out-of-range Rx (conventional-knobs 8.2) |
| 12 | A converter bug silently produces a plausible DB | wrong behavior with no error | diff the converted DB against a hand-written expectation of 9.4/9.5 in a test; keep the previous `u3db.db` for rollback (backup before every write) |

Detection on the pager is by listening, plus two cheap checks after each
write: the knob shows its alias, and a readback of the live `u3db.db`
contains the new signaling and receive-list rows.

## 8. Corrections to earlier notes

1. conventional-knobs 6 said the grouping is required "otherwise the C#
   converter fails"; it is also enforced by the knob editor and by B3
   (section 2.3), so PPS can never produce a digital knob without one.
2. data-model 3.6 lists `DMRCompatibility` among the DMR protocol fields;
   the converters ignore that column and use `tabVoiceCallVocoderSetting`
   (1.5).
3. data-model 3.5 gives 16 members for DMR; the picker allows 16 but the
   paste helper caps DMR at 15 (2.2).
4. converter-1.41 3.2: `tabDMRRepeaterSetting` also carries the knob's Rx
   and Tx color codes, and `tabP25RepeaterSetting.TxNAC` is the knob's
   never-set `TxNAC` (`''`).
5. converter-1.41 3.4: a DMR or P25C knob gets **no** all-call
   `tabContactOfChan` row; PPS computes it and drops it (`CONV:1576`).
6. conventional-knobs 6 quoted the importer's NAC default 0xF7E without
   meaning; per the help it is "receive any NAC and any call".

## 9. Implementation spec

### 9.1 Common rules

Everything in conventional-knobs 8.1 applies: one profile transaction,
TEXT values, `''` never NULL, lowercase hyphenated GUIDs, append new rows,
update in place, `FinishDate` on save, column-name writes from the table's
own column list. Protocol settings, groups and groupings are separate
objects in PPS; Knobset may create them inside the knob transaction, but
reuses an existing one when it matches exactly (same type and, for DMR,
same slot and default parameters; same address and name for groups; same
type and member set for groupings), the way the importer reuses settings.

### 9.2 Inputs and validation (refuse otherwise)

| Input | Rule |
|---|---|
| protocol | DMR only when `tabProtocolMode.DMRC = 1`; P25C only when `P25C = 1` (refuse DMR on unit A) |
| zone, knob, alias, frequency, prompt | as conventional-knobs 8.2; bandwidth must be 1 (12.5 kHz); prompt 0 or the mode's built-in (2 Monitor, 1 Selective) |
| DMR slot | 1 or 2, stored 0 or 1 |
| DMR color code | 0..15 |
| P25C NAC | 0..0xFFF entered in hex; refuse 0xF7F; offer 0xF7E as "any NAC" |
| P25C squelch option | Monitor: 1 Normal or 3 Monitor (default 3, explain that 3 ignores the NAC); Selective: 0, 1 or 2 (default 0) |
| repeater mode | default off (`TransmitMode 1`); optional on (0) |
| talkgroups | decimal; DMR 1..16776415 or 16777200..16777215; P25C 1..65535; unique within the list; names 1..14; at most 15 (DMR) or 20 (P25C) per knob |
| capacity | fewer than 16 `tabProtocolSetting` rows when creating one; fewer than 8192 groups; fewer than 1024 groupings |
| profile state | a `tabVoiceCallVocoderSetting` row exists (DMR); the `000000000000` template signaling rows are never used |

### 9.3 Profile rows

**(a) DMR protocol setting** (skip when an existing type-4 setting has the
same slot and default parameters):

1. `tabProtocolSetting (ID = new GUID, ProtocolNo = smallest unused integer,
   ProtocolName = "Protocol N" smallest unused, or a caller name up to 14,
   ProtocolType '4')`.
2. `tabDMRSignaling (ID, TXPreFrames '4', RCRxEnable '0', NoSigActTimeout
   '600', ACKWaitTimeout '5000', MFID '0', SAP4NRS '4', SAP4RS '5',
   DMRCompatibility '2')`.
3. `tabDMRRepeaterSetting (ID, TimeSlot '0' or '1', TimingAlign '0')`.

**(b) P25C protocol setting** (skip when any type-8 setting exists with
default parameters):

1. `tabProtocolSetting (ID, ProtocolNo, ProtocolName, ProtocolType '8')`.
2. `tabP25Signaling (ID, TXPreTime '20', NoSigActTimeout '600',
   ACKWaitTimeout '5000', MFID '0', AlgId '133',
   MinShortDigitalToneDuration '200', MinLongDigitalToneDuration '2500',
   MinToneAddrGap '1000', OTAToneFlag '1', ShortDigitalToneATxDuration
   '1000', ShortDigitalToneBTxDuration '2000', LongDigitalToneTxDuration
   '5000', TxToneAddrGap '1500')`.

**Talkgroup rows** (shared by c, d, e). For each talkgroup not already
present with the same address and name, append to `tabGroup_New` the
editor's row: `GroupID` new GUID, `GroupName`, `GroupNo` next integer,
`Note ''`, `RecordFlag '1'`, `Type '0'`, `Relay '0'`, `Priority '0'`,
`DutyStatus '0'`, `AlertMode '1'`, `AlertToneID '0'` (none; PPS's editor
default 9999 adds a tone before every call and two more contact rows),
`BackgroundColor '0'`, `EncryptionKeyID` = the first
`tabEncryptionKeyList` ID, `KeyStrapOption '2'`, `DMRAddr` or `P25Addr` =
the decimal address, every other address column `''`. Then one
`tabGroupGrouping (GroupingID new GUID, GroupingAlias "TGID List N"
smallest unused, Type '4' or '8', Note '')` and one `tabGroupingOfGroup
(GroupingID, GroupID)` per talkgroup in list order. Reuse an existing
grouping only when its type and member list are identical.

**Monitor placeholder.** When the user gives no talkgroup for a Monitor
knob, use one group named `All` with DMR address 16777215 or P25C address
65535 (the template's all-call group; a single row may carry both
addresses, as the template's does), in a one-member grouping. Fallback for
DMR if risk 10 shows: 16776415, the importer's placeholder.

**(c) DMR Monitor knob**:

1. Frequency, receive row (`CurrentMode '33'`), as conventional-knobs 8.3
   steps 1 and 2.
2. `tabSingleConvCHKnob`: the conventional-knobs 1.2 default row with
   `ProtocolSettingID` = the DMR setting, `RxColorCode` = color code,
   `RxNAC '0'`, `SquelchOption '3'`, `TransmitMode '1'` (or `'0'`),
   `TxColorCode ''`, `TxNAC ''`.
3. `tabChannelGroupList (ChannelID, GroupingID, '0')`.
4. `tabChannelGroup`, one row per member, appended last member first:
   `(ChannelID, GroupID, SigType '4', SeqNo = member position 1..N,
   OffDuty '0', Priority '0', OutOfRange '0', OTA '0', all nine ack columns
   '0')`.
5. No `tabRecvSaList`, `tabP25TSubGroupEnable`, `tabP25TCallAlertEnable`,
   `tabChannelRevertGroup` or sub-group rows. `tabOrganization_New.FinishDate`.

**(d) P25C Monitor knob**: as (c) with the P25C setting, `RxColorCode '0'`,
`RxNAC` = the NAC as decimal, `SquelchOption '3'` or `'1'`, `SigType '8'`
in `tabChannelGroup`.

**(e) Selective Call variants**: as (c)/(d) with `CurrentMode '1'`,
`AlertToneID '0'` or `'1'`, `SquelchOption '0'` for DMR and `'0'`, `'1'`
or `'2'` for P25C, `RecordFlag` 0 or 1 (default 1); `PushToListen '0'`,
`Privacy '0'`. The talkgroups are the user's list (at least one, no
placeholder).

**Deleting such a knob** removes its `tabSingleConvCHKnob`,
`tabChannelGroupList` and `tabChannelGroup` rows like `deleteKnob`;
groups, groupings and protocol settings stay (PPS keeps them too), but an
unused grouping costs nothing only if no `tabChannelGroupList` row
references it (5.6).

### 9.4 Expected radio rows, DMR Monitor (unit B, Metro knob 8, ZK 108)

With one new group (contact 10000), slot 2, color code `cc`, frequency F,
alias A, prompt 0, all other defaults:

- the 17 analog channel tables of conventional-knobs 5.1 with ZK 108,
  except `tabChannelAdvInfo (108, 0, 1, 1, 0, 1, 51, 4, 1, 0, 2, 500, 0,
  cc, 0, 0, 0, 0, 670, 1, 19, 1, 0, 0, 0)`, `tabRepeaterSettingOfChan
  (108, 4, 5, 108, 0, 670, 1, 19, 1, 0, 0, 670, 19, 0)` and the receive
  triple below;
- `tabDMRSignaling (108, 4, cc, 0, 600, 5000, 0, 4, 5, 1, 1)` (columns as
  in 5.1);
- `tabDMRRepeaterSetting (SACID 108, TimeSlot 1, TimingAlign 0,
  RxColorCode cc, TxColorCode '')`;
- `tabContactOfChan (108, 10000, 4)`, `tabRecvGroupList (108, 4, 108,
  2164260863, 10000, 0)` for address 16777215, `tabRecvBehaviorByContactID
  (108, 10000, 0, 1, 0, 1, 4, 0 x 9)`;
- group contact 10000 with `tabContactAddr (10000, 4, 2164260863)`, and
  every trunking talkgroup contact shifted by one (experiment g1);
- `tabZone` only if the zone had no active knob.

### 9.5 Expected radio rows, P25C Monitor (same position)

As 9.4 with `tabChannelAdvInfo` `ColorCode 0`, `NAC` = nac, `SquelchOption
3` (or 1); `tabRepeaterSettingOfChan (108, 8, 6, ...)`; `tabP25Signaling
(108, 20, 600, 5000, nac, 0, 133, 200, 2500, 1000, 0, 1000, 2000, 5000,
1500)`; `tabP25RepeaterSetting (108, nac, '')`; receive triple with
`SigType 8`, address `2147549183` for 65535 and `SeqNo 1`; contact address
`(10000, 8, 2147549183)`; on unit B (1.41c, encryption type 3)
`tabKMFSystemByChannel (108, 1)`, `tabP25OTARFeatureControlByChannel (108,
1)` and `tabCipherKeyAssignment (108, 'Key 1', CKR, KeyID, 1, 7, 2, 0)`.

### 9.6 Converter additions

Section 6, items 1 to 5. The rows of 9.4 and 9.5 are the acceptance test.

### 9.7 Write guard

Profile tables allowed to change, per edit:

| Edit | Tables |
|---|---|
| DMR protocol setting | `tabProtocolSetting`, `tabDMRSignaling`, `tabDMRRepeaterSetting` (one row each, appended) |
| P25C protocol setting | `tabProtocolSetting`, `tabP25Signaling` |
| talkgroups | `tabGroup_New` (appended), `tabGroupGrouping` (one appended), `tabGroupingOfGroup` (appended) |
| digital knob | `tabReceivingMode_NewG4` (one updated), `tabSingleConvCHKnob` (one appended), `tabConvFreqList` (only when a frequency is created), `tabChannelGroupList` (one appended), `tabChannelGroup` (appended), `tabOrganization_New` |

Never: `tabRecvSaList`, `tabVoiceCallVocoderSetting`, the
`000000000000` signaling rows, `tabGroupAddress`, `tabGroupMembers_New`,
sub-group, revert and call-alert tables.

Radio tables, as new constants next to `CONV_KNOB_CHANNEL_TABLES`
(`crates/g5-codeplug/src/edit.rs:325-345`):

```text
DMR_KNOB_CHANNEL_TABLES = CONV_KNOB_CHANNEL_TABLES without tabRecvSaList
  + tabDMRSignaling, tabDMRRepeaterSetting
P25C_KNOB_CHANNEL_TABLES = CONV_KNOB_CHANNEL_TABLES without tabRecvSaList
  + tabP25Signaling, tabP25RepeaterSetting
  + tabKMFSystemByChannel, tabCipherKeyAssignment,
    tabP25OTARFeatureControlByChannel, tabP25OTARFeatureControl
    (only when tabEncryptionSetting.EncryptionType is 3 and keys exist;
    tabP25OTARFeatureControl only on 1.41b)
both, whenever the edit adds or removes a conventional group contact:
  + TALKGROUP_EDIT_TABLES (contact renumbering, experiment g1)
```

`tabZone` stays conditional as for analog. Any change to `tabP25TSetting`,
`tabP25Trunking*`, `tabSystemAlias`, `tabRecvSaList`, `tabRevertListGroup`,
`tabSubgroupWithMultiSubAddr`, `tabP25TQCDelayRecordParm` or the other
protocols' signaling tables means the edit did something it should not.
Because renumbering moves every trunking contact, the guard should compare
contact tables by content modulo the ID shift, or the edit should be
checked with a test that converts before and after and maps old to new
contact IDs by (name, signaling type, address).

### 9.8 PPS behavior Knobset should skip

| PPS behavior | Why skip |
|---|---|
| knob may reference a deleted protocol setting (`CheckIfCanDelete` unused) | blocks programming (B2); refuse the delete |
| duplicate protocol or grouping aliases (dead checks) | confusing lists; enforce uniqueness |
| digital knob with a missing signaling row converts anyway | channel with no signaling; refuse |
| P25C Monitor default ignores the NAC silently | surface the choice (Monitor vs Normal) |
| editor default group tone 9999 | every call starts with a beep and adds contact rows; default to none |
| importer's legacy `tabGroupAddress` and `tabGroupMembers_New` rows, Tx = Rx color code/NAC | the editor path writes none of them; unread by the 1.41 converter |
| one new grouping per knob (importer) | reuse identical groupings |
| `DMRCompatibility 2` in the protocol setting presented as a setting | dead column; the device row decides |

### 9.9 On-pager test plan

1. P25C first (both units support it): a local P25 conventional frequency
   with known traffic, Monitor, squelch option 3, placeholder `All`. Expect
   audio from any talkgroup. Then option 1 with the right NAC (still
   audio) and with a wrong NAC (silence).
2. Selective Call P25C with one real talkgroup: audio only for that
   talkgroup.
3. DMR on unit B: the local repeater, Monitor, correct slot and color code,
   placeholder `All`. If silent: repeater mode on; then real talkgroups
   (risk 2); then the other slot; then vocoder compatibility Standard
   (engineer row). If still silent, treat DMR as unsupported by the
   firmware (risk 1) and keep it out of the UI.
4. After every write: alias on the knob, trunking talkgroups still named
   and alerting correctly (renumbering), live `u3db.db` readback contains
   the new rows.
5. Not a pager test: open a Knobset-made profile in PPS on Windows when one
   becomes available, save the knob, and confirm PPS rewrites the same
   rows.
