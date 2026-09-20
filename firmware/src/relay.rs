//! Lamp switch via a Digital Loggers IoT Power Relay.
//!
//! The IoT Relay's trigger input is opto-isolated and accepts 3–60V DC,
//! so the ESP32's 3.3V GPIO drives it directly: GPIO high energises the
//! "normally OFF" outlets (the lamp), GPIO low releases them. Mains stays
//! inside the sealed relay box; nothing on this board ever sees line
//! voltage. The unit debounces its input, so the signal must be held —
//! this is a level, not a pulse.

use anyhow::Result;
use esp_idf_svc::hal::gpio::{AnyOutputPin, Output, PinDriver};
use log::info;

pub struct Relay {
    pin: PinDriver<'static, AnyOutputPin, Output>,
    on: bool,
}

impl Relay {
    /// Takes the trigger pin and forces the lamp off.
    pub fn new(pin: AnyOutputPin) -> Result<Self> {
        let mut pin = PinDriver::output(pin)?;
        pin.set_low()?;
        info!(
            "Relay: trigger ready on GPIO {}, lamp off",
            crate::pins::RELAY_IN
        );
        Ok(Self { pin, on: false })
    }

    pub fn on(&mut self) -> Result<()> {
        if !self.on {
            self.pin.set_high()?;
            self.on = true;
            info!("Relay: lamp ON");
        }
        Ok(())
    }

    pub fn off(&mut self) -> Result<()> {
        if self.on {
            self.pin.set_low()?;
            self.on = false;
            info!("Relay: lamp OFF");
        }
        Ok(())
    }

    pub fn is_on(&self) -> bool {
        self.on
    }
}
