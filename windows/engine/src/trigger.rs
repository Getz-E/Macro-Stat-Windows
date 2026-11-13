// SPDX-License-Identifier: GPL-3.0-or-later
//! Trigger compilation and the 7-mode trigger state machine.
//!
//! Pure logic: no Windows API, no threads, no sleeping. The runtime feeds key
//! transitions in, receives [`TriggerCommand`]s, and owns the workers that
//! execute actions. Time is injected so the state machine is deterministic in
//! tests; callers must pass a monotonically non-decreasing [`Instant`].
//!
//! Semantics are ported one-to-one from the Linux engine:
//!
//! * auto-repeat (`value == 2` on Linux, repeated `WM_KEYDOWN` on Windows) is
//!   ignored because a press for an already-held key does nothing;
//! * combo macros latch until one of their keys is released;
//! * toggling a `TOGGLE_LOOP`/`REPEAT_COUNT` macro while it runs stops it;
//! * `WHILE_HELD` workers stop when a required key is released;
//! * `HOLD_LONG_PRESS` fires only if the triggering key is still held after
//!   the threshold; the timer itself lives in the runtime;
//! * `PRESS_AND_RELEASE` runs on press and its `releaseActions` once on the
//!   first matching release.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::keymap::{keys_are_equivalent, parse_detailed, Key, KeyParseError};
use crate::profile::{ProfileConfig, TriggerMode};

/// Why a macro was skipped while compiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileWarningKind {
    /// Token is not in the key dictionary.
    UnknownKey,
    /// `CODE_<n>` / bare numeric evdev code: Linux-only key language.
    NumericKey,
}

/// A non-fatal problem found while compiling a profile's triggers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileWarning {
    /// Index into [`ProfileConfig::macros`].
    pub source_index: usize,
    pub macro_name: String,
    pub trigger_key: String,
    /// The offending token (empty when the trigger string had no tokens).
    pub token: String,
    pub kind: CompileWarningKind,
}

/// Result of compiling a profile's triggers.
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub macros: Vec<CompiledMacro>,
    pub warnings: Vec<CompileWarning>,
}

/// A macro whose trigger and mode are resolved. Actions stay in the profile
/// (indexed by [`CompiledMacro::source_index`]) so they are not cloned per
/// event.
#[derive(Debug, Clone)]
pub struct CompiledMacro {
    /// Stable log identifier: `<macro id>#<index>` (`idx-<index>` if unnamed).
    pub id: String,
    pub source_index: usize,
    pub name: String,
    pub trigger_key: String,
    pub mode: TriggerMode,
    pub single: Option<Key>,
    pub combo: Vec<Key>,
    pub repeat_count: u32,
    pub double_tap_ms: u64,
    pub hold_ms: u64,
    pub has_release_actions: bool,
}

impl CompiledMacro {
    /// Keys that must all be held for combo matching / `WHILE_HELD`.
    pub fn required_keys(&self) -> Vec<Key> {
        if let Some(single) = self.single {
            vec![single]
        } else {
            self.combo.clone()
        }
    }
}

/// Compile enabled macros into trigger form; invalid triggers are skipped and
/// reported, matching the Linux engine's warning-and-continue behaviour.
pub fn compile_macros(profile: &ProfileConfig) -> CompileResult {
    let mut macros = Vec::new();
    let mut warnings = Vec::new();

    for (index, macro_config) in profile.macros.iter().enumerate() {
        if !macro_config.enabled {
            continue;
        }
        let parts: Vec<&str> = macro_config
            .trigger_key
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();

        if parts.is_empty() {
            warnings.push(CompileWarning {
                source_index: index,
                macro_name: macro_config.name.clone(),
                trigger_key: macro_config.trigger_key.clone(),
                token: String::new(),
                kind: CompileWarningKind::UnknownKey,
            });
            continue;
        }

        let mut keys = Vec::with_capacity(parts.len());
        let mut failed = false;
        for part in &parts {
            match parse_detailed(part) {
                Ok(key) => keys.push(key),
                Err(error) => {
                    failed = true;
                    warnings.push(CompileWarning {
                        source_index: index,
                        macro_name: macro_config.name.clone(),
                        trigger_key: macro_config.trigger_key.clone(),
                        token: (*part).to_string(),
                        kind: match error {
                            KeyParseError::NumericPlatformSpecific => {
                                CompileWarningKind::NumericKey
                            }
                            _ => CompileWarningKind::UnknownKey,
                        },
                    });
                }
            }
        }
        if parts.is_empty() || failed {
            continue;
        }

        let id = if macro_config.id.is_empty() {
            format!("idx-{}", index)
        } else {
            format!("{}#{}", macro_config.id, index)
        };
        let single = if keys.len() == 1 { Some(keys[0]) } else { None };
        macros.push(CompiledMacro {
            id,
            source_index: index,
            name: macro_config.name.clone(),
            trigger_key: macro_config.trigger_key.clone(),
            mode: macro_config.trigger_mode,
            single,
            combo: if keys.len() > 1 { keys } else { Vec::new() },
            repeat_count: macro_config.repeat_count.max(1),
            double_tap_ms: macro_config.double_tap_timeout_ms.max(1),
            hold_ms: macro_config.hold_duration_ms.max(1),
            has_release_actions: !macro_config.release_actions.is_empty(),
        });
    }

    CompileResult { macros, warnings }
}

fn trigger_matches(macro_config: &CompiledMacro, key: Key, held: &HashSet<Key>) -> bool {
    if let Some(single) = macro_config.single {
        keys_are_equivalent(single, key)
    } else {
        macro_config
            .combo
            .iter()
            .any(|required| keys_are_equivalent(*required, key))
            && macro_config
                .combo
                .iter()
                .all(|required| held.iter().any(|h| keys_are_equivalent(*required, *h)))
    }
}

fn all_required_held(macro_config: &CompiledMacro, held: &HashSet<Key>) -> bool {
    macro_config
        .required_keys()
        .iter()
        .all(|required| held.iter().any(|h| keys_are_equivalent(*required, *h)))
}

/// What the runtime should do in response to a trigger transition.
#[derive(Debug, Clone)]
pub enum TriggerCommand {
    /// Panic key pressed. The runtime must release injected keys and exit.
    Panic,
    /// Run the macro's actions once (ONCE / DOUBLE_TAP / press phase).
    RunOnce { macro_index: usize },
    /// Start the looping worker; it must stop when `flag` turns false.
    StartLoop {
        macro_index: usize,
        /// `Some(n)` for REPEAT_COUNT, `None` for TOGGLE_LOOP.
        repeat: Option<u32>,
        flag: Arc<AtomicBool>,
    },
    /// The same trigger was pressed again while looping; stop and release.
    StopLoop { macro_index: usize },
    /// Start a WHILE_HELD worker; it must stop when `flag` turns false or a
    /// required key is released.
    StartWhileHeld {
        macro_index: usize,
        flag: Arc<AtomicBool>,
    },
    /// The key was held for `hold_ms`; the runtime checks the key is still
    /// held and then runs the actions.
    StartHold {
        macro_index: usize,
        key: Key,
        hold_ms: u64,
    },
    /// Run the macro's `releaseActions` once (PRESS_AND_RELEASE).
    RunReleaseActions { macro_index: usize },
}

/// The pure trigger state machine.
pub struct TriggerEngine {
    macros: Vec<CompiledMacro>,
    panic_key: Key,
    held: Arc<RwLock<HashSet<Key>>>,
    latched_combos: HashSet<usize>,
    /// Running REPEAT_COUNT / TOGGLE_LOOP workers, keyed by macro index.
    loops: HashMap<usize, Arc<AtomicBool>>,
    /// Running WHILE_HELD workers; run id -> (macro index, stop flag).
    while_held: HashMap<u64, (usize, Arc<AtomicBool>)>,
    next_run_id: u64,
    press_release_armed: HashSet<usize>,
    double_tap: HashMap<(usize, Key), Instant>,
}

impl TriggerEngine {
    /// Compile `profile` and build the engine. Warnings are returned for the
    /// log; they do not prevent the engine from starting.
    pub fn new(profile: &ProfileConfig) -> (Self, CompileResult) {
        let compiled = compile_macros(profile);
        let engine = TriggerEngine {
            macros: compiled.macros.clone(),
            panic_key: profile.resolved_panic_key(),
            held: Arc::new(RwLock::new(HashSet::new())),
            latched_combos: HashSet::new(),
            loops: HashMap::new(),
            while_held: HashMap::new(),
            next_run_id: 0,
            press_release_armed: HashSet::new(),
            double_tap: HashMap::new(),
        };
        (engine, compiled)
    }

    pub fn macros(&self) -> &[CompiledMacro] {
        &self.macros
    }

    pub fn macro_at(&self, index: usize) -> Option<&CompiledMacro> {
        self.macros.get(index)
    }

    pub fn panic_key(&self) -> Key {
        self.panic_key
    }

    /// Shared physical-held snapshot for worker threads (`WHILE_HELD`,
    /// `HOLD_LONG_PRESS`).
    pub fn held_handle(&self) -> Arc<RwLock<HashSet<Key>>> {
        Arc::clone(&self.held)
    }

    pub fn is_held(&self, key: Key) -> bool {
        self.held
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .any(|h| keys_are_equivalent(*h, key))
    }

    /// Called by the runtime when a looping worker exits on its own.
    pub fn loop_finished(&mut self, macro_index: usize) {
        self.loops.remove(&macro_index);
    }

    /// Called by the runtime when a WHILE_HELD worker exits on its own.
    pub fn while_held_finished(&mut self, macro_index: usize) {
        self.while_held
            .retain(|_, (index, _)| *index != macro_index);
    }

    /// Number of looping workers currently registered.
    pub fn running_loops(&self) -> usize {
        self.loops.len()
    }

    /// Build a new trigger set after a profile reload, stopping all workers.
    pub fn reset_with(&mut self, profile: &ProfileConfig) -> CompileResult {
        self.reset_transient();
        let compiled = compile_macros(profile);
        self.macros = compiled.macros.clone();
        self.panic_key = profile.resolved_panic_key();
        compiled
    }

    /// Stop every worker and clear transient trigger state. The held set is
    /// physical state and is left untouched; the runtime releases injected
    /// keys separately.
    pub fn reset_transient(&mut self) {
        for flag in self.loops.values() {
            flag.store(false, Ordering::SeqCst);
        }
        for (_, flag) in self.while_held.values() {
            flag.store(false, Ordering::SeqCst);
        }
        self.loops.clear();
        self.while_held.clear();
        self.latched_combos.clear();
        self.press_release_armed.clear();
        self.double_tap.clear();
    }

    /// Feed one key transition. `is_press == true` for key-down, `false` for
    /// key-up. Repeated key-down for an already-held key is ignored.
    pub fn on_key(&mut self, key: Key, is_press: bool, now: Instant) -> Vec<TriggerCommand> {
        if is_press {
            {
                let mut held = self
                    .held
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if held.contains(&key) {
                    return Vec::new();
                }
                held.insert(key);
            }
            if keys_are_equivalent(key, self.panic_key) {
                self.reset_transient();
                return vec![TriggerCommand::Panic];
            }
            self.handle_press(key, now)
        } else {
            let removed = {
                let mut held = self
                    .held
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                held.remove(&key)
            };
            if !removed {
                return Vec::new();
            }
            self.handle_release(key)
        }
    }

    fn handle_press(&mut self, key: Key, now: Instant) -> Vec<TriggerCommand> {
        let held_snapshot = self
            .held
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();

        let matches: Vec<usize> = self
            .macros
            .iter()
            .enumerate()
            .filter(|(_, m)| trigger_matches(m, key, &held_snapshot))
            .map(|(index, _)| index)
            .collect();

        let mut commands = Vec::new();
        for macro_index in matches {
            let macro_config = self.macros[macro_index].clone();
            if !macro_config.combo.is_empty() {
                if self.latched_combos.contains(&macro_index) {
                    continue;
                }
                self.latched_combos.insert(macro_index);
            }

            match macro_config.mode {
                TriggerMode::ONCE => {
                    commands.push(TriggerCommand::RunOnce { macro_index });
                }
                TriggerMode::REPEAT_COUNT | TriggerMode::TOGGLE_LOOP => {
                    if let Some(flag) = self.loops.get(&macro_index) {
                        if flag.load(Ordering::SeqCst) {
                            flag.store(false, Ordering::SeqCst);
                            self.loops.remove(&macro_index);
                            commands.push(TriggerCommand::StopLoop { macro_index });
                            continue;
                        }
                    }
                    let flag = Arc::new(AtomicBool::new(true));
                    self.loops.insert(macro_index, Arc::clone(&flag));
                    commands.push(TriggerCommand::StartLoop {
                        macro_index,
                        repeat: (macro_config.mode == TriggerMode::REPEAT_COUNT)
                            .then_some(macro_config.repeat_count),
                        flag,
                    });
                }
                TriggerMode::WHILE_HELD => {
                    let flag = Arc::new(AtomicBool::new(true));
                    let run_id = self.next_run_id;
                    self.next_run_id += 1;
                    self.while_held
                        .insert(run_id, (macro_index, Arc::clone(&flag)));
                    commands.push(TriggerCommand::StartWhileHeld { macro_index, flag });
                }
                TriggerMode::DOUBLE_TAP => {
                    let tracker_key = (macro_index, key.physical_class());
                    let window = Duration::from_millis(macro_config.double_tap_ms);
                    let retention = Duration::from_secs(5).max(window);
                    self.double_tap.retain(|_, last| {
                        now.checked_duration_since(*last)
                            .is_some_and(|elapsed| elapsed < retention)
                    });
                    let is_double = self
                        .double_tap
                        .get(&tracker_key)
                        .and_then(|last| now.checked_duration_since(*last))
                        .is_some_and(|elapsed| elapsed <= window);
                    if is_double {
                        self.double_tap.remove(&tracker_key);
                        commands.push(TriggerCommand::RunOnce { macro_index });
                    } else {
                        self.double_tap.insert(tracker_key, now);
                    }
                }
                TriggerMode::HOLD_LONG_PRESS => {
                    commands.push(TriggerCommand::StartHold {
                        macro_index,
                        key,
                        hold_ms: macro_config.hold_ms,
                    });
                }
                TriggerMode::PRESS_AND_RELEASE => {
                    if macro_config.has_release_actions {
                        self.press_release_armed.insert(macro_index);
                    }
                    commands.push(TriggerCommand::RunOnce { macro_index });
                }
            }
        }
        commands
    }

    fn handle_release(&mut self, key: Key) -> Vec<TriggerCommand> {
        // Release a combo latch when any of its keys is released.
        self.latched_combos.retain(|&macro_index| {
            !self.macros[macro_index]
                .combo
                .iter()
                .any(|required| keys_are_equivalent(*required, key))
        });

        let mut commands = Vec::new();

        // Stop WHILE_HELD workers whose required keys are no longer all held.
        let held_snapshot = self
            .held
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let finished: Vec<u64> = self
            .while_held
            .iter()
            .filter(|(_, (macro_index, _))| {
                !all_required_held(&self.macros[*macro_index], &held_snapshot)
            })
            .map(|(run_id, _)| *run_id)
            .collect();
        for run_id in finished {
            if let Some((_, flag)) = self.while_held.remove(&run_id) {
                flag.store(false, Ordering::SeqCst);
            }
        }

        // PRESS_AND_RELEASE: fire releaseActions once per press.
        let released_triggers: Vec<usize> = self
            .macros
            .iter()
            .enumerate()
            .filter(|(_, macro_config)| macro_config.mode == TriggerMode::PRESS_AND_RELEASE)
            .filter(|(_, macro_config)| {
                if let Some(single) = macro_config.single {
                    keys_are_equivalent(single, key)
                } else {
                    macro_config
                        .combo
                        .iter()
                        .any(|required| keys_are_equivalent(*required, key))
                }
            })
            .map(|(index, _)| index)
            .collect();
        for macro_index in released_triggers {
            if self.press_release_armed.remove(&macro_index) {
                commands.push(TriggerCommand::RunReleaseActions { macro_index });
            }
        }

        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{ActionType, MacroAction, MacroConfig, ProfileConfig};
    use std::time::Duration;

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

    fn macro_config(id: &str, trigger: &str, mode: TriggerMode) -> MacroConfig {
        MacroConfig {
            id: id.to_string(),
            name: format!("macro {id}"),
            enabled: true,
            trigger_key: trigger.to_string(),
            trigger_mode: mode,
            repeat_count: 1,
            double_tap_timeout_ms: 300,
            hold_duration_ms: 300,
            release_actions: Vec::new(),
            actions: vec![action(ActionType::KEY_TAP)],
        }
    }

    fn profile_with(macros: Vec<MacroConfig>) -> ProfileConfig {
        ProfileConfig {
            id: "p".to_string(),
            name: "P".to_string(),
            panic_key: "KEY_PAUSE".to_string(),
            virtual_device_name: "V".to_string(),
            macros,
        }
    }

    fn engine_for(macros: Vec<MacroConfig>) -> TriggerEngine {
        let (engine, result) = TriggerEngine::new(&profile_with(macros));
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        engine
    }

    /// One fixed base per process so all test timestamps are comparable.
    fn t(ms: u64) -> Instant {
        static BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        *BASE.get_or_init(Instant::now) + Duration::from_millis(ms)
    }

    fn run_once_index(commands: &[TriggerCommand]) -> Option<usize> {
        commands.iter().find_map(|command| match command {
            TriggerCommand::RunOnce { macro_index } => Some(*macro_index),
            _ => None,
        })
    }

    #[test]
    fn compile_skips_disabled_and_reports_bad_triggers() {
        let mut disabled = macro_config("d", "KEY_A", TriggerMode::ONCE);
        disabled.enabled = false;
        let profile = profile_with(vec![
            macro_config("ok", "KEY_F8", TriggerMode::ONCE),
            disabled,
            macro_config("bad", "KEY_A+NOPE", TriggerMode::ONCE),
            macro_config("numeric", "65", TriggerMode::ONCE),
            macro_config("empty", " + ", TriggerMode::ONCE),
        ]);
        let result = compile_macros(&profile);
        assert_eq!(result.macros.len(), 1);
        assert_eq!(result.macros[0].id, "ok#0");
        assert_eq!(result.warnings.len(), 3);
        assert_eq!(result.warnings[0].kind, CompileWarningKind::UnknownKey);
        assert_eq!(result.warnings[0].token, "NOPE");
        assert_eq!(result.warnings[1].kind, CompileWarningKind::NumericKey);
        assert_eq!(result.warnings[2].token, "");
    }

    #[test]
    fn compile_maps_optional_prefix_and_combos() {
        let profile = profile_with(vec![
            macro_config("single", "f8", TriggerMode::ONCE),
            macro_config("combo", "KEY_LEFTSHIFT+KEY_LEFTCTRL", TriggerMode::ONCE),
        ]);
        let result = compile_macros(&profile);
        assert_eq!(result.macros[0].single, Some(Key::KEY_F8));
        assert!(result.macros[0].combo.is_empty());
        assert_eq!(result.macros[1].single, None);
        assert_eq!(
            result.macros[1].combo,
            vec![Key::KEY_LEFTSHIFT, Key::KEY_LEFTCTRL]
        );
    }

    #[test]
    fn compile_clamps_zero_timers_and_repeat() {
        let mut macro_config = macro_config("m", "KEY_A", TriggerMode::REPEAT_COUNT);
        macro_config.repeat_count = 0;
        macro_config.double_tap_timeout_ms = 0;
        macro_config.hold_duration_ms = 0;
        let result = compile_macros(&profile_with(vec![macro_config]));
        assert_eq!(result.macros[0].repeat_count, 1);
        assert_eq!(result.macros[0].double_tap_ms, 1);
        assert_eq!(result.macros[0].hold_ms, 1);
    }

    #[test]
    fn once_runs_on_press_and_ignores_auto_repeat() {
        let mut engine = engine_for(vec![macro_config("m", "KEY_A", TriggerMode::ONCE)]);
        let commands = engine.on_key(Key::KEY_A, true, t(0));
        assert_eq!(run_once_index(&commands), Some(0));
        assert!(engine.on_key(Key::KEY_A, true, t(10)).is_empty());
        assert!(engine.on_key(Key::KEY_B, false, t(20)).is_empty());
        engine.on_key(Key::KEY_A, false, t(30));
        let commands = engine.on_key(Key::KEY_A, true, t(40));
        assert_eq!(run_once_index(&commands), Some(0));
    }

    #[test]
    fn combo_matches_only_when_all_keys_are_held_and_latches() {
        let mut engine = engine_for(vec![macro_config(
            "combo",
            "KEY_LEFTCTRL+KEY_A",
            TriggerMode::ONCE,
        )]);
        assert!(engine.on_key(Key::KEY_LEFTCTRL, true, t(0)).is_empty());
        let commands = engine.on_key(Key::KEY_A, true, t(10));
        assert_eq!(run_once_index(&commands), Some(0));
        // Releasing any combo key opens the latch (Linux semantics), so
        // pressing Ctrl again while A is held fires once more.
        assert!(engine.on_key(Key::KEY_LEFTCTRL, false, t(20)).is_empty());
        let commands = engine.on_key(Key::KEY_LEFTCTRL, true, t(30));
        assert_eq!(run_once_index(&commands), Some(0));
        // Releasing A and pressing it again also fires.
        engine.on_key(Key::KEY_A, false, t(40));
        let commands = engine.on_key(Key::KEY_A, true, t(50));
        assert_eq!(run_once_index(&commands), Some(0));
    }

    #[test]
    fn right_modifier_matches_left_trigger() {
        let mut engine = engine_for(vec![macro_config(
            "combo",
            "KEY_LEFTCTRL+KEY_A",
            TriggerMode::ONCE,
        )]);
        engine.on_key(Key::KEY_RIGHTCTRL, true, t(0));
        let commands = engine.on_key(Key::KEY_A, true, t(1));
        assert_eq!(run_once_index(&commands), Some(0));
    }

    #[test]
    fn toggle_loop_starts_then_stops_on_second_press() {
        let mut engine = engine_for(vec![macro_config(
            "loop",
            "KEY_F8",
            TriggerMode::TOGGLE_LOOP,
        )]);
        let commands = engine.on_key(Key::KEY_F8, true, t(0));
        let flag = match &commands[0] {
            TriggerCommand::StartLoop {
                repeat: None, flag, ..
            } => Arc::clone(flag),
            other => panic!("unexpected {other:?}"),
        };
        assert!(flag.load(Ordering::SeqCst));
        engine.on_key(Key::KEY_F8, false, t(1));
        let commands = engine.on_key(Key::KEY_F8, true, t(2));
        assert!(matches!(commands[0], TriggerCommand::StopLoop { .. }));
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn repeat_count_carries_repeat_limit() {
        let mut macro_config = macro_config("rep", "KEY_F8", TriggerMode::REPEAT_COUNT);
        macro_config.repeat_count = 4;
        let mut engine = engine_for(vec![macro_config]);
        let commands = engine.on_key(Key::KEY_F8, true, t(0));
        match &commands[0] {
            TriggerCommand::StartLoop {
                repeat: Some(4),
                flag,
                ..
            } => assert!(flag.load(Ordering::SeqCst)),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn loop_finished_allows_a_fresh_start() {
        let mut engine = engine_for(vec![macro_config(
            "rep",
            "KEY_F8",
            TriggerMode::REPEAT_COUNT,
        )]);
        engine.on_key(Key::KEY_F8, true, t(0));
        engine.on_key(Key::KEY_F8, false, t(1));
        engine.loop_finished(0);
        let commands = engine.on_key(Key::KEY_F8, true, t(2));
        assert!(matches!(commands[0], TriggerCommand::StartLoop { .. }));
    }

    #[test]
    fn while_held_stops_on_release() {
        let mut engine = engine_for(vec![macro_config(
            "hold",
            "KEY_F8",
            TriggerMode::WHILE_HELD,
        )]);
        let commands = engine.on_key(Key::KEY_F8, true, t(0));
        let flag = match &commands[0] {
            TriggerCommand::StartWhileHeld { flag, .. } => Arc::clone(flag),
            other => panic!("unexpected {other:?}"),
        };
        assert!(flag.load(Ordering::SeqCst));
        engine.on_key(Key::KEY_F8, false, t(5));
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn while_held_combo_stops_when_any_key_released() {
        let mut engine = engine_for(vec![macro_config(
            "hold",
            "KEY_LEFTCTRL+KEY_A",
            TriggerMode::WHILE_HELD,
        )]);
        engine.on_key(Key::KEY_LEFTCTRL, true, t(0));
        let commands = engine.on_key(Key::KEY_A, true, t(1));
        let flag = match &commands[0] {
            TriggerCommand::StartWhileHeld { flag, .. } => Arc::clone(flag),
            other => panic!("unexpected {other:?}"),
        };
        assert!(flag.load(Ordering::SeqCst));
        engine.on_key(Key::KEY_LEFTCTRL, false, t(2));
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn double_tap_fires_on_second_press_within_window() {
        let mut engine = engine_for(vec![macro_config("dt", "KEY_F8", TriggerMode::DOUBLE_TAP)]);
        assert!(engine.on_key(Key::KEY_F8, true, t(0)).is_empty());
        engine.on_key(Key::KEY_F8, false, t(50));
        let commands = engine.on_key(Key::KEY_F8, true, t(100));
        assert_eq!(run_once_index(&commands), Some(0));
        // The pair is consumed: a third press starts a new pair.
        engine.on_key(Key::KEY_F8, false, t(120));
        assert!(engine.on_key(Key::KEY_F8, true, t(150)).is_empty());
    }

    #[test]
    fn double_tap_outside_window_does_not_fire() {
        let mut engine = engine_for(vec![macro_config("dt", "KEY_F8", TriggerMode::DOUBLE_TAP)]);
        engine.on_key(Key::KEY_F8, true, t(0));
        engine.on_key(Key::KEY_F8, false, t(10));
        let commands = engine.on_key(Key::KEY_F8, true, t(1000));
        assert!(run_once_index(&commands).is_none());
    }

    #[test]
    fn double_tap_treats_left_and_right_modifiers_as_one_key() {
        let mut engine = engine_for(vec![macro_config(
            "dt",
            "KEY_LEFTCTRL",
            TriggerMode::DOUBLE_TAP,
        )]);
        assert!(engine.on_key(Key::KEY_LEFTCTRL, true, t(0)).is_empty());
        engine.on_key(Key::KEY_LEFTCTRL, false, t(10));
        let commands = engine.on_key(Key::KEY_RIGHTCTRL, true, t(20));
        assert_eq!(run_once_index(&commands), Some(0));
    }

    #[test]
    fn hold_emits_threshold_for_runtime() {
        let mut macro_config = macro_config("hold", "KEY_F8", TriggerMode::HOLD_LONG_PRESS);
        macro_config.hold_duration_ms = 450;
        let mut engine = engine_for(vec![macro_config]);
        let commands = engine.on_key(Key::KEY_F8, true, t(0));
        match &commands[0] {
            TriggerCommand::StartHold {
                key, hold_ms: 450, ..
            } => assert_eq!(*key, Key::KEY_F8),
            other => panic!("unexpected {other:?}"),
        }
        assert!(engine.is_held(Key::KEY_F8));
        engine.on_key(Key::KEY_F8, false, t(100));
        assert!(!engine.is_held(Key::KEY_F8));
    }

    #[test]
    fn press_and_release_runs_release_actions_once() {
        let mut macro_config = macro_config("pr", "KEY_F8", TriggerMode::PRESS_AND_RELEASE);
        macro_config.release_actions = vec![action(ActionType::KEY_UP)];
        let mut engine = engine_for(vec![macro_config]);
        let commands = engine.on_key(Key::KEY_F8, true, t(0));
        assert_eq!(run_once_index(&commands), Some(0));
        let commands = engine.on_key(Key::KEY_F8, false, t(10));
        assert!(matches!(
            commands[0],
            TriggerCommand::RunReleaseActions { macro_index: 0 }
        ));
        // Only once: another release without a new press does nothing.
        assert!(engine.on_key(Key::KEY_F8, false, t(20)).is_empty());
        let commands = engine.on_key(Key::KEY_F8, true, t(30));
        assert_eq!(run_once_index(&commands), Some(0));
        engine.on_key(Key::KEY_F8, false, t(40));
    }

    #[test]
    fn press_and_release_without_release_actions_is_not_armed() {
        let mut engine = engine_for(vec![macro_config(
            "pr",
            "KEY_F8",
            TriggerMode::PRESS_AND_RELEASE,
        )]);
        engine.on_key(Key::KEY_F8, true, t(0));
        assert!(engine.on_key(Key::KEY_F8, false, t(10)).is_empty());
    }

    #[test]
    fn panic_stops_loops_and_returns_panic() {
        let mut engine = engine_for(vec![macro_config(
            "loop",
            "KEY_F8",
            TriggerMode::TOGGLE_LOOP,
        )]);
        let commands = engine.on_key(Key::KEY_F8, true, t(0));
        let flag = match &commands[0] {
            TriggerCommand::StartLoop { flag, .. } => Arc::clone(flag),
            other => panic!("unexpected {other:?}"),
        };
        let commands = engine.on_key(Key::KEY_PAUSE, true, t(1));
        assert!(matches!(commands[0], TriggerCommand::Panic));
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn reset_with_compiles_new_profile_and_stops_old_loops() {
        let mut engine = engine_for(vec![macro_config(
            "loop",
            "KEY_F8",
            TriggerMode::TOGGLE_LOOP,
        )]);
        let commands = engine.on_key(Key::KEY_F8, true, t(0));
        let flag = match &commands[0] {
            TriggerCommand::StartLoop { flag, .. } => Arc::clone(flag),
            other => panic!("unexpected {other:?}"),
        };
        let result = engine.reset_with(&profile_with(vec![macro_config(
            "new",
            "KEY_F9",
            TriggerMode::ONCE,
        )]));
        assert_eq!(result.macros.len(), 1);
        assert!(!flag.load(Ordering::SeqCst));
        assert!(engine.on_key(Key::KEY_F8, false, t(1)).is_empty());
        let commands = engine.on_key(Key::KEY_F9, true, t(2));
        assert_eq!(run_once_index(&commands), Some(0));
    }

    #[test]
    fn panic_key_is_configurable_and_falls_back_to_pause() {
        let mut profile = profile_with(Vec::new());
        profile.panic_key = "KEY_ESC".to_string();
        let (engine, _) = TriggerEngine::new(&profile);
        assert_eq!(engine.panic_key(), Key::KEY_ESC);

        profile.panic_key = "not-a-key".to_string();
        let (engine, _) = TriggerEngine::new(&profile);
        assert_eq!(engine.panic_key(), Key::KEY_PAUSE);
    }

    #[test]
    fn unknown_release_is_ignored() {
        let mut engine = engine_for(vec![macro_config("m", "KEY_A", TriggerMode::ONCE)]);
        assert!(engine.on_key(Key::KEY_A, false, t(0)).is_empty());
        assert!(!engine.is_held(Key::KEY_A));
    }
}
