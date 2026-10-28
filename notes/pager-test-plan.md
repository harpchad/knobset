# Pager test plan for the 0.5 features (2026-10-19)

Pager B (G5B67BF, DBV1.41c), Knobset test build from branch `test/overnight`
(the 0.5 features combined). One kind of write per step, lowest risk first.
Knobset takes a safety backup before every write and verifies every file
afterwards; to undo a step, restore its safety backup from the Backups page.

Before starting: take a fresh backup in Knobset and open it. Every step below
starts from the newest backup (Knobset refuses to edit an older one).

After every write, check three things: the knob shows its new name, the
pager behaves as the step expects, and two or three trunked knobs still
receive (several features renumber radio IDs).

| # | Feature (PR) | What to do | Expect | Notes |
|---|---|---|---|---|
| 1 | Analog conventional knob | Lake knob 7: Add conventional knob, NOAA 162.550 MHz, 25 kHz, carrier squelch, prompt off | Knob "NOAA Local" (or your name) plays the weather broadcast | NOAA channels are 25 kHz; local fire is usually 12.5 |
| 2 | Conventional edit | Same knob: switch squelch to CTCSS on a local fire channel with a known PL, then to a DCS one if you know one | Opens only with the tone | DCS has never been on a pager; pick a code other than 023 |
| 3 | New zone | "+ Zone", name it, add one conventional knob in it | Zone appears on the pager with the knob | Also check the zone list order didn't change |
| 4 | Scan knob | Empty position: Add scan knob, Normal Scan of two analog knobs (one in another zone) | Pager scans both | Cross-zone lists are untested |
| 5 | TG Scan / TG Monitor switch | A TG Scan trunked knob: Mode, TG Monitor; write; then back | Monitor plays every talkgroup on the system, Scan only the list | The round trip should give back the original radio DB |
| 6 | RadioReference import | Import a large local system as a new system (CSV or paste), put it on an empty knob (TG Scan, a few talkgroups) | Knob receives the system | Largest system Knobset has written (hundreds of talkgroups) |
| 7 | Remove a knob | Remove a test knob from steps 1 to 4 | Position is empty on the pager | Watch what an emptied position shows if the pager was on it |
| 8 | Convert a knob | Remove a conventional knob and put a trunked knob on the same position in one write | Position becomes a trunked knob | Either direction; neither has been on a pager |
| 9 | Zone reorder | Move a zone left (with no other pending changes) | Pager shows the new order and starts on the first zone | Renumbers every channel; check talkgroup hold/lockout and recordings afterwards |
| 10 | P25 conventional, experimental | A local P25 conventional frequency: Type P25 conventional, NAC F7E (any), no talkgroups | Audio from any talkgroup | If heard, try "Only this NAC" with the right NAC (still audio) and a wrong one (silence) |
| 11 | DMR, experimental | Your local repeater: Type DMR, its timeslot and color code, no talkgroups | Audio on that slot | Pager B may not decode DMR at all (its handshake has no DMR bit). If silent: Repeater mode on; then the repeater's talkgroups; then the other slot. If still silent, DMR is unsupported on this pager |

Things to note for each step, so the notes can record results: what the
pager showed, whether audio came through, anything odd (announcements,
the pager getting stuck, trunked talkgroups losing names or alerts).

Not covered by pager tests: opening a Knobset-edited profile in PPS on
Windows to confirm PPS keeps the rows. Do that if Windows becomes available.

## Results (pager B, 2026-10-19)

Steps 1 to 5 on the iPad, step 6 onward on the Mac.

| # | Result | Notes |
|---|---|---|
| 1 | Pass | Lake knob 7, NOAA 162.550, 25 kHz, carrier squelch: weather broadcast plays |
| 2 | Pass | CTCSS 100.0 and DCS 065 mute the channel; carrier squelch at 25 kHz brings the audio back |
| 3 | Pass | New zone "Test" with WX 162.40 (162.400 MHz) appears on the pager and plays |
| 4 | Pass | Normal Scan in the Test zone of WX 162.40 and Lake knob 7 (cross-zone) scans both |
| 5 | Pass | Metro knob 4 (MCFD), TG Scan to TG Monitor and back. Radio DB after the round trip is identical to before; the profile differs only in its modified time. The switch itself changed only the knob's mode (16 to 48 in the profile, 16 to 64 in the radio DB), one tabP25TSetting flag and one tabP25TSubGroupEnable row |
| 6 | Pass | A 12-site system from the RR CSVs as new system "Metro RR" (WACN and System ID entered by hand): 12 sites, 44 control channels, 655 clear talkgroups. RR lists two TGIDs twice under different agencies; the import kept the first of each. Test knob 3, TG Scan of 10 talkgroups, receives. Write added only rows (no existing row changed); Metro knob 4 still receives. Imported talkgroups alerted on every call (fixed before 0.5); after removing the knob and deleting the system in one write (the pager matched its pre-import state apart from step 7) and re-importing with the fix (site MO Metro only, 5 talkgroups on Test knob 3), the 655 imported talkgroups are AlertMode 3 with voice storage off, and the knob receives silently |
| 7 | Pass | Removed Test knob 2 (WX Scan) while the pager sat on it. The write dropped only the scan knob's rows. The pager came back on zone 1, as after every write (the converter sets the power-on zone to 1, as PPS does); Test position 2 shows "Unprogrammed". Test knobs 1 and 3 still work |
| 8 | Pass | Test knob 1, conventional WX 162.40 to trunked Metro RR "TEST" (TG Scan, 11 talkgroups via Add all) in one write, after fixing the rename check. The conventional frequency and its knob rows are gone; channel 401 now has exactly the radio tables of a fresh trunked channel (403), differing only in talkgroup count. Receives |
| 9 | Pass | Moved Test one place left (swap with NOAA). Profile changed only the zone order; the radio DB is a pure 3xx/4xx channel swap (scan list rewritten to 402..408, zones relabelled), plus regenerated cipher-key assignment IDs and the power-on zone 1. Pager shows Metro, Lake, Test, NOAA; all knobs receive |
| 10 | Not run | No known local P25 conventional channel with traffic. P25 conventional stays experimental |
| 11 | Written, audio pending | Pager B, Test knob 2 "DMR Test": a local ham DMR repeater (VHF), CC 1, slot 2 (a BrandMeister talkgroup is static there; pager B is 700/800 + VHF, so UHF DMR repeaters are out). The pager accepted the DB and its live DB after reboot keeps every DMR row (TimeSlot 1 = slot 2, RxColorCode 1, mode 32, all-call 16777215). No traffic heard yet. Pager C (G5B67BF) reports the same handshake mode 0x00010517 as A and B |

Found while testing, fixed on the test build:

- An accidental tap on a zone arrow left a pending reorder, which blocked the
  next write. Zone order now has its own labeled dialog, and knob edits are
  blocked while a reorder is pending.
- A write that stopped before programming forced a fresh backup though the
  profile hadn't changed.
- iPad backup times showed UTC as if it were local time.
- The write dialog's time covered only the file transfer, so a 30 s write
  showed as 6 s; the time is gone.
- RadioReference-imported talkgroups alerted on every call: they got PPS's
  talkgroup-editor defaults (tone alert, voice storage). They now get PPS's
  importer defaults, silent with no voice storage.
- Delete system stayed disabled while its only knob was pending removal,
  though the write removes knobs first.
- Every local rebuild was ad-hoc signed with a new identity, so macOS Local
  Network privacy blocked it until allowed again. Local builds signed with
  the Developer ID keep the release build's permission.
- "Add all" in the talkgroup picker needed a search first, which read as
  broken on small systems.
- Converting to a trunked knob failed with "removed and changed in other
  ways": the new knob's name is a separate rename, which the check refused
 .
- The backup after a write can fail and leave an empty folder (seen once,
  after the DMR write; the pager had dropped off USB). The list ignores it.
- The empty-position panel only mentions analog channels; DMR and P25
  conventional hide in its Type menu. The Repeater mode tooltip overflows.

## Test plan for the 0.6 features (2026-10-20)

Pagers B and C (both G5B67BF, DBV1.41c), Knobset test build from `main`
with the 0.6 features. Pager C moved to firmware V2.05 (DBV1.41e) on
2026-10-25 ([`firmware-update.md`](firmware-update.md)); B stays on V2.02. Same rules as above: one kind of write per step,
lowest risk first, a fresh backup before starting, and after every write
check the knob names, the step's behavior, and two or three trunked knobs.
Steps 12 to 34 are new kinds of pager write; none has been on a pager yet.
Step 11 (DMR audio on the local ham repeater) is still open from the 0.5 run.

| # | Feature (PR) | What to do | Expect | Notes |
|---|---|---|---|---|
| 12 | Display format | Pager settings: date YYYY/MM/DD, 24 hour clock; write; then set them back | The pager's clock and date change format | Lowest risk: two display columns |
| 13 | Talkgroup colour | P25 systems: give one talkgroup that's on a knob a background colour | The call screen shows that colour on its next call | Touches one radio table |
| 14 | Talkgroup alert | On Metro RR (silent from the import), set one talkgroup on Test knob 3 to Tone with voice storage on | That talkgroup beeps and records; its neighbors stay silent | The common case after an import |
| 15 | Alert tones | One talkgroup Chirp, another Tone 3; then one at Tone with No tone | The two sound different; note what Tone with No tone does | PPS offers No tone; behavior unknown |
| 16 | Bulk edit | Search a group of talkgroups, tick the header, set Vibrate on all | Every one vibrates; spot-check three | |
| 17 | Hold and block | Hold and block soft keys on, hold timeout 0; on a TG Scan knob during a call, hold, then block another talkgroup | Hold keeps the pager on the talkgroup until released; a blocked one is skipped until the knob turns | If the soft keys don't show, turn on the TGID Hold Timeout menu item and retry |
| 18 | Hold timeout | Timeout 1 minute and the TGID Hold Timeout menu item on | The item shows under Function Setting; a hold releases after a minute | |
| 19 | Alert volume and reminders | Call alert tone by the volume knob, out-of-range beep off, unread reminder off | Beeps follow the volume knob; no out-of-range or unread beeps | |
| 20 | Backlight and voice | Brightness 30%, screen timeout 30 s, longest stored message 2 minutes | Dimmer screen, 30 s timeout, recordings stop at 2 minutes | |
| 21 | RSSI in dBm | Status bar signal as dBm | The status bar shows dBm | Pager B only; refused on 1.41b pagers |
| 22 | Copy an analog knob | Copy a conventional knob into another zone and rename it; in a second write, change the copy's frequency | Both receive; after the second write the original keeps its frequency | The copy shares the original's frequency entry until edited |
| 23 | Copy trunked knobs | Copy a TG Scan knob within its zone, then a TG Monitor knob | Each copy receives like its original | |
| 24 | Copy a scan knob | Copy a scan knob to an empty position | The copy scans the same channels | |
| 25 | Move a priority channel | Move Lake knob 3 (the ElmwoodCo Fire scan's priority channel) to another zone | The priority scan still samples it at its new place | A move renumbers the channel |
| 26 | Swap knobs | Swap two knobs in one zone, then two across zones | Each knob works at its new position | |
| 27 | State after a move | After steps 25 and 26, check held talkgroups, recordings, message history and the last selected channel | Note anything the pager lost | The pager keeps this state itself; Knobset can't see it |
| 28 | Copy the DMR knob | Copy Test knob 2 (DMR) to an empty position | The copy has the same frequency, slot and color code | Experimental |
| 29 | Zone move with a DMR knob | Move the Test zone one place | The write goes through and every knob works | Refused in 0.5.0 on a pager with a DMR knob |
| 30 | Copy a system from a backup | On pager B, P25 systems, Copy from a backup: one of pager A's systems; write without putting it on a knob | Nothing changes on the pager | A system only reaches the pager through a knob |
| 31 | Use the copied system | Put it on an empty knob, TG Scan, a few talkgroups | The knob receives | |
| 32 | Clone preview | `g5 clone <B's newest backup> --target <C's newest backup>` (no `--yes`) | A plan: what changes on C and the identity fields C keeps | No pager needed |
| 33 | Clone B onto C | Plug in C, back it up, then Write to another pager from B's newest backup | C shows B's zones and knobs and still says User-001; C's profile matches B's and its radio DB matches B's profile converted for 1.41e | C is on 1.41e now, so the radio DB is regenerated for it (the dialog says so) and won't match B's byte for byte |
| 34 | C after the clone | Check C's trunked knobs, then rename a knob on C and write | Knobs receive and the normal write path works on C | Restore C's safety backup afterwards if you want C back |
| 35 | Linux | Install the .deb or AppImage from a release; plug in a pager | `ip -br addr` shows an `enx...` interface (`enp...u...` on Arch) on 10.10.0.x; back up, then rename one knob and write | Needs a Linux machine |
| 36 | Windows | Run the installer (SmartScreen: More info, Run anyway); plug in a pager | If it shows as USB Serial Device (COM3) or under Other devices, install Unication's RNDIS driver (x64) or run `install-rndis.ps1` (ARM); `ipconfig` shows 10.10.0.x; back up, rename, write | Needs a Windows machine |

## Results (0.6, 2026-10-20 to 2026-10-28)

| # | Result | Notes |
|---|---|---|
| 12 | Pass | Pager B |
| 13 | Pass | Pager B |
| 14 | Pass | Pager B, covered before the step was run on purpose |
| 15 to 34 | Not run | Steps 30 to 34 needed 1.41e support once pager C was updated |
| 35 | Pass | 2026-10-28, x64 Arch laptop (Wayland), 0.6.2 AppImage, pager A: link up with no setup, template import, backup, knob rename written and renamed back; the profile matches the first backup table for table. The write reset the backlight setting changed on the pager's menu, as PPS's writes do. Later the same day: the 0.6.2 .deb on an Ubuntu 24.04 x64 VM (link up under systemd-networkd), and the ARM builds on an Ubuntu ARM VM with no pager; see `desktop-platforms.md`. |
| 36 | Pass | x64: 2026-10-28, Windows 11 x64 in a KVM VM on the Arch laptop, 0.6.2 x64 build (SmartScreen checked separately on a downloaded copy: More info, Run anyway), pager C: Unication's driver setup turned the COM3 binding into a working RNDIS link with no re-plug, then template import, a backup byte-identical to the Mac's, and a knob rename confirmed on the display and renamed back (last backup matches the first table for table). ARM: Windows 11 on ARM (UTM VM on the Studio, pager C passed through over USB), ARM64 build of 0.6.1: installer, `install-rndis.ps1` bringing up the link, template import from Unication's installer, a backup identical to the Mac's, and a knob rename (Metro knob 5) written and verified after the reboot. Renamed back through the VM on 2026-10-28; a Mac backup afterwards matches the pre-test one table for table. |

Also on 2026-10-25:

- Pager C updated to V2.05 / DBV1.41e with Unication's updater, then
  programmed once by PPS. Knobset regenerates that PPS-written radio
  DB with 194/194 tables identical. A Knobset knob rename on C (Metro
  knob 5) showed on the display and was renamed back; the live DB matched
  PPS's again.
- Found while testing: the app's device poll handshook every few seconds,
  so the pager showed "PC Connected" and stopped scanning while Knobset was
  open.
