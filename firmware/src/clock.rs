//! Wall clock with no network and no RTC chip: the user sets the time on
//! the touchscreen and the ESP32's crystal keeps it (roughly a second a
//! day of drift). Only the time of day matters; the date is fictitious.
//!
//! The time lives in the ESP-IDF system clock, which survives crashes and
//! reflashes but not a power cut. `main.rs` checkpoints it to flash every
//! minute and, after a power-on reset, restores the checkpoint — late by
//! however long the power was out, so the restored time is flagged until
//! the user confirms it.
//!
//! That flag rides in the fictitious date so it survives soft resets too:
//! a user-set clock counts from `BASE_SET`, a restored one from
//! `BASE_RESTORED`, and an untouched system clock starts at the epoch.

use esp_idf_svc::sys::{settimeofday, timeval};
use std::time::{SystemTime, UNIX_EPOCH};

const SECS_PER_DAY: u64 = 86_400;
/// 2000-01-01T00:00:00Z
const BASE_RESTORED: u64 = 946_684_800;
/// 2026-01-01T00:00:00Z
const BASE_SET: u64 = 1_767_225_600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// Never set since power-on and nothing to restore.
    Unset,
    /// Restored from the flash checkpoint after a power cut.
    Restored,
    /// Set by the user.
    Set,
}

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn trust() -> Trust {
    match epoch_secs() {
        t if t >= BASE_SET => Trust::Set,
        t if t >= BASE_RESTORED => Trust::Restored,
        _ => Trust::Unset,
    }
}

/// Seconds since midnight, or None while the clock is unset.
pub fn secs_of_day() -> Option<u32> {
    (trust() != Trust::Unset).then(|| (epoch_secs() % SECS_PER_DAY) as u32)
}

/// Set the time of day. `restored` marks a checkpoint restore rather
/// than a time the user entered.
pub fn set(secs_of_day: u32, restored: bool) {
    let base = if restored { BASE_RESTORED } else { BASE_SET };
    let tv = timeval {
        tv_sec: (base + u64::from(secs_of_day) % SECS_PER_DAY) as _,
        tv_usec: 0,
    };
    // Only fails on a bad pointer or an out-of-range time.
    unsafe { settimeofday(&tv, core::ptr::null()) };
}
