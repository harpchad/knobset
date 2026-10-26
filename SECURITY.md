# Security policy

## Reporting a vulnerability

Use GitHub's private reporting: on the repo's **Security** tab, choose
**Report a vulnerability**, or go straight to
<https://github.com/harpchad/knobset/security/advisories/new>. Don't open
a public issue for it.

Expect a reply within a week. Knobset is maintained by one person in
spare time, so fixes ship in the next release rather than on a schedule.

## Scope

Knobset talks to a pager over its USB network link (`10.10.0.0/16`) and
reads and writes files on your computer. It runs no network services and
sends nothing to the internet. RadioReference import reads a file or paste
you supply; Knobset doesn't contact RadioReference.

In scope:

- Anything that lets a file, a backup, a template import or a RadioReference
  export run code or write outside the folders Knobset is meant to use.
- Anything that exposes a pager's access password. The pager sends it in
  plain text during the handshake; Knobset keeps only whether one is set.
- A write that changes a pager in a way the read-back check doesn't catch.
- Tampered release downloads. The Mac DMG is signed and notarized. The
  Windows and Linux installers aren't signed yet, so download them only
  from this repo's Releases page and check them against the release's
  `SHA256SUMS.txt`. In releases after 0.6.1, `gh attestation verify`
  also confirms that this repo's CI built them (see the README).

Microsoft Defender has flagged the unsigned Windows on ARM installer as
`Trojan:Win32/Bearfoos.A!ml`. That's a known false positive from
Defender's machine-learning model. If the file's SHA-256 matches
`SHA256SUMS.txt` (and, for releases after 0.6.1, its attestation
verifies), it's the file this repo published. If it doesn't match, report it.

The repo runs CodeQL code scanning and OpenSSF Scorecard; both report to
the Security tab.

Out of scope: weaknesses in the pager's own protocol, such as its
unauthenticated USB link. Knobset can't fix those, but notes on them are
welcome in [`notes/protocol.md`](notes/protocol.md).
