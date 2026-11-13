// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! Mouse geometry: virtual-desktop capture, absolute-coordinate normalization
//! and scroll-unit conversion.
//!
//! `SendInput` absolute motion uses a 0..65535 grid stretched over the whole
//! virtual desktop (`MOUSEEVENTF_VIRTUALDESK`), which can start at negative
//! coordinates with multiple monitors. The normalization helpers are pure so
//! the multi-monitor and DPI cases are unit-tested without a Windows host.

/// Virtual desktop rectangle in physical pixels (virtual-screen coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualScreen {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl VirtualScreen {
    /// Capture the current virtual desktop.
    #[cfg(windows)]
    pub fn capture() -> Self {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        };
        unsafe {
            Self {
                left: GetSystemMetrics(SM_XVIRTUALSCREEN),
                top: GetSystemMetrics(SM_YVIRTUALSCREEN),
                width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
                height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
            }
        }
    }

    #[cfg(not(windows))]
    pub fn capture() -> Self {
        Self {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        }
    }

    /// Normalize a virtual-desktop point onto the `0..=65535` axis used by
    /// `MOUSEEVENTF_ABSOLUTE`. The last pixel maps exactly to 65535.
    pub fn normalize(&self, x: i32, y: i32) -> (u16, u16) {
        (
            normalize_axis(x, self.left, self.width),
            normalize_axis(y, self.top, self.height),
        )
    }

    /// Inverse of [`Self::normalize`], used for diagnostics and tests.
    pub fn denormalize(&self, nx: u16, ny: u16) -> (i32, i32) {
        (
            denormalize_axis(nx, self.left, self.width),
            denormalize_axis(ny, self.top, self.height),
        )
    }
}

fn normalize_axis(value: i32, origin: i32, extent: i32) -> u16 {
    if extent <= 1 {
        return 0;
    }
    let span = i64::from(extent - 1);
    let offset = i64::from(value) - i64::from(origin);
    let scaled = (offset * 65_535 + span / 2) / span;
    scaled.clamp(0, 65_535) as u16
}

fn denormalize_axis(normalized: u16, origin: i32, extent: i32) -> i32 {
    if extent <= 1 {
        return origin;
    }
    let span = i64::from(extent - 1);
    let offset = (i64::from(normalized) * span + 65_535 / 2) / 65_535;
    i32::try_from(i64::from(origin) + offset).unwrap_or(if offset >= 0 {
        i32::MAX
    } else {
        i32::MIN
    })
}

/// Convert profile scroll notches into `MOUSEEVENTF_WHEEL` units.
///
/// The profile uses one unit per detent, exactly like Linux `REL_WHEEL`;
/// Windows expects 120 units per detent. Saturates instead of wrapping.
pub fn wheel_units(delta: i32) -> i32 {
    delta.saturating_mul(120)
}

/// Current cursor position in virtual-desktop coordinates.
#[cfg(windows)]
pub fn cursor_position() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) != 0 {
            Some((point.x, point.y))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn cursor_position() -> Option<(i32, i32)> {
    None
}

/// Move the cursor without generating an injected macro event.
///
/// Used only by the recording UI to restore a start point; macro playback uses
/// `SendInput` in [`crate::inject`] so the movement is a real input event.
#[cfg(windows)]
pub fn set_cursor_position(x: i32, y: i32) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::SetCursorPos;
    unsafe { SetCursorPos(x, y) != 0 }
}

#[cfg(not(windows))]
pub fn set_cursor_position(_x: i32, _y: i32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_monitor_edges_map_exactly() {
        let screen = VirtualScreen {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
        };
        assert_eq!(screen.normalize(0, 0), (0, 0));
        assert_eq!(screen.normalize(1919, 1079), (65535, 65535));
        let (nx, ny) = screen.normalize(960, 540);
        assert!((32_700..=32_900).contains(&nx), "nx = {nx}");
        assert!((32_700..=32_900).contains(&ny), "ny = {ny}");
    }

    #[test]
    fn negative_multi_monitor_origin_is_supported() {
        let screen = VirtualScreen {
            left: -1920,
            top: -200,
            width: 3840,
            height: 1280,
        };
        assert_eq!(screen.normalize(-1920, -200), (0, 0));
        assert_eq!(screen.normalize(1919, 1079), (65535, 65535));
        // A point left of the origin is clamped, never wrapped.
        assert_eq!(screen.normalize(-5000, -5000), (0, 0));
    }

    #[test]
    fn normalization_round_trips_within_one_pixel() {
        let screen = VirtualScreen {
            left: -100,
            top: 50,
            width: 1600,
            height: 900,
        };
        for (x, y) in [(-100, 50), (700, 500), (1499, 949)] {
            let (nx, ny) = screen.normalize(x, y);
            let (rx, ry) = screen.denormalize(nx, ny);
            assert!((rx - x).abs() <= 1, "{x} -> {nx} -> {rx}");
            assert!((ry - y).abs() <= 1, "{y} -> {ny} -> {ry}");
        }
    }

    #[test]
    fn degenerate_screen_does_not_divide_by_zero() {
        let screen = VirtualScreen {
            left: 10,
            top: 20,
            width: 1,
            height: 0,
        };
        assert_eq!(screen.normalize(10, 20), (0, 0));
        assert_eq!(screen.denormalize(1234, 5678), (10, 20));
    }

    #[test]
    fn scroll_notches_become_wheel_units() {
        assert_eq!(wheel_units(0), 0);
        assert_eq!(wheel_units(1), 120);
        assert_eq!(wheel_units(-3), -360);
        assert_eq!(wheel_units(i32::MAX), i32::MAX);
        assert_eq!(wheel_units(i32::MIN), i32::MIN);
    }
}
