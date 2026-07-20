//! Relay control (Adafruit 2895, non-latching, active-high signal).

use anyhow::Result;
use esp_idf_svc::hal::gpio::{AnyOutputPin, Output, PinDriver};
use log::info;

pub struct Vend {
    relay: PinDriver<'static, AnyOutputPin, Output>,
}

impl Vend {
    pub fn new(pin: AnyOutputPin) -> Result<Self> {
        let mut relay = PinDriver::output(pin)?;
        relay.set_low()?;
        info!("Vend: relay ready on GPIO {}", crate::pins::RELAY_IN);
        Ok(Self { relay })
    }

    /// Close the relay for `pulse_ms`, then release.
    pub fn dispense(&mut self, pulse_ms: u64) -> Result<()> {
        info!("Vend: firing relay for {pulse_ms}ms");
        self.relay.set_high()?;
        std::thread::sleep(std::time::Duration::from_millis(pulse_ms));
        self.relay.set_low()?;
        info!("Vend: relay off");
        Ok(())
    }
}
