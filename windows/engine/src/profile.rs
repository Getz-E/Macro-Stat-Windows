// SPDX-License-Identifier: GPL-3.0-or-later
//! Profile format shared with the Linux engine (frozen contract v1).
//!
//! Field names are camelCase on the wire; snake_case aliases are accepted for
//! hand-written files, exactly like the Linux engine. Unknown fields are
//! ignored on purpose so that newer writers can add data without breaking an
//! older engine.
//!
//! This module is pure: no filesystem, no Windows API. The Windows runtime
//! reads the file (bounded by [`MAX_PROFILE_BYTES`]) and hands the text here.

use serde::{Deserialize, Serialize};

/// Hard ceiling for a profile file, matching the Linux engine.
///
/// Guards against a corrupt or maliciously large file exhausting memory.
pub const MAX_PROFILE_BYTES: u64 = 32 * 1024 * 1024;

/// Maximum number of points replayed from one `MOUSE_PATH` action.
pub const MAX_MOUSE_PATH_POINTS: usize = 200_000;

/// Maximum accepted `dt` (ms) for one `MOUSE_PATH` point.
pub const MAX_MOUSE_PATH_STEP_MS: i64 = 600_000;

/// `MOUSE_PATH.start` coordinates are clamped to this magnitude.
pub const MAX_MOUSE_PATH_START: i32 = 1_000_000;

/// Maximum text payload accepted by the profile editors, per action.
pub const MAX_TEXT_BYTES: usize = 64 * 1024;

/// Display/identity fields are bounded so a 32 MiB profile cannot make token
/// matching, per-keystroke cloning or 250 ms status logging arbitrarily slow.
pub const MAX_ID_CHARS: usize = 128;
pub const MAX_NAME_CHARS: usize = 200;
pub const MAX_TRIGGER_CHARS: usize = 128;
pub const MAX_MACROS: usize = 500;
pub const MAX_ACTIONS_PER_MACRO: usize = 2_000;
/// Upper bounds for profile-controlled timings (contract §limits).
pub const MAX_DELAY_MS: u64 = 600_000;
pub const MAX_JITTER_MS: u64 = 1_000;
pub const MAX_DOUBLE_TAP_MS: u64 = 5_000;
pub const MAX_HOLD_MS: u64 = 10_000;
pub const MAX_REPEAT_COUNT: u32 = 9_999;
pub const MAX_MOUSE_DELTA: i32 = 1_000_000;
/// Wheel detents per action (one detent = 120 Windows wheel units). The
/// magnitude is preserved — only absurd values are clamped.
pub const MAX_SCROLL_DETENTS: i32 = 120;

/// One action inside a macro. Mirrors the Linux `MacroAction`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MacroAction {
    #[serde(default)]
    pub id: String,
    pub r#type: ActionType,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(rename = "mouseButton", alias = "mouse_button", default)]
    pub mouse_button: Option<String>,
    #[serde(rename = "delayMs", alias = "delay_ms", default)]
    pub delay_ms: Option<u64>,
    #[serde(rename = "jitterMs", alias = "jitter_ms", default)]
    pub jitter_ms: Option<u64>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub dx: Option<i32>,
    #[serde(default)]
    pub dy: Option<i32>,
    #[serde(rename = "scrollDelta", alias = "scroll_delta", default)]
    pub scroll_delta: Option<i32>,
    /// `MOUSE_PATH`: `[elapsed_ms, dx, dy]` triples. The duration is measured
    /// from the previous point; the first point waits from replay start.
    #[serde(default)]
    pub points: Option<Vec<[i64; 3]>>,
    /// `MOUSE_PATH`: virtual-desktop cursor position `[x, y]` at record start.
    /// When present the cursor is warped there before replay.
    #[serde(default)]
    pub start: Option<[i32; 2]>,
}

/// The 11 action types of contract v1.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionType {
    #[serde(
        alias = "key_down",
        alias = "KEY_DOWN",
        alias = "KeyDown",
        alias = "keyDown"
    )]
    KEY_DOWN,
    #[serde(alias = "key_up", alias = "KEY_UP", alias = "KeyUp", alias = "keyUp")]
    KEY_UP,
    #[serde(
        alias = "key_tap",
        alias = "KEY_TAP",
        alias = "KeyTap",
        alias = "keyTap"
    )]
    KEY_TAP,
    #[serde(
        alias = "mouse_down",
        alias = "MOUSE_DOWN",
        alias = "MouseDown",
        alias = "mouseDown"
    )]
    MOUSE_DOWN,
    #[serde(
        alias = "mouse_up",
        alias = "MOUSE_UP",
        alias = "MouseUp",
        alias = "mouseUp"
    )]
    MOUSE_UP,
    #[serde(
        alias = "mouse_click",
        alias = "MOUSE_CLICK",
        alias = "MouseClick",
        alias = "mouseClick"
    )]
    MOUSE_CLICK,
    #[serde(
        alias = "mouse_move",
        alias = "MOUSE_MOVE",
        alias = "MouseMove",
        alias = "mouseMove"
    )]
    MOUSE_MOVE,
    #[serde(
        alias = "mouse_scroll",
        alias = "MOUSE_SCROLL",
        alias = "MouseScroll",
        alias = "mouseScroll"
    )]
    MOUSE_SCROLL,
    #[serde(
        alias = "mouse_path",
        alias = "MOUSE_PATH",
        alias = "MousePath",
        alias = "mousePath"
    )]
    MOUSE_PATH,
    #[serde(alias = "text", alias = "TEXT", alias = "Text")]
    TEXT,
    #[serde(alias = "delay", alias = "DELAY", alias = "Delay")]
    DELAY,
}

/// The 7 trigger modes of contract v1.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TriggerMode {
    #[serde(alias = "once", alias = "ONCE", alias = "Once")]
    ONCE,
    #[serde(
        alias = "while_held",
        alias = "WHILE_HELD",
        alias = "WhileHeld",
        alias = "whileHeld"
    )]
    WHILE_HELD,
    #[serde(
        alias = "toggle_loop",
        alias = "TOGGLE_LOOP",
        alias = "ToggleLoop",
        alias = "toggleLoop"
    )]
    TOGGLE_LOOP,
    #[serde(
        alias = "repeat_count",
        alias = "REPEAT_COUNT",
        alias = "RepeatCount",
        alias = "repeatCount"
    )]
    REPEAT_COUNT,
    #[serde(
        alias = "double_tap",
        alias = "DOUBLE_TAP",
        alias = "DoubleTap",
        alias = "doubleTap"
    )]
    DOUBLE_TAP,
    #[serde(
        alias = "hold_long_press",
        alias = "HOLD_LONG_PRESS",
        alias = "HoldLongPress",
        alias = "holdLongPress",
        alias = "hold"
    )]
    HOLD_LONG_PRESS,
    #[serde(
        alias = "press_and_release",
        alias = "PRESS_AND_RELEASE",
        alias = "PressAndRelease",
        alias = "pressAndRelease"
    )]
    PRESS_AND_RELEASE,
}

fn default_macro_name() -> String {
    "Yeni Makro".to_string()
}
fn default_true() -> bool {
    true
}
fn default_trigger_mode() -> TriggerMode {
    TriggerMode::ONCE
}
fn default_repeat_count() -> u32 {
    1
}
fn default_double_tap_timeout() -> u64 {
    300
}
fn default_hold_duration() -> u64 {
    300
}

/// One macro definition. Mirrors the Linux `MacroConfig`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MacroConfig {
    #[serde(default)]
    pub id: String,
    #[serde(default = "default_macro_name")]
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(rename = "triggerKey", alias = "trigger_key", default)]
    pub trigger_key: String,
    #[serde(
        rename = "triggerMode",
        alias = "trigger_mode",
        default = "default_trigger_mode"
    )]
    pub trigger_mode: TriggerMode,
    #[serde(
        rename = "repeatCount",
        alias = "repeat_count",
        default = "default_repeat_count"
    )]
    pub repeat_count: u32,
    #[serde(
        rename = "doubleTapTimeoutMs",
        alias = "double_tap_timeout_ms",
        default = "default_double_tap_timeout"
    )]
    pub double_tap_timeout_ms: u64,
    #[serde(
        rename = "holdDurationMs",
        alias = "hold_duration_ms",
        default = "default_hold_duration"
    )]
    pub hold_duration_ms: u64,
    #[serde(rename = "releaseActions", alias = "release_actions", default)]
    pub release_actions: Vec<MacroAction>,
    #[serde(default)]
    pub actions: Vec<MacroAction>,
}

/// One profile. Mirrors the Linux `ProfileConfig`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileConfig {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "panicKey", alias = "panic_key", default)]
    pub panic_key: String,
    #[serde(rename = "virtualDeviceName", alias = "virtual_device_name", default)]
    pub virtual_device_name: String,
    #[serde(default)]
    pub macros: Vec<MacroConfig>,
}

/// Multi-profile wrapper written by the GUI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultiProfileWrapper {
    #[serde(rename = "schemaVersion", alias = "schema_version", default)]
    pub schema_version: Option<u32>,
    #[serde(rename = "activeProfileId", alias = "active_profile_id", default)]
    pub active_profile_id: Option<String>,
    #[serde(default)]
    pub profiles: Vec<ProfileConfig>,
}

impl MultiProfileWrapper {
    /// The active profile, or the first one when `activeProfileId` does not
    /// match. Returns `None` for an empty wrapper.
    pub fn selected(&self) -> Option<&ProfileConfig> {
        if self.profiles.is_empty() {
            return None;
        }
        let active = self.active_profile_id.as_deref().unwrap_or_default();
        Some(
            self.profiles
                .iter()
                .find(|p| p.id == active)
                .unwrap_or(&self.profiles[0]),
        )
    }
}

impl ProfileConfig {
    /// Panic key with the Linux fallback (`KEY_PAUSE`) applied when the field
    /// is missing or unparseable.
    pub fn resolved_panic_key(&self) -> crate::keymap::Key {
        crate::keymap::parse(&self.panic_key).unwrap_or(crate::keymap::Key::KEY_PAUSE)
    }
}

/// Parse a profile document.
///
/// Order mirrors the Linux engine: a non-empty multi-profile wrapper wins,
/// otherwise the text is parsed as a single profile. Unknown fields are
/// tolerated (`schemaVersion` is available but not validated by the engine).
pub fn parse_profile(contents: &str) -> Option<ProfileConfig> {
    if let Ok(wrapper) = serde_json::from_str::<MultiProfileWrapper>(contents) {
        if let Some(selected) = wrapper.selected() {
            let mut profile = selected.clone();
            sanitize_limits(&mut profile);
            return Some(profile);
        }
    }
    let mut profile = serde_json::from_str::<ProfileConfig>(contents).ok()?;
    sanitize_limits(&mut profile);
    Some(profile)
}

/// UTF-8 sınırında güvenli kırpma (geçersiz bayt üretmez).
fn truncate_chars(text: &mut String, max_chars: usize) {
    if let Some((index, _)) = text.char_indices().nth(max_chars) {
        text.truncate(index);
    }
}

/// Bayt sınırına göre UTF-8 güvenli kırpma.
fn truncate_bytes(text: &mut String, max_bytes: usize) {
    if text.len() <= max_bytes {
        return;
    }
    let mut cut = max_bytes;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    text.truncate(cut);
}

/// Apply the documented profile limits to untrusted content.
///
/// The file-size cap alone is not enough: a single 32 MiB `text` or an
/// unbounded `delayMs` is valid JSON. The contract (`docs/PROFILE-CONTRACT.md`)
/// states these limits, so they are enforced here, once, at parse time.
pub fn sanitize_limits(profile: &mut ProfileConfig) {
    truncate_chars(&mut profile.id, MAX_ID_CHARS);
    truncate_chars(&mut profile.name, MAX_NAME_CHARS);
    truncate_chars(&mut profile.panic_key, MAX_TRIGGER_CHARS);
    truncate_chars(&mut profile.virtual_device_name, MAX_NAME_CHARS);
    profile.macros.truncate(MAX_MACROS);
    for macro_config in &mut profile.macros {
        truncate_chars(&mut macro_config.id, MAX_ID_CHARS);
        truncate_chars(&mut macro_config.name, MAX_NAME_CHARS);
        truncate_chars(&mut macro_config.trigger_key, MAX_TRIGGER_CHARS);
        macro_config.repeat_count = macro_config.repeat_count.clamp(1, MAX_REPEAT_COUNT);
        macro_config.double_tap_timeout_ms = macro_config
            .double_tap_timeout_ms
            .clamp(50, MAX_DOUBLE_TAP_MS);
        macro_config.hold_duration_ms = macro_config.hold_duration_ms.clamp(50, MAX_HOLD_MS);
        macro_config.actions.truncate(MAX_ACTIONS_PER_MACRO);
        macro_config.release_actions.truncate(MAX_ACTIONS_PER_MACRO);
        for action in macro_config
            .actions
            .iter_mut()
            .chain(macro_config.release_actions.iter_mut())
        {
            sanitize_action(action);
        }
    }
}

fn sanitize_action(action: &mut MacroAction) {
    if let Some(text) = action.text.as_mut() {
        truncate_bytes(text, MAX_TEXT_BYTES);
    }
    action.delay_ms = action.delay_ms.map(|value| value.min(MAX_DELAY_MS));
    action.jitter_ms = action.jitter_ms.map(|value| value.min(MAX_JITTER_MS));
    action.dx = action
        .dx
        .map(|value| value.clamp(-MAX_MOUSE_DELTA, MAX_MOUSE_DELTA));
    action.dy = action
        .dy
        .map(|value| value.clamp(-MAX_MOUSE_DELTA, MAX_MOUSE_DELTA));
    action.scroll_delta = action
        .scroll_delta
        .map(|value| value.clamp(-MAX_SCROLL_DETENTS, MAX_SCROLL_DETENTS));
    if let Some(points) = action.points.as_mut() {
        points.truncate(MAX_MOUSE_PATH_POINTS);
        for point in points.iter_mut() {
            point[0] = point[0].clamp(0, MAX_MOUSE_PATH_STEP_MS);
            point[1] = point[1].clamp(-(MAX_MOUSE_DELTA as i64), MAX_MOUSE_DELTA as i64);
            point[2] = point[2].clamp(-(MAX_MOUSE_DELTA as i64), MAX_MOUSE_DELTA as i64);
        }
    }
    if let Some(start) = action.start.as_mut() {
        start[0] = start[0].clamp(-MAX_MOUSE_PATH_START, MAX_MOUSE_PATH_START);
        start[1] = start[1].clamp(-MAX_MOUSE_PATH_START, MAX_MOUSE_PATH_START);
    }
}

/// Parse the multi-profile wrapper without the single-profile fallback.
pub fn parse_wrapper(contents: &str) -> Result<MultiProfileWrapper, serde_json::Error> {
    serde_json::from_str(contents)
}

/// Built-in fallback profile, byte-identical to the Linux asset of the same
/// name at contract freeze time (`linux/assets/default_profile.json`).
pub const DEFAULT_PROFILE_JSON: &str = include_str!("../assets/default_profile.json");

/// Built-in fallback profile. Both demo macros are disabled in the asset.
pub fn default_profile() -> ProfileConfig {
    parse_profile(DEFAULT_PROFILE_JSON).unwrap_or_else(|| ProfileConfig {
        id: "profile-default".to_string(),
        name: "MacroStat Varsayılan Profil".to_string(),
        panic_key: "KEY_PAUSE".to_string(),
        virtual_device_name: "MacroStat Virtual Engine".to_string(),
        macros: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRAPPER: &str = r#"{
        "schemaVersion": 1,
        "activeProfileId": "p2",
        "profiles": [
            {"id": "p1", "name": "First", "panicKey": "KEY_PAUSE", "macros": []},
            {"id": "p2", "name": "Second", "panicKey": "KEY_F12", "macros": []}
        ]
    }"#;

    #[test]
    fn selects_active_profile() {
        let wrapper = parse_wrapper(WRAPPER).expect("wrapper");
        assert_eq!(wrapper.schema_version, Some(1));
        assert_eq!(wrapper.selected().map(|p| p.name.as_str()), Some("Second"));
    }

    #[test]
    fn falls_back_to_first_profile_when_active_id_is_unknown() {
        let text = WRAPPER.replace(
            "\"activeProfileId\": \"p2\"",
            "\"activeProfileId\": \"missing\"",
        );
        let wrapper = parse_wrapper(&text).expect("wrapper");
        assert_eq!(wrapper.selected().map(|p| p.name.as_str()), Some("First"));
    }

    #[test]
    fn parse_profile_prefers_wrapper() {
        let parsed = parse_profile(WRAPPER).expect("profile");
        assert_eq!(parsed.id, "p2");
        assert_eq!(parsed.panic_key, "KEY_F12");
    }

    #[test]
    fn parse_profile_accepts_legacy_single_profile() {
        let parsed = parse_profile(
            r#"{"id": "legacy", "name": "Legacy", "panicKey": "KEY_ESC",
                "virtualDeviceName": "MacroStat Virtual Engine",
                "macros": [{"name": "M", "triggerKey": "KEY_A"}]}"#,
        )
        .expect("legacy profile");
        assert_eq!(parsed.id, "legacy");
        assert_eq!(parsed.macros.len(), 1);
        assert!(parsed.macros[0].enabled);
        assert_eq!(parsed.macros[0].trigger_mode, TriggerMode::ONCE);
        assert_eq!(parsed.macros[0].repeat_count, 1);
        assert_eq!(parsed.macros[0].double_tap_timeout_ms, 300);
        assert_eq!(parsed.macros[0].hold_duration_ms, 300);
    }

    #[test]
    fn snake_case_aliases_and_unknown_fields_are_accepted() {
        let parsed = parse_profile(
            r#"{"id": "p", "schemaVersion": 7, "active_profile_id": "p",
                "futureField": {"nested": true},
                "profiles": [{"id": "p", "name": "P",
                    "panic_key": "KEY_PAUSE",
                    "virtual_device_name": "V",
                    "macros": [{
                        "id": "m", "name": "M", "enabled": true,
                        "trigger_key": "KEY_LEFTCTRL+KEY_A",
                        "trigger_mode": "toggle_loop",
                        "repeat_count": 3,
                        "double_tap_timeout_ms": 400,
                        "hold_duration_ms": 500,
                        "release_actions": [{"type": "KEY_UP", "key": "KEY_A"}],
                        "actions": [{
                            "type": "MOUSE_CLICK", "mouse_button": "BTN_LEFT",
                            "delay_ms": 15, "scroll_delta": -120
                        }]
                    }]
                }]}"#,
        )
        .expect("profile");
        let macro_config = &parsed.macros[0];
        assert_eq!(macro_config.trigger_mode, TriggerMode::TOGGLE_LOOP);
        assert_eq!(macro_config.repeat_count, 3);
        assert_eq!(macro_config.double_tap_timeout_ms, 400);
        assert_eq!(macro_config.hold_duration_ms, 500);
        assert_eq!(macro_config.release_actions[0].r#type, ActionType::KEY_UP);
        let action = &macro_config.actions[0];
        assert_eq!(action.r#type, ActionType::MOUSE_CLICK);
        assert_eq!(action.mouse_button.as_deref(), Some("BTN_LEFT"));
        assert_eq!(action.delay_ms, Some(15));
        assert_eq!(action.scroll_delta, Some(-120));
    }

    #[test]
    fn all_action_and_trigger_spellings_parse() {
        for (wire, expected) in [
            ("KEY_DOWN", ActionType::KEY_DOWN),
            ("key_down", ActionType::KEY_DOWN),
            ("KeyDown", ActionType::KEY_DOWN),
            ("mousePath", ActionType::MOUSE_PATH),
            ("TEXT", ActionType::TEXT),
            ("delay", ActionType::DELAY),
        ] {
            let action: MacroAction =
                serde_json::from_str(&format!(r#"{{"type": "{}"}}"#, wire)).expect(wire);
            assert_eq!(action.r#type, expected);
        }
        for (wire, expected) in [
            ("ONCE", TriggerMode::ONCE),
            ("once", TriggerMode::ONCE),
            ("whileHeld", TriggerMode::WHILE_HELD),
            ("hold", TriggerMode::HOLD_LONG_PRESS),
            ("pressAndRelease", TriggerMode::PRESS_AND_RELEASE),
        ] {
            let parsed: TriggerMode = serde_json::from_str(&format!(r#""{}""#, wire)).expect(wire);
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn bundled_default_profile_is_valid_and_demos_are_disabled() {
        let profile = default_profile();
        assert_eq!(profile.id, "profile-default");
        assert_eq!(profile.resolved_panic_key(), crate::keymap::Key::KEY_PAUSE);
        assert_eq!(profile.macros.len(), 2);
        assert!(profile.macros.iter().all(|m| !m.enabled));
    }

    #[test]
    fn bundled_asset_matches_linux_asset_contract_fields() {
        let wrapper = parse_wrapper(DEFAULT_PROFILE_JSON).expect("asset is a wrapper");
        assert_eq!(wrapper.schema_version, Some(1));
        let profile = wrapper.selected().expect("selected profile");
        assert_eq!(profile.virtual_device_name, "MacroStat Virtual Engine");
        assert_eq!(profile.macros[0].trigger_mode, TriggerMode::TOGGLE_LOOP);
        assert_eq!(profile.macros[1].trigger_key, "KEY_LEFTSHIFT+KEY_LEFTCTRL");
    }

    #[test]
    fn panic_key_falls_back_to_pause() {
        let profile: ProfileConfig =
            serde_json::from_str(r#"{"panicKey": "NOT_A_KEY"}"#).expect("profile");
        assert_eq!(profile.resolved_panic_key(), crate::keymap::Key::KEY_PAUSE);
    }

    #[test]
    fn invalid_documents_are_rejected() {
        assert!(parse_profile("not json").is_none());
        assert!(parse_profile(r#"{"macros": [{"actions": [{"type": "NOPE"}]}]}"#).is_none());
    }

    #[test]
    fn sanitizer_clamps_timings_and_preserves_scroll_magnitude() {
        let mut profile: ProfileConfig = serde_json::from_str(
            r#"{"name": "p", "macros": [{
                "triggerKey": "KEY_A",
                "repeatCount": 100000,
                "doubleTapTimeoutMs": 999999,
                "holdDurationMs": 999999,
                "actions": [
                    {"type": "DELAY", "delayMs": 99999999, "jitterMs": 99999},
                    {"type": "MOUSE_SCROLL", "scrollDelta": -360},
                    {"type": "MOUSE_MOVE", "dx": 99999999, "dy": -99999999}
                ]
            }]}"#,
        )
        .expect("profile");
        sanitize_limits(&mut profile);
        let macro_config = &profile.macros[0];
        assert_eq!(macro_config.repeat_count, MAX_REPEAT_COUNT);
        assert_eq!(macro_config.double_tap_timeout_ms, MAX_DOUBLE_TAP_MS);
        assert_eq!(macro_config.hold_duration_ms, MAX_HOLD_MS);
        assert_eq!(macro_config.actions[0].delay_ms, Some(MAX_DELAY_MS));
        assert_eq!(macro_config.actions[0].jitter_ms, Some(MAX_JITTER_MS));
        assert_eq!(
            macro_config.actions[1].scroll_delta,
            Some(-MAX_SCROLL_DETENTS),
            "sign and multi-detent magnitude must survive the clamp"
        );
        assert_eq!(macro_config.actions[2].dx, Some(MAX_MOUSE_DELTA));
        assert_eq!(macro_config.actions[2].dy, Some(-MAX_MOUSE_DELTA));
    }

    #[test]
    fn sanitizer_truncates_oversized_collections_and_names() {
        let mut profile: ProfileConfig = serde_json::from_str(
            r#"{"name": "p", "macros": [{"triggerKey": "KEY_A", "actions": [{"type": "DELAY"}]}]}"#,
        )
        .expect("template");
        let template_macro = profile.macros[0].clone();
        let template_action = template_macro.actions[0].clone();
        profile.name = "x".repeat(MAX_NAME_CHARS + 50);
        profile.macros = vec![template_macro; MAX_MACROS + 5];
        profile.macros[0].trigger_key = "y".repeat(MAX_TRIGGER_CHARS + 10);
        profile.macros[0].actions = vec![template_action; MAX_ACTIONS_PER_MACRO + 3];
        sanitize_limits(&mut profile);
        assert_eq!(profile.name.chars().count(), MAX_NAME_CHARS);
        assert_eq!(profile.macros.len(), MAX_MACROS);
        assert_eq!(
            profile.macros[0].trigger_key.chars().count(),
            MAX_TRIGGER_CHARS
        );
        assert_eq!(profile.macros[0].actions.len(), MAX_ACTIONS_PER_MACRO);
    }
}
