//! Settings that outlive a power cut, in the default NVS partition: the
//! schedule, and a once-a-minute checkpoint of the clock (see clock.rs).
//!
//! Wear: NVS appends entries and erases a 4KB page only once it fills
//! (126 entries), so a write a minute is ~850 erases per page per year
//! against a 100k-cycle rating.

use anyhow::Result;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};

use crate::schedule::{Minutes, Schedule, MINUTES_PER_DAY};

const NAMESPACE: &str = "lamp";
const KEY_ON: &str = "sched_on";
const KEY_OFF: &str = "sched_off";
const KEY_CLOCK: &str = "clock_secs";

pub struct Store {
    nvs: EspDefaultNvs,
}

impl Store {
    pub fn new() -> Result<Self> {
        let partition = EspDefaultNvsPartition::take()?;
        Ok(Self {
            nvs: EspDefaultNvs::new(partition, NAMESPACE, true)?,
        })
    }

    fn minutes(&self, key: &str) -> Option<Minutes> {
        let value = self.nvs.get_u16(key).ok()??;
        (value < MINUTES_PER_DAY).then_some(value)
    }

    pub fn schedule(&self) -> Option<Schedule> {
        Some(Schedule {
            on: self.minutes(KEY_ON)?,
            off: self.minutes(KEY_OFF)?,
        })
    }

    pub fn save_schedule(&self, schedule: Schedule) -> Result<()> {
        self.nvs.set_u16(KEY_ON, schedule.on)?;
        self.nvs.set_u16(KEY_OFF, schedule.off)?;
        Ok(())
    }

    /// Last checkpointed time of day, in seconds since midnight.
    pub fn clock(&self) -> Option<u32> {
        let secs = self.nvs.get_u32(KEY_CLOCK).ok()??;
        (secs < 86_400).then_some(secs)
    }

    pub fn save_clock(&self, secs_of_day: u32) -> Result<()> {
        self.nvs.set_u32(KEY_CLOCK, secs_of_day)?;
        Ok(())
    }
}
