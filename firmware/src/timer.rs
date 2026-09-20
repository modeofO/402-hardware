//! Countdown state machine. Pure logic, no hardware: `main.rs` feeds it
//! button presses and the clock, and drives the relay from `is_running()`.
//!
//! Idle holds the seconds that START would run for (also where STOP parks
//! the remaining time, so STOP/START is pause/resume). Running holds the
//! wall-clock deadline so the countdown does not drift with loop timing.

use std::time::{Duration, Instant};

/// Largest settable countdown: 9:59:59 fits the display's H:MM:SS.
pub const MAX_SECS: u64 = 9 * 3600 + 59 * 60 + 59;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Lamp off, time can be edited. `secs` is what START will run.
    Idle,
    /// Lamp on, counting down.
    Running,
    /// Lamp off, countdown reached zero. Any button returns to Idle.
    Done,
}

#[derive(Debug)]
pub struct Timer {
    phase: Phase,
    /// Idle/Done: seconds to run. Running: unused.
    secs: u64,
    /// Running: when the lamp switches off.
    ends_at: Instant,
    /// Last value STARTed, restored after Done so re-running is one tap.
    last_started: u64,
}

impl Timer {
    pub fn new(initial_secs: u64) -> Self {
        Self {
            phase: Phase::Idle,
            secs: initial_secs.min(MAX_SECS),
            ends_at: Instant::now(),
            last_started: initial_secs.min(MAX_SECS),
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_running(&self) -> bool {
        self.phase == Phase::Running
    }

    /// Seconds left (rounded up while running, so the display never shows
    /// 0:00 with the lamp still on).
    pub fn remaining(&self, now: Instant) -> u64 {
        match self.phase {
            Phase::Idle => self.secs,
            Phase::Done => 0,
            Phase::Running => {
                let left = self.ends_at.saturating_duration_since(now);
                left.as_secs() + u64::from(left.subsec_nanos() > 0)
            }
        }
    }

    /// Advance the clock. Returns true on the tick that expires the
    /// countdown — the caller must switch the relay off.
    pub fn tick(&mut self, now: Instant) -> bool {
        if self.phase == Phase::Running && now >= self.ends_at {
            self.phase = Phase::Done;
            self.secs = self.last_started;
            return true;
        }
        false
    }

    /// Add time. While running this extends the deadline in place.
    pub fn add(&mut self, secs: u64, now: Instant) {
        match self.phase {
            Phase::Running => {
                let left = self.remaining(now);
                let total = (left + secs).min(MAX_SECS);
                self.ends_at = now + Duration::from_secs(total);
            }
            Phase::Idle | Phase::Done => {
                self.phase = Phase::Idle;
                self.secs = (self.secs + secs).min(MAX_SECS);
            }
        }
    }

    /// Zero the countdown. Stops the lamp if it was running.
    pub fn clear(&mut self) {
        self.phase = Phase::Idle;
        self.secs = 0;
    }

    /// START when idle/done (no-op with nothing set), STOP (pause) when
    /// running. The caller drives the relay from `is_running()`.
    pub fn toggle(&mut self, now: Instant) {
        match self.phase {
            Phase::Running => {
                self.secs = self.remaining(now);
                self.phase = Phase::Idle;
            }
            Phase::Idle | Phase::Done => {
                if self.secs > 0 {
                    self.last_started = self.secs;
                    self.ends_at = now + Duration::from_secs(self.secs);
                    self.phase = Phase::Running;
                }
            }
        }
    }
}
