# Roadmap

What Knobset intends to do, and not do, from November 2026 through
October 2027. Priorities can shift with what testers report; the
[CHANGELOG](CHANGELOG.md) records what actually shipped.

## Next (0.7)

- Finish the on-pager test plan (`notes/pager-test-plan.md`, steps 15 to
  34): talkgroup settings, pager settings, cloning, knob copy and move.
- Verify P25 conventional on a pager, and DMR audio on a local repeater.

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
