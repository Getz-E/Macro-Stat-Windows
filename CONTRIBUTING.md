# Contributing to MacroStat for Windows

Thanks for your interest. This document covers the development environment, the
commands used by CI, and the repository rules. All commands below are the ones
defined in the project's own files; if a command is not listed here, it is not a
project convention.

## 1. Development environment

| Tool | Version used | Purpose |
|---|---|---|
| Windows | 11 x64 (supported); 10 22H2 build 19045 (development/test only) | The engine and GUI only run on Windows |
| Rust | 1.79+ with the MSVC toolchain (`stable-x86_64-pc-windows-msvc`) | Builds `macrostat-engine.exe`; the platform-independent core can also be tested on Linux |
| Python | 3.10+ (CI uses 3.12) | GUI, tests, tooling |
| PyQt6 | `windows/gui/requirements-win.txt` | GUI runtime |
| PyInstaller 6 | `windows/gui/requirements-build.txt` | Optional: package the onedir build |
| Inno Setup 6.3+ | External installer | Optional: produce `MacroStat-Setup-1.0.0.exe` |
| PyYAML 6 | `ci/requirements.txt` | Required by `ci/preflight.py` only; not a runtime dependency |

Windows 11 x64 is the officially supported platform. Windows 10 22H2 (build
19045) is past its regular support period, as the root `README.md` notes; it is
kept only as a development and test environment and is not presented here as an
officially supported version.

Setup on Windows:

```powershell
python -m pip install -r windows/gui/requirements-win.txt
python -m pip install -r ci/requirements.txt
```

For packaging, install `windows/gui/requirements-build.txt` instead of the GUI
file; it includes it.

## 2. Formatting, linting and tests

Run the Rust commands from `windows/engine`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

`keymap.rs` is generated from `keymap` sources by `tools/gen_keymap.py`, which
formats its output with `rustfmt`. Do not edit `windows/engine/src/keymap.rs` by
hand.

```bash
# run from windows/engine; rustfmt must be installed
python tools/gen_keymap.py --check
```

`--check` compares the generated output with the committed file and fails when it
is stale. To regenerate after an intentional mapping change:

```bash
python tools/gen_keymap.py
```

GUI tests (from the repository root):

```bash
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -p "test_gui*.py" -v
```

Preflight (from the repository root; requires PyYAML):

```bash
python ci/preflight.py
```

Expected output: `CI on kontrolu gecti`. It validates the workflow job graph,
triggers, action SHA pins, forbidden network commands, version consistency and
that `ci/windows.yml` and `.github/workflows/windows.yml` are byte-identical.

### Windows E2E tests

`windows/tests/test_e2e_windows.py` runs the real `macrostat-engine.exe` and is
**Windows-only**; on other platforms its 7 E2E test methods are skipped. It
requires a Windows desktop session for the input-injection scenarios.

```powershell
$env:MACROSTAT_E2E_ENGINE = "windows\engine\target\release\macrostat-engine.exe"
python -m unittest discover -s windows/tests -p "test_e2e*.py" -v
```

If the environment cannot deliver hook events, set
`MACROSTAT_E2E_SKIP_INPUT=1`; the input-dependent scenarios are then skipped and
reported as skipped, not as passed. A skip is not a pass.

The full suite, including the platform-independent regression tests, is:

```bash
QT_QPA_PLATFORM=offscreen python -m unittest discover -s windows/tests -v
```

## 3. Continuous integration

The active workflow is `.github/workflows/windows.yml`. Its source of truth is
`ci/windows.yml`; the two files must be **byte-identical**. After changing the
workflow:

```bash
cp ci/windows.yml .github/workflows/windows.yml
python ci/preflight.py
```

CI jobs:

| Job | What it runs |
|---|---|
| `core` | keymap `--check`, `cargo fmt --check`, clippy with `-D warnings`, `cargo test --all` |
| `gui-tests` | `ci/preflight.py`, then the GUI tests offscreen |
| `build` | `windows/scripts/build.ps1 -SkipTests`; uploads the installer, portable ZIP, checksums and raw bundle as artifact `macrostat-win-x64` (30-day retention) |
| `e2e` | E2E suite against the real engine from the `build` artifact |
| `audit` | `cargo audit --deny warnings` and `pip-audit --strict`; informative on pull requests, blocking on push, `v*` tags and the weekly schedule |
| `sign` | Only when `SIGNING_ENABLED` is set; requires the protected `signing` environment |

All `uses:` entries must remain pinned to full 40-character commit SHAs;
`ci/preflight.py` rejects tags and branches.

`windows/ci/windows.yml` is a **historical Phase 0 draft**. It is not used and
must never be copied to `.github/workflows/`. The active files above are the only
ones to edit.

## 4. Repository rules

- The Linux project is a **separate repository**. Do not copy its files into this
  repository and do not modify it as part of a change here.
- Do not commit generated or secret files: `windows/engine/target/`,
  `windows/dist/`, `windows/build/`, `**/__pycache__/`, `*.pyc`,
  `.pytest_cache/`, `.venv/`, `*.pfx`.
- Never edit `windows/engine/src/keymap.rs` by hand; regenerate it and verify with
  `--check`.
- Keep the CI copy equality described above; do not bypass or weaken
  `ci/preflight.py`.
- New source files should carry the same header style as the existing code:
  `SPDX-License-Identifier: GPL-3.0-or-later` and
  `Copyright (C) 2026 MacroStat contributors`.
- Do not weaken or delete tests or security checks to make a change pass.

There is currently no commit-message convention, no Developer Certificate of
Origin (DCO) requirement and no documented branch-protection rule for this
project. Do not assume one exists.

## 5. Pull requests

Before opening a pull request:

1. Run the formatting, lint, test and preflight commands in section 2.
2. Re-run `--check` after any keymap change.
3. If you changed the workflow, update both copies and run preflight.
4. Describe what changed, why, and what you verified. State explicitly which
   Windows-only tests you ran and which you could not run.

Security issues must follow [`SECURITY.md`](SECURITY.md), not the public issue
tracker.
