# Programming another pager from a backup, and copying systems between profiles

Spec for two features: writing one pager's backup to a different pager of
the same model ("clone"), and copying a P25 system from any backup into the
open profile (PPS's "Add System from Resources"). Section 8 covers zone
copying, which isn't built. PPS paths are relative to `decomp/`.

## 0. Summary

- A Knobset write sends exactly five files, all PPS profile files
  (section 1). Nothing else on the pager is ever written, so passwords,
  keys and battery data can't be copied between pagers by construction.
- PPS has no notion of per-pager identity when it programs a profile: the
  same profile file goes to every pager, radio number and unit IDs
  included (section 3). All three test pagers carry the same pager unit
  ID because PPS programmed them from one profile lineage.
- Knobset does better: the target keeps its own radio number, alias, unit
  IDs, access passwords and PIN lock; everything else comes from the
  source (section 2).
- Clone only between pagers of the same model (first 21 characters of the
  model number, as PPS compares). Different radio DB versions are fine: the
  radio DB is generated for the target's version. Anything else is refused
  with a message naming the difference (section 4).
- The guard is exact: the result is the source profile with the target's
  identity restored, table for table, and its radio DB differs from the
  source's only in the tables the identity reaches (section 6).
- Copying a system copies the source's system DB file under a new ID, as
  PPS does, and then goes through the normal new-system write path and
  guard (section 7).

## 1. Files

### 1.1 What a write sends

`g5_core::write::PROFILE_FILES`, the same five PPS sends in this order
(`View/View.ProgramVersion/ProgramController.cs:441-466`):

| # | Pager path | Content |
|---|---|---|
| 1 | `/defpara/Applications/PPS/u3db_PC.db` | the profile DB (PPS schema) |
| 2 | `/defpara/Applications/PPS/u3db.db` | radio DB generated from the profile |
| 3 | `/defpara/Applications/PPS/u3db.db.md5` | MD5 of #2, uppercase hex |
| 4 | `/defpara/Applications/PPS/u3db_PC.zip` | zip of `<GUID>.db` and `<GUID>/` (system DBs, tone and prompt catalogs) |
| 5 | `/defpara/Applications/PPS/u3db_PC.zip.md5` | MD5 of #4 |

PPS also sends custom tone `.dat` files and `customer.png` (`:467-486`).
Knobset sends neither, and its converter refuses profiles with custom tones
or channel announcements (they're "unverified"), so a clone can't leave the
target referring to audio files it doesn't have.

### 1.2 What else lives on the pager

From every backup (`g5 backup` reads both profile directories):

| Path | What it is | Pager-specific? |
|---|---|---|
| `/root/Applications/u3db/u3db.db` | the live radio DB; the pager copies the uploaded one here and then changes it as settings change on the keypad | the pager's own state |
| `/root/Applications/u3db/battery.db`, `voicepager_battery.db` | charge and discharge measurement tables (`tabBatteryChargeMeasure`, `tabBatteryDischargeMeasure`, 66 to 1818 rows) | yes: calibration and history |
| `/root/Applications/u3db/business.db` | `tabTxPowerSettingOption` (3 rows) | factory data |
| `/root/Applications/u3db/morse.db` | `tabMorseAlphabet` (36 rows) | factory data |
| `/root/Applications/u3db/keytab` | 12 KB binary key table, written only by `ProgrammingKey` (no callers, notes/operations.md 3) | yes: key material |
| `/root/Applications/u3db/SuperPassword.conf`, `/defpara/Applications/PPS/SuperPassword.conf` | MD5 of the super password (notes/operations.md 2) | yes: credentials |
| `/defpara/Applications/PPS/Readme` | "for PPS download u3db.db" | no |

None of these is in the write set, and `program` has no way to send a file
outside `PROFILE_FILES`. Keep it that way: a clone must never copy
`SuperPassword.conf`, `keytab` or the battery DBs, and no future feature
should either. The on-pager settings in the live radio DB (backlight and the
like) are overwritten by any profile write, a clone included; that's PPS's
behavior too.

## 2. What in the profile is per-pager

| Where | Field | Reaches the pager as | Clone takes |
|---|---|---|---|
| `tabMember_New`, row `RadioNo = 99999` (the pager itself) | `UserDeviceID` ("User-001", the handshake's radio number) | `tabRadioBaseSetting.RadioNo` (converter-1.41.md 2.6) | target's |
| same row | `MemberName` (the pager's alias in its own contacts) | `tabContacts.name` of contact 99999 | target's |
| same row | `TwoToneAddr`, `FiveToneAddr`, `MDCAddr`, `UDCAddr`, `DMRAddr`, `P25Addr` (its unit IDs on conventional protocols) | `tabContactAddr`, `tabContactAddrAndAlertTone` of contact 99999 | target's |
| same row | `AlertMode`, `BackgroundColor`, `AlertToneID` (how calls to this pager alert) | `tabRecvToneForContacts`, `tabContactOfBackground` | source's (settings, not identity) |
| `tabPagerSUID.P25TAddr` | the pager's P25 trunking unit ID | `tabContactAddr (99999, 9, ...)` | target's |
| `tabAccessPassword` | read/program password enable flags and MD5 | `tabAccessPassword` | target's |
| `tabPINPasswordSetting` | boot/keypad lock and PIN hashes | `tabPINPasswordSetting` | target's |
| `tabGSMBaseSettingPPS` | GSM PIN, IMEI, IMSI, pager ID (GSM models only) | `tabGSMBaseSetting` | target's (whole table) |
| `tabDeviceMember` | `RadioNo`, `SerialNo` device list (empty on every test pager; only converters read it) | nothing | target's |
| `tabOrganization_New.OrgName` | profile name ("User Group") | `tabRadioBaseSetting.UserCompany` | source's |
| `tabP25TrunkingSecurity.SUID`, `tabP25TrunkingSystem.SUID` | a unit ID typed per system | nothing: the converter ignores it (converter-1.41.md 2.4) | source's |
| system DB `tabSUID` | MSN to SUID map, keyed by serial (empty everywhere) | nothing | source's (it's keyed per pager by design) |

The pager member row is what PPS's Pager ID page edits: alias, alert
settings, the per-protocol addresses, and `tabPagerSUID` kept in step with
it (`View/View.GroupSetting/PagerID.cs:130-185` loads, `:230-251` saves).
`UserDeviceID` has no editor in PPS at all; only the schema migrations and
converters touch it (`View/View.UpdateVersion/UpdateVersion4P0_4P1.cs:59-76`,
`View/View.ProgramVersion/PPSU4ParamConverter_01P41b.cs:197-202`). Access
passwords come from `View/View.UserGroupInformationSetting/DevicePasswordSettingControl.cs`.

The programming log isn't on the pager. PPS keeps it in a PC-local DB
(`tabOrganizationProgramLog`, `tabSingleGroupProgramLog`, and a
`tabCloneLog` with `FromMSN`/`ToMSN`;
`View/View.ProgramLog/ProgramLogDataRepository.cs:12-90`), and nothing in
this build calls the writers (`ProgramLogDBHandler.cs:11-29` have no
callers). Knobset's equivalent is the backups folder.

Identity tables are copied from the target's own profile verbatim, so every
row is one PPS (or a Knobset write of a PPS row) wrote; the pager row's
columns are updated in place, keeping the source's `MemberID`.

## 3. How PPS programs one profile into several pagers

`ProgramController.Program` (`:61-169`) compares only the model:

1. The device model (first 21 characters, `:79-80`) must parse and be an
   enabled model (`:83-93`); its U3DB version must be supported (`:94-98`).
2. If the profile's model (first 21) differs from the device's (`:107-153`):
   the product code must match (`:111-116`), both band codes must match
   (`:117-122`), tow-truck software can't cross either way (`:123-137`), and
   a hardware downgrade to B/D or protocols the device lacks bring up a
   confirmation listing what's stripped (`:138-152`; the protocol diff is
   `getRemoveProtoclList`, `:189-203`, a bitmask subtraction of the two
   `tabProtocolCode` values).
3. `ProgramData` (`:205-260`) copies the profile, and `ChangeDB`
   (`:262-393`) rewrites it only when the models differ: new `tabModel`, the
   device model's `MFG_G4A.db`/`BSS_G4A.db` tables, frequencies clamped to
   the device's bands, rows of stripped protocols deleted.
4. Conversion and upload are the same for every pager.

Nothing checks or rewrites the serial, radio number, unit IDs or passwords.
Programming one profile into ten pagers gives ten pagers with the same
radio number, the same P25 unit ID and the same access password. Our three
backups show exactly that: same profile GUID on B and C, same pager unit ID
on all three. For a receive-only scanner the shared unit ID mostly doesn't
matter, but it's wrong for anything that addresses a pager individually
(call alerts, DMR or P25 private calls), and copying another pager's
password settings can lock its owner out. Hence section 2.

## 4. Compatibility rules (Knobset)

`g5_core::clone::check_clone_compatible(source, target)` on the two pagers'
`PagerId`s (model, serial, radio DB version from the handshake or the
backup's MANIFEST), plus profile checks in `clone_set`:

1. Different serials. Writing a backup to its own pager is a restore or an
   edit, which have their own paths.
2. Same model, first 21 characters (series, hardware, protocol code,
   software code, bands, language, certification, safety, revision). On a
   mismatch the message names each differing part, e.g. pager A to B:
   "hardware D vs B, protocols 66 vs 67, software BE vs BF, revision B702UNI
   vs 1401UNI". This is stricter than PPS, which would rewrite the model
   tables (section 3, step 3). Knobset has never validated `ChangeDB`'s
   output against PPS, so it refuses rather than guesses; section 9 says
   what supporting it would take.
3. Both radio DB versions supported (DBV1.41b to DBV1.41e). If they differ,
   the radio DB is generated for the target's (`Variant` of the target).
4. Each profile's `tabModel` matches its own pager's model, so neither
   backup holds a profile made for another model.
5. Every protocol the source's `tabProtocolMode` enables is enabled in the
   target's (`clone::missing_protocols`), e.g. a DMR profile onto a model
   without DMR is refused. With rule 2 this can only trip on a hand-edited
   profile; it's cheap insurance.
6. Every system's `BandTypeCode` is made of the target model's band codes.
7. Both backups' MD5 sidecars verify, and the source's own radio DB
   regenerates exactly from its profile (otherwise the converter doesn't
   reproduce what that pager runs, and nothing below proves anything).

## 5. Flow

`clone_set(source, target_backup, templates)` builds the write set offline
against the target's newest backup; `program_clone` writes it:

1. Connect; `check_clone_compatible(source, live pager)` again with the
   handshake's values.
2. `program_on` with the target backup as the source of truth, unchanged:
   the attached pager must match the target backup (model, serial, DB
   version); read access; **safety backup of the target**; refuse if the
   target's profile changed since the target backup (`require_unchanged_pager`);
   write access; one write business; read-back verify of all five files;
   post-write backup.

Every safety property in `write.rs`'s module docs holds because the clone
reuses `program_on`. The only new code on the pager path is the extra
compatibility check.

Building the set:

1. Load the source profile twice (one stays pristine) and the target's.
2. `keep_identity`: copy the target's identity tables and pager-row columns
   into the working copy (section 2).
3. `check_clone` (section 6); convert for the target's variant; refuse
   unverified features; radio guard (section 6).
4. Files: if no identity field differs and the versions match, the source's
   five files go out byte for byte. Otherwise the new profile DB, its zip
   (the source zip repacked with the new root DB), the new radio DB and
   fresh MD5 sidecars.

The UI shows source and target side by side (model, serial, DB version,
profile name), the identity fields the target keeps with a marker where the
source differs, and before/after counts of zones, knobs, systems,
talkgroups and conventional frequencies with zones and systems added or
removed (by name). The write needs an explicit checkbox ("Replace the whole
profile on pager X with the one from Y") and a destructive button naming
the target. The CLI is `g5 clone <source> --target <target backup>`; dry run
by default, `--yes` writes.

## 6. The clone's guard

Instead of an allowed-table list, the result must equal a precise
expectation:

- Profile (`clone::check_clone(result, source, target)`): every table is
  identical to the source's, except that the identity tables equal the
  target's, and `tabMember_New` has the source's rows with the target's
  values in the identity columns of the pager row. Any other difference, an
  identity table matching neither side, or an identity field still carrying
  the source's value, is refused.
- Radio DB: compared with the source's radio DB (same version) or with the
  source profile converted for the target's version. Only
  `IDENTITY_RADIO_TABLES` may differ: `tabRadioBaseSetting`, `tabContacts`,
  `tabContactAddr`, `tabContactAddrAndAlertTone`, `tabAccessPassword`,
  `tabPINPasswordSetting`, `tabGSMBaseSetting`. If no identity field
  differs, nothing may differ.
- System DBs travel unchanged (the zip is repacked from the source's).

With a changed radio number, alias, conventional address, pager unit ID
and access password (test `clone_keeps_a_different_target_identity`), the
profile differs from the source's in exactly `tabAccessPassword`,
`tabMember_New` and `tabPagerSUID`, and the radio DB in exactly
`tabAccessPassword`, `tabContactAddr`, `tabContactAddrAndAlertTone`,
`tabContacts` and `tabRadioBaseSetting`.

Against the target's current radio DB a clone changes whatever the two
profiles differ in; B onto C changes 37 tables (C holds a 2024 profile, B
the current one). That list is shown, not guarded.

## 7. Copying a P25 system from another backup

PPS's "Add System from Resources" (`View/View.P25TSystem/P25TSyetemTotal.cs:701-716`)
lists library systems whose `BandTypeCode` is in the profile's bands
(`:509-522`) and `addBySelect` (`:628-668`) adds each one:

- new GUID for `P25TrunkingID` and `DBFileName`; `SystemSource` becomes
  `resource_<old ID>`; the name is renamed to the next free "Trs Sys N" if
  taken (`:641-648`);
- the index row is otherwise the source's row as is (`ItemArray` copy,
  `:637-638`), counts and `SUID` included;
- the system DB file is copied and `tabP25TrunkingSystem` gets the new
  `ID`, `SystemSource` and `Name` (`:652-653`); everything else in the file
  (sites, control channels, site links, talkgroups with every setting,
  sub-groups, individuals, signaling, edit permissions) stays;
- a library system with no band gets the profile's band and has its
  frequencies remapped (`:654-658`).

Knobset (`system_edit::copy_system`, edit kind `Edits.copied_systems`) does
the same from any backup, with these choices:

- The source is a backup's profile, not PPS's library. Backup systems
  always carry a band, so a system without one is refused instead of
  remapped.
- The band must be made of the open profile's model band codes (a two-band
  `SXVX` system needs both), else "<name> is a VX system; this pager's
  bands are SX, UC".
- The caller picks the name (the UI appends " 2", " 3" when taken, within
  14 characters) instead of "Trs Sys N".
- After the file copy and index row, `apply_system_edit` applies the
  caller's state, so the copy can be edited before writing like any new
  system. An untouched copy is rewritten verbatim: the test checks every
  system DB table equals the source's except the three PPS changes, and
  every index column likewise.
- Talkgroup GroupIDs stay the source's, as in PPS. Knob lists resolve
  GroupIDs within the knob's system, so two systems sharing GroupIDs is
  fine (every system already shares the "All" group's ID).
- On any failure the file and index row are removed.

Write guard: a copy is a created system (`SystemChange.created`), so it's
the new-system rule: nothing changes on the radio until a knob uses it;
with a knob, the usual knob/system tables. Conflict check: a copied system
can't be deleted in the same batch.

The UI's "Copy from a backup…" (P25 systems page) lists the newest backup
of each pager (or all backups), its systems with band, sizes and a note
when a system with the same WACN and System ID is already in the profile,
and disables systems whose band the pager lacks. The copy is an ordinary
pending change.

## 8. Copying a zone (not built)

A zone copy from another backup needs, per knob kind:

- Zone: a new `tabZone_New` row (new ID, free name, appended), like
  `zone_edit::create_zone`.
- Every knob: new `ChannelID`s for `tabReceivingMode_NewG4` and the knob
  table, with names, prompts and alert settings from the source.
- Trunked knobs: the system must exist in the target. Either copy it with
  `copy_system` in the same batch, or map to an existing system that's
  provably identical (same WACN, System ID, sites and control channels, and
  every GroupID on the knob present with the same TGID). Then copy the knob
  row (`tabP25TrunkingSystemKnob`, its `ProtocolSettingID` mapped to a P25T
  protocol setting in the target) and its lists (`tabChannelGroup`,
  `tabChannelRevertGroup`, call-alert tables).
- Conventional knobs: new `tabConvFreqList` rows (frequencies, bandwidth,
  squelch), `tabSingleConvCHKnob`/`tabAdvancedConvCHKnob`, and their
  protocol settings (`tabProtocolSetting` plus `tabP25Signaling` or
  `tabDMRSignaling`), deduplicated against the target's; DMR and P25
  conventional groups from `tabGroup_New`/`tabGroupGrouping`.
- Scan knobs: member ChannelIDs remapped to the copied knobs; members
  outside the zone either copied too or dropped with a warning.
- Guard: the union of `ZONE_TABLES`, the knob-kind tables and, for copied
  systems, the system tables, plus a check that every copied knob converts
  to the same radio rows as in the source (modulo channel and contact IDs).

That's most of the machinery of the knob copy/move work on another branch,
across two profiles. Build it on top of that once it merges.

## 9. Not supported, and what it would take

- Cloning across models (pager A, G5D66BE, to B, G5B67BF). Needs `ChangeDB`:
  rewrite `tabModel`, replace the MFG/BSS-derived tables
  (`tabMfgProductCode`, `tabProtocolMode`, `tabFunctionList`,
  `tabMemberFunctionList` intersection, `tabCustomer`,
  `tabProductAppearance`, `tabPowerLevel`, `tabTowTruckSetting`,
  `tabVoiceCallVocoderSetting`) from `DataBase/<model>/`, clamp frequencies,
  strip missing protocols with PPS's delete list (`:302-391`). Then validate
  against a PPS cross-model program, which needs Windows.
- Radio DB versions outside 1.41b to 1.41e.

## 10. Tests

`crates/g5-core/tests/clone.rs` (discovers the newest backup of each pager
in `G5_BACKUPS`; no serials in code):

- `compatibility_rules`: the pure rules on made-up `PagerId`s.
- `clone_onto_another_pager_of_the_same_model`: every same-model pair (B
  onto C and C onto B): identity is the target's, counts match, sidecars
  match, the attached-pager check passes for the target and fails for the
  source, and with equal identities the files are the source's byte for
  byte.
- `clone_keeps_a_different_target_identity`: the target's copy gets a
  different radio number, alias, P25 conventional address, unit ID and
  write password; exact profile and radio table lists as in section 6; no
  password hash in the summary.
- `clone_regenerates_for_a_target_on_another_radio_db_version`: a target
  reporting 1.41b gets the source profile converted for 1.41b.
- `clone_between_models_is_refused`: A onto B and back.
- `clone_refuses_a_target_backup_with_a_broken_sidecar`.

`crates/g5-codeplug/tests/clone.rs`: the guard refuses a renamed zone, a
changed alert setting on the pager row, an identity table matching neither
side, a source value left in an identity column, a dropped password row,
and an untouched source.

`crates/g5-codeplug/tests/system_copy.rs` and `crates/g5-core/tests/system_copy.rs`:
row-for-row copy, refusals (duplicate ID, taken name, missing band), no
leftovers on failure, no radio change until a knob uses the copy, and the
guard with the copy on a new knob.

## 11. Pager test plan

Lowest risk first. Every write takes a safety backup; restore it from the
Backups page to undo.

1. Copy a system: open B's newest backup, P25 systems, Copy from a backup…,
   pick a system from A's backup, write without putting it on a knob.
   Expect: nothing changes on the pager; the next backup's profile has the
   system.
2. Same, then put the copied system on an empty knob with two or three
   talkgroups. Expect: the knob receives, with the talkgroups' alert
   settings from A.
3. Clone dry run: `g5 clone <B newest> --target <C newest>`; read the
   plan (no pager needed).
4. Clone B onto C: back up C first, then from B's backup choose Write to
   another pager…, target C, tick the box, write. Expect: C shows B's
   zones and knobs, its radio number stays User-001, and a backup of C
   afterwards holds B's profile files byte for byte (identities are equal
   today).
5. On C, check two or three trunked knobs receive and that C still accepts
   writes (rename a knob from C's newest backup and write).
6. Optional: restore C's safety backup from step 4 to put C back.
