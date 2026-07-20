//! Diagnostic: slow-toggle every display control line so each path can be
//! verified end-to-end with a multimeter at the display header solder
//! joints. All pins toggle together: ~1s high, ~1s low, forever.
//!
//! Expected at the display pins: CLK, MOSI, CS, D/C, RST all alternating
//! ~0V / ~3.3V every second. Any pin that sits still is the broken path.

use esp_idf_svc::hal::gpio::{OutputPin, PinDriver};
use esp_idf_svc::hal::prelude::*;
use log::info;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let p = Peripherals::take()?;
    let mut pins = [
        (
            "CLK/12",
            PinDriver::output(p.pins.gpio12.downgrade_output())?,
        ),
        (
            "MOSI/11",
            PinDriver::output(p.pins.gpio11.downgrade_output())?,
        ),
        (
            "CS/10",
            PinDriver::output(p.pins.gpio10.downgrade_output())?,
        ),
        ("DC/9", PinDriver::output(p.pins.gpio9.downgrade_output())?),
        (
            "RST/14",
            PinDriver::output(p.pins.gpio14.downgrade_output())?,
        ),
    ];

    info!("pin_blink: toggling CLK=12 MOSI=11 CS=10 DC=9 RST=14 at ~0.5Hz");
    let mut level = false;
    loop {
        level = !level;
        for (_, pin) in pins.iter_mut() {
            if level {
                pin.set_high()?;
            } else {
                pin.set_low()?;
            }
        }
        info!("pin_blink: all pins {}", if level { "HIGH" } else { "LOW" });
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
