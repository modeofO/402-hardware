//! Daily on/off window plus the manual override. Pure logic, no hardware:
//! `main.rs` feeds it the time of day and drives the relay from `update()`.
//!
//! The override is a smart-plug style "until the next switch": toggling
//! the lamp by hand forces the opposite of what the schedule wants, and
//! is dropped as soon as the schedule agrees with it again.

/// Minutes since midnight, `0..MINUTES_PER_DAY`.
pub type Minutes = u16;

pub const MINUTES_PER_DAY: Minutes = 24 * 60;

/// Move a time of day by `delta` minutes, wrapping around midnight.
pub fn wrapping_add(time: Minutes, delta: i32) -> Minutes {
    (time as i32 + delta).rem_euclid(MINUTES_PER_DAY as i32) as Minutes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    /// Lamp switches on at this time...
    pub on: Minutes,
    /// ...and off at this one. 00:00 doubles as "midnight / 24:00".
    pub off: Minutes,
}

impl Schedule {
    /// Whether the window covers `now`. `on > off` spans midnight
    /// (17:00–00:00, 22:00–06:00); `on == off` is an empty window.
    pub fn is_on(&self, now: Minutes) -> bool {
        if self.on <= self.off {
            (self.on..self.off).contains(&now)
        } else {
            now >= self.on || now < self.off
        }
    }
}

#[derive(Debug)]
pub struct Lamp {
    schedule: Schedule,
    /// Forced lamp state, always the opposite of the schedule's.
    manual: Option<bool>,
}

impl Lamp {
    pub fn new(schedule: Schedule) -> Self {
        Self {
            schedule,
            manual: None,
        }
    }

    pub fn schedule(&self) -> Schedule {
        self.schedule
    }

    pub fn set_schedule(&mut self, schedule: Schedule) {
        self.schedule = schedule;
        self.manual = None;
    }

    pub fn is_manual(&self) -> bool {
        self.manual.is_some()
    }

    /// With no clock there is no schedule to follow: the lamp stays off
    /// unless switched on by hand.
    fn scheduled(&self, now: Option<Minutes>) -> bool {
        now.is_some_and(|t| self.schedule.is_on(t))
    }

    /// Whether the lamp should be on right now. Drops the override once
    /// the schedule has caught up with it.
    pub fn update(&mut self, now: Option<Minutes>) -> bool {
        let scheduled = self.scheduled(now);
        if self.manual == Some(scheduled) {
            self.manual = None;
        }
        self.manual.unwrap_or(scheduled)
    }

    /// Flip the lamp by hand. Flipping back to what the schedule wants
    /// simply ends the override.
    pub fn toggle(&mut self, now: Option<Minutes>) {
        let target = !self.update(now);
        self.manual = (target != self.scheduled(now)).then_some(target);
    }
}
