# Contributing to Knobset

Bug reports, pager test results and pull requests are welcome. Knobset
writes to real hardware, so a few rules here are stricter than usual.
Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md);
[GOVERNANCE.md](GOVERNANCE.md) explains who decides what, and
[ARCHITECTURE.md](ARCHITECTURE.md) is the map of the code.

## Reporting a problem

Open an issue with:

- The Knobset version and your OS.
- The pager's model number, firmware version and radio DB version. Knobset
  shows all three on the Pager page, and `g5 info` prints them.
- What you did, what you expected and what happened.

Don't paste your pager's serial number, access password, unit IDs or a
backup folder. If a backup would help, say so and we'll work out what to
share.

## Ground rules

- **No Unication code or files.** Don't paste decompiled PPS or firmware
  code into the repo, and don't commit Unication's template databases,
  installers or manuals. Describe behavior in your own words and cite where
  you saw it (a class, method or table name is fine). See
  [`notes/README.md`](notes/README.md) for how the existing specs do this.
- **No RadioReference data.** RadioReference's terms don't allow
  redistributing their database. Use made-up talkgroups, sites and
  frequencies in tests, mocks and docs. Real data for local testing goes in
  the gitignored `rr-data/`.
- **No pager backups.** They hold serials and personal profiles, and they
  stay in the gitignored `backups/`.
- **A write must stay byte-for-byte checkable.** Every change that adds a
  new kind of pager write needs a spec in `notes/` first, an entry in the
  write guard's allowlist, and a test against a real PPS-written database.
  Say in the PR which pager and firmware you tested on. If you couldn't
  test on a pager, say that instead.
- **New behavior comes with tests.** A PR that adds a feature or fixes a
  bug adds an automated test for it to `cargo test --workspace`, or says
  why it can't (UI-only changes, for example). Changes to a parser that
  reads pager or RadioReference input should keep its fuzz target in
  `fuzz/` working.

## Code style

CI rejects a PR unless `cargo clippy --workspace --all-targets -- -D warnings`
is clean and the frontend type-checks (`npx tsc --noEmit` in `app/`). Match
the surrounding code otherwise: lines in this repo run long (rustfmt's
defaults aren't applied), and comments explain why, not what.

## Building

You need stable Rust, Node 24 and
[Tauri 2's prerequisites](https://v2.tauri.app/start/prerequisites/) for
your OS.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cd app && npm install && npm run tauri dev
```

The golden tests compare against private pager backups in `./backups` and
skip without them, so CI runs fewer tests than a maintainer's machine does.
Those tests use stand-in zone and knob names; `backups/fixture-names.txt`
(`stand-in=real name` per line) maps them to the names in the backups.
Writing needs Unication's templates: import PPS in Settings, or run
`g5 import-templates`.

The parsers for pager frames and responses and for RadioReference
exports have fuzz targets in `fuzz/`. They need nightly Rust and
cargo-fuzz:

```sh
cargo install cargo-fuzz
cd fuzz && cargo +nightly fuzz run frame   # or pager_responses, rr_import
```

## Pull requests

- Branch from `main` and name the branch `<type>/<description>`, for
  example `fix/backup-timestamp` or `feat/dmr-import`.
- Use [Conventional Commits](https://www.conventionalcommits.org) for
  commit messages: `fix(codeplug): ...`, `feat(app): ...`.
- Add a line under `## [Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md) for
  anything a user would notice.
- Keep each PR to one change. A PR that also reformats unrelated code is
  hard to review.

To merge, a PR needs an approving review from someone other than whoever
pushed its last commit (CodeRabbit's automated review counts), a passing
"Test and clippy" check, and to be up to date with `main`. Review
comments get answered or fixed first. The maintainer can bypass these
rules, but only by merging through a PR, never by pushing to `main`.

By contributing, you agree that your contribution is licensed under the
[Mozilla Public License 2.0](LICENSE), the same as the rest of Knobset.

## Security

Report security issues privately. See [`SECURITY.md`](SECURITY.md).
