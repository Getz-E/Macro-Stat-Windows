// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 MacroStat contributors
//! Rotating engine log (1 MB, one `.1` backup).
//!
//! Design constraints from `REPORT.md`:
//!
//! * the log lives under `%LOCALAPPDATA%\MacroStat\` (a user-only directory
//!   by inheritance), never in a shared location;
//! * one line per event, UTC timestamp, numeric/technical messages only —
//!   typed `TEXT` content and key payloads are never logged;
//! * rotation is size-based and best effort: a failure to rotate must not
//!   stop the engine, so the logger never returns an error to its callers.
//!
//! The file is opened per line. MacroStat writes at most a few lines per
//! trigger, so the extra `CreateFileW` cost is irrelevant next to `SendInput`
//! latencies, and it avoids holding a file handle that would block log
//! inspection and rotation on Windows.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Rotation threshold, shared with the Linux engine's stderr rotation.
pub const MAX_LOG_BYTES: u64 = 1_000_000;

/// File name inside the engine data directory.
pub const LOG_FILE_NAME: &str = "engine.log";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "UYARI",
            Level::Error => "HATA",
        }
    }
}

/// Thread-safe rotating logger.
///
/// Cloning is not supported: callers share it through `Arc<Logger>`. The
/// mutex only serializes appends; the actual work is a single buffered write.
pub struct Logger {
    path: PathBuf,
    max_bytes: u64,
    lock: Mutex<()>,
}

impl Logger {
    /// Create a logger writing to `path`. The parent directory is created when
    /// missing; failure is deferred to [`Logger::log`] so a read-only disk
    /// cannot prevent the engine from starting.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self {
            path,
            max_bytes: MAX_LOG_BYTES,
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one line. Never panics; a write failure is reported once to
    /// stderr and otherwise ignored (the engine must keep running headless).
    pub fn log(&self, level: Level, message: &str) {
        let line = format!(
            "[{}] [{}] {}\n",
            format_timestamp(unix_seconds()),
            level.as_str(),
            sanitize(message)
        );
        let _guard = self
            .lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Err(error) = self.append(&line) {
            eprintln!("[GÜNLÜK HATASI] {}: {error}", self.path.display());
        }
    }

    fn append(&self, line: &str) -> std::io::Result<()> {
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            if metadata.len().saturating_add(line.len() as u64) > self.max_bytes {
                self.rotate()?;
            }
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())
    }

    fn rotate(&self) -> std::io::Result<()> {
        let backup = self.path.with_extension("log.1");
        if backup.exists() {
            std::fs::remove_file(&backup)?;
        }
        std::fs::rename(&self.path, &backup)
    }
}

/// Replace line breaks and control characters so one event is always one
/// physical line and a malicious profile name cannot inject terminal escape
/// sequences into the log viewer.
fn sanitize(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    for ch in message.chars() {
        match ch {
            '\r' | '\n' | '\t' => out.push(' '),
            ch if ch.is_control() => out.push('?'),
            other => out.push(other),
        }
    }
    out
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Format seconds since the Unix epoch as UTC `YYYY-MM-DD HH:MM:SS`.
///
/// Pure and unit-tested; the civil-date conversion is Howard Hinnant's
/// `civil_from_days` algorithm, valid for the whole proleptic Gregorian range.
pub fn format_timestamp(unix_secs: u64) -> String {
    let days = i64::try_from(unix_secs / 86_400).unwrap_or(i64::MAX);
    let seconds_of_day = unix_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    if month <= 2 {
        year += 1;
    }
    (
        year,
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "macrostat-log-{name}-{}-{}",
            std::process::id(),
            unix_seconds()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn timestamps_are_utc_and_padded() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00:00");
        assert_eq!(format_timestamp(86_399), "1970-01-01 23:59:59");
        // 2000-02-29 12:34:56 UTC — leap day.
        assert_eq!(format_timestamp(951_827_696), "2000-02-29 12:34:56");
        // 2026-11-10 00:00:00 UTC.
        assert_eq!(format_timestamp(1_794_268_800), "2026-11-10 00:00:00");
    }

    #[test]
    fn newlines_are_flattened() {
        assert_eq!(sanitize("a\nb\r\nc"), "a b  c");
    }

    #[test]
    fn writes_and_rotates_at_the_limit() {
        let dir = temp_dir("rotate");
        let path = dir.join(LOG_FILE_NAME);
        let logger = Logger {
            path: path.clone(),
            max_bytes: 120,
            lock: Mutex::new(()),
        };
        for _ in 0..20 {
            logger.log(Level::Info, "0123456789");
        }
        let current = std::fs::read_to_string(&path).unwrap();
        let backup = std::fs::read_to_string(dir.join("engine.log.1")).unwrap();
        assert!(!current.is_empty());
        assert!(!backup.is_empty());
        assert!(current.len() as u64 <= logger.max_bytes);
        assert!(current.starts_with('['));
        assert!(current.contains("[INFO]"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn write_failure_does_not_panic() {
        let logger = Logger::new("/proc/definitely-not-writable/engine.log");
        logger.log(Level::Error, "test");
    }
}
