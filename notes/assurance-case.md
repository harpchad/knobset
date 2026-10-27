# Security assurance case

Why Knobset's security requirements are met: what it must protect, from
whom, where the trust boundaries are, and the evidence. Reviewed
2026-10-27. Reporting and scope are in [SECURITY.md](../SECURITY.md);
the components are in [ARCHITECTURE.md](../ARCHITECTURE.md).

## Security requirements (claims)

1. **No unintended pager writes.** Knobset writes only the files PPS
   itself programs, only the tables an edit is meant to change, only to
   the pager the backup came from, and it proves the result byte for byte.
2. **Hostile input can't run code or escape its folders.** Files the user
   opens (backups, PPS installers, RadioReference exports) and bytes from
   the pager can't make Knobset run code, crash unsafely, or write outside
   the folders it means to use.
3. **The pager's access password stays private.** Knobset never stores,
   logs or displays it or its MD5.
4. **Downloads are what the project built.** Users can check that a
   release file is the one this repository's CI (or the maintainer's
   signed Mac build) produced.

## Threat model

**Assets:** the pager's programming (a bad write can leave it unusable
until restored), the user's backups, the pager's access password, and the
integrity of Knobset's releases.

**Threat actors and entry points:**

| Actor | Entry point |
|---|---|
| A crafted file someone gives the user | Backup folders, `u3db_PC.zip`, PPS installers (zip, exe, msi), RadioReference CSVs and pastes |
| A malicious or faulty USB device posing as a pager | Every byte on the pager link |
| Another local user on the same machine | Shared temp directories |
| An attacker in the supply chain | Dependencies, GitHub Actions, the release pipeline, a compromised app with repo access (such as a code review bot) |
| Someone who obtains a debug log | `g5 -vv` packet traces |

Out of scope: weaknesses in the pager's own protocol (unauthenticated
USB link, MD5 access check, plaintext handshake), which Knobset can't
change, and an attacker who already controls the user's account.

## Trust boundaries

1. **Pager ↔ Knobset.** Everything from `10.10.0.12` is untrusted:
   frames are checksum- and length-checked, responses parsed defensively.
2. **Files ↔ Knobset.** Every file the user opens is untrusted, including
   Knobset's own earlier backups, which could have been swapped.
3. **Rust ↔ vendored C (SQLite 2.8.17).** Memory-unsafe code reads
   databases that came from the pager or from files.
4. **App UI (webview) ↔ Rust core.** The webview can only call the
   Tauri commands the app defines.
5. **Repository ↔ CI ↔ release.** Code reaches `main` only through pull
   requests; release tags are admin-only; installers are built and
   attested by CI.

## Secure design principles applied

- **Economy of mechanism:** a client with no server, accounts or internet
  access; programming is whole-file upload plus read-back.
- **Fail-safe defaults:** the write guard refuses any table change not
  allowed for the edit; `WRITABLE_PATHS` refuses any other pager path;
  CI tokens are read-only unless a job needs more.
- **Complete mediation:** the CLI and the app share one write path
  (`g5_core::write::program`), and every pager write goes through the
  path allowlist before a byte is sent.
- **Open design:** the code and protocol notes are public; release trust
  rests on Sigstore and Apple's notarization, not secrecy.
- **Separation of privilege:** a pager write needs both Knobset's guard
  and the pager's access check; a release needs an admin-only tag and a
  CI build.
- **Least privilege:** per-user installers with no admin rights (the
  Windows on ARM driver script is the one admin step, and does one thing);
  per-job workflow permissions.
- **Least common mechanism:** per-user app data, and SQLite's temp files
  in the per-user temp directory.
- **Psychological acceptability:** safety backups and read-back happen
  automatically; checksum and SmartScreen steps are spelled out.
- **Limited attack surface:** no listening sockets; one network peer.
- **Input validation with allowlists:** paths, tables, opcodes and import
  formats are checked against what's known to be valid.

## Common weaknesses countered

| Weakness (CWE) | Countermeasure | Evidence |
|---|---|---|
| Memory corruption (787, 125) | Rust; `unsafe` limited to the SQLite 2 bindings and an iOS call | CodeQL (Rust, C); cargo-fuzz with AddressSanitizer |
| Malformed input, integer overflow (20, 190) | Checksum and length checks; errors, not panics | Unit tests; fuzz targets with overflow checks |
| SQL injection (89) | Values bound with `sqlite_bind`; table names from code | Code review; CodeQL |
| Path traversal, zip slip, symlinks (22, 59) | Enclosed zip paths, symlinks refused, single-component MSI names, pager path allowlist | Unit tests (`write_path_allowlist` and import tests) |
| Cross-site scripting (79) | React escaping; no raw HTML; CSP `default-src 'self'` | `app/src-tauri/tauri.conf.json` |
| Command injection (78) | No shell or process spawning | Code search; CodeQL |
| Credentials in logs (532) | Handshake and access checks redacted in `-vv` traces | `wire_trace_never_shows_the_password` test |
| Broken crypto (327) | MD5 only where the pager requires it, risk documented | SECURITY.md, Cryptography |
| Vulnerable or tampered dependencies (1395, 494) | Dependabot, OSV triage, SHA-pinned actions, attestations | `osv-scanner.toml`, Scorecard |

## Residual risks

- The pager protocol has no encryption or strong authentication; anyone
  with the USB link can reprogram a pager. Knobset can't fix that.
- The vendored SQLite 2 C code isn't fuzzed directly yet (on the
  [roadmap](../ROADMAP.md)).
- Windows installers aren't Authenticode-signed; checksums and
  attestations are the verification path.
- One maintainer (see [GOVERNANCE.md](../GOVERNANCE.md)), and CodeRabbit
  has write access to the repository; `main` and release tags are
  protected by rulesets.
