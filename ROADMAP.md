# Roadmap

What Knobset intends to do, and not do, from November 2026 through
October 2027. Priorities can shift with what testers report; the
[CHANGELOG](CHANGELOG.md) records what actually shipped.

## Next (0.8)

- Finish the on-pager test plan (`notes/pager-test-plan.md`, steps 15 to
  31): talkgroup settings, pager settings, knob copy and move.
- Hear audio on the P25 conventional knob already written to a pager.
- Hear audio on a DMR knob. DMR knobs ship in 0.7, marked experimental,
  so anyone near an active DMR repeater can try them and report back.
- A demo mode with built-in sample data, so the iPad app can be used
  (and reviewed by Apple) without a pager.
- A Mac App Store build: sandboxed, with access to the backups folder
  kept through a security-scoped bookmark. The notarized DMG stays.

## Later in the year

- Converting a conventional knob to trunking.
- Copying a whole zone between pagers or backups.
- Alert tones (M5), once a non-GPL G.729A encoder is settled.
- Fuzzing the vendored SQLite 2 reader with AddressSanitizer, and adopting
  rustfmt with a config that matches the code's style.

## Not planned

- DMR trunking: no G5 supports it.
- Firmware updates: use Unication's updater.
- Shipping Unication's template databases or RadioReference data.
- Accounts, cloud sync or telemetry.
- Any network traffic beyond the pager's USB link.
