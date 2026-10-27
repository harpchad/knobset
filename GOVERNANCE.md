# Governance

Knobset has a single maintainer, Chad Harp ([@harpchad](https://github.com/harpchad)),
who started the project and makes the final call on what goes in. Anyone
can take part through issues and pull requests.

## Roles

**Maintainer** (currently one person):

- Reviews and merges pull requests. Nothing reaches `main` without one,
  and a ruleset requires an approval from someone other than the last
  pusher plus passing tests. The maintainer can bypass that, only
  through a pull request, when no other reviewer is available.
- Triages issues and answers questions.
- Receives and handles private vulnerability reports (see
  [SECURITY.md](SECURITY.md)).
- Tests every new kind of pager write on a real pager before it ships.
- Cuts releases: tags them (only the admin role can create `v*` tags),
  publishes the GitHub Release, signs and notarizes the Mac app, and
  uploads the iPad build to TestFlight. The Apple signing credentials are
  the maintainer's own.
- Enforces the [Code of Conduct](CODE_OF_CONDUCT.md).

**Contributors**: anyone who files an issue, reports test results from
their pager, or opens a pull request. [CONTRIBUTING.md](CONTRIBUTING.md)
covers what a contribution needs.

**Automated reviewers** advise; they don't decide. CodeRabbit reviews and
can approve pull requests, CodeQL and OpenSSF Scorecard report to the
Security tab, fuzzing runs weekly, and Dependabot proposes dependency
updates. The maintainer still merges.

## How decisions are made

Proposals and discussion happen in public, in issues and pull requests.
The maintainer decides, aiming for what helps hobbyists program their
pagers safely. A change that writes something new to a pager also needs a
spec in `notes/`, an entry in the write guard and a test against a real
PPS-written database, as CONTRIBUTING.md describes. Plans for the coming
year are in [ROADMAP.md](ROADMAP.md).

If you disagree with a decision, say so in the issue or pull request.
The maintainer's decision stands, and the MPL-2.0 license lets anyone
fork the project (under a different name; see the README).

## Continuity

With one maintainer, Knobset depends on one person. If the maintainer
stops working on it, the repository stays public and everything needed
to build it is in the repository, so anyone can fork and continue under
MPL-2.0. Release signing (Apple Developer ID, App Store Connect) is tied
to the maintainer's accounts and wouldn't carry over; a fork would sign
with its own.

New maintainers can be added by the current one, typically after a run
of sustained, careful contributions. When that happens, this file will
list them and how decisions are shared.
