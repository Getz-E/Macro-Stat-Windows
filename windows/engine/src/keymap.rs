// SPDX-License-Identifier: GPL-3.0-or-later
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
pub const KEY_COUNT: usize = 119;

/// A canonical MacroStat key (keyboard key or mouse button).
///
/// The variant/identity is an opaque index into [`KEY_TABLE`]; use the
/// associated `KEY_*`/`BTN_*` constants for named access.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key(u8);

impl Key {
    pub const BTN_EXTRA: Key = Key(0);
    pub const BTN_LEFT: Key = Key(1);
    pub const BTN_MIDDLE: Key = Key(2);
    pub const BTN_RIGHT: Key = Key(3);
    pub const BTN_SIDE: Key = Key(4);
    pub const KEY_0: Key = Key(5);
    pub const KEY_1: Key = Key(6);
    pub const KEY_2: Key = Key(7);
    pub const KEY_3: Key = Key(8);
    pub const KEY_4: Key = Key(9);
    pub const KEY_5: Key = Key(10);
    pub const KEY_6: Key = Key(11);
    pub const KEY_7: Key = Key(12);
    pub const KEY_8: Key = Key(13);
    pub const KEY_9: Key = Key(14);
    pub const KEY_A: Key = Key(15);
    pub const KEY_APOSTROPHE: Key = Key(16);
    pub const KEY_B: Key = Key(17);
    pub const KEY_BACKSLASH: Key = Key(18);
    pub const KEY_BACKSPACE: Key = Key(19);
    pub const KEY_C: Key = Key(20);
    pub const KEY_CAPSLOCK: Key = Key(21);
    pub const KEY_COMMA: Key = Key(22);
    pub const KEY_D: Key = Key(23);
    pub const KEY_DELETE: Key = Key(24);
    pub const KEY_DOT: Key = Key(25);
    pub const KEY_DOWN: Key = Key(26);
    pub const KEY_E: Key = Key(27);
    pub const KEY_END: Key = Key(28);
    pub const KEY_ENTER: Key = Key(29);
    pub const KEY_EQUAL: Key = Key(30);
    pub const KEY_ESC: Key = Key(31);
    pub const KEY_F: Key = Key(32);
    pub const KEY_F1: Key = Key(33);
    pub const KEY_F10: Key = Key(34);
    pub const KEY_F11: Key = Key(35);
    pub const KEY_F12: Key = Key(36);
    pub const KEY_F13: Key = Key(37);
    pub const KEY_F14: Key = Key(38);
    pub const KEY_F15: Key = Key(39);
    pub const KEY_F16: Key = Key(40);
    pub const KEY_F17: Key = Key(41);
    pub const KEY_F18: Key = Key(42);
    pub const KEY_F19: Key = Key(43);
    pub const KEY_F2: Key = Key(44);
    pub const KEY_F20: Key = Key(45);
    pub const KEY_F21: Key = Key(46);
    pub const KEY_F22: Key = Key(47);
    pub const KEY_F23: Key = Key(48);
    pub const KEY_F24: Key = Key(49);
    pub const KEY_F3: Key = Key(50);
    pub const KEY_F4: Key = Key(51);
    pub const KEY_F5: Key = Key(52);
    pub const KEY_F6: Key = Key(53);
    pub const KEY_F7: Key = Key(54);
    pub const KEY_F8: Key = Key(55);
    pub const KEY_F9: Key = Key(56);
    pub const KEY_G: Key = Key(57);
    pub const KEY_GRAVE: Key = Key(58);
    pub const KEY_H: Key = Key(59);
    pub const KEY_HOME: Key = Key(60);
    pub const KEY_I: Key = Key(61);
    pub const KEY_INSERT: Key = Key(62);
    pub const KEY_J: Key = Key(63);
    pub const KEY_K: Key = Key(64);
    pub const KEY_KP0: Key = Key(65);
    pub const KEY_KP1: Key = Key(66);
    pub const KEY_KP2: Key = Key(67);
    pub const KEY_KP3: Key = Key(68);
    pub const KEY_KP4: Key = Key(69);
    pub const KEY_KP5: Key = Key(70);
    pub const KEY_KP6: Key = Key(71);
    pub const KEY_KP7: Key = Key(72);
    pub const KEY_KP8: Key = Key(73);
    pub const KEY_KP9: Key = Key(74);
    pub const KEY_KPASTERISK: Key = Key(75);
    pub const KEY_KPDOT: Key = Key(76);
    pub const KEY_KPENTER: Key = Key(77);
    pub const KEY_KPMINUS: Key = Key(78);
    pub const KEY_KPPLUS: Key = Key(79);
    pub const KEY_KPSLASH: Key = Key(80);
    pub const KEY_L: Key = Key(81);
    pub const KEY_LEFT: Key = Key(82);
    pub const KEY_LEFTALT: Key = Key(83);
    pub const KEY_LEFTBRACE: Key = Key(84);
    pub const KEY_LEFTCTRL: Key = Key(85);
    pub const KEY_LEFTMETA: Key = Key(86);
    pub const KEY_LEFTSHIFT: Key = Key(87);
    pub const KEY_M: Key = Key(88);
    pub const KEY_MINUS: Key = Key(89);
    pub const KEY_N: Key = Key(90);
    pub const KEY_NUMLOCK: Key = Key(91);
    pub const KEY_O: Key = Key(92);
    pub const KEY_P: Key = Key(93);
    pub const KEY_PAGEDOWN: Key = Key(94);
    pub const KEY_PAGEUP: Key = Key(95);
    pub const KEY_PAUSE: Key = Key(96);
    pub const KEY_Q: Key = Key(97);
    pub const KEY_R: Key = Key(98);
    pub const KEY_RIGHT: Key = Key(99);
    pub const KEY_RIGHTALT: Key = Key(100);
    pub const KEY_RIGHTBRACE: Key = Key(101);
    pub const KEY_RIGHTCTRL: Key = Key(102);
    pub const KEY_RIGHTMETA: Key = Key(103);
    pub const KEY_RIGHTSHIFT: Key = Key(104);
    pub const KEY_S: Key = Key(105);
    pub const KEY_SCROLLLOCK: Key = Key(106);
    pub const KEY_SEMICOLON: Key = Key(107);
    pub const KEY_SLASH: Key = Key(108);
    pub const KEY_SPACE: Key = Key(109);
    pub const KEY_T: Key = Key(110);
    pub const KEY_TAB: Key = Key(111);
    pub const KEY_U: Key = Key(112);
    pub const KEY_UP: Key = Key(113);
    pub const KEY_V: Key = Key(114);
    pub const KEY_W: Key = Key(115);
    pub const KEY_X: Key = Key(116);
    pub const KEY_Y: Key = Key(117);
    pub const KEY_Z: Key = Key(118);

    /// Stable internal index in `0..KEY_COUNT`.
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Canonical dictionary name, e.g. `"KEY_LEFTCTRL"`.
    pub fn canonical(self) -> &'static str {
        KEY_TABLE[self.0 as usize].canonical
    }

    /// Every accepted spelling for this key, including the canonical name
    /// when the dictionary lists it as a valid alias.
    pub fn aliases(self) -> &'static [&'static str] {
        KEY_TABLE[self.0 as usize].aliases
    }

    /// Windows virtual-key code / scan code / extended flag.
    pub fn windows(self) -> WinKey {
        KEY_TABLE[self.0 as usize].win
    }

    /// `Some(button)` for mouse buttons, `None` for keyboard keys.
    pub fn mouse_button(self) -> Option<MouseButton> {
        KEY_TABLE[self.0 as usize].mouse
    }

    pub fn is_mouse(self) -> bool {
        self.mouse_button().is_some()
    }

    /// Left/right modifier variants collapse to the left key; everything else
    /// is its own class. Used for combo latching and double-tap tracking.
    pub fn physical_class(self) -> Key {
        match self {
            Key::KEY_RIGHTCTRL => Key::KEY_LEFTCTRL,
            Key::KEY_RIGHTSHIFT => Key::KEY_LEFTSHIFT,
            Key::KEY_RIGHTALT => Key::KEY_LEFTALT,
            Key::KEY_RIGHTMETA => Key::KEY_LEFTMETA,
            _ => self,
        }
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical())
    }
}

/// Declarative Windows mapping for one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinKey {
    /// Windows virtual-key code. `0` for mouse buttons.
    pub vk: u16,
    /// Set-1 scan code, or `None` when only the VK path is safe.
    pub scan_code: Option<u16>,
    /// `KEYEVENTF_EXTENDEDKEY` is required for this key.
    pub extended: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Side,
    Extra,
}

/// Why a token could not be resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyParseError {
    /// The token was empty after trimming.
    Empty,
    /// `CODE_<n>` or a bare numeric evdev code: meaningful on Linux only.
    NumericPlatformSpecific,
    /// Not a canonical name or alias in the dictionary.
    Unknown,
}

struct KeyInfo {
    canonical: &'static str,
    aliases: &'static [&'static str],
    win: WinKey,
    mouse: Option<MouseButton>,
}

/// Canonical keys in dictionary order; index == [`Key::index`].
pub const ALL_KEYS: [Key; KEY_COUNT] = [
    Key(0),
    Key(1),
    Key(2),
    Key(3),
    Key(4),
    Key(5),
    Key(6),
    Key(7),
    Key(8),
    Key(9),
    Key(10),
    Key(11),
    Key(12),
    Key(13),
    Key(14),
    Key(15),
    Key(16),
    Key(17),
    Key(18),
    Key(19),
    Key(20),
    Key(21),
    Key(22),
    Key(23),
    Key(24),
    Key(25),
    Key(26),
    Key(27),
    Key(28),
    Key(29),
    Key(30),
    Key(31),
    Key(32),
    Key(33),
    Key(34),
    Key(35),
    Key(36),
    Key(37),
    Key(38),
    Key(39),
    Key(40),
    Key(41),
    Key(42),
    Key(43),
    Key(44),
    Key(45),
    Key(46),
    Key(47),
    Key(48),
    Key(49),
    Key(50),
    Key(51),
    Key(52),
    Key(53),
    Key(54),
    Key(55),
    Key(56),
    Key(57),
    Key(58),
    Key(59),
    Key(60),
    Key(61),
    Key(62),
    Key(63),
    Key(64),
    Key(65),
    Key(66),
    Key(67),
    Key(68),
    Key(69),
    Key(70),
    Key(71),
    Key(72),
    Key(73),
    Key(74),
    Key(75),
    Key(76),
    Key(77),
    Key(78),
    Key(79),
    Key(80),
    Key(81),
    Key(82),
    Key(83),
    Key(84),
    Key(85),
    Key(86),
    Key(87),
    Key(88),
    Key(89),
    Key(90),
    Key(91),
    Key(92),
    Key(93),
    Key(94),
    Key(95),
    Key(96),
    Key(97),
    Key(98),
    Key(99),
    Key(100),
    Key(101),
    Key(102),
    Key(103),
    Key(104),
    Key(105),
    Key(106),
    Key(107),
    Key(108),
    Key(109),
    Key(110),
    Key(111),
    Key(112),
    Key(113),
    Key(114),
    Key(115),
    Key(116),
    Key(117),
    Key(118),
];

/// Every accepted spelling, for diagnostics and tests.
pub const ALL_ALIASES: &[(&str, Key)] = &[
    ("BTN_EXTRA", Key(0)),
    ("EXTRA", Key(0)),
    ("MOUSE_5", Key(0)),
    ("MOUSE5", Key(0)),
    ("M5", Key(0)),
    ("M5_SIDE", Key(0)),
    ("BTN_FORWARD", Key(0)),
    ("FORWARD", Key(0)),
    ("BTN_LEFT", Key(1)),
    ("LEFT", Key(1)),
    ("MOUSE_1", Key(1)),
    ("MOUSE1", Key(1)),
    ("M1", Key(1)),
    ("LEFTCLICK", Key(1)),
    ("BTN_MIDDLE", Key(2)),
    ("MIDDLE", Key(2)),
    ("MOUSE_3", Key(2)),
    ("MOUSE3", Key(2)),
    ("M3", Key(2)),
    ("MIDDLECLICK", Key(2)),
    ("BTN_RIGHT", Key(3)),
    ("RIGHT", Key(3)),
    ("MOUSE_2", Key(3)),
    ("MOUSE2", Key(3)),
    ("M2", Key(3)),
    ("RIGHTCLICK", Key(3)),
    ("BTN_SIDE", Key(4)),
    ("SIDE", Key(4)),
    ("MOUSE_4", Key(4)),
    ("MOUSE4", Key(4)),
    ("M4", Key(4)),
    ("M4_SIDE", Key(4)),
    ("BTN_BACK", Key(4)),
    ("BACK", Key(4)),
    ("0", Key(5)),
    ("1", Key(6)),
    ("2", Key(7)),
    ("3", Key(8)),
    ("4", Key(9)),
    ("5", Key(10)),
    ("6", Key(11)),
    ("7", Key(12)),
    ("8", Key(13)),
    ("9", Key(14)),
    ("A", Key(15)),
    ("APOSTROPHE", Key(16)),
    ("QUOTE", Key(16)),
    ("B", Key(17)),
    ("BACKSLASH", Key(18)),
    ("BACKSPACE", Key(19)),
    ("C", Key(20)),
    ("CAPSLOCK", Key(21)),
    ("CAPS", Key(21)),
    ("COMMA", Key(22)),
    ("D", Key(23)),
    ("DELETE", Key(24)),
    ("DEL", Key(24)),
    ("DOT", Key(25)),
    ("PERIOD", Key(25)),
    ("DOWN", Key(26)),
    ("ARROWDOWN", Key(26)),
    ("E", Key(27)),
    ("END", Key(28)),
    ("ENTER", Key(29)),
    ("RETURN", Key(29)),
    ("EQUAL", Key(30)),
    ("EQUALS", Key(30)),
    ("ESC", Key(31)),
    ("ESCAPE", Key(31)),
    ("F", Key(32)),
    ("F1", Key(33)),
    ("F10", Key(34)),
    ("F11", Key(35)),
    ("F12", Key(36)),
    ("F13", Key(37)),
    ("F14", Key(38)),
    ("F15", Key(39)),
    ("F16", Key(40)),
    ("F17", Key(41)),
    ("F18", Key(42)),
    ("F19", Key(43)),
    ("F2", Key(44)),
    ("F20", Key(45)),
    ("F21", Key(46)),
    ("F22", Key(47)),
    ("F23", Key(48)),
    ("F24", Key(49)),
    ("F3", Key(50)),
    ("F4", Key(51)),
    ("F5", Key(52)),
    ("F6", Key(53)),
    ("F7", Key(54)),
    ("F8", Key(55)),
    ("F9", Key(56)),
    ("G", Key(57)),
    ("GRAVE", Key(58)),
    ("TILDE", Key(58)),
    ("BACKTICK", Key(58)),
    ("H", Key(59)),
    ("HOME", Key(60)),
    ("I", Key(61)),
    ("INSERT", Key(62)),
    ("INS", Key(62)),
    ("J", Key(63)),
    ("K", Key(64)),
    ("KP0", Key(65)),
    ("KP_0", Key(65)),
    ("NUMPAD0", Key(65)),
    ("KP1", Key(66)),
    ("KP_1", Key(66)),
    ("NUMPAD1", Key(66)),
    ("KP2", Key(67)),
    ("KP_2", Key(67)),
    ("NUMPAD2", Key(67)),
    ("KP3", Key(68)),
    ("KP_3", Key(68)),
    ("NUMPAD3", Key(68)),
    ("KP4", Key(69)),
    ("KP_4", Key(69)),
    ("NUMPAD4", Key(69)),
    ("KP5", Key(70)),
    ("KP_5", Key(70)),
    ("NUMPAD5", Key(70)),
    ("KP6", Key(71)),
    ("KP_6", Key(71)),
    ("NUMPAD6", Key(71)),
    ("KP7", Key(72)),
    ("KP_7", Key(72)),
    ("NUMPAD7", Key(72)),
    ("KP8", Key(73)),
    ("KP_8", Key(73)),
    ("NUMPAD8", Key(73)),
    ("KP9", Key(74)),
    ("KP_9", Key(74)),
    ("NUMPAD9", Key(74)),
    ("KPASTERISK", Key(75)),
    ("KPDOT", Key(76)),
    ("KPENTER", Key(77)),
    ("KPMINUS", Key(78)),
    ("KPPLUS", Key(79)),
    ("KPSLASH", Key(80)),
    ("L", Key(81)),
    ("LEFT_ARROW", Key(82)),
    ("ARROWLEFT", Key(82)),
    ("LEFTALT", Key(83)),
    ("ALT", Key(83)),
    ("LALT", Key(83)),
    ("LEFTBRACE", Key(84)),
    ("LBRACE", Key(84)),
    ("BRACKETLEFT", Key(84)),
    ("LEFTCTRL", Key(85)),
    ("CTRL", Key(85)),
    ("LCTRL", Key(85)),
    ("CONTROL", Key(85)),
    ("LEFTMETA", Key(86)),
    ("SUPER", Key(86)),
    ("META", Key(86)),
    ("WIN", Key(86)),
    ("WINDOWS", Key(86)),
    ("LEFTSHIFT", Key(87)),
    ("SHIFT", Key(87)),
    ("LSHIFT", Key(87)),
    ("M", Key(88)),
    ("MINUS", Key(89)),
    ("DASH", Key(89)),
    ("N", Key(90)),
    ("NUMLOCK", Key(91)),
    ("O", Key(92)),
    ("P", Key(93)),
    ("PAGEDOWN", Key(94)),
    ("PGDN", Key(94)),
    ("PAGEUP", Key(95)),
    ("PGUP", Key(95)),
    ("PAUSE", Key(96)),
    ("BREAK", Key(96)),
    ("Q", Key(97)),
    ("R", Key(98)),
    ("RIGHT_ARROW", Key(99)),
    ("ARROWRIGHT", Key(99)),
    ("RIGHTALT", Key(100)),
    ("RALT", Key(100)),
    ("RIGHTBRACE", Key(101)),
    ("RBRACE", Key(101)),
    ("BRACKETRIGHT", Key(101)),
    ("RIGHTCTRL", Key(102)),
    ("RCTRL", Key(102)),
    ("RIGHTMETA", Key(103)),
    ("RIGHTSHIFT", Key(104)),
    ("RSHIFT", Key(104)),
    ("S", Key(105)),
    ("SCROLLLOCK", Key(106)),
    ("SEMICOLON", Key(107)),
    ("SLASH", Key(108)),
    ("SPACE", Key(109)),
    ("T", Key(110)),
    ("TAB", Key(111)),
    ("U", Key(112)),
    ("UP", Key(113)),
    ("ARROWUP", Key(113)),
    ("V", Key(114)),
    ("W", Key(115)),
    ("X", Key(116)),
    ("Y", Key(117)),
    ("Z", Key(118)),
];

static KEY_TABLE: [KeyInfo; KEY_COUNT] = [
    KeyInfo {
        canonical: "BTN_EXTRA",
        aliases: &[
            "BTN_EXTRA",
            "EXTRA",
            "MOUSE_5",
            "MOUSE5",
            "M5",
            "M5_SIDE",
            "BTN_FORWARD",
            "FORWARD",
        ],
        win: WinKey {
            vk: 0x00,
            scan_code: None,
            extended: false,
        },
        mouse: Some(MouseButton::Extra),
    },
    KeyInfo {
        canonical: "BTN_LEFT",
        aliases: &["BTN_LEFT", "LEFT", "MOUSE_1", "MOUSE1", "M1", "LEFTCLICK"],
        win: WinKey {
            vk: 0x00,
            scan_code: None,
            extended: false,
        },
        mouse: Some(MouseButton::Left),
    },
    KeyInfo {
        canonical: "BTN_MIDDLE",
        aliases: &[
            "BTN_MIDDLE",
            "MIDDLE",
            "MOUSE_3",
            "MOUSE3",
            "M3",
            "MIDDLECLICK",
        ],
        win: WinKey {
            vk: 0x00,
            scan_code: None,
            extended: false,
        },
        mouse: Some(MouseButton::Middle),
    },
    KeyInfo {
        canonical: "BTN_RIGHT",
        aliases: &[
            "BTN_RIGHT",
            "RIGHT",
            "MOUSE_2",
            "MOUSE2",
            "M2",
            "RIGHTCLICK",
        ],
        win: WinKey {
            vk: 0x00,
            scan_code: None,
            extended: false,
        },
        mouse: Some(MouseButton::Right),
    },
    KeyInfo {
        canonical: "BTN_SIDE",
        aliases: &[
            "BTN_SIDE", "SIDE", "MOUSE_4", "MOUSE4", "M4", "M4_SIDE", "BTN_BACK", "BACK",
        ],
        win: WinKey {
            vk: 0x00,
            scan_code: None,
            extended: false,
        },
        mouse: Some(MouseButton::Side),
    },
    KeyInfo {
        canonical: "KEY_0",
        aliases: &["0"],
        win: WinKey {
            vk: 0x30,
            scan_code: Some(0x0B),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_1",
        aliases: &["1"],
        win: WinKey {
            vk: 0x31,
            scan_code: Some(0x02),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_2",
        aliases: &["2"],
        win: WinKey {
            vk: 0x32,
            scan_code: Some(0x03),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_3",
        aliases: &["3"],
        win: WinKey {
            vk: 0x33,
            scan_code: Some(0x04),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_4",
        aliases: &["4"],
        win: WinKey {
            vk: 0x34,
            scan_code: Some(0x05),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_5",
        aliases: &["5"],
        win: WinKey {
            vk: 0x35,
            scan_code: Some(0x06),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_6",
        aliases: &["6"],
        win: WinKey {
            vk: 0x36,
            scan_code: Some(0x07),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_7",
        aliases: &["7"],
        win: WinKey {
            vk: 0x37,
            scan_code: Some(0x08),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_8",
        aliases: &["8"],
        win: WinKey {
            vk: 0x38,
            scan_code: Some(0x09),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_9",
        aliases: &["9"],
        win: WinKey {
            vk: 0x39,
            scan_code: Some(0x0A),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_A",
        aliases: &["A"],
        win: WinKey {
            vk: 0x41,
            scan_code: Some(0x1E),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_APOSTROPHE",
        aliases: &["APOSTROPHE", "QUOTE"],
        win: WinKey {
            vk: 0xDE,
            scan_code: Some(0x28),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_B",
        aliases: &["B"],
        win: WinKey {
            vk: 0x42,
            scan_code: Some(0x30),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_BACKSLASH",
        aliases: &["BACKSLASH"],
        win: WinKey {
            vk: 0xDC,
            scan_code: Some(0x2B),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_BACKSPACE",
        aliases: &["BACKSPACE"],
        win: WinKey {
            vk: 0x08,
            scan_code: Some(0x0E),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_C",
        aliases: &["C"],
        win: WinKey {
            vk: 0x43,
            scan_code: Some(0x2E),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_CAPSLOCK",
        aliases: &["CAPSLOCK", "CAPS"],
        win: WinKey {
            vk: 0x14,
            scan_code: Some(0x3A),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_COMMA",
        aliases: &["COMMA"],
        win: WinKey {
            vk: 0xBC,
            scan_code: Some(0x33),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_D",
        aliases: &["D"],
        win: WinKey {
            vk: 0x44,
            scan_code: Some(0x20),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_DELETE",
        aliases: &["DELETE", "DEL"],
        win: WinKey {
            vk: 0x2E,
            scan_code: Some(0x53),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_DOT",
        aliases: &["DOT", "PERIOD"],
        win: WinKey {
            vk: 0xBE,
            scan_code: Some(0x34),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_DOWN",
        aliases: &["DOWN", "ARROWDOWN"],
        win: WinKey {
            vk: 0x28,
            scan_code: Some(0x50),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_E",
        aliases: &["E"],
        win: WinKey {
            vk: 0x45,
            scan_code: Some(0x12),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_END",
        aliases: &["END"],
        win: WinKey {
            vk: 0x23,
            scan_code: Some(0x4F),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_ENTER",
        aliases: &["ENTER", "RETURN"],
        win: WinKey {
            vk: 0x0D,
            scan_code: Some(0x1C),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_EQUAL",
        aliases: &["EQUAL", "EQUALS"],
        win: WinKey {
            vk: 0xBB,
            scan_code: Some(0x0D),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_ESC",
        aliases: &["ESC", "ESCAPE"],
        win: WinKey {
            vk: 0x1B,
            scan_code: Some(0x01),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F",
        aliases: &["F"],
        win: WinKey {
            vk: 0x46,
            scan_code: Some(0x21),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F1",
        aliases: &["F1"],
        win: WinKey {
            vk: 0x70,
            scan_code: Some(0x3B),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F10",
        aliases: &["F10"],
        win: WinKey {
            vk: 0x79,
            scan_code: Some(0x44),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F11",
        aliases: &["F11"],
        win: WinKey {
            vk: 0x7A,
            scan_code: Some(0x57),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F12",
        aliases: &["F12"],
        win: WinKey {
            vk: 0x7B,
            scan_code: Some(0x58),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F13",
        aliases: &["F13"],
        win: WinKey {
            vk: 0x7C,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F14",
        aliases: &["F14"],
        win: WinKey {
            vk: 0x7D,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F15",
        aliases: &["F15"],
        win: WinKey {
            vk: 0x7E,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F16",
        aliases: &["F16"],
        win: WinKey {
            vk: 0x7F,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F17",
        aliases: &["F17"],
        win: WinKey {
            vk: 0x80,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F18",
        aliases: &["F18"],
        win: WinKey {
            vk: 0x81,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F19",
        aliases: &["F19"],
        win: WinKey {
            vk: 0x82,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F2",
        aliases: &["F2"],
        win: WinKey {
            vk: 0x71,
            scan_code: Some(0x3C),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F20",
        aliases: &["F20"],
        win: WinKey {
            vk: 0x83,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F21",
        aliases: &["F21"],
        win: WinKey {
            vk: 0x84,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F22",
        aliases: &["F22"],
        win: WinKey {
            vk: 0x85,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F23",
        aliases: &["F23"],
        win: WinKey {
            vk: 0x86,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F24",
        aliases: &["F24"],
        win: WinKey {
            vk: 0x87,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F3",
        aliases: &["F3"],
        win: WinKey {
            vk: 0x72,
            scan_code: Some(0x3D),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F4",
        aliases: &["F4"],
        win: WinKey {
            vk: 0x73,
            scan_code: Some(0x3E),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F5",
        aliases: &["F5"],
        win: WinKey {
            vk: 0x74,
            scan_code: Some(0x3F),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F6",
        aliases: &["F6"],
        win: WinKey {
            vk: 0x75,
            scan_code: Some(0x40),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F7",
        aliases: &["F7"],
        win: WinKey {
            vk: 0x76,
            scan_code: Some(0x41),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F8",
        aliases: &["F8"],
        win: WinKey {
            vk: 0x77,
            scan_code: Some(0x42),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_F9",
        aliases: &["F9"],
        win: WinKey {
            vk: 0x78,
            scan_code: Some(0x43),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_G",
        aliases: &["G"],
        win: WinKey {
            vk: 0x47,
            scan_code: Some(0x22),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_GRAVE",
        aliases: &["GRAVE", "TILDE", "BACKTICK"],
        win: WinKey {
            vk: 0xC0,
            scan_code: Some(0x29),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_H",
        aliases: &["H"],
        win: WinKey {
            vk: 0x48,
            scan_code: Some(0x23),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_HOME",
        aliases: &["HOME"],
        win: WinKey {
            vk: 0x24,
            scan_code: Some(0x47),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_I",
        aliases: &["I"],
        win: WinKey {
            vk: 0x49,
            scan_code: Some(0x17),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_INSERT",
        aliases: &["INSERT", "INS"],
        win: WinKey {
            vk: 0x2D,
            scan_code: Some(0x52),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_J",
        aliases: &["J"],
        win: WinKey {
            vk: 0x4A,
            scan_code: Some(0x24),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_K",
        aliases: &["K"],
        win: WinKey {
            vk: 0x4B,
            scan_code: Some(0x25),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP0",
        aliases: &["KP0", "KP_0", "NUMPAD0"],
        win: WinKey {
            vk: 0x60,
            scan_code: Some(0x52),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP1",
        aliases: &["KP1", "KP_1", "NUMPAD1"],
        win: WinKey {
            vk: 0x61,
            scan_code: Some(0x4F),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP2",
        aliases: &["KP2", "KP_2", "NUMPAD2"],
        win: WinKey {
            vk: 0x62,
            scan_code: Some(0x50),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP3",
        aliases: &["KP3", "KP_3", "NUMPAD3"],
        win: WinKey {
            vk: 0x63,
            scan_code: Some(0x51),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP4",
        aliases: &["KP4", "KP_4", "NUMPAD4"],
        win: WinKey {
            vk: 0x64,
            scan_code: Some(0x4B),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP5",
        aliases: &["KP5", "KP_5", "NUMPAD5"],
        win: WinKey {
            vk: 0x65,
            scan_code: Some(0x4C),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP6",
        aliases: &["KP6", "KP_6", "NUMPAD6"],
        win: WinKey {
            vk: 0x66,
            scan_code: Some(0x4D),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP7",
        aliases: &["KP7", "KP_7", "NUMPAD7"],
        win: WinKey {
            vk: 0x67,
            scan_code: Some(0x47),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP8",
        aliases: &["KP8", "KP_8", "NUMPAD8"],
        win: WinKey {
            vk: 0x68,
            scan_code: Some(0x48),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KP9",
        aliases: &["KP9", "KP_9", "NUMPAD9"],
        win: WinKey {
            vk: 0x69,
            scan_code: Some(0x49),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KPASTERISK",
        aliases: &["KPASTERISK"],
        win: WinKey {
            vk: 0x6A,
            scan_code: Some(0x37),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KPDOT",
        aliases: &["KPDOT"],
        win: WinKey {
            vk: 0x6E,
            scan_code: Some(0x53),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KPENTER",
        aliases: &["KPENTER"],
        win: WinKey {
            vk: 0x0D,
            scan_code: Some(0x1C),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KPMINUS",
        aliases: &["KPMINUS"],
        win: WinKey {
            vk: 0x6C,
            scan_code: Some(0x4A),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KPPLUS",
        aliases: &["KPPLUS"],
        win: WinKey {
            vk: 0x6B,
            scan_code: Some(0x4E),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_KPSLASH",
        aliases: &["KPSLASH"],
        win: WinKey {
            vk: 0x6F,
            scan_code: Some(0x35),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_L",
        aliases: &["L"],
        win: WinKey {
            vk: 0x4C,
            scan_code: Some(0x26),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_LEFT",
        aliases: &["LEFT_ARROW", "ARROWLEFT"],
        win: WinKey {
            vk: 0x25,
            scan_code: Some(0x4B),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_LEFTALT",
        aliases: &["LEFTALT", "ALT", "LALT"],
        win: WinKey {
            vk: 0xA4,
            scan_code: Some(0x38),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_LEFTBRACE",
        aliases: &["LEFTBRACE", "LBRACE", "BRACKETLEFT"],
        win: WinKey {
            vk: 0xDB,
            scan_code: Some(0x1A),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_LEFTCTRL",
        aliases: &["LEFTCTRL", "CTRL", "LCTRL", "CONTROL"],
        win: WinKey {
            vk: 0xA2,
            scan_code: Some(0x1D),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_LEFTMETA",
        aliases: &["LEFTMETA", "SUPER", "META", "WIN", "WINDOWS"],
        win: WinKey {
            vk: 0x5B,
            scan_code: Some(0x5B),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_LEFTSHIFT",
        aliases: &["LEFTSHIFT", "SHIFT", "LSHIFT"],
        win: WinKey {
            vk: 0xA0,
            scan_code: Some(0x2A),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_M",
        aliases: &["M"],
        win: WinKey {
            vk: 0x4D,
            scan_code: Some(0x32),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_MINUS",
        aliases: &["MINUS", "DASH"],
        win: WinKey {
            vk: 0xBD,
            scan_code: Some(0x0C),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_N",
        aliases: &["N"],
        win: WinKey {
            vk: 0x4E,
            scan_code: Some(0x31),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_NUMLOCK",
        aliases: &["NUMLOCK"],
        win: WinKey {
            vk: 0x90,
            scan_code: Some(0x45),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_O",
        aliases: &["O"],
        win: WinKey {
            vk: 0x4F,
            scan_code: Some(0x18),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_P",
        aliases: &["P"],
        win: WinKey {
            vk: 0x50,
            scan_code: Some(0x19),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_PAGEDOWN",
        aliases: &["PAGEDOWN", "PGDN"],
        win: WinKey {
            vk: 0x22,
            scan_code: Some(0x51),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_PAGEUP",
        aliases: &["PAGEUP", "PGUP"],
        win: WinKey {
            vk: 0x21,
            scan_code: Some(0x49),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_PAUSE",
        aliases: &["PAUSE", "BREAK"],
        win: WinKey {
            vk: 0x13,
            scan_code: None,
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_Q",
        aliases: &["Q"],
        win: WinKey {
            vk: 0x51,
            scan_code: Some(0x10),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_R",
        aliases: &["R"],
        win: WinKey {
            vk: 0x52,
            scan_code: Some(0x13),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_RIGHT",
        aliases: &["RIGHT_ARROW", "ARROWRIGHT"],
        win: WinKey {
            vk: 0x27,
            scan_code: Some(0x4D),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_RIGHTALT",
        aliases: &["RIGHTALT", "RALT"],
        win: WinKey {
            vk: 0xA5,
            scan_code: Some(0x38),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_RIGHTBRACE",
        aliases: &["RIGHTBRACE", "RBRACE", "BRACKETRIGHT"],
        win: WinKey {
            vk: 0xDD,
            scan_code: Some(0x1B),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_RIGHTCTRL",
        aliases: &["RIGHTCTRL", "RCTRL"],
        win: WinKey {
            vk: 0xA3,
            scan_code: Some(0x1D),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_RIGHTMETA",
        aliases: &["RIGHTMETA"],
        win: WinKey {
            vk: 0x5C,
            scan_code: Some(0x5C),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_RIGHTSHIFT",
        aliases: &["RIGHTSHIFT", "RSHIFT"],
        win: WinKey {
            vk: 0xA1,
            scan_code: Some(0x36),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_S",
        aliases: &["S"],
        win: WinKey {
            vk: 0x53,
            scan_code: Some(0x1F),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_SCROLLLOCK",
        aliases: &["SCROLLLOCK"],
        win: WinKey {
            vk: 0x91,
            scan_code: Some(0x46),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_SEMICOLON",
        aliases: &["SEMICOLON"],
        win: WinKey {
            vk: 0xBA,
            scan_code: Some(0x27),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_SLASH",
        aliases: &["SLASH"],
        win: WinKey {
            vk: 0xBF,
            scan_code: Some(0x35),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_SPACE",
        aliases: &["SPACE"],
        win: WinKey {
            vk: 0x20,
            scan_code: Some(0x39),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_T",
        aliases: &["T"],
        win: WinKey {
            vk: 0x54,
            scan_code: Some(0x14),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_TAB",
        aliases: &["TAB"],
        win: WinKey {
            vk: 0x09,
            scan_code: Some(0x0F),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_U",
        aliases: &["U"],
        win: WinKey {
            vk: 0x55,
            scan_code: Some(0x16),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_UP",
        aliases: &["UP", "ARROWUP"],
        win: WinKey {
            vk: 0x26,
            scan_code: Some(0x48),
            extended: true,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_V",
        aliases: &["V"],
        win: WinKey {
            vk: 0x56,
            scan_code: Some(0x2F),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_W",
        aliases: &["W"],
        win: WinKey {
            vk: 0x57,
            scan_code: Some(0x11),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_X",
        aliases: &["X"],
        win: WinKey {
            vk: 0x58,
            scan_code: Some(0x2D),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_Y",
        aliases: &["Y"],
        win: WinKey {
            vk: 0x59,
            scan_code: Some(0x15),
            extended: false,
        },
        mouse: None,
    },
    KeyInfo {
        canonical: "KEY_Z",
        aliases: &["Z"],
        win: WinKey {
            vk: 0x5A,
            scan_code: Some(0x2C),
            extended: false,
        },
        mouse: None,
    },
];

/// Parse a key token using the frozen key language.
pub fn parse(token: &str) -> Option<Key> {
    parse_detailed(token).ok()
}

/// Parse with a machine-readable reason for the failure.
pub fn parse_detailed(token: &str) -> Result<Key, KeyParseError> {
    let clean = token.trim().to_uppercase();
    if clean.is_empty() {
        return Err(KeyParseError::Empty);
    }
    let s = clean.strip_prefix("KEY_").unwrap_or(&clean);
    if is_numeric(s) {
        return Err(KeyParseError::NumericPlatformSpecific);
    }
    lookup(s).ok_or(KeyParseError::Unknown)
}

fn is_numeric(s: &str) -> bool {
    let digits = s.strip_prefix("CODE_").unwrap_or(s);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn lookup(s: &str) -> Option<Key> {
    ALL_ALIASES
        .iter()
        .find(|(alias, _)| *alias == s)
        .map(|(_, key)| *key)
}

/// Resolve a low-level hook event to a canonical key.
///
/// Windows reports a virtual-key code, the Set-1 scan code and the extended
/// flag. VK + extended is authoritative because it distinguishes Enter from
/// keypad Enter and Pause from NumLock; the scan code is only a fallback when
/// a driver reports an unexpected VK. Mouse VKs resolve to `None`; button
/// messages use [`MouseButton::key`].
pub fn from_windows(vk: u16, extended: bool, scan: u16) -> Option<Key> {
    if let Some(key) = match (vk, extended) {
        (0x08, false) => Some(Key(19)),
        (0x09, false) => Some(Key(111)),
        (0x0D, false) => Some(Key(29)),
        (0x0D, true) => Some(Key(77)),
        (0x13, false) => Some(Key(96)),
        (0x14, false) => Some(Key(21)),
        (0x1B, false) => Some(Key(31)),
        (0x20, false) => Some(Key(109)),
        (0x21, true) => Some(Key(95)),
        (0x22, true) => Some(Key(94)),
        (0x23, true) => Some(Key(28)),
        (0x24, true) => Some(Key(60)),
        (0x25, true) => Some(Key(82)),
        (0x26, true) => Some(Key(113)),
        (0x27, true) => Some(Key(99)),
        (0x28, true) => Some(Key(26)),
        (0x2D, true) => Some(Key(62)),
        (0x2E, true) => Some(Key(24)),
        (0x30, false) => Some(Key(5)),
        (0x31, false) => Some(Key(6)),
        (0x32, false) => Some(Key(7)),
        (0x33, false) => Some(Key(8)),
        (0x34, false) => Some(Key(9)),
        (0x35, false) => Some(Key(10)),
        (0x36, false) => Some(Key(11)),
        (0x37, false) => Some(Key(12)),
        (0x38, false) => Some(Key(13)),
        (0x39, false) => Some(Key(14)),
        (0x41, false) => Some(Key(15)),
        (0x42, false) => Some(Key(17)),
        (0x43, false) => Some(Key(20)),
        (0x44, false) => Some(Key(23)),
        (0x45, false) => Some(Key(27)),
        (0x46, false) => Some(Key(32)),
        (0x47, false) => Some(Key(57)),
        (0x48, false) => Some(Key(59)),
        (0x49, false) => Some(Key(61)),
        (0x4A, false) => Some(Key(63)),
        (0x4B, false) => Some(Key(64)),
        (0x4C, false) => Some(Key(81)),
        (0x4D, false) => Some(Key(88)),
        (0x4E, false) => Some(Key(90)),
        (0x4F, false) => Some(Key(92)),
        (0x50, false) => Some(Key(93)),
        (0x51, false) => Some(Key(97)),
        (0x52, false) => Some(Key(98)),
        (0x53, false) => Some(Key(105)),
        (0x54, false) => Some(Key(110)),
        (0x55, false) => Some(Key(112)),
        (0x56, false) => Some(Key(114)),
        (0x57, false) => Some(Key(115)),
        (0x58, false) => Some(Key(116)),
        (0x59, false) => Some(Key(117)),
        (0x5A, false) => Some(Key(118)),
        (0x5B, true) => Some(Key(86)),
        (0x5C, true) => Some(Key(103)),
        (0x60, false) => Some(Key(65)),
        (0x61, false) => Some(Key(66)),
        (0x62, false) => Some(Key(67)),
        (0x63, false) => Some(Key(68)),
        (0x64, false) => Some(Key(69)),
        (0x65, false) => Some(Key(70)),
        (0x66, false) => Some(Key(71)),
        (0x67, false) => Some(Key(72)),
        (0x68, false) => Some(Key(73)),
        (0x69, false) => Some(Key(74)),
        (0x6A, false) => Some(Key(75)),
        (0x6B, false) => Some(Key(79)),
        (0x6C, false) => Some(Key(78)),
        (0x6E, false) => Some(Key(76)),
        (0x6F, true) => Some(Key(80)),
        (0x70, false) => Some(Key(33)),
        (0x71, false) => Some(Key(44)),
        (0x72, false) => Some(Key(50)),
        (0x73, false) => Some(Key(51)),
        (0x74, false) => Some(Key(52)),
        (0x75, false) => Some(Key(53)),
        (0x76, false) => Some(Key(54)),
        (0x77, false) => Some(Key(55)),
        (0x78, false) => Some(Key(56)),
        (0x79, false) => Some(Key(34)),
        (0x7A, false) => Some(Key(35)),
        (0x7B, false) => Some(Key(36)),
        (0x7C, false) => Some(Key(37)),
        (0x7D, false) => Some(Key(38)),
        (0x7E, false) => Some(Key(39)),
        (0x7F, false) => Some(Key(40)),
        (0x80, false) => Some(Key(41)),
        (0x81, false) => Some(Key(42)),
        (0x82, false) => Some(Key(43)),
        (0x83, false) => Some(Key(45)),
        (0x84, false) => Some(Key(46)),
        (0x85, false) => Some(Key(47)),
        (0x86, false) => Some(Key(48)),
        (0x87, false) => Some(Key(49)),
        (0x90, false) => Some(Key(91)),
        (0x91, false) => Some(Key(106)),
        (0xA0, false) => Some(Key(87)),
        (0xA1, false) => Some(Key(104)),
        (0xA2, false) => Some(Key(85)),
        (0xA3, true) => Some(Key(102)),
        (0xA4, false) => Some(Key(83)),
        (0xA5, true) => Some(Key(100)),
        (0xBA, false) => Some(Key(107)),
        (0xBB, false) => Some(Key(30)),
        (0xBC, false) => Some(Key(22)),
        (0xBD, false) => Some(Key(89)),
        (0xBE, false) => Some(Key(25)),
        (0xBF, false) => Some(Key(108)),
        (0xC0, false) => Some(Key(58)),
        (0xDB, false) => Some(Key(84)),
        (0xDC, false) => Some(Key(18)),
        (0xDD, false) => Some(Key(101)),
        (0xDE, false) => Some(Key(16)),
        _ => None,
    } {
        return Some(key);
    }
    if scan != 0 {
        if let Some(key) = match (scan, extended) {
            (0x01, false) => Some(Key(31)),
            (0x02, false) => Some(Key(6)),
            (0x03, false) => Some(Key(7)),
            (0x04, false) => Some(Key(8)),
            (0x05, false) => Some(Key(9)),
            (0x06, false) => Some(Key(10)),
            (0x07, false) => Some(Key(11)),
            (0x08, false) => Some(Key(12)),
            (0x09, false) => Some(Key(13)),
            (0x0A, false) => Some(Key(14)),
            (0x0B, false) => Some(Key(5)),
            (0x0C, false) => Some(Key(89)),
            (0x0D, false) => Some(Key(30)),
            (0x0E, false) => Some(Key(19)),
            (0x0F, false) => Some(Key(111)),
            (0x10, false) => Some(Key(97)),
            (0x11, false) => Some(Key(115)),
            (0x12, false) => Some(Key(27)),
            (0x13, false) => Some(Key(98)),
            (0x14, false) => Some(Key(110)),
            (0x15, false) => Some(Key(117)),
            (0x16, false) => Some(Key(112)),
            (0x17, false) => Some(Key(61)),
            (0x18, false) => Some(Key(92)),
            (0x19, false) => Some(Key(93)),
            (0x1A, false) => Some(Key(84)),
            (0x1B, false) => Some(Key(101)),
            (0x1C, false) => Some(Key(29)),
            (0x1C, true) => Some(Key(77)),
            (0x1D, false) => Some(Key(85)),
            (0x1D, true) => Some(Key(102)),
            (0x1E, false) => Some(Key(15)),
            (0x1F, false) => Some(Key(105)),
            (0x20, false) => Some(Key(23)),
            (0x21, false) => Some(Key(32)),
            (0x22, false) => Some(Key(57)),
            (0x23, false) => Some(Key(59)),
            (0x24, false) => Some(Key(63)),
            (0x25, false) => Some(Key(64)),
            (0x26, false) => Some(Key(81)),
            (0x27, false) => Some(Key(107)),
            (0x28, false) => Some(Key(16)),
            (0x29, false) => Some(Key(58)),
            (0x2A, false) => Some(Key(87)),
            (0x2B, false) => Some(Key(18)),
            (0x2C, false) => Some(Key(118)),
            (0x2D, false) => Some(Key(116)),
            (0x2E, false) => Some(Key(20)),
            (0x2F, false) => Some(Key(114)),
            (0x30, false) => Some(Key(17)),
            (0x31, false) => Some(Key(90)),
            (0x32, false) => Some(Key(88)),
            (0x33, false) => Some(Key(22)),
            (0x34, false) => Some(Key(25)),
            (0x35, false) => Some(Key(108)),
            (0x35, true) => Some(Key(80)),
            (0x36, false) => Some(Key(104)),
            (0x37, false) => Some(Key(75)),
            (0x38, false) => Some(Key(83)),
            (0x38, true) => Some(Key(100)),
            (0x39, false) => Some(Key(109)),
            (0x3A, false) => Some(Key(21)),
            (0x3B, false) => Some(Key(33)),
            (0x3C, false) => Some(Key(44)),
            (0x3D, false) => Some(Key(50)),
            (0x3E, false) => Some(Key(51)),
            (0x3F, false) => Some(Key(52)),
            (0x40, false) => Some(Key(53)),
            (0x41, false) => Some(Key(54)),
            (0x42, false) => Some(Key(55)),
            (0x43, false) => Some(Key(56)),
            (0x44, false) => Some(Key(34)),
            (0x45, false) => Some(Key(91)),
            (0x46, false) => Some(Key(106)),
            (0x47, false) => Some(Key(72)),
            (0x47, true) => Some(Key(60)),
            (0x48, false) => Some(Key(73)),
            (0x48, true) => Some(Key(113)),
            (0x49, false) => Some(Key(74)),
            (0x49, true) => Some(Key(95)),
            (0x4A, false) => Some(Key(78)),
            (0x4B, false) => Some(Key(69)),
            (0x4B, true) => Some(Key(82)),
            (0x4C, false) => Some(Key(70)),
            (0x4D, false) => Some(Key(71)),
            (0x4D, true) => Some(Key(99)),
            (0x4E, false) => Some(Key(79)),
            (0x4F, false) => Some(Key(66)),
            (0x4F, true) => Some(Key(28)),
            (0x50, false) => Some(Key(67)),
            (0x50, true) => Some(Key(26)),
            (0x51, false) => Some(Key(68)),
            (0x51, true) => Some(Key(94)),
            (0x52, false) => Some(Key(65)),
            (0x52, true) => Some(Key(62)),
            (0x53, false) => Some(Key(76)),
            (0x53, true) => Some(Key(24)),
            (0x57, false) => Some(Key(35)),
            (0x58, false) => Some(Key(36)),
            (0x5B, true) => Some(Key(86)),
            (0x5C, true) => Some(Key(103)),
            _ => None,
        } {
            return Some(key);
        }
    }
    // Be tolerant of a missing extended flag for unambiguous VKs.
    match vk {
        0x08 => Some(Key(19)),
        0x09 => Some(Key(111)),
        0x13 => Some(Key(96)),
        0x14 => Some(Key(21)),
        0x1B => Some(Key(31)),
        0x20 => Some(Key(109)),
        0x21 => Some(Key(95)),
        0x22 => Some(Key(94)),
        0x23 => Some(Key(28)),
        0x24 => Some(Key(60)),
        0x25 => Some(Key(82)),
        0x26 => Some(Key(113)),
        0x27 => Some(Key(99)),
        0x28 => Some(Key(26)),
        0x2D => Some(Key(62)),
        0x2E => Some(Key(24)),
        0x30 => Some(Key(5)),
        0x31 => Some(Key(6)),
        0x32 => Some(Key(7)),
        0x33 => Some(Key(8)),
        0x34 => Some(Key(9)),
        0x35 => Some(Key(10)),
        0x36 => Some(Key(11)),
        0x37 => Some(Key(12)),
        0x38 => Some(Key(13)),
        0x39 => Some(Key(14)),
        0x41 => Some(Key(15)),
        0x42 => Some(Key(17)),
        0x43 => Some(Key(20)),
        0x44 => Some(Key(23)),
        0x45 => Some(Key(27)),
        0x46 => Some(Key(32)),
        0x47 => Some(Key(57)),
        0x48 => Some(Key(59)),
        0x49 => Some(Key(61)),
        0x4A => Some(Key(63)),
        0x4B => Some(Key(64)),
        0x4C => Some(Key(81)),
        0x4D => Some(Key(88)),
        0x4E => Some(Key(90)),
        0x4F => Some(Key(92)),
        0x50 => Some(Key(93)),
        0x51 => Some(Key(97)),
        0x52 => Some(Key(98)),
        0x53 => Some(Key(105)),
        0x54 => Some(Key(110)),
        0x55 => Some(Key(112)),
        0x56 => Some(Key(114)),
        0x57 => Some(Key(115)),
        0x58 => Some(Key(116)),
        0x59 => Some(Key(117)),
        0x5A => Some(Key(118)),
        0x5B => Some(Key(86)),
        0x5C => Some(Key(103)),
        0x60 => Some(Key(65)),
        0x61 => Some(Key(66)),
        0x62 => Some(Key(67)),
        0x63 => Some(Key(68)),
        0x64 => Some(Key(69)),
        0x65 => Some(Key(70)),
        0x66 => Some(Key(71)),
        0x67 => Some(Key(72)),
        0x68 => Some(Key(73)),
        0x69 => Some(Key(74)),
        0x6A => Some(Key(75)),
        0x6B => Some(Key(79)),
        0x6C => Some(Key(78)),
        0x6E => Some(Key(76)),
        0x6F => Some(Key(80)),
        0x70 => Some(Key(33)),
        0x71 => Some(Key(44)),
        0x72 => Some(Key(50)),
        0x73 => Some(Key(51)),
        0x74 => Some(Key(52)),
        0x75 => Some(Key(53)),
        0x76 => Some(Key(54)),
        0x77 => Some(Key(55)),
        0x78 => Some(Key(56)),
        0x79 => Some(Key(34)),
        0x7A => Some(Key(35)),
        0x7B => Some(Key(36)),
        0x7C => Some(Key(37)),
        0x7D => Some(Key(38)),
        0x7E => Some(Key(39)),
        0x7F => Some(Key(40)),
        0x80 => Some(Key(41)),
        0x81 => Some(Key(42)),
        0x82 => Some(Key(43)),
        0x83 => Some(Key(45)),
        0x84 => Some(Key(46)),
        0x85 => Some(Key(47)),
        0x86 => Some(Key(48)),
        0x87 => Some(Key(49)),
        0x90 => Some(Key(91)),
        0x91 => Some(Key(106)),
        0xA0 => Some(Key(87)),
        0xA1 => Some(Key(104)),
        0xA2 => Some(Key(85)),
        0xA3 => Some(Key(102)),
        0xA4 => Some(Key(83)),
        0xA5 => Some(Key(100)),
        0xBA => Some(Key(107)),
        0xBB => Some(Key(30)),
        0xBC => Some(Key(22)),
        0xBD => Some(Key(89)),
        0xBE => Some(Key(25)),
        0xBF => Some(Key(108)),
        0xC0 => Some(Key(58)),
        0xDB => Some(Key(84)),
        0xDC => Some(Key(18)),
        0xDD => Some(Key(101)),
        0xDE => Some(Key(16)),
        _ => None,
    }
}

impl MouseButton {
    /// Canonical key this button is written as in profiles.
    pub fn key(self) -> Key {
        match self {
            MouseButton::Left => Key::BTN_LEFT,
            MouseButton::Right => Key::BTN_RIGHT,
            MouseButton::Middle => Key::BTN_MIDDLE,
            MouseButton::Side => Key::BTN_SIDE,
            MouseButton::Extra => Key::BTN_EXTRA,
        }
    }
}

/// Left/right modifier variants are the same physical key class.
pub fn keys_are_equivalent(a: Key, b: Key) -> bool {
    a.physical_class() == b.physical_class()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_contract_count() {
        assert_eq!(KEY_COUNT, 119);
        assert_eq!(ALL_KEYS.len(), 119);
    }

    #[test]
    fn every_alias_parses_to_its_key() {
        for (alias, expected) in ALL_ALIASES {
            // Bare digit aliases ("0".."9") are shadowed by numeric evdev
            // parsing on Linux and are platform-specific on Windows. The
            // canonical spelling KEY_<digit> always works.
            if alias.bytes().all(|b| b.is_ascii_digit()) {
                assert_eq!(
                    parse_detailed(alias),
                    Err(KeyParseError::NumericPlatformSpecific),
                    "numeric alias {alias}"
                );
                continue;
            }
            assert_eq!(parse(alias), Some(*expected), "alias {alias}");
            assert_eq!(
                parse(&alias.to_lowercase()),
                Some(*expected),
                "lowercase {alias}"
            );
        }
    }

    #[test]
    fn canonical_names_round_trip_where_listed() {
        for key in ALL_KEYS {
            let canonical = key.canonical();
            if key.aliases().contains(&canonical) {
                assert_eq!(parse(canonical), Some(key), "canonical {canonical}");
            }
        }
    }

    #[test]
    fn optional_key_prefix_is_stripped_once() {
        assert_eq!(parse("key_a"), Some(Key::KEY_A));
        assert_eq!(parse("KEY_F12"), Some(Key::KEY_F12));
        // Dictionary trap preserved from Linux: LEFT is the mouse button.
        assert_eq!(parse("KEY_LEFT"), Some(Key::BTN_LEFT));
        assert_eq!(parse("KEY_LEFT_ARROW"), Some(Key::KEY_LEFT));
        assert_eq!(parse("key_left_arrow"), Some(Key::KEY_LEFT));
    }

    #[test]
    fn numeric_codes_are_rejected_on_windows() {
        assert_eq!(
            parse_detailed("65"),
            Err(KeyParseError::NumericPlatformSpecific)
        );
        assert_eq!(
            parse_detailed("CODE_65"),
            Err(KeyParseError::NumericPlatformSpecific)
        );
        assert_eq!(
            parse_detailed("  code_32 "),
            Err(KeyParseError::NumericPlatformSpecific)
        );
    }

    #[test]
    fn unknown_and_empty_tokens_are_rejected() {
        assert_eq!(parse_detailed(""), Err(KeyParseError::Empty));
        assert_eq!(parse_detailed("NOT_A_KEY"), Err(KeyParseError::Unknown));
        assert_eq!(
            parse_detailed("KEY_LEFTCTRL_EXTRA"),
            Err(KeyParseError::Unknown)
        );
    }

    #[test]
    fn left_and_right_modifiers_are_equivalent() {
        assert!(keys_are_equivalent(Key::KEY_LEFTCTRL, Key::KEY_RIGHTCTRL));
        assert!(keys_are_equivalent(Key::KEY_LEFTSHIFT, Key::KEY_RIGHTSHIFT));
        assert!(keys_are_equivalent(Key::KEY_LEFTALT, Key::KEY_RIGHTALT));
        assert!(keys_are_equivalent(Key::KEY_LEFTMETA, Key::KEY_RIGHTMETA));
        assert!(!keys_are_equivalent(Key::KEY_LEFTCTRL, Key::KEY_LEFTSHIFT));
        assert!(keys_are_equivalent(Key::KEY_A, Key::KEY_A));
        assert!(!keys_are_equivalent(Key::KEY_A, Key::KEY_B));
    }

    #[test]
    fn mouse_buttons_are_flagged() {
        assert_eq!(Key::BTN_LEFT.mouse_button(), Some(MouseButton::Left));
        assert_eq!(Key::BTN_RIGHT.mouse_button(), Some(MouseButton::Right));
        assert_eq!(Key::BTN_MIDDLE.mouse_button(), Some(MouseButton::Middle));
        assert_eq!(Key::BTN_SIDE.mouse_button(), Some(MouseButton::Side));
        assert_eq!(Key::BTN_EXTRA.mouse_button(), Some(MouseButton::Extra));
        assert!(Key::KEY_A.mouse_button().is_none());
        assert!(Key::BTN_LEFT.is_mouse());
        assert!(!Key::KEY_A.is_mouse());
    }

    #[test]
    fn windows_mapping_spot_checks() {
        assert_eq!(
            Key::KEY_A.windows(),
            WinKey {
                vk: 0x41,
                scan_code: Some(0x1E),
                extended: false
            }
        );
        assert_eq!(Key::KEY_0.windows().vk, 0x30);
        assert_eq!(
            Key::KEY_F12.windows(),
            WinKey {
                vk: 0x7B,
                scan_code: Some(0x58),
                extended: false
            }
        );
        assert_eq!(
            Key::KEY_LEFT.windows(),
            WinKey {
                vk: 0x25,
                scan_code: Some(0x4B),
                extended: true
            }
        );
        assert_eq!(
            Key::KEY_RIGHTCTRL.windows(),
            WinKey {
                vk: 0xA3,
                scan_code: Some(0x1D),
                extended: true
            }
        );
        assert_eq!(
            Key::KEY_KP5.windows(),
            WinKey {
                vk: 0x65,
                scan_code: Some(0x4C),
                extended: false
            }
        );
        assert!(Key::KEY_KPENTER.windows().extended);
        assert_eq!(Key::KEY_PAUSE.windows().scan_code, None);
        assert_eq!(Key::KEY_F13.windows().scan_code, None);
    }

    #[test]
    fn every_key_has_a_mapping() {
        for key in ALL_KEYS {
            let win = key.windows();
            if key.is_mouse() {
                assert_eq!(win.vk, 0);
                assert_eq!(win.scan_code, None);
            } else {
                assert_ne!(win.vk, 0, "{key} has no VK");
            }
        }
    }

    #[test]
    fn every_keyboard_key_round_trips_through_windows() {
        for key in ALL_KEYS {
            if key.is_mouse() {
                continue;
            }
            let win = key.windows();
            let scan = win.scan_code.unwrap_or(0);
            assert_eq!(
                from_windows(win.vk, win.extended, scan),
                Some(key),
                "round trip for {key}"
            );
        }
    }

    #[test]
    fn windows_ambiguities_are_resolved() {
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
    }

    #[test]
    fn mouse_buttons_map_to_canonical_keys() {
        for button in [
            MouseButton::Left,
            MouseButton::Right,
            MouseButton::Middle,
            MouseButton::Side,
            MouseButton::Extra,
        ] {
            assert_eq!(button.key().mouse_button(), Some(button));
        }
    }

    #[test]
    fn debug_uses_canonical_name() {
        assert_eq!(format!("{:?}", Key::KEY_LEFTCTRL), "KEY_LEFTCTRL");
        assert_eq!(format!("{}", Key::BTN_LEFT), "BTN_LEFT");
    }
}
