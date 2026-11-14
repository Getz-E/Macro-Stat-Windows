#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate src/keymap.rs from docs/KEY-DICTIONARY.tsv + the Windows mapping table.

The canonical key names and aliases are NOT invented here: they are read from
the dictionary generated from the Linux engine's own parser, so the Windows
engine accepts exactly the key language the frozen contract promises.

The Windows (VK, Set-1 scan code, extended) values below are declarative
Phase-1 data. Phase 2 verifies every non-None scan code against
MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) on a real Windows host and logs mismatches.
`scan_code == None` means: inject with the VK path (no KEYEVENTF_SCANCODE).

Usage: python3 tools/gen_keymap.py [--check]
  --check  do not write; exit 1 if src/keymap.rs is out of date.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]          # windows/engine
DICT_PATH = ROOT.parent / "docs" / "KEY-DICTIONARY.tsv"
OUT_PATH = ROOT / "src" / "keymap.rs"

# ---------------------------------------------------------------------------
# Windows mapping data.
# tuple: (virtual-key code, Set-1 scan code or None, extended flag, mouse button)
# ---------------------------------------------------------------------------

LETTERS = {
    "A": 0x1E, "B": 0x30, "C": 0x2E, "D": 0x20, "E": 0x12, "F": 0x21,
    "G": 0x22, "H": 0x23, "I": 0x17, "J": 0x24, "K": 0x25, "L": 0x26,
    "M": 0x32, "N": 0x31, "O": 0x18, "P": 0x19, "Q": 0x10, "R": 0x13,
    "S": 0x1F, "T": 0x14, "U": 0x16, "V": 0x2F, "W": 0x11, "X": 0x2D,
    "Y": 0x15, "Z": 0x2C,
}

DIGITS = {
    "1": 0x02, "2": 0x03, "3": 0x04, "4": 0x05, "5": 0x06,
    "6": 0x07, "7": 0x08, "8": 0x09, "9": 0x0A, "0": 0x0B,
}

# canonical name: (VK, scan, extended)
PUNCTUATION = {
    "KEY_APOSTROPHE": (0xDE, 0x28, False),
    "KEY_BACKSLASH": (0xDC, 0x2B, False),
    "KEY_COMMA": (0xBC, 0x33, False),
    "KEY_DOT": (0xBE, 0x34, False),
    "KEY_EQUAL": (0xBB, 0x0D, False),
    "KEY_GRAVE": (0xC0, 0x29, False),
    "KEY_LEFTBRACE": (0xDB, 0x1A, False),
    "KEY_MINUS": (0xBD, 0x0C, False),
    "KEY_RIGHTBRACE": (0xDD, 0x1B, False),
    "KEY_SEMICOLON": (0xBA, 0x27, False),
    "KEY_SLASH": (0xBF, 0x35, False),
}

SPECIAL = {
    "KEY_BACKSPACE": (0x08, 0x0E, False),
    "KEY_CAPSLOCK": (0x14, 0x3A, False),
    "KEY_DELETE": (0x2E, 0x53, True),
    "KEY_DOWN": (0x28, 0x50, True),
    "KEY_END": (0x23, 0x4F, True),
    "KEY_ENTER": (0x0D, 0x1C, False),
    "KEY_ESC": (0x1B, 0x01, False),
    "KEY_HOME": (0x24, 0x47, True),
    "KEY_INSERT": (0x2D, 0x52, True),
    "KEY_LEFT": (0x25, 0x4B, True),
    "KEY_NUMLOCK": (0x90, 0x45, False),
    "KEY_PAGEDOWN": (0x22, 0x51, True),
    "KEY_PAGEUP": (0x21, 0x49, True),
    # Pause is an E1-prefixed sequence; no single Set-1 byte. Use the VK path.
    "KEY_PAUSE": (0x13, None, False),
    "KEY_RIGHT": (0x27, 0x4D, True),
    "KEY_SCROLLLOCK": (0x91, 0x46, False),
    "KEY_SPACE": (0x20, 0x39, False),
    "KEY_TAB": (0x09, 0x0F, False),
    "KEY_UP": (0x26, 0x48, True),
}

NUMPAD = {
    "KEY_KP0": (0x60, 0x52, False),
    "KEY_KP1": (0x61, 0x4F, False),
    "KEY_KP2": (0x62, 0x50, False),
    "KEY_KP3": (0x63, 0x51, False),
    "KEY_KP4": (0x64, 0x4B, False),
    "KEY_KP5": (0x65, 0x4C, False),
    "KEY_KP6": (0x66, 0x4D, False),
    "KEY_KP7": (0x67, 0x47, False),
    "KEY_KP8": (0x68, 0x48, False),
    "KEY_KP9": (0x69, 0x49, False),
    "KEY_KPASTERISK": (0x6A, 0x37, False),
    "KEY_KPDOT": (0x6E, 0x53, False),
    "KEY_KPENTER": (0x0D, 0x1C, True),
    "KEY_KPMINUS": (0x6C, 0x4A, False),
    "KEY_KPPLUS": (0x6B, 0x4E, False),
    "KEY_KPSLASH": (0x6F, 0x35, True),
}

MODIFIERS = {
    "KEY_LEFTALT": (0xA4, 0x38, False),
    "KEY_RIGHTALT": (0xA5, 0x38, True),
    "KEY_LEFTCTRL": (0xA2, 0x1D, False),
    "KEY_RIGHTCTRL": (0xA3, 0x1D, True),
    "KEY_LEFTSHIFT": (0xA0, 0x2A, False),
    "KEY_RIGHTSHIFT": (0xA1, 0x36, False),
    "KEY_LEFTMETA": (0x5B, 0x5B, True),
    "KEY_RIGHTMETA": (0x5C, 0x5C, True),
}

MOUSE = {
    "BTN_LEFT": "Left",
    "BTN_RIGHT": "Right",
    "BTN_MIDDLE": "Middle",
    "BTN_SIDE": "Side",
    "BTN_EXTRA": "Extra",
}

# Physical equivalence classes: right modifier -> left canonical key.
EQUIVALENCE = {
    "KEY_RIGHTCTRL": "KEY_LEFTCTRL",
    "KEY_RIGHTSHIFT": "KEY_LEFTSHIFT",
    "KEY_RIGHTALT": "KEY_LEFTALT",
    "KEY_RIGHTMETA": "KEY_LEFTMETA",
}


def build_mapping() -> dict[str, tuple[int, int | None, bool, str | None]]:
    m: dict[str, tuple[int, int | None, bool, str | None]] = {}
    for ch, scan in LETTERS.items():
        m[f"KEY_{ch}"] = (ord(ch), scan, False, None)
    for ch, scan in DIGITS.items():
        m[f"KEY_{ch}"] = (ord(ch), scan, False, None)
    for i in range(1, 25):
        vk = 0x70 + i - 1
        if i <= 10:
            scan: int | None = 0x3B + i - 1
        elif i == 11:
            scan = 0x57
        elif i == 12:
            scan = 0x58
        else:
            # F13-F24 have no reliable Set-1 code on PC keyboards.
            scan = None
        m[f"KEY_F{i}"] = (vk, scan, False, None)
    for table in (PUNCTUATION, SPECIAL, NUMPAD, MODIFIERS):
        for name, (vk, scan, ext) in table.items():
            m[name] = (vk, scan, ext, None)
    for name, button in MOUSE.items():
        m[name] = (0, None, False, button)
    return m


def read_dictionary() -> list[tuple[str, list[str]]]:
    rows: list[tuple[str, list[str]]] = []
    for line in DICT_PATH.read_text(encoding="utf-8").splitlines()[1:]:
        if not line.strip():
            continue
        canonical, aliases, _present = line.split("\t")
        rows.append((canonical, [a.strip() for a in aliases.split(",") if a.strip()]))
    return rows


def rust_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def generate() -> str:
    rows = read_dictionary()
    mapping = build_mapping()

    missing = [name for name, _ in rows if name not in mapping]
    extra = sorted(set(mapping) - {name for name, _ in rows})
    if missing:
        raise SystemExit(f"missing Windows mapping for: {missing}")
    if extra:
        raise SystemExit(f"mapping has keys absent from the dictionary: {extra}")

    # Alias uniqueness: a duplicate would silently shadow a key.
    seen: dict[str, str] = {}
    for canonical, aliases in rows:
        for alias in aliases:
            if alias in seen:
                raise SystemExit(f"alias {alias!r} defined by {seen[alias]} and {canonical}")
            seen[alias] = canonical

    count = len(rows)
    consts = "\n".join(
        f"    pub const {canonical}: Key = Key({i});" for i, (canonical, _) in enumerate(rows)
    )
    infos = []
    for i, (canonical, aliases) in enumerate(rows):
        vk, scan, ext, mouse = mapping[canonical]
        scan_expr = "Some(0x%02X)" % scan if scan is not None else "None"
        mouse_expr = f"Some(MouseButton::{mouse})" if mouse else "None"
        alias_expr = ", ".join(rust_str(a) for a in aliases)
        infos.append(
            "    KeyInfo {\n"
            f"        canonical: {rust_str(canonical)},\n"
            f"        aliases: &[{alias_expr}],\n"
            f"        win: WinKey {{ vk: 0x{vk:02X}, scan_code: {scan_expr}, extended: {str(ext).lower()} }},\n"
            f"        mouse: {mouse_expr},\n"
            "    },"
        )
    infos_src = "\n".join(infos)

    all_keys_src = ", ".join(f"Key({i})" for i in range(count))

    # Reverse mapping for the low-level hook: Windows -> canonical Key.
    # Keyboard keys only; mouse buttons arrive as WM_* button messages.
    exact: dict[tuple[int, bool], int] = {}
    scan_exact: dict[tuple[int, bool], int] = {}
    per_vk: dict[int, set[tuple[int, bool]]] = {}
    for i, (canonical, _aliases) in enumerate(rows):
        vk, scan, ext, mouse = mapping[canonical]
        if mouse:
            continue
        pair = (vk, bool(ext))
        if pair in exact:
            raise SystemExit(f"ambiguous VK mapping {pair!r}: Key({exact[pair]}) and Key({i})")
        exact[pair] = i
        per_vk.setdefault(vk, set()).add(pair)
        if scan is not None:
            spair = (scan, bool(ext))
            if spair in scan_exact:
                raise SystemExit(
                    f"ambiguous scan mapping {spair!r}: Key({scan_exact[spair]}) and Key({i})"
                )
            scan_exact[spair] = i

    vk_arms = "\n".join(
        f"        (0x{vk:02X}, {str(ext).lower()}) => Some(Key({i})),"
        for (vk, ext), i in sorted(exact.items())
    )
    scan_arms = "\n".join(
        f"        (0x{scan:02X}, {str(ext).lower()}) => Some(Key({i})),"
        for (scan, ext), i in sorted(scan_exact.items())
    )
    # VKs with exactly one (vk, extended) pairing tolerate a wrong extended
    # flag; 0x0D (Enter vs keypad Enter) is intentionally excluded.
    unique_vk_arms = "\n".join(
        f"        0x{vk:02X} => Some(Key({exact[next(iter(pairs))]})),"
        for vk, pairs in sorted(per_vk.items())
        if len(pairs) == 1
    )
    mouse_arms = "\n".join(
        f"            MouseButton::{button} => Key::{name}," for name, button in MOUSE.items()
    )

    all_alias_pairs = []
    for i, (_canonical, aliases) in enumerate(rows):
        for alias in aliases:
            all_alias_pairs.append(f"    ({rust_str(alias)}, Key({i})),")
    all_aliases_src = "\n".join(all_alias_pairs)

    equivalence_arms = "\n".join(
        f"            Key::{right} => Key::{left}," for right, left in EQUIVALENCE.items()
    )

    return f'''// SPDX-License-Identifier: GPL-3.0-or-later
//! Canonical key identifiers and their Windows (VK, scan code) mapping.
//!
//! GENERATED FILE — do not edit by hand.
//! Source: `windows/docs/KEY-DICTIONARY.tsv` + `engine/tools/gen_keymap.py`.
//! Regenerate: `python3 tools/gen_keymap.py`.
//!
//! # Platform notes
//!
//! * Parsing mirrors the Linux engine: trim, uppercase, strip one optional
//!   `KEY_` prefix, then look the remainder up among the dictionary aliases.
//!   That is why `KEY_LEFT` resolves to [`Key::BTN_LEFT`] (the alias `LEFT`
//!   belongs to the mouse button) and the left arrow must be written
//!   `KEY_LEFT_ARROW`/`ARROWLEFT` — exactly as on Linux.
//! * Numeric evdev codes (`CODE_65`, `65`) are Linux-only; they are rejected
//!   here with [`KeyParseError::NumericPlatformSpecific`].
//! * `scan_code == None` means "inject through the VK path"; it does not mean
//!   the key is unsupported. Phase 2 cross-checks every table value on a real
//!   Windows host with `MapVirtualKeyW(vk, MAPVK_VK_TO_VSC)`.

use std::fmt;

/// Number of canonical keys in the frozen dictionary.
pub const KEY_COUNT: usize = {count};

/// A canonical MacroStat key (keyboard key or mouse button).
///
/// The variant/identity is an opaque index into [`KEY_TABLE`]; use the
/// associated `KEY_*`/`BTN_*` constants for named access.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key(u8);

impl Key {{
{consts}

    /// Stable internal index in `0..KEY_COUNT`.
    pub const fn index(self) -> u8 {{
        self.0
    }}

    /// Canonical dictionary name, e.g. `"KEY_LEFTCTRL"`.
    pub fn canonical(self) -> &'static str {{
        KEY_TABLE[self.0 as usize].canonical
    }}

    /// Every accepted spelling for this key, including the canonical name
    /// when the dictionary lists it as a valid alias.
    pub fn aliases(self) -> &'static [&'static str] {{
        KEY_TABLE[self.0 as usize].aliases
    }}

    /// Windows virtual-key code / scan code / extended flag.
    pub fn windows(self) -> WinKey {{
        KEY_TABLE[self.0 as usize].win
    }}

    /// `Some(button)` for mouse buttons, `None` for keyboard keys.
    pub fn mouse_button(self) -> Option<MouseButton> {{
        KEY_TABLE[self.0 as usize].mouse
    }}

    pub fn is_mouse(self) -> bool {{
        self.mouse_button().is_some()
    }}

    /// Left/right modifier variants collapse to the left key; everything else
    /// is its own class. Used for combo latching and double-tap tracking.
    pub fn physical_class(self) -> Key {{
        match self {{
{equivalence_arms}
            _ => self,
        }}
    }}
}}

impl fmt::Debug for Key {{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{
        f.write_str(self.canonical())
    }}
}}

impl fmt::Display for Key {{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{
        f.write_str(self.canonical())
    }}
}}

/// Declarative Windows mapping for one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinKey {{
    /// Windows virtual-key code. `0` for mouse buttons.
    pub vk: u16,
    /// Set-1 scan code, or `None` when only the VK path is safe.
    pub scan_code: Option<u16>,
    /// `KEYEVENTF_EXTENDEDKEY` is required for this key.
    pub extended: bool,
}}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {{
    Left,
    Right,
    Middle,
    Side,
    Extra,
}}

/// Why a token could not be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyParseError {{
    /// The token was empty after trimming.
    Empty,
    /// `CODE_<n>` or a bare numeric evdev code: meaningful on Linux only.
    NumericPlatformSpecific,
    /// Not a canonical name or alias in the dictionary.
    Unknown,
}}

struct KeyInfo {{
    canonical: &'static str,
    aliases: &'static [&'static str],
    win: WinKey,
    mouse: Option<MouseButton>,
}}

/// Canonical keys in dictionary order; index == [`Key::index`].
pub const ALL_KEYS: [Key; KEY_COUNT] = [{all_keys_src}];

/// Every accepted spelling, for diagnostics and tests.
pub const ALL_ALIASES: &[(&str, Key)] = &[
{all_aliases_src}
];

static KEY_TABLE: [KeyInfo; KEY_COUNT] = [
{infos_src}
];

/// Parse a key token using the frozen key language.
pub fn parse(token: &str) -> Option<Key> {{
    parse_detailed(token).ok()
}}

/// Parse with a machine-readable reason for the failure.
pub fn parse_detailed(token: &str) -> Result<Key, KeyParseError> {{
    let clean = token.trim().to_uppercase();
    if clean.is_empty() {{
        return Err(KeyParseError::Empty);
    }}
    let s = clean.strip_prefix("KEY_").unwrap_or(&clean);
    if is_numeric(s) {{
        return Err(KeyParseError::NumericPlatformSpecific);
    }}
    lookup(s).ok_or(KeyParseError::Unknown)
}}

fn is_numeric(s: &str) -> bool {{
    let digits = s.strip_prefix("CODE_").unwrap_or(s);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}}

fn lookup(s: &str) -> Option<Key> {{
    ALL_ALIASES
        .iter()
        .find(|(alias, _)| *alias == s)
        .map(|(_, key)| *key)
}}

/// Resolve a low-level hook event to a canonical key.
///
/// Windows reports a virtual-key code, the Set-1 scan code and the extended
/// flag. VK + extended is authoritative because it distinguishes Enter from
/// keypad Enter and Pause from NumLock; the scan code is only a fallback when
/// a driver reports an unexpected VK. Mouse VKs resolve to `None`; button
/// messages use [`MouseButton::key`].
pub fn from_windows(vk: u16, extended: bool, scan: u16) -> Option<Key> {{
    if let Some(key) = match (vk, extended) {{
{vk_arms}
        _ => None,
    }} {{
        return Some(key);
    }}
    if scan != 0 {{
        if let Some(key) = match (scan, extended) {{
{scan_arms}
            _ => None,
        }} {{
            return Some(key);
        }}
    }}
    // Be tolerant of a missing extended flag for unambiguous VKs.
    match vk {{
{unique_vk_arms}
        _ => None,
    }}
}}

impl MouseButton {{
    /// Canonical key this button is written as in profiles.
    pub fn key(self) -> Key {{
        match self {{
{mouse_arms}
        }}
    }}
}}

/// Left/right modifier variants are the same physical key class.
pub fn keys_are_equivalent(a: Key, b: Key) -> bool {{
    a.physical_class() == b.physical_class()
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn table_matches_contract_count() {{
        assert_eq!(KEY_COUNT, 119);
        assert_eq!(ALL_KEYS.len(), 119);
    }}

    #[test]
    fn every_alias_parses_to_its_key() {{
        for (alias, expected) in ALL_ALIASES {{
            // Bare digit aliases ("0".."9") are shadowed by numeric evdev
            // parsing on Linux and are platform-specific on Windows. The
            // canonical spelling KEY_<digit> always works.
            if alias.bytes().all(|b| b.is_ascii_digit()) {{
                assert_eq!(
                    parse_detailed(alias),
                    Err(KeyParseError::NumericPlatformSpecific),
                    "numeric alias {{alias}}"
                );
                continue;
            }}
            assert_eq!(parse(alias), Some(*expected), "alias {{alias}}");
            assert_eq!(parse(&alias.to_lowercase()), Some(*expected), "lowercase {{alias}}");
        }}
    }}

    #[test]
    fn canonical_names_round_trip_where_listed() {{
        for key in ALL_KEYS {{
            let canonical = key.canonical();
            if key.aliases().contains(&canonical) {{
                assert_eq!(parse(canonical), Some(key), "canonical {{canonical}}");
            }}
        }}
    }}

    #[test]
    fn optional_key_prefix_is_stripped_once() {{
        assert_eq!(parse("key_a"), Some(Key::KEY_A));
        assert_eq!(parse("KEY_F12"), Some(Key::KEY_F12));
        // Dictionary trap preserved from Linux: LEFT is the mouse button.
        assert_eq!(parse("KEY_LEFT"), Some(Key::BTN_LEFT));
        assert_eq!(parse("KEY_LEFT_ARROW"), Some(Key::KEY_LEFT));
        assert_eq!(parse("key_left_arrow"), Some(Key::KEY_LEFT));
    }}

    #[test]
    fn numeric_codes_are_rejected_on_windows() {{
        assert_eq!(parse_detailed("65"), Err(KeyParseError::NumericPlatformSpecific));
        assert_eq!(parse_detailed("CODE_65"), Err(KeyParseError::NumericPlatformSpecific));
        assert_eq!(parse_detailed("  code_32 "), Err(KeyParseError::NumericPlatformSpecific));
    }}

    #[test]
    fn unknown_and_empty_tokens_are_rejected() {{
        assert_eq!(parse_detailed(""), Err(KeyParseError::Empty));
        assert_eq!(parse_detailed("NOT_A_KEY"), Err(KeyParseError::Unknown));
        assert_eq!(parse_detailed("KEY_LEFTCTRL_EXTRA"), Err(KeyParseError::Unknown));
    }}

    #[test]
    fn left_and_right_modifiers_are_equivalent() {{
        assert!(keys_are_equivalent(Key::KEY_LEFTCTRL, Key::KEY_RIGHTCTRL));
        assert!(keys_are_equivalent(Key::KEY_LEFTSHIFT, Key::KEY_RIGHTSHIFT));
        assert!(keys_are_equivalent(Key::KEY_LEFTALT, Key::KEY_RIGHTALT));
        assert!(keys_are_equivalent(Key::KEY_LEFTMETA, Key::KEY_RIGHTMETA));
        assert!(!keys_are_equivalent(Key::KEY_LEFTCTRL, Key::KEY_LEFTSHIFT));
        assert!(keys_are_equivalent(Key::KEY_A, Key::KEY_A));
        assert!(!keys_are_equivalent(Key::KEY_A, Key::KEY_B));
    }}

    #[test]
    fn mouse_buttons_are_flagged() {{
        assert_eq!(Key::BTN_LEFT.mouse_button(), Some(MouseButton::Left));
        assert_eq!(Key::BTN_RIGHT.mouse_button(), Some(MouseButton::Right));
        assert_eq!(Key::BTN_MIDDLE.mouse_button(), Some(MouseButton::Middle));
        assert_eq!(Key::BTN_SIDE.mouse_button(), Some(MouseButton::Side));
        assert_eq!(Key::BTN_EXTRA.mouse_button(), Some(MouseButton::Extra));
        assert!(Key::KEY_A.mouse_button().is_none());
        assert!(Key::BTN_LEFT.is_mouse());
        assert!(!Key::KEY_A.is_mouse());
    }}

    #[test]
    fn windows_mapping_spot_checks() {{
        assert_eq!(Key::KEY_A.windows(), WinKey {{ vk: 0x41, scan_code: Some(0x1E), extended: false }});
        assert_eq!(Key::KEY_0.windows().vk, 0x30);
        assert_eq!(Key::KEY_F12.windows(), WinKey {{ vk: 0x7B, scan_code: Some(0x58), extended: false }});
        assert_eq!(Key::KEY_LEFT.windows(), WinKey {{ vk: 0x25, scan_code: Some(0x4B), extended: true }});
        assert_eq!(Key::KEY_RIGHTCTRL.windows(), WinKey {{ vk: 0xA3, scan_code: Some(0x1D), extended: true }});
        assert_eq!(Key::KEY_KP5.windows(), WinKey {{ vk: 0x65, scan_code: Some(0x4C), extended: false }});
        assert!(Key::KEY_KPENTER.windows().extended);
        assert_eq!(Key::KEY_PAUSE.windows().scan_code, None);
        assert_eq!(Key::KEY_F13.windows().scan_code, None);
    }}

    #[test]
    fn every_key_has_a_mapping() {{
        for key in ALL_KEYS {{
            let win = key.windows();
            if key.is_mouse() {{
                assert_eq!(win.vk, 0);
                assert_eq!(win.scan_code, None);
            }} else {{
                assert_ne!(win.vk, 0, "{{key}} has no VK");
            }}
        }}
    }}

    #[test]
    fn every_keyboard_key_round_trips_through_windows() {{
        for key in ALL_KEYS {{
            if key.is_mouse() {{
                continue;
            }}
            let win = key.windows();
            let scan = win.scan_code.unwrap_or(0);
            assert_eq!(
                from_windows(win.vk, win.extended, scan),
                Some(key),
                "round trip for {{key}}"
            );
        }}
    }}

    #[test]
    fn windows_ambiguities_are_resolved() {{
        assert_eq!(from_windows(0x0D, false, 0x1C), Some(Key::KEY_ENTER));
        assert_eq!(from_windows(0x0D, true, 0x1C), Some(Key::KEY_KPENTER));
        // Pause and NumLock share scan code 0x45; VK wins.
        assert_eq!(from_windows(0x13, false, 0x45), Some(Key::KEY_PAUSE));
        assert_eq!(from_windows(0x90, false, 0x45), Some(Key::KEY_NUMLOCK));
        // Scan fallback when the VK is unknown/zero.
        assert_eq!(from_windows(0, false, 0x1E), Some(Key::KEY_A));
        assert_eq!(from_windows(0xFF, false, 0x00), None);
        // Mouse virtual-key codes are not keyboard keys.
        assert_eq!(from_windows(0x01, false, 0), None);
    }}

    #[test]
    fn mouse_buttons_map_to_canonical_keys() {{
        for button in [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Side,
            MouseButton::Extra,
        ] {{
            assert_eq!(button.key().mouse_button(), Some(button));
        }}
    }}

    #[test]
    fn debug_uses_canonical_name() {{
        assert_eq!(format!("{{:?}}", Key::KEY_LEFTCTRL), "KEY_LEFTCTRL");
        assert_eq!(format!("{{}}", Key::BTN_LEFT), "BTN_LEFT");
    }}
}}
'''


def resolve_rustfmt() -> list[str] | None:
    """Locate rustfmt, falling back to rustup when it is not on PATH.

    GitHub Actions installs the component with `rustup component add rustfmt`,
    but the proxy directory is not guaranteed to be first on PATH (and some
    images only expose `rustup`). Resolving through rustup keeps the generator
    deterministic instead of silently emitting unformatted source.
    """
    found = shutil.which("rustfmt")
    if found:
        return [found]
    rustup = shutil.which("rustup")
    if rustup:
        try:
            resolved = subprocess.run(
                [rustup, "which", "rustfmt"],
                text=True,
                capture_output=True,
                check=True,
            )
        except (OSError, subprocess.CalledProcessError):
            return None
        path = resolved.stdout.strip()
        if path and Path(path).is_file():
            return [path]
    return None


def format_source(source: str) -> str:
    """Format with rustfmt so `--check` matches `cargo fmt`.

    A missing or failing rustfmt is a hard error: writing or comparing
    unformatted output would make the check depend on the environment and
    produce a misleading "out of date" result. CI installs the component in
    the `core` job (`rustup component add rustfmt`).

    Source and output are exchanged as explicit UTF-8 bytes. Text mode would
    use the platform default encoding (e.g. cp1252 on the Windows runner),
    which rustfmt rejects with "stream did not contain valid UTF-8".
    """
    command = resolve_rustfmt()
    if command is None:
        raise SystemExit(
            "rustfmt not found; install it with `rustup component add rustfmt` "
            "before running tools/gen_keymap.py"
        )
    try:
        result = subprocess.run(
            command + ["--edition", "2021", "--emit", "stdout"],
            input=source.encode("utf-8"),
            capture_output=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        stderr = getattr(error, "stderr", None)
        if isinstance(stderr, bytes):
            detail = stderr.decode("utf-8", errors="replace")
        else:
            detail = stderr or str(error)
        raise SystemExit(f"rustfmt failed: {detail.strip() or error}")
    return result.stdout.decode("utf-8")


def main() -> int:
    source = format_source(generate())
    if "--check" in sys.argv:
        current = OUT_PATH.read_text(encoding="utf-8") if OUT_PATH.exists() else ""
        if current != source:
            print(f"{OUT_PATH} is out of date; run tools/gen_keymap.py", file=sys.stderr)
            return 1
        print(f"{OUT_PATH} is up to date")
        return 0
    OUT_PATH.write_text(source, encoding="utf-8")
    print(f"wrote {OUT_PATH} ({len(source)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
