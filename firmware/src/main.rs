mod api;
mod display;
mod pins;
mod touch;
mod types;
mod vend;
mod wifi;

use anyhow::Result;
use esp_idf_svc::hal::gpio::{OutputPin, PinDriver};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::{config as spi_config, SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use log::info;
use types::MenuItem;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("x402 vending terminal starting");

    let p = Peripherals::take()?;

    // Display SPI on the S3's FSPI (SPI2) IOMUX pins — see pins.rs
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

    let mut display = display::Display::new(spi, cs, dc, rst)?;
    display.show_message("Booting...");

    // Milestone 1: display bring-up with a hardcoded menu (matches backend).
    // WiFi + live menu fetch land with the API integration.
    let items = vec![
        MenuItem {
            id: "1".into(),
            name: "Soda".into(),
            price_usdc: "1.50".into(),
        },
        MenuItem {
            id: "2".into(),
            name: "Water".into(),
            price_usdc: "1.00".into(),
        },
        MenuItem {
            id: "3".into(),
            name: "Snack".into(),
            price_usdc: "2.00".into(),
        },
    ];
    display.show_menu(&items);
    info!("Menu displayed — idle");

    let mut touch = touch::Touch::new(
        p.adc1,
        p.pins.gpio4,
        p.pins.gpio5,
        p.pins.gpio6,
        p.pins.gpio7,
    )?;

    // Milestone 1 loop: log touches and flash the tapped item.
    // The full state machine (session/QR/poll/vend) lands with the API work.
    loop {
        if let Some(point) = touch.poll() {
            info!("Touch at ({}, {})", point.x, point.y);
            if let Some(i) = touch::Touch::item_at(point, items.len()) {
                info!("Selected item: {}", items[i].name);
                display.show_message(&format!("Selected: {}", items[i].name));
                std::thread::sleep(std::time::Duration::from_millis(1200));
                display.show_menu(&items);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
}
