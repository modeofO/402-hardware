//! Lamp timer: tap to set minutes, START drives the relay, lamp goes off
//! when the countdown hits zero. Single-threaded loop: poll touch, tick
//! the timer, redraw when the visible state changes (a full-frame flush
//! costs ~100ms at 26MHz, so it is throttled to once per second).

mod display;
mod pins;
mod relay;
mod timer;
mod touch;
mod ui;

use anyhow::Result;
use display::Display;
use esp_idf_svc::hal::gpio::{OutputPin, PinDriver};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::{config as spi_config, SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use log::{error, info};
use std::time::{Duration, Instant};
use timer::{Phase, Timer};
use touch::Touch;
use ui::Button;

/// Countdown offered at boot before any button is pressed.
const DEFAULT_SECS: u64 = 15 * 60;

/// Block until the current touch is released, so one tap acts once.
fn wait_release(touch: &mut Touch) {
    while touch.poll().is_some() {
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("lamp timer starting");

    let p = Peripherals::take()?;

    // Display SPI on the S3's FSPI (SPI2) IOMUX pins — see pins.rs.
    // No MISO: routing GPIO13 into the SPI driver made the panel ignore
    // all traffic (see display.rs) — the display is driven write-only.
    let spi_driver = SpiDriver::new(
        p.spi2,
        p.pins.gpio12, // SCK
        p.pins.gpio11, // MOSI
        None::<esp_idf_svc::hal::gpio::AnyIOPin>,
        &SpiDriverConfig::new(),
    )?;
    let spi = SpiDeviceDriver::new(
        spi_driver,
        None::<esp_idf_svc::hal::gpio::AnyOutputPin>, // CS is manual, see display.rs
        &spi_config::Config::new().baudrate(26.MHz().into()),
    )?;
    let cs = PinDriver::output(p.pins.gpio10.downgrade_output())?;
    let dc = PinDriver::output(p.pins.gpio9.downgrade_output())?;
    let rst = PinDriver::output(p.pins.gpio14.downgrade_output())?;

    let mut display = Display::new(spi, cs, dc, rst)?;
    display.show_message("Booting...");

    let mut touch = Touch::new(
        p.adc1,
        p.pins.gpio4,
        p.pins.gpio5,
        p.pins.gpio6,
        p.pins.gpio7,
    )?;
    let mut relay = relay::Relay::new(p.pins.gpio21.downgrade_output())?;

    let mut timer = Timer::new(DEFAULT_SECS);
    let mut shown: Option<(u64, Phase)> = None;

    loop {
        let now = Instant::now();

        if timer.tick(now) {
            info!("Countdown finished");
            if let Err(e) = relay.off() {
                error!("relay off failed: {e:#}");
            }
        }

        if let Some(point) = touch.poll() {
            if let Some(button) = Button::at(point, display::WIDTH as i32) {
                info!("Button: {button:?}");
                match button {
                    Button::Add1 => timer.add(60, now),
                    Button::Add5 => timer.add(5 * 60, now),
                    Button::Add15 => timer.add(15 * 60, now),
                    Button::Clear => timer.clear(),
                    Button::StartStop => timer.toggle(now),
                }
            }
            wait_release(&mut touch);
        }

        // Relay state always follows the timer, whatever changed it above.
        if relay.is_on() != timer.is_running() {
            let res = if timer.is_running() {
                relay.on()
            } else {
                relay.off()
            };
            if let Err(e) = res {
                error!("relay sync failed: {e:#}");
            }
        }

        let state = (timer.remaining(now), timer.phase());
        if shown != Some(state) {
            display.show_timer(state.0, state.1);
            shown = Some(state);
        }

        std::thread::sleep(Duration::from_millis(30));
    }
}
