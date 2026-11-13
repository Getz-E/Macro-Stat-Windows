// SPDX-License-Identifier: GPL-3.0-or-later
//! Action sequencing: the 11 action types, delays/jitter, cancellation and
//! stuck-key cleanup.
//!
//! Pure with respect to the OS: injection, waiting, cancellation and random
//! jitter are traits. The Phase 2 Windows layer implements [`Injector`] with
//! `SendInput`, [`Waiter`] with a waitable timer, and [`Cancel`] over the panic
//! flag / loop flag. That keeps every timing and ordering rule unit-testable
//! on Linux CI.
//!
//! Ported semantics from the Linux engine:
//!
//! * `KEY_TAP` holds for `delayMs` (default 25, minimum 10) then waits 10 ms;
//! * `MOUSE_CLICK` holds for `delayMs` (default 20, minimum 10) then 10 ms;
//! * `DELAY` defaults to 20 ms, jitter is uniform in `[-j, +j]` and the
//!   effective delay is at least 1 ms (a zero-delay DELAY returns immediately
//!   without applying jitter);
//! * `MOUSE_PATH` clamps `dt` to `0..=600_000`, coordinates to `i32`, replays
//!   at most 200k points, skips zero deltas and warps to `start` first;
//! * `TEXT` injects one character at a time with a 16 ms pause and supports
//!   every `char` on Windows (Unicode injection), unlike Linux's ASCII-only
//!   evdev path;
//! * cancellation releases every key the macro pressed before returning.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};

use rand::Rng;

use crate::keymap::{self, Key};
use crate::profile::{
    ActionType, MacroAction, MAX_MOUSE_PATH_POINTS, MAX_MOUSE_PATH_START, MAX_MOUSE_PATH_STEP_MS,
};

/// Platform injection backend.
pub trait Injector {
    /// Press/release a keyboard key or a mouse button ([`Key::is_mouse`]).
    fn key(&mut self, key: Key, down: bool);
    /// Relative pointer movement (`MOUSE_MOVE`, `MOUSE_PATH`).
    fn mouse_move(&mut self, dx: i32, dy: i32);
    /// Vertical wheel delta (`MOUSE_SCROLL`); positive is up like Linux.
    fn mouse_wheel(&mut self, delta: i32);
    /// Warp the cursor to virtual-desktop coordinates (`MOUSE_PATH.start`).
    fn mouse_absolute(&mut self, x: i32, y: i32);
    /// Inject one Unicode character. Returns `false` when unsupported.
    fn text_char(&mut self, ch: char) -> bool;
}

/// Sleeping backend; test doubles record instead of sleeping.
pub trait Waiter {
    fn sleep_ms(&mut self, ms: u64);
}

/// Cooperative cancellation (panic flag, loop flag, shutdown).
pub trait Cancel {
    fn is_cancelled(&self) -> bool;
}

/// Random source for delay jitter; deterministic in tests.
pub trait Jitter {
    /// Uniform value in `[-max_ms, +max_ms]`.
    fn jitter(&mut self, max_ms: u64) -> i64;
}

/// Non-fatal diagnostics sink (invalid action key, unsupported character).
pub trait Diagnostics {
    fn warn(&mut self, message: String);
}

/// Diagnostics sink that drops messages, for tests and headless callers.
pub struct NoDiagnostics;

impl Diagnostics for NoDiagnostics {
    fn warn(&mut self, _message: String) {}
}

/// `rand`-backed jitter used by the runtime.
pub struct RandJitter;

impl Jitter for RandJitter {
    fn jitter(&mut self, max_ms: u64) -> i64 {
        if max_ms == 0 {
            return 0;
        }
        let bound = i64::try_from(max_ms).unwrap_or(i64::MAX);
        rand::thread_rng().gen_range(-bound..=bound)
    }
}

/// All state one action-sequence execution needs.
pub struct ActionContext<'a> {
    pub injector: &'a mut dyn Injector,
    pub pressed: &'a mut HashSet<Key>,
    pub waiter: &'a mut dyn Waiter,
    pub cancel: &'a dyn Cancel,
    pub jitter: &'a mut dyn Jitter,
    pub diagnostics: &'a mut dyn Diagnostics,
}

/// Serializes action sequences between worker threads, mirroring the Linux
/// `ACTION_LOCK`. The guard is held for one sequence/loop iteration and then
/// released, so a looping macro cannot block other macros forever.
#[derive(Clone, Default)]
pub struct ActionGate {
    inner: Arc<Mutex<()>>,
}

impl ActionGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lock(&self) -> MutexGuard<'_, ()> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Release every key this macro currently holds and clear the set.
pub fn release_all_keys(injector: &mut dyn Injector, pressed: &mut HashSet<Key>) {
    let held: Vec<Key> = pressed.iter().copied().collect();
    for key in held {
        injector.key(key, false);
    }
    pressed.clear();
}

fn abort(ctx: &mut ActionContext<'_>) -> bool {
    release_all_keys(ctx.injector, ctx.pressed);
    false
}

fn is_cancelled(ctx: &ActionContext<'_>) -> bool {
    ctx.cancel.is_cancelled()
}

/// Sleep that can be interrupted. Returns `false` if cancelled.
///
/// A zero duration returns immediately, exactly like Linux `cancellable_sleep`
/// (jitter is not applied to zero delays).
pub fn cancellable_sleep(ctx: &mut ActionContext<'_>, ms: u64, jitter_ms: u64) -> bool {
    if ms == 0 {
        return true;
    }
    let effective_ms = if jitter_ms > 0 {
        // Profil `delayMs` değeri u64'tür; `as i64` taşması büyük bir
        // gecikmeyi 1 ms'ye çeviriyordu. Doygun aritmetik kullanılır.
        let delta = ctx.jitter.jitter(jitter_ms);
        let base = i64::try_from(ms).unwrap_or(i64::MAX);
        u64::try_from(base.saturating_add(delta).max(1)).unwrap_or(u64::MAX)
    } else {
        ms
    };
    let mut remaining = effective_ms;
    while remaining > 0 {
        if is_cancelled(ctx) {
            return false;
        }
        let chunk = remaining.min(4);
        ctx.waiter.sleep_ms(chunk);
        remaining -= chunk;
    }
    true
}

/// Resolve the key token of an action. `mouse_first` follows the Linux field
/// precedence: KEY_* prefers `key`, MOUSE_* prefers `mouseButton`.
fn resolve_or_warn(
    ctx: &mut ActionContext<'_>,
    action: &MacroAction,
    mouse_first: bool,
) -> Option<Key> {
    let token = if mouse_first {
        action.mouse_button.as_deref().or(action.key.as_deref())
    } else {
        action.key.as_deref().or(action.mouse_button.as_deref())
    };
    let key = token.and_then(keymap::parse);
    if key.is_none() {
        match token {
            Some(token) => ctx
                .diagnostics
                .warn(format!("action key not understood on Windows: '{token}'")),
            None => ctx
                .diagnostics
                .warn("action has neither 'key' nor 'mouseButton'".to_string()),
        }
    }
    key
}

/// Execute an action sequence. Returns `false` when cancelled; in that case
/// every key pressed by this sequence has already been released.
pub fn execute_actions(actions: &[MacroAction], ctx: &mut ActionContext<'_>) -> bool {
    for action in actions {
        if is_cancelled(ctx) {
            return abort(ctx);
        }

        match action.r#type {
            ActionType::KEY_DOWN => {
                if let Some(key) = resolve_or_warn(ctx, action, false) {
                    ctx.injector.key(key, true);
                    ctx.pressed.insert(key);
                }
            }
            ActionType::KEY_UP => {
                if let Some(key) = resolve_or_warn(ctx, action, false) {
                    ctx.injector.key(key, false);
                    ctx.pressed.remove(&key);
                }
            }
            ActionType::KEY_TAP => {
                if let Some(key) = resolve_or_warn(ctx, action, false) {
                    ctx.injector.key(key, true);
                    ctx.pressed.insert(key);
                    let tap_ms = action.delay_ms.unwrap_or(25).max(10);
                    if !cancellable_sleep(ctx, tap_ms, 0) {
                        return abort(ctx);
                    }
                    ctx.injector.key(key, false);
                    ctx.pressed.remove(&key);
                    if !cancellable_sleep(ctx, 10, 0) {
                        return abort(ctx);
                    }
                }
            }
            ActionType::MOUSE_DOWN => {
                if let Some(key) = resolve_or_warn(ctx, action, true) {
                    ctx.injector.key(key, true);
                    ctx.pressed.insert(key);
                }
            }
            ActionType::MOUSE_UP => {
                if let Some(key) = resolve_or_warn(ctx, action, true) {
                    ctx.injector.key(key, false);
                    ctx.pressed.remove(&key);
                }
            }
            ActionType::MOUSE_CLICK => {
                if let Some(key) = resolve_or_warn(ctx, action, true) {
                    ctx.injector.key(key, true);
                    ctx.pressed.insert(key);
                    let click_ms = action.delay_ms.unwrap_or(20).max(10);
                    if !cancellable_sleep(ctx, click_ms, 0) {
                        return abort(ctx);
                    }
                    ctx.injector.key(key, false);
                    ctx.pressed.remove(&key);
                    if !cancellable_sleep(ctx, 10, 0) {
                        return abort(ctx);
                    }
                }
            }
            ActionType::MOUSE_MOVE => {
                let dx = action.dx.unwrap_or(0);
                let dy = action.dy.unwrap_or(0);
                ctx.injector.mouse_move(dx, dy);
            }
            ActionType::MOUSE_SCROLL => {
                let delta = action.scroll_delta.unwrap_or(0);
                ctx.injector.mouse_wheel(delta);
            }
            ActionType::MOUSE_PATH => {
                if is_cancelled(ctx) {
                    return abort(ctx);
                }
                if let Some([x, y]) = action.start {
                    let x = x.clamp(-MAX_MOUSE_PATH_START, MAX_MOUSE_PATH_START);
                    let y = y.clamp(-MAX_MOUSE_PATH_START, MAX_MOUSE_PATH_START);
                    ctx.injector.mouse_absolute(x, y);
                }
                let points = action.points.as_deref().unwrap_or(&[]);
                for point in points.iter().take(MAX_MOUSE_PATH_POINTS) {
                    let dt = point[0].clamp(0, MAX_MOUSE_PATH_STEP_MS) as u64;
                    let dx = point[1].clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                    let dy = point[2].clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                    if !cancellable_sleep(ctx, dt, 0) {
                        return abort(ctx);
                    }
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    ctx.injector.mouse_move(dx, dy);
                }
            }
            ActionType::TEXT => {
                if let Some(text) = action.text.as_deref() {
                    for ch in text.chars() {
                        if is_cancelled(ctx) {
                            return abort(ctx);
                        }
                        if ctx.injector.text_char(ch) {
                            // Linux waits 16 ms between characters; Windows
                            // Unicode injection keeps the same cadence.
                            if !cancellable_sleep(ctx, 16, 0) {
                                return abort(ctx);
                            }
                        } else {
                            // Karakterin kendisi ASLA günlüğe yazılmaz:
                            // TEXT eylemi parola/2FA kodu taşıyabilir.
                            ctx.diagnostics
                                .warn("bir metin karakteri enjekte edilemedi".to_string());
                        }
                    }
                }
            }
            ActionType::DELAY => {
                let ms = action.delay_ms.unwrap_or(20);
                let jitter = action.jitter_ms.unwrap_or(0);
                if !cancellable_sleep(ctx, ms, jitter) {
                    return abort(ctx);
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Event {
        Key(Key, bool),
        Move(i32, i32),
        Wheel(i32),
        Absolute(i32, i32),
        Char(char),
    }

    #[derive(Default)]
    struct FakeInjector {
        events: Vec<Event>,
        unsupported: HashSet<char>,
    }

    impl Injector for FakeInjector {
        fn key(&mut self, key: Key, down: bool) {
            self.events.push(Event::Key(key, down));
        }
        fn mouse_move(&mut self, dx: i32, dy: i32) {
            self.events.push(Event::Move(dx, dy));
        }
        fn mouse_wheel(&mut self, delta: i32) {
            self.events.push(Event::Wheel(delta));
        }
        fn mouse_absolute(&mut self, x: i32, y: i32) {
            self.events.push(Event::Absolute(x, y));
        }
        fn text_char(&mut self, ch: char) -> bool {
            if self.unsupported.contains(&ch) {
                return false;
            }
            self.events.push(Event::Char(ch));
            true
        }
    }

    #[derive(Default)]
    struct FakeWaiter {
        total_ms: u64,
        calls: u64,
    }

    impl Waiter for FakeWaiter {
        fn sleep_ms(&mut self, ms: u64) {
            self.total_ms += ms;
            self.calls += 1;
        }
    }

    struct Flag(bool);
    impl Cancel for Flag {
        fn is_cancelled(&self) -> bool {
            self.0
        }
    }

    struct FixedJitter(i64);
    impl Jitter for FixedJitter {
        fn jitter(&mut self, _max_ms: u64) -> i64 {
            self.0
        }
    }

    #[derive(Default)]
    struct Diag(Vec<String>);
    impl Diagnostics for Diag {
        fn warn(&mut self, message: String) {
            self.0.push(message);
        }
    }

    struct Harness {
        injector: FakeInjector,
        pressed: HashSet<Key>,
        waiter: FakeWaiter,
        cancel: Flag,
        jitter: FixedJitter,
        diagnostics: Diag,
    }

    impl Harness {
        fn new() -> Self {
            Self {
                injector: FakeInjector::default(),
                pressed: HashSet::new(),
                waiter: FakeWaiter::default(),
                cancel: Flag(false),
                jitter: FixedJitter(0),
                diagnostics: Diag::default(),
            }
        }

        fn run(&mut self, actions: &[MacroAction]) -> bool {
            let mut ctx = ActionContext {
                injector: &mut self.injector,
                pressed: &mut self.pressed,
                waiter: &mut self.waiter,
                cancel: &self.cancel,
                jitter: &mut self.jitter,
                diagnostics: &mut self.diagnostics,
            };
            execute_actions(actions, &mut ctx)
        }
    }

    fn action(action_type: ActionType) -> MacroAction {
        MacroAction {
            id: String::new(),
            r#type: action_type,
            key: None,
            mouse_button: None,
            delay_ms: None,
            jitter_ms: None,
            text: None,
            dx: None,
            dy: None,
            scroll_delta: None,
            points: None,
            start: None,
        }
    }

    #[test]
    fn key_tap_uses_default_and_minimum_timing() {
        let mut harness = Harness::new();
        let mut tap = action(ActionType::KEY_TAP);
        tap.key = Some("KEY_X".to_string());
        assert!(harness.run(&[tap.clone()]));
        assert_eq!(
            harness.injector.events,
            vec![Event::Key(Key::KEY_X, true), Event::Key(Key::KEY_X, false)]
        );
        assert_eq!(harness.waiter.total_ms, 35); // 25 + 10
        assert!(harness.pressed.is_empty());

        let mut fast = tap.clone();
        fast.delay_ms = Some(1);
        let mut harness = Harness::new();
        assert!(harness.run(&[fast]));
        assert_eq!(harness.waiter.total_ms, 20); // max(1,10) + 10
    }

    #[test]
    fn mouse_click_uses_default_timing() {
        let mut harness = Harness::new();
        let mut click = action(ActionType::MOUSE_CLICK);
        click.mouse_button = Some("BTN_LEFT".to_string());
        assert!(harness.run(&[click]));
        assert_eq!(
            harness.injector.events,
            vec![
                Event::Key(Key::BTN_LEFT, true),
                Event::Key(Key::BTN_LEFT, false)
            ]
        );
        assert_eq!(harness.waiter.total_ms, 30); // 20 + 10
    }

    #[test]
    fn key_and_mouse_field_precedence_matches_linux() {
        let mut harness = Harness::new();
        let mut tap = action(ActionType::KEY_TAP);
        tap.key = Some("KEY_A".to_string());
        tap.mouse_button = Some("BTN_RIGHT".to_string());
        harness.run(&[tap]);
        assert_eq!(harness.injector.events[0], Event::Key(Key::KEY_A, true));

        let mut harness = Harness::new();
        let mut click = action(ActionType::MOUSE_CLICK);
        click.key = Some("KEY_A".to_string());
        click.mouse_button = Some("BTN_RIGHT".to_string());
        harness.run(&[click]);
        assert_eq!(harness.injector.events[0], Event::Key(Key::BTN_RIGHT, true));
    }

    #[test]
    fn down_up_and_release_all_clear_stuck_keys() {
        let mut harness = Harness::new();
        let mut down_a = action(ActionType::KEY_DOWN);
        down_a.key = Some("KEY_A".to_string());
        let mut down_shift = action(ActionType::KEY_DOWN);
        down_shift.key = Some("KEY_LEFTSHIFT".to_string());
        assert!(harness.run(&[down_a, down_shift]));
        assert_eq!(harness.pressed.len(), 2);

        release_all_keys(&mut harness.injector, &mut harness.pressed);
        assert!(harness.pressed.is_empty());
        let releases = harness
            .injector
            .events
            .iter()
            .filter(|event| matches!(event, Event::Key(_, false)))
            .count();
        assert_eq!(releases, 2);
    }

    #[test]
    fn move_and_scroll_default_to_zero() {
        let mut harness = Harness::new();
        assert!(harness.run(&[
            action(ActionType::MOUSE_MOVE),
            action(ActionType::MOUSE_SCROLL),
        ]));
        assert_eq!(
            harness.injector.events,
            vec![Event::Move(0, 0), Event::Wheel(0)]
        );
    }

    #[test]
    fn delay_applies_jitter_and_never_goes_below_one_ms() {
        let mut harness = Harness::new();
        harness.jitter = FixedJitter(5);
        let mut delay = action(ActionType::DELAY);
        delay.delay_ms = Some(10);
        delay.jitter_ms = Some(5);
        assert!(harness.run(&[delay]));
        assert_eq!(harness.waiter.total_ms, 15);

        let mut harness = Harness::new();
        harness.jitter = FixedJitter(-100);
        let mut delay = action(ActionType::DELAY);
        delay.delay_ms = Some(10);
        delay.jitter_ms = Some(100);
        assert!(harness.run(&[delay]));
        assert_eq!(harness.waiter.total_ms, 1);
    }

    #[test]
    fn zero_delay_returns_without_sleeping_or_jitter() {
        let mut harness = Harness::new();
        harness.jitter = FixedJitter(100);
        let mut delay = action(ActionType::DELAY);
        delay.delay_ms = Some(0);
        delay.jitter_ms = Some(100);
        assert!(harness.run(&[delay]));
        assert_eq!(harness.waiter.calls, 0);
    }

    #[test]
    fn text_injects_unicode_with_16ms_cadence() {
        let mut harness = Harness::new();
        let mut text = action(ActionType::TEXT);
        text.text = Some("aé😀".to_string());
        assert!(harness.run(&[text]));
        assert_eq!(
            harness.injector.events,
            vec![Event::Char('a'), Event::Char('é'), Event::Char('😀')]
        );
        assert_eq!(harness.waiter.total_ms, 48);
        assert!(harness.diagnostics.0.is_empty());
    }

    #[test]
    fn unsupported_character_warns_without_delay() {
        let mut harness = Harness::new();
        harness.injector.unsupported.insert('?');
        let mut text = action(ActionType::TEXT);
        text.text = Some("a?".to_string());
        assert!(harness.run(&[text]));
        assert_eq!(harness.injector.events, vec![Event::Char('a')]);
        assert_eq!(harness.waiter.total_ms, 16);
        assert_eq!(harness.diagnostics.0.len(), 1);
    }

    #[test]
    fn mouse_path_warps_clamps_and_skips_zero_deltas() {
        let mut harness = Harness::new();
        let mut path = action(ActionType::MOUSE_PATH);
        path.start = Some([2_000_000, -2_000_000]);
        path.points = Some(vec![
            [10, 5, -5],
            [0, 0, 0],
            [-10, 1, 2],
            [MAX_MOUSE_PATH_STEP_MS + 1, 3, 4],
        ]);
        assert!(harness.run(&[path]));
        assert_eq!(
            harness.injector.events,
            vec![
                Event::Absolute(MAX_MOUSE_PATH_START, -MAX_MOUSE_PATH_START),
                Event::Move(5, -5),
                Event::Move(1, 2),
                Event::Move(3, 4),
            ]
        );
        // dt: 10 + 0 + 0 + clamp(600001) = 600010
        assert_eq!(harness.waiter.total_ms, 600_010);
    }

    #[test]
    fn mouse_path_caps_point_count() {
        let mut harness = Harness::new();
        let mut path = action(ActionType::MOUSE_PATH);
        let points = vec![[0_i64, 1_i64, 0_i64]; MAX_MOUSE_PATH_POINTS + 2];
        path.points = Some(points);
        assert!(harness.run(&[path]));
        assert_eq!(harness.injector.events.len(), MAX_MOUSE_PATH_POINTS);
    }

    #[test]
    fn cancellation_releases_keys_and_returns_false() {
        let mut harness = Harness::new();
        harness.cancel = Flag(true);
        let mut down = action(ActionType::KEY_DOWN);
        down.key = Some("KEY_A".to_string());
        assert!(!harness.run(&[down]));
        // Cancelled before the down event: nothing was pressed.
        assert!(harness.injector.events.is_empty());
        assert!(harness.pressed.is_empty());
    }

    #[test]
    fn cancellation_after_key_down_releases_it() {
        let mut harness = Harness::new();
        let mut down = action(ActionType::KEY_DOWN);
        down.key = Some("KEY_A".to_string());
        let mut delay = action(ActionType::DELAY);
        delay.delay_ms = Some(1000);
        // Run down first, then a cancelled delay.
        assert!(harness.run(&[down]));
        assert_eq!(harness.pressed.len(), 1);
        harness.cancel = Flag(true);
        assert!(!harness.run(&[delay]));
        assert!(harness.pressed.is_empty());
        assert!(harness
            .injector
            .events
            .contains(&Event::Key(Key::KEY_A, false)));
    }

    #[test]
    fn invalid_action_key_warns_and_is_skipped() {
        let mut harness = Harness::new();
        let mut tap = action(ActionType::KEY_TAP);
        tap.key = Some("NOT_A_KEY".to_string());
        assert!(harness.run(&[tap]));
        assert!(harness.injector.events.is_empty());
        assert_eq!(harness.diagnostics.0.len(), 1);

        let mut missing = action(ActionType::KEY_TAP);
        missing.key = None;
        assert!(harness.run(&[missing]));
        assert_eq!(harness.diagnostics.0.len(), 2);
    }

    #[test]
    fn action_gate_serializes() {
        let gate = ActionGate::new();
        let clone = gate.clone();
        let first = gate.lock();
        let handle = std::thread::spawn(move || {
            let _guard = clone.lock();
            true
        });
        // The helper thread must still be blocked; drop and join.
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(!handle.is_finished());
        drop(first);
        assert!(handle.join().unwrap());
    }
}
