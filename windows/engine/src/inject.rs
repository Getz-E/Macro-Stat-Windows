// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! `SendInput` backend: keyboard, mouse buttons, relative/absolute motion,
//! wheel and Unicode text.
//!
//! Every synthetic event carries [`INJECTION_SIGNATURE`](crate::winutil::INJECTION_SIGNATURE)
//! in `dwExtraInfo`; the low-level hook ignores matching events, which is the
//! Windows equivalent of Linux's `uinput` virtual-device separation.
//!
//! `SendInput` returns how many events it inserted. A shortfall usually means
//! UIPI blocked the target (elevated foreground window), but Windows does not
//! say which window was blocked; the counter is therefore diagnostic only and
//! must not be presented as a definitive UIPI signal.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::actions::{Cancel, Diagnostics, Waiter};

#[cfg(windows)]
use crate::actions::Injector;

#[cfg(windows)]
use crate::mouse;
#[cfg(windows)]
use crate::winutil::INJECTION_SIGNATURE;

/// Counters exposed for logs and the status badge.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InjectionStats {
    /// Events passed to `SendInput`.
    pub attempts: u64,
    /// Events `SendInput` reported as inserted.
    pub sent: u64,
    /// Calls where `SendInput` inserted fewer events than requested.
    pub shortfalls: u64,
}

/// Convert a character into the UTF-16 code units `KEYEVENTF_UNICODE` accepts.
///
/// Returns an empty vector for NUL (cannot be typed) and for other C0 control
/// characters; newline/tab are handled as real key presses by
/// [`WindowsInjector::text_char`].
pub fn text_units(ch: char) -> Vec<u16> {
    if ch == '\0' || (ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t') {
        return Vec::new();
    }
    let mut buffer = [0u16; 2];
    ch.encode_utf16(&mut buffer).to_vec()
}

/// Millisecond waiter. The runtime keeps `timeBeginPeriod(1)` active.
#[derive(Debug, Default, Clone, Copy)]
pub struct SleepWaiter;

impl Waiter for SleepWaiter {
    fn sleep_ms(&mut self, ms: u64) {
        if ms > 0 {
            thread::sleep(Duration::from_millis(ms));
        }
    }
}

/// Cancellation backed by an atomic flag (panic key / loop stop).
#[derive(Debug, Clone, Default)]
pub struct FlagCancel {
    flag: Arc<AtomicBool>,
}

impl FlagCancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_flag(flag: Arc<AtomicBool>) -> Self {
        Self { flag }
    }

    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn clear(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

impl Cancel for FlagCancel {
    fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Writes diagnostics to stderr, matching the Linux engine's `[UYARI]` lines.
#[derive(Debug, Default, Clone, Copy)]
pub struct StderrDiagnostics;

impl Diagnostics for StderrDiagnostics {
    fn warn(&mut self, message: String) {
        eprintln!("[UYARI] {message}");
    }
}

/// No-op cancellation for one-shot test runs.
#[derive(Debug, Default, Clone, Copy)]
pub struct NeverCancel;

impl Cancel for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use crate::keymap::{Key, MouseButton};
    use crate::mouse::VirtualScreen;
    use std::mem::size_of;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE,
        MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP,
        MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK,
        MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{XBUTTON1, XBUTTON2};

    /// `SendInput`-backed [`Injector`].
    #[derive(Debug, Default)]
    pub struct WindowsInjector {
        stats: InjectionStats,
    }

    impl WindowsInjector {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn stats(&self) -> InjectionStats {
            self.stats
        }

        fn send_keyboard(&mut self, vk: u16, scan: Option<u16>, extended: bool, down: bool) {
            let mut flags = 0u32;
            let (w_vk, w_scan) = match scan {
                Some(scan) => {
                    flags |= KEYEVENTF_SCANCODE;
                    (0, scan)
                }
                None => (vk, 0),
            };
            if extended {
                flags |= KEYEVENTF_EXTENDEDKEY;
            }
            if !down {
                flags |= KEYEVENTF_KEYUP;
            }
            let input = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: w_vk,
                        wScan: w_scan,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: INJECTION_SIGNATURE,
                    },
                },
            };
            self.send(&input);
        }

        fn send_unicode(&mut self, unit: u16, down: bool) {
            let mut flags = KEYEVENTF_UNICODE;
            if !down {
                flags |= KEYEVENTF_KEYUP;
            }
            let input = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: unit,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: INJECTION_SIGNATURE,
                    },
                },
            };
            self.send(&input);
        }

        fn send_mouse(&mut self, dx: i32, dy: i32, data: u32, flags: u32) {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx,
                        dy,
                        mouseData: data,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: INJECTION_SIGNATURE,
                    },
                },
            };
            self.send(&input);
        }

        fn send(&mut self, input: &INPUT) {
            self.stats.attempts += 1;
            let inserted =
                unsafe { SendInput(1, input, i32::try_from(size_of::<INPUT>()).unwrap_or(0)) };
            if inserted == 1 {
                self.stats.sent += 1;
            } else {
                self.stats.shortfalls += 1;
            }
        }

        fn mouse_button(&mut self, button: MouseButton, down: bool) {
            let (flags, data) = match (button, down) {
                (MouseButton::Left, true) => (MOUSEEVENTF_LEFTDOWN, 0),
                (MouseButton::Left, false) => (MOUSEEVENTF_LEFTUP, 0),
                (MouseButton::Right, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
                (MouseButton::Right, false) => (MOUSEEVENTF_RIGHTUP, 0),
                (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
                (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
                (MouseButton::Side, true) => (MOUSEEVENTF_XDOWN, u32::from(XBUTTON1)),
                (MouseButton::Side, false) => (MOUSEEVENTF_XUP, u32::from(XBUTTON1)),
                (MouseButton::Extra, true) => (MOUSEEVENTF_XDOWN, u32::from(XBUTTON2)),
                (MouseButton::Extra, false) => (MOUSEEVENTF_XUP, u32::from(XBUTTON2)),
            };
            self.send_mouse(0, 0, data, flags);
        }
    }

    impl Injector for WindowsInjector {
        fn key(&mut self, key: Key, down: bool) {
            if let Some(button) = key.mouse_button() {
                self.mouse_button(button, down);
                return;
            }
            let win = key.windows();
            self.send_keyboard(win.vk, win.scan_code, win.extended, down);
        }

        fn mouse_move(&mut self, dx: i32, dy: i32) {
            self.send_mouse(dx, dy, 0, MOUSEEVENTF_MOVE);
        }

        fn mouse_wheel(&mut self, delta: i32) {
            self.send_mouse(0, 0, mouse::wheel_units(delta) as u32, MOUSEEVENTF_WHEEL);
        }

        fn mouse_absolute(&mut self, x: i32, y: i32) {
            let screen = VirtualScreen::capture();
            let (nx, ny) = screen.normalize(x, y);
            self.send_mouse(
                i32::from(nx),
                i32::from(ny),
                0,
                MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
            );
        }

        fn text_char(&mut self, ch: char) -> bool {
            match ch {
                '\n' | '\r' => {
                    let enter = Key::KEY_ENTER.windows();
                    self.send_keyboard(enter.vk, enter.scan_code, enter.extended, true);
                    self.send_keyboard(enter.vk, enter.scan_code, enter.extended, false);
                    return true;
                }
                '\t' => {
                    let tab = Key::KEY_TAB.windows();
                    self.send_keyboard(tab.vk, tab.scan_code, tab.extended, true);
                    self.send_keyboard(tab.vk, tab.scan_code, tab.extended, false);
                    return true;
                }
                _ => {}
            }
            let units = text_units(ch);
            if units.is_empty() {
                return false;
            }
            for unit in units {
                self.send_unicode(unit, true);
                self.send_unicode(unit, false);
            }
            true
        }
    }
}

#[cfg(windows)]
pub use windows_impl::WindowsInjector;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_units_cover_bmp_and_surrogates() {
        assert_eq!(text_units('A'), vec![0x0041]);
        assert_eq!(text_units('ş'), vec![0x015F]);
        assert_eq!(text_units('😀'), vec![0xD83D, 0xDE00]);
        assert_eq!(text_units('\n'), vec![0x000A]);
        assert_eq!(text_units('\t'), vec![0x0009]);
        assert!(text_units('\0').is_empty());
        assert!(text_units('\u{0007}').is_empty());
    }

    #[test]
    fn flag_cancel_is_shared_between_clones() {
        let cancel = FlagCancel::new();
        let clone = cancel.clone();
        assert!(!cancel.is_cancelled());
        clone.cancel();
        assert!(cancel.is_cancelled());
        cancel.clear();
        assert!(!clone.is_cancelled());
    }

    #[test]
    fn never_cancel_never_cancels() {
        assert!(!NeverCancel.is_cancelled());
    }
}
