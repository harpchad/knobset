# G5 pager programmer: reverse-engineering notes

Source: Unication G Series PPS V00-04-00-48 and the Unication RNDIS driver
package, decompiled with ILSpy (`../decomp/`), extracted with innoextract and
msiextract (`../extract/`), with REA (`rea-agents`) for managed/native
boundary evidence.

| File | Covers |
|---|---|
| [protocol.md](protocol.md) | USB/RNDIS link, TCP port, packet framing, CRC, opcodes, exchange state machines |
| [operations.md](operations.md) | Handshake fields, access password, file ops payloads, read and write flows, safety checks, native DLLs |
| [knob-editing.md](knob-editing.md) | Spec for removing knobs, editing conventional knobs, TG Scan/Monitor switch, knob type conversion |
| [digital-conventional.md](digital-conventional.md) | Spec for P25 conventional and DMR channels: protocol settings, groups, converter additions |
| [zones-and-scan.md](zones-and-scan.md) | Spec for zone create/rename/delete/reorder and scan knobs (rows, renumbering, write guard) |
| [knob-copy-move.md](knob-copy-move.md) | Spec for copying a knob to an empty position, moving and swapping knobs (rows, renumbering, conflicts) |
| [pager-settings.md](pager-settings.md) | Spec for the Pager settings page: talkgroup hold and remove, display format, alerts, backlight, voice storage, pager menu items (profile columns, PPS ranges, converter paths, write guard) |
| [conventional-knobs.md](conventional-knobs.md) | Spec for putting analog conventional channels on knobs (rows, squelch encodings, write guard) |
| [data-model.md](data-model.md) | SQLite 2 formats, PC profile vs pager `u3db.db`, converter, full schema, models, versioning |
| [schema/](schema/) | DDL dumps of every shipped template DB (Unication's column comments removed; a vendor phone-number default replaced with zeros) |
| [limits-and-misc.md](limits-and-misc.md) | Capacity limits, ID/frequency ranges, validation, login, audio, telemetry, file formats |
| [radioreference.md](radioreference.md) | OEM RadioReference SOAP client, mapping, bugs, implications |
| [radioreference-import.md](radioreference-import.md) | Spec for Knobset's RadioReference CSV/paste import |
| [ui-inventory.md](ui-inventory.md) | OEM screens, receive modes, hobbyist workflows, UX pain points |
| [converter-1.41.md](converter-1.41.md) | DBV1.41a-e converter (what the test pagers run), validated row-for-row against real uploads including a PPS-written 1.41e |
| [system-editing.md](system-editing.md) | Editing an existing P25 system: PPS save mechanics, row order, validation, knob references, index sync, implementation spec and write-guard allowlist |
| [system-creation.md](system-creation.md) | Creating and deleting P25 systems and assigning them to knobs: PPS row writes, receive modes, knob defaults, conversion experiments, implementation spec and write-guard allowlists |
| [firmware-update.md](firmware-update.md) | Unication's V2.05 firmware package and updater: contents, DB migrations, how it reaches the pager, a real update run from a Windows ARM VM |
| [desktop-platforms.md](desktop-platforms.md) | Linux and Windows builds: CI, the pager's USB link per OS (Unication's RNDIS INF), file locations, signing, what's untested |
| [cloning.md](cloning.md) | Writing one pager's backup to another pager of the same model (per-pager identity, PPS behavior, compatibility, exact guard) and copying P25 systems between backups |
| [talkgroup-settings.md](talkgroup-settings.md) | Per-talkgroup alert mode, alert tone, voice storage and background colour: PPS values and defaults, converter mapping, radio tables per field, bulk edit spec |
| [architecture-proposal.md](architecture-proposal.md) | Proposed stack, layout, safety rails, UX direction, milestones, open decisions |

Tools in `../scratch/`: `sqlite2/sqlite2` (SQLite 2.8.17 build),
`dump_sqlite2.sh` (SQLite 2 to SQL text or SQLite 3; **lossy**: turns numeric-looking text such as `'0000000000000000'` into numbers, use `conv141/tools/load2.py` or the Rust `sqlite2` crate instead), `chm/` (extracted
manual), `tools/resx_fields.py`.
