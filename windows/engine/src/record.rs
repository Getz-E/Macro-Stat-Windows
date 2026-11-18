// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! `--record-mouse`: aggregate pointer movement into the same
//! `MOUSE_PATH` recording schema the Linux engine writes.
//!
//! Schema (unchanged from Linux, so recordings are interchangeable):
//!
//! ```json
//! {"version":1,"sample_ms":4,"duration_ms":1234,
//!  "points":[[dt,dx,dy], ...],"buttons":[[t,"BTN_LEFT",1], ...],
//!  "keys":[[t,"KEY_A",1], ...],"start":[x,y]}
//! ```
//!
//! `keys` is written only by `--record-input` (the Phase 3 system-wide
//! keyboard+mouse recorder); `--record-mouse` emits the Linux-compatible
//! document without it.
//!
//! The hook reports absolute cursor positions; the recorder converts them to
//! deltas and coalesces them in 4 ms slices, exactly like Linux's `REL_X/REL_Y`
//! aggregation. Limits are shared with the profile (`200k` points) plus a
//! 600 s wall-clock cap.
//!
//! The recorder itself is pure (caller supplies `Instant`s) and testable on
//! Linux; only [`record_to_file`] touches Windows.

use std::io::Write;
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::keymap::Key;
use crate::profile::MAX_MOUSE_PATH_POINTS;

/// Coalescing slice; mirrors Linux `MOUSE_RECORD_SAMPLE_MS`.
pub const RECORD_SAMPLE_MS: u64 = 4;
/// Hard wall-clock limit; mirrors Linux `MOUSE_RECORD_MAX_SECS`.
pub const RECORD_MAX_SECS: u64 = 600;
/// Button/key transition cap. The 600 s cap alone lets an injected input
/// flood grow the document without bound; 50 000 transitions is far beyond
/// any human recording session.
pub const MAX_RECORD_EVENTS: usize = 50_000;

/// On-disk recording document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MousePathRecording {
    pub version: u32,
    pub sample_ms: u64,
    pub duration_ms: u64,
    pub points: Vec<[i64; 3]>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub buttons: Vec<(i64, String, i32)>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub keys: Vec<(i64, String, i32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<[i32; 2]>,
}

/// Accumulates hook events into a [`MousePathRecording`].
#[derive(Debug)]
pub struct MousePathRecorder {
    started_at: Instant,
    last_emit: Instant,
    last_pos: Option<(i32, i32)>,
    acc_dx: i64,
    acc_dy: i64,
    points: Vec<[i64; 3]>,
    buttons: Vec<(i64, String, i32)>,
    keys: Vec<(i64, String, i32)>,
    start: Option<[i32; 2]>,
    stopped_at: Option<Instant>,
}

impl MousePathRecorder {
    pub fn new(start: Option<[i32; 2]>, now: Instant) -> Self {
        Self {
            started_at: now,
            last_emit: now,
            last_pos: start.map(|[x, y]| (x, y)),
            acc_dx: 0,
            acc_dy: 0,
            points: Vec::new(),
            buttons: Vec::new(),
            keys: Vec::new(),
            start,
            stopped_at: None,
        }
    }

    /// Feed an absolute cursor position from the hook.
    pub fn on_move(&mut self, x: i32, y: i32, now: Instant) {
        if self.stopped_at.is_some() {
            return;
        }
        match self.last_pos {
            Some((last_x, last_y)) => {
                self.acc_dx += i64::from(x) - i64::from(last_x);
                self.acc_dy += i64::from(y) - i64::from(last_y);
            }
            None => {
                // First event defines the origin when no start was available.
                self.start = Some([x, y]);
            }
        }
        self.last_pos = Some((x, y));
        self.flush(now, false);
    }

    /// Feed a mouse button transition (`BTN_*` keys only).
    pub fn on_button(&mut self, key: Key, down: bool, now: Instant) {
        if !key.is_mouse() || self.stopped_at.is_some() {
            return;
        }
        let at = self.elapsed_ms(now);
        self.buttons
            .push((at, key.canonical().to_string(), if down { 1 } else { 0 }));
    }

    /// Feed a keyboard transition (`KEY_*` keys only).
    ///
    /// Only `--record-input` calls this; `--record-mouse` leaves `keys` empty
    /// and the field is omitted from the document, matching Linux output.
    pub fn on_key(&mut self, key: Key, down: bool, now: Instant) {
        if key.is_mouse() || self.stopped_at.is_some() {
            return;
        }
        let at = self.elapsed_ms(now);
        self.keys
            .push((at, key.canonical().to_string(), if down { 1 } else { 0 }));
    }

    /// Number of coalesced points so far.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// True once the profile's point limit is reached.
    pub fn at_point_limit(&self) -> bool {
        self.points.len() >= MAX_MOUSE_PATH_POINTS
    }

    /// True once the 600 s cap is reached.
    pub fn expired(&self, now: Instant) -> bool {
        self.elapsed_ms(now) >= (RECORD_MAX_SECS * 1000) as i64
    }

    /// True once the button/key transition cap is reached.
    pub fn at_event_limit(&self) -> bool {
        self.buttons.len() + self.keys.len() >= MAX_RECORD_EVENTS
    }

    pub fn duration_ms(&self, now: Instant) -> u64 {
        self.elapsed_ms(now).max(0) as u64
    }

    /// Flush pending movement and finish the document.
    pub fn finish(mut self, now: Instant) -> MousePathRecording {
        self.flush(now, true);
        self.stopped_at = Some(now);
        MousePathRecording {
            version: 1,
            sample_ms: RECORD_SAMPLE_MS,
            duration_ms: self.duration_ms(now),
            points: self.points,
            buttons: self.buttons,
            keys: self.keys,
            start: self.start,
        }
    }

    fn elapsed_ms(&self, now: Instant) -> i64 {
        let micros = now.saturating_duration_since(self.started_at).as_micros();
        let millis = micros / 1000;
        i64::try_from(millis).unwrap_or(i64::MAX)
    }

    fn flush(&mut self, now: Instant, force: bool) {
        let elapsed = now.saturating_duration_since(self.last_emit);
        let due = elapsed.as_millis() >= u128::from(RECORD_SAMPLE_MS);
        if (due || force) && (self.acc_dx != 0 || self.acc_dy != 0) {
            let dt = i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX);
            self.points.push([
                dt,
                self.acc_dx.clamp(i64::from(i32::MIN), i64::from(i32::MAX)),
                self.acc_dy.clamp(i64::from(i32::MIN), i64::from(i32::MAX)),
            ]);
            self.acc_dx = 0;
            self.acc_dy = 0;
            self.last_emit = now;
        }
        if force {
            // A forced flush also closes the sampling window even when there
            // was nothing to emit.
            self.last_emit = now;
        }
    }
}

/// Symlink/junction ise yazmayı reddeder (hedef başka bir dosya olabilir).
///
/// Windows'ta junction'lar `is_symlink()` ile görünmez; ortak yardımcı
/// `FILE_ATTRIBUTE_REPARSE_POINT` bayrağını da denetler.
fn reject_link(path: &Path) -> std::io::Result<()> {
    crate::winutil::reject_reparse(path)
}

/// Atomic write: unique temp file then rename, so a crash never leaves a
/// truncated recording and a pre-planted `<name>.tmp` is never followed.
///
/// The recording contains typed key names, so it must not be redirected.
/// The final file inherits the user-only `%LOCALAPPDATA%` ACL; an explicit
/// DACL is applied by the GUI-side recorder flow.
pub fn write_recording(path: &Path, recording: &MousePathRecording) -> std::io::Result<()> {
    let data = serde_json::to_vec(recording)?;
    reject_link(path)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut temp = None;
    let mut file = None;
    for attempt in 0..8u32 {
        let candidate = path.with_extension(format!("tmp.{}.{}", std::process::id(), attempt));
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&candidate)
        {
            Ok(handle) => {
                temp = Some(candidate);
                file = Some(handle);
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let (tmp, mut file) = match (temp, file) {
        (Some(tmp), Some(file)) => (tmp, file),
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "geçici kayıt adı üretilemedi",
            ))
        }
    };
    let write_result = file.write_all(&data).and_then(|()| file.sync_all());
    drop(file);
    let result = write_result
        .and_then(|()| reject_link(path))
        .and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Summary printed by `--record-mouse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordSummary {
    pub points: usize,
    pub buttons: usize,
    pub keys: usize,
    pub duration_ms: u64,
}

/// Outcome of the Windows recording run.
#[derive(Debug)]
pub enum RecordError {
    Hook(crate::hook::HookError),
    Io(String),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordError::Hook(error) => write!(f, "{error}"),
            RecordError::Io(error) => f.write_str(error),
        }
    }
}

impl std::error::Error for RecordError {}

/// Record pointer movement until the panic key is pressed, a limit is hit, or
/// the channel closes, then write the document.
#[cfg(windows)]
pub fn record_to_file(
    output: &Path,
    panic_key: Key,
    start_override: Option<[i32; 2]>,
    record_keys: bool,
) -> Result<RecordSummary, RecordError> {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    use crate::hook::{self, InputEvent};
    use crate::keymap::keys_are_equivalent;

    let start = start_override.or_else(|| crate::mouse::cursor_position().map(|(x, y)| [x, y]));
    let (handle, receiver) = hook::start(4096).map_err(RecordError::Hook)?;
    let mut recorder = MousePathRecorder::new(start, Instant::now());

    loop {
        let now = Instant::now();
        if recorder.expired(now) {
            eprintln!(
                "[KAYIT] Süre sınırına ulaşıldı ({} sn); kayıt bitiriliyor.",
                RECORD_MAX_SECS
            );
            break;
        }
        match receiver.recv_timeout(Duration::from_millis(20)) {
            // Panik tuşu profil tarafında da fare düğmesi olabilir ve
            // Left/Right varyantları eşdeğer sayılır (motorla aynı kural).
            Ok(InputEvent::Key {
                key, down: true, ..
            })
            | Ok(InputEvent::MouseButton {
                key, down: true, ..
            }) if keys_are_equivalent(key, panic_key) => {
                println!("[KAYIT] STOP tuşuna basıldı; kayıt bitiriliyor.");
                break;
            }
            Ok(InputEvent::MouseMove { x, y, .. }) => recorder.on_move(x, y, now),
            Ok(InputEvent::MouseButton { key, down, .. }) => recorder.on_button(key, down, now),
            Ok(InputEvent::Key { key, down, .. }) => {
                if record_keys {
                    recorder.on_key(key, down, now);
                }
            }
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if recorder.at_point_limit() {
            eprintln!(
                "[KAYIT] Nokta sınırına ulaşıldı ({}); kayıt bitiriliyor.",
                MAX_MOUSE_PATH_POINTS
            );
            break;
        }
        if recorder.at_event_limit() {
            eprintln!(
                "[KAYIT] Olay sınırına ulaşıldı ({}); kayıt bitiriliyor.",
                MAX_RECORD_EVENTS
            );
            break;
        }
    }

    drop(handle);
    let recording = recorder.finish(Instant::now());
    write_recording(output, &recording)
        .map_err(|error| RecordError::Io(format!("{}: {error}", output.display())))?;
    Ok(RecordSummary {
        points: recording.points.len(),
        buttons: recording.buttons.len(),
        keys: recording.keys.len(),
        duration_ms: recording.duration_ms,
    })
}

#[cfg(not(windows))]
pub fn record_to_file(
    _output: &Path,
    _panic_key: Key,
    _start_override: Option<[i32; 2]>,
    _record_keys: bool,
) -> Result<RecordSummary, RecordError> {
    Err(RecordError::Io(
        "input recording requires Windows".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn movement_is_coalesced_and_deltas_preserved() {
        let t0 = Instant::now();
        let mut recorder = MousePathRecorder::new(Some([100, 100]), t0);
        recorder.on_move(103, 104, t0 + Duration::from_millis(5));
        // Below the 4 ms window since the last emit: accumulates.
        recorder.on_move(105, 106, t0 + Duration::from_millis(7));
        let recording = recorder.finish(t0 + Duration::from_millis(9));
        assert_eq!(recording.version, 1);
        assert_eq!(recording.sample_ms, RECORD_SAMPLE_MS);
        assert_eq!(recording.start, Some([100, 100]));
        assert_eq!(
            recording.points,
            vec![[5, 3, 4], [4, 2, 2]],
            "first slice then the finish flush (dt since last emit)"
        );
        assert_eq!(recording.duration_ms, 9);
    }

    #[test]
    fn buttons_use_canonical_linux_names() {
        let t0 = Instant::now();
        let mut recorder = MousePathRecorder::new(None, t0);
        recorder.on_button(Key::BTN_LEFT, true, t0 + Duration::from_millis(2));
        recorder.on_button(Key::BTN_LEFT, false, t0 + Duration::from_millis(30));
        // Keyboard keys are not recorded as buttons.
        recorder.on_button(Key::KEY_A, true, t0 + Duration::from_millis(31));
        let recording = recorder.finish(t0 + Duration::from_millis(40));
        assert_eq!(
            recording.buttons,
            vec![
                (2, "BTN_LEFT".to_string(), 1),
                (30, "BTN_LEFT".to_string(), 0)
            ]
        );
    }

    #[test]
    fn key_events_are_recorded_and_mouse_ignored() {
        let t0 = Instant::now();
        let mut recorder = MousePathRecorder::new(None, t0);
        recorder.on_key(Key::KEY_A, true, t0 + Duration::from_millis(3));
        recorder.on_button(Key::BTN_LEFT, true, t0 + Duration::from_millis(4));
        recorder.on_key(Key::KEY_A, false, t0 + Duration::from_millis(40));
        let recording = recorder.finish(t0 + Duration::from_millis(41));
        assert_eq!(
            recording.keys,
            vec![(3, "KEY_A".to_string(), 1), (40, "KEY_A".to_string(), 0)]
        );
        assert_eq!(recording.buttons.len(), 1, "mouse buttons stay separate");
    }

    #[test]
    fn space_is_not_the_panic_key() {
        use crate::keymap::keys_are_equivalent;
        // Space must reach `on_key`; only the configured panic key ends the
        // recording. Left/right modifier equivalence is unchanged.
        assert!(!keys_are_equivalent(Key::KEY_SPACE, Key::KEY_PAUSE));
        assert!(!keys_are_equivalent(Key::KEY_SPACE, Key::KEY_ESC));
        assert!(keys_are_equivalent(Key::KEY_RIGHTCTRL, Key::KEY_LEFTCTRL));
    }

    #[test]
    fn repeated_space_presses_are_recorded() {
        let t0 = Instant::now();
        let mut recorder = MousePathRecorder::new(None, t0);
        recorder.on_key(Key::KEY_SPACE, true, t0 + Duration::from_millis(5));
        recorder.on_key(Key::KEY_SPACE, false, t0 + Duration::from_millis(60));
        recorder.on_key(Key::KEY_SPACE, true, t0 + Duration::from_millis(80));
        recorder.on_key(Key::KEY_SPACE, false, t0 + Duration::from_millis(120));
        let recording = recorder.finish(t0 + Duration::from_millis(121));
        assert_eq!(
            recording.keys,
            vec![
                (5, "KEY_SPACE".to_string(), 1),
                (60, "KEY_SPACE".to_string(), 0),
                (80, "KEY_SPACE".to_string(), 1),
                (120, "KEY_SPACE".to_string(), 0),
            ]
        );
    }

    #[test]
    fn empty_keys_are_omitted_from_the_document() {
        let t0 = Instant::now();
        let mut recorder = MousePathRecorder::new(Some([1, 2]), t0);
        recorder.on_move(3, 4, t0 + Duration::from_millis(5));
        let recording = recorder.finish(t0 + Duration::from_millis(5));
        let json = serde_json::to_value(&recording).unwrap();
        assert!(!json.as_object().unwrap().contains_key("keys"));
    }

    #[test]
    fn first_move_defines_start_when_unknown() {
        let t0 = Instant::now();
        let mut recorder = MousePathRecorder::new(None, t0);
        recorder.on_move(7, 9, t0 + Duration::from_millis(1));
        recorder.on_move(10, 14, t0 + Duration::from_millis(6));
        let recording = recorder.finish(t0 + Duration::from_millis(6));
        assert_eq!(recording.start, Some([7, 9]));
        assert_eq!(recording.points, vec![[6, 3, 5]]);
    }

    #[test]
    fn zero_deltas_do_not_create_points() {
        let t0 = Instant::now();
        let mut recorder = MousePathRecorder::new(Some([5, 5]), t0);
        recorder.on_move(5, 5, t0 + Duration::from_millis(10));
        let recording = recorder.finish(t0 + Duration::from_millis(20));
        assert!(recording.points.is_empty());
    }

    #[test]
    fn limits_and_expiry_are_reported() {
        let t0 = Instant::now();
        let recorder = MousePathRecorder::new(None, t0);
        assert!(!recorder.expired(t0 + Duration::from_secs(599)));
        assert!(recorder.expired(t0 + Duration::from_secs(600)));
        assert!(!recorder.at_point_limit());
        assert_eq!(recorder.point_count(), 0);
    }

    #[test]
    fn schema_keys_match_linux() {
        let t0 = Instant::now();
        let mut recorder = MousePathRecorder::new(Some([1, 2]), t0);
        recorder.on_move(3, 4, t0 + Duration::from_millis(5));
        let recording = recorder.finish(t0 + Duration::from_millis(5));
        let json = serde_json::to_value(&recording).unwrap();
        let object = json.as_object().unwrap();
        for key in ["version", "sample_ms", "duration_ms", "points", "start"] {
            assert!(object.contains_key(key), "missing {key}");
        }
        assert!(
            !object.contains_key("buttons"),
            "empty buttons must be omitted like Linux"
        );
        assert_eq!(object["points"], serde_json::json!([[5, 2, 2]]));
    }

    #[test]
    fn writing_through_a_symlink_is_refused() {
        #[cfg(unix)]
        {
            let dir = std::env::temp_dir().join(format!("macrostat-record-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let target = dir.join("target.json");
            std::fs::write(&target, b"keep").unwrap();
            let link = dir.join("recording.json");
            std::os::unix::fs::symlink(&target, &link).unwrap();

            let t0 = Instant::now();
            let recording = MousePathRecorder::new(None, t0).finish(t0);
            let error = write_recording(&link, &recording).expect_err("symlink is refused");
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
            assert_eq!(std::fs::read(&target).unwrap(), b"keep");

            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
