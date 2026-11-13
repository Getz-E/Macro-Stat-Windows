// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! Global low-level hooks (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`) on a dedicated
//! message-loop thread, feeding a bounded channel of [`InputEvent`]s.
//!
//! Design rules:
//!
//! * the hook callback never blocks and never allocates on the hot path; it
//!   uses `try_send` and drops events if the queue is full (the queue is for
//!   the macro engine and recording, not for replaying every movement);
//! * events carrying [`INJECTION_SIGNATURE`](crate::winutil::INJECTION_SIGNATURE)
//!   are ignored, so our own `SendInput` output cannot re-trigger a macro;
//! * external synthetic events (`LLKHF_INJECTED` without our signature) are
//!   delivered with `injected: true`, matching Linux, where other tools'
//!   `uinput` devices are indistinguishable from hardware;
//! * nothing is swallowed. Linux cannot suppress input either, and blanket
//!   suppression would also block the panic key. Per-macro suppression is a
//!   separate, documented follow-up.
//!
//! Only one hook set can exist per process (the callback uses a `OnceLock`).

use crate::keymap::Key;

/// A low-level input event in engine vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    Key {
        key: Key,
        down: bool,
        /// Monotonic Windows tick in milliseconds (informational).
        time_ms: u32,
        /// Event came from another injector, not from hardware.
        injected: bool,
    },
    MouseMove {
        x: i32,
        y: i32,
        time_ms: u32,
    },
    MouseButton {
        key: Key,
        down: bool,
        x: i32,
        y: i32,
        time_ms: u32,
    },
    /// `mouseData` high word in Windows wheel units (120 per detent).
    Wheel {
        delta_units: i32,
        time_ms: u32,
    },
}

/// Which part of the hook set failed to install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookError {
    /// A hook set already exists in this process.
    AlreadyInstalled,
    /// `SetWindowsHookExW` failed for the keyboard hook.
    KeyboardInstall { error: u32 },
    /// `SetWindowsHookExW` failed for the mouse hook.
    MouseInstall { error: u32 },
    /// The hook thread died before reporting readiness.
    ThreadExited,
    /// This build has no Windows input backend.
    UnsupportedPlatform,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookError::AlreadyInstalled => f.write_str("input hook is already installed"),
            HookError::KeyboardInstall { error } => {
                write!(f, "keyboard hook install failed (Windows error {error})")
            }
            HookError::MouseInstall { error } => {
                write!(f, "mouse hook install failed (Windows error {error})")
            }
            HookError::ThreadExited => f.write_str("hook thread exited unexpectedly"),
            HookError::UnsupportedPlatform => f.write_str("input hooks require Windows"),
        }
    }
}

impl std::error::Error for HookError {}

/// Register the key that must always be able to stop the engine.
///
/// The hook queue is bounded and a saturated queue drops events, including
/// the single Pause key-down (Pause does not auto-repeat). The callback
/// latches a matching key-down in an atomic flag before attempting the
/// send, so the panic key works even when no event reaches the main loop.
#[cfg(windows)]
pub fn set_panic_key(key: Key) {
    windows_impl::set_panic_key(key);
}

#[cfg(not(windows))]
pub fn set_panic_key(_key: Key) {}

/// Take the latched panic-key press (returns `true` at most once).
#[cfg(windows)]
pub fn take_panic_press() -> bool {
    windows_impl::take_panic_press()
}

#[cfg(not(windows))]
pub fn take_panic_press() -> bool {
    false
}

/// Number of events dropped because the consumer queue was full.
#[cfg(windows)]
pub fn dropped_events() -> u64 {
    use std::sync::atomic::Ordering;
    windows_impl::DROPPED.load(Ordering::Relaxed)
}

#[cfg(not(windows))]
pub fn dropped_events() -> u64 {
    0
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
    use std::sync::OnceLock;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use crate::keymap::{from_windows, keys_are_equivalent, MouseButton};
    use crate::winutil::INJECTION_SIGNATURE;
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, PeekMessageW, PostThreadMessageW, SetWindowsHookExW,
        UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, MSG, MSLLHOOKSTRUCT,
        WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT,
        WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
        XBUTTON2,
    };

    pub(super) static DROPPED: AtomicU64 = AtomicU64::new(0);
    static SENDER: OnceLock<SyncSender<InputEvent>> = OnceLock::new();
    /// Panic key, updated on start and profile reload.
    static PANIC_KEY: std::sync::RwLock<Option<Key>> = std::sync::RwLock::new(None);
    /// Set by the hook callback when the panic key goes down.
    static PANIC_PRESSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    pub(super) fn set_panic_key(key: Key) {
        if let Ok(mut slot) = PANIC_KEY.write() {
            *slot = Some(key);
        }
    }

    pub(super) fn take_panic_press() -> bool {
        PANIC_PRESSED.swap(false, Ordering::SeqCst)
    }

    /// Record a panic-key press before the (possibly full) queue is tried.
    fn latch_panic(key: Key, down: bool) {
        if !down {
            return;
        }
        let matches = match PANIC_KEY.read() {
            Ok(slot) => slot.map(|panic| keys_are_equivalent(key, panic)),
            Err(poisoned) => poisoned.into_inner().map(|p| keys_are_equivalent(key, p)),
        };
        if matches == Some(true) {
            PANIC_PRESSED.store(true, Ordering::SeqCst);
        }
    }

    fn emit(event: InputEvent) {
        if let Some(sender) = SENDER.get() {
            if let Err(TrySendError::Full(_)) = sender.try_send(event) {
                DROPPED.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let info = &*(lparam as *const KBDLLHOOKSTRUCT);
            if info.dwExtraInfo != INJECTION_SIGNATURE {
                let message = wparam as u32;
                let down = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
                let up = matches!(message, WM_KEYUP | WM_SYSKEYUP);
                if down || up {
                    let extended = info.flags & LLKHF_EXTENDED != 0;
                    if let Some(key) =
                        from_windows(info.vkCode as u16, extended, info.scanCode as u16)
                    {
                        latch_panic(key, down);
                        emit(InputEvent::Key {
                            key,
                            down,
                            time_ms: info.time,
                            injected: info.flags & LLKHF_INJECTED != 0,
                        });
                    }
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let info = &*(lparam as *const MSLLHOOKSTRUCT);
            if info.dwExtraInfo != INJECTION_SIGNATURE {
                let message = wparam as u32;
                let x = info.pt.x;
                let y = info.pt.y;
                match message {
                    WM_MOUSEMOVE => emit(InputEvent::MouseMove {
                        x,
                        y,
                        time_ms: info.time,
                    }),
                    WM_LBUTTONDOWN | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP
                    | WM_MBUTTONDOWN | WM_MBUTTONUP => {
                        let (button, down) = match message {
                            WM_LBUTTONDOWN => (MouseButton::Left, true),
                            WM_LBUTTONUP => (MouseButton::Left, false),
                            WM_RBUTTONDOWN => (MouseButton::Right, true),
                            WM_RBUTTONUP => (MouseButton::Right, false),
                            WM_MBUTTONDOWN => (MouseButton::Middle, true),
                            _ => (MouseButton::Middle, false),
                        };
                        latch_panic(button.key(), down);
                        emit(InputEvent::MouseButton {
                            key: button.key(),
                            down,
                            x,
                            y,
                            time_ms: info.time,
                        });
                    }
                    WM_XBUTTONDOWN | WM_XBUTTONUP => {
                        let which = (info.mouseData >> 16) as u16;
                        let button = if which == XBUTTON2 {
                            MouseButton::Extra
                        } else {
                            MouseButton::Side
                        };
                        let down = message == WM_XBUTTONDOWN;
                        latch_panic(button.key(), down);
                        emit(InputEvent::MouseButton {
                            key: button.key(),
                            down,
                            x,
                            y,
                            time_ms: info.time,
                        });
                    }
                    WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                        let delta = ((info.mouseData >> 16) & 0xFFFF) as u16 as i16 as i32;
                        emit(InputEvent::Wheel {
                            delta_units: delta,
                            time_ms: info.time,
                        });
                    }
                    _ => {}
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }

    pub struct HookHandle {
        thread_id: u32,
        join: Option<JoinHandle<()>>,
    }

    impl HookHandle {
        pub fn thread_id(&self) -> u32 {
            self.thread_id
        }
    }

    impl Drop for HookHandle {
        fn drop(&mut self) {
            unsafe {
                PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
            }
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }

    /// Install the hook pair and wait until the message loop is ready.
    ///
    /// The channel is bounded; `capacity` should cover a few hundred
    /// milliseconds of typing (4096 is the default used by the runtime).
    pub fn start(
        capacity: usize,
    ) -> Result<(HookHandle, std::sync::mpsc::Receiver<InputEvent>), HookError> {
        let (sender, receiver) = sync_channel(capacity.max(1));
        if SENDER.set(sender).is_err() {
            return Err(HookError::AlreadyInstalled);
        }

        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<u32, HookError>>();
        let join = thread::Builder::new()
            .name("macrostat-hook".to_string())
            .spawn(move || unsafe {
                let module = GetModuleHandleW(std::ptr::null());
                let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), module, 0);
                if keyboard.is_null() {
                    let _ = ready_tx.send(Err(HookError::KeyboardInstall {
                        error: windows_sys::Win32::Foundation::GetLastError(),
                    }));
                    return;
                }
                let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0);
                if mouse.is_null() {
                    let error = windows_sys::Win32::Foundation::GetLastError();
                    UnhookWindowsHookEx(keyboard);
                    let _ = ready_tx.send(Err(HookError::MouseInstall { error }));
                    return;
                }
                // Force this thread's message queue before its owner can
                // post WM_QUIT to it.
                let mut msg: MSG = std::mem::zeroed();
                PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, 0);
                let _ = ready_tx.send(Ok(GetCurrentThreadId()));

                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
                UnhookWindowsHookEx(mouse);
                UnhookWindowsHookEx(keyboard);
            })
            .map_err(|_| HookError::ThreadExited)?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(thread_id)) => Ok((
                HookHandle {
                    thread_id,
                    join: Some(join),
                },
                receiver,
            )),
            Ok(Err(error)) => {
                let _ = join.join();
                Err(error)
            }
            Err(_) => {
                // The thread never reported readiness; detaching avoids
                // blocking forever on a join for a thread with no message
                // queue to wake.
                drop(join);
                Err(HookError::ThreadExited)
            }
        }
    }
}

#[cfg(windows)]
pub use windows_impl::{start, HookHandle};

#[cfg(not(windows))]
pub struct HookHandle;

#[cfg(not(windows))]
pub fn start(
    _capacity: usize,
) -> Result<(HookHandle, std::sync::mpsc::Receiver<InputEvent>), HookError> {
    Err(HookError::UnsupportedPlatform)
}
