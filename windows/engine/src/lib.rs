// SPDX-License-Identifier: GPL-3.0-or-later
//! MacroStat Windows engine library.
//!
//! Layer split (see `windows/docs/WINDOWS-ARCHITECTURE.md`):
//!
//! * pure core — [`actions`], [`keymap`], [`profile`], [`trigger`]: no OS
//!   calls, unit-tested on any platform (including Linux CI);
//! * Windows layer — [`hook`], [`inject`], [`ipc`], [`log`], [`mouse`],
//!   [`record`], [`winutil`]: implemented in Phase 2+.

// Wire names are SCREAMING_SNAKE_CASE by contract (KEY_DOWN, LEFT_CTRL, ...);
// Rust's CamelCase convention does not apply to this on-disk vocabulary.
#![allow(non_camel_case_types)]

pub mod actions;
pub mod hook;
pub mod inject;
pub mod ipc;
pub mod keymap;
pub mod log;
pub mod mouse;
pub mod profile;
pub mod record;
#[cfg(windows)]
pub mod runtime;
pub mod selftest;
pub mod trigger;
pub mod winutil;
