# Security Policy

MacroStat for Windows is a release candidate (1.0.0). This policy describes how
security issues should be reported and what is in scope.

## Supported versions

Security fixes are provided for the **1.x** release line. Older or unreleased
development snapshots are not supported. Because 1.0.0 is still a release
candidate, the 1.x line is the only line that exists; a stable release has not
been published yet.

## Reporting a vulnerability

**Do not report security vulnerabilities in public GitHub Issues.** Never post
technical details, proof-of-concept code, or any other sensitive information in
a public issue.

GitHub Private Vulnerability Reporting / Security Advisories is **not verified
to be enabled** for this repository. The **Report a vulnerability** option can
be used only if a maintainer has enabled that feature in the repository
settings.

If **Report a vulnerability** is available:

1. Open the repository's **Security** tab.
2. Open **Advisories** (Security Advisories).
3. Choose **Report a vulnerability** and submit the report privately.

If it is not available, the feature is not enabled. In that case:

1. Open a normal GitHub Issue that contains **no vulnerability details** — do
   not include the affected component, reproduction steps, a proof of concept,
   or any other sensitive information.
2. Ask the maintainers in that issue to provide a private reporting channel.
3. Do not share any details until a maintainer has provided a private channel
   and explained how to use it.

A report submitted through the private channel should include, where possible:

- affected version or commit;
- Windows version and architecture;
- a description of the issue and its impact;
- reproduction steps or a proof of concept;
- any suggested mitigation.

No email address or other private contact channel is published for security
reports; do not assume that one exists. No fixed response time, service-level
agreement, or support guarantee is offered.

## Scope

In scope:

- the Rust engine (`windows/engine/`);
- the PyQt6 interface (`windows/gui/`);
- the installer and packaging scripts (`windows/installer/`,
  `windows/scripts/`);
- the CI workflow and dependency-audit configuration (`ci/`,
  `.github/workflows/`);
- the profile/IPC contract and its validation.

Out of scope by design (see the threat model):

- injection into elevated windows or the secure desktop (blocked by UIPI and the
  Windows security model);
- kernel drivers, services, `uiAccess`, anti-cheat evasion, or process
  injection — none of these are used;
- attacks that require code execution as the same user, which Windows cannot
  isolate from a same-session process.

The full Windows threat model, assets, mitigations and residual risks are in
[`windows/docs/SECURITY-WINDOWS.md`](windows/docs/SECURITY-WINDOWS.md).

## Handling process

1. Once a private channel is available, a report received through it is reviewed
   and reproduced.
2. A fix is prepared and validated with the relevant tests (Rust unit tests,
   Python GUI tests, and Windows E2E where applicable).
3. The fix and any advisory are published after review and verification.
4. Credit is given to the reporter if requested and if doing so does not expose
   them to risk.

No fixed deadlines are promised. Reports are handled on a best-effort basis.

## General bugs

Non-security bugs and feature requests belong in
[GitHub Issues](../../issues). Do not include vulnerability details,
proof-of-concept code, or other sensitive information there.
