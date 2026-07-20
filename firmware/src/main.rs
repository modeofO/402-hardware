mod api;
mod display;
mod pins;
mod touch;
mod types;
mod vend;
mod wifi;

use anyhow::Result;
use display::Display;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{OutputPin, PinDriver};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::{config as spi_config, SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use log::{error, info, warn};
use touch::Touch;
use types::{MenuItem, PaymentStatus};

/// Build-time configuration; set these env vars when building/flashing:
///   WIFI_SSID, WIFI_PASS, BACKEND_URL (e.g. http://192.168.1.50:3000)
/// With any missing, the terminal runs in offline demo mode.
const WIFI_SSID: Option<&str> = option_env!("WIFI_SSID");
const WIFI_PASS: Option<&str> = option_env!("WIFI_PASS");
const BACKEND_URL: Option<&str> = option_env!("BACKEND_URL");

/// Relay pulse duration for one vend.
const VEND_PULSE_MS: u64 = 500;
/// How long a customer gets to pay before the session is abandoned.
const PAYMENT_TIMEOUT_S: u64 = 120;
/// Seconds between status polls while awaiting payment.
const POLL_INTERVAL_S: u64 = 2;

fn demo_menu() -> Vec<MenuItem> {
    ["Soda", "Water", "Snack"]
        .iter()
        .zip([("1", "1.50"), ("2", "1.00"), ("3", "2.00")])
        .map(|(name, (id, price))| MenuItem {
            id: id.to_string(),
            name: name.to_string(),
            price_usdc: price.to_string(),
        })
        .collect()
}

/// Block until the current touch is released, so one tap acts once.
fn wait_release(touch: &mut Touch) {
    while touch.poll().is_some() {
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("x402 vending terminal starting");

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
    let mut vend = vend::Vend::new(p.pins.gpio21.downgrade_output())?;

    // WiFi + backend, if configured at build time
    let online = match (WIFI_SSID, WIFI_PASS, BACKEND_URL) {
        (Some(ssid), Some(pass), Some(url)) => {
            display.show_message("Connecting to WiFi...");
            let sysloop = EspSystemEventLoop::take()?;
            let nvs = EspDefaultNvsPartition::take()?;
            match wifi::connect(p.modem, sysloop, Some(nvs), ssid, pass) {
                Ok(wifi_handle) => {
                    // keep the connection alive for the life of the program
                    std::mem::forget(wifi_handle);
                    Some(api::ApiClient::new(url))
                }
                Err(e) => {
                    error!("WiFi connect failed: {e:#}");
                    display.show_message("WiFi failed - offline demo");
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    None
                }
            }
        }
        _ => {
            warn!("WIFI_SSID/WIFI_PASS/BACKEND_URL not set at build time - offline demo");
            None
        }
    };

    // FETCH_MENU
    let items = match &online {
        Some(client) => {
            display.show_message("Loading menu...");
            let mut fetched = None;
            for attempt in 1..=3 {
                match client.fetch_menu() {
                    Ok(items) if !items.is_empty() => {
                        fetched = Some(items);
                        break;
                    }
                    Ok(_) => warn!("menu fetch attempt {attempt}: empty menu"),
                    Err(e) => warn!("menu fetch attempt {attempt}: {e:#}"),
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            match fetched {
                Some(items) => items,
                None => {
                    error!("could not fetch menu, falling back to demo items");
                    display.show_message("Backend unreachable - demo menu");
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    demo_menu()
                }
            }
        }
        None => demo_menu(),
    };

    // IDLE and the rest of the state machine
    loop {
        display.show_menu(&items);
        info!("Idle - menu displayed");

        // wait for a tap on an item
        let selected = loop {
            if let Some(point) = touch.poll() {
                if let Some(i) = Touch::item_at(point, items.len()) {
                    break i;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        };
        wait_release(&mut touch);
        info!("Selected: {}", items[selected].name);

        let Some(client) = &online else {
            display.show_message(&format!(
                "{} - offline demo, no payment",
                items[selected].name
            ));
            std::thread::sleep(std::time::Duration::from_millis(1500));
            continue;
        };

        // ITEM_SELECTED -> create session, show QR
        display.show_message("Creating payment session...");
        let session = match client.create_session(&items[selected].id) {
            Ok(s) => s,
            Err(e) => {
                error!("create_session failed: {e:#}");
                display.show_message("Backend error - try again");
                std::thread::sleep(std::time::Duration::from_secs(2));
                continue;
            }
        };
        info!(
            "Session {} created, payment url {}",
            session.session_id, session.payment_url
        );
        display.show_qr(&session.payment_url);

        // AWAITING_PAYMENT — poll status; tap the screen to cancel
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(PAYMENT_TIMEOUT_S);
        let mut last_poll =
            std::time::Instant::now() - std::time::Duration::from_secs(POLL_INTERVAL_S);
        let outcome = loop {
            if std::time::Instant::now() >= deadline {
                break None;
            }
            if touch.poll().is_some() {
                wait_release(&mut touch);
                info!("Payment wait cancelled by touch");
                break Some(PaymentStatus::Failed);
            }
            if last_poll.elapsed().as_secs() >= POLL_INTERVAL_S {
                last_poll = std::time::Instant::now();
                match client.poll_status(&session.session_id) {
                    Ok(s) => match s.status {
                        PaymentStatus::Pending => {}
                        done => break Some(done),
                    },
                    Err(e) => warn!("poll_status: {e:#}"),
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };

        match outcome {
            Some(PaymentStatus::Confirmed) => {
                info!("Payment confirmed - dispensing");
                display.show_message("Payment received!\nDispensing...");
                if let Err(e) = vend.dispense(VEND_PULSE_MS) {
                    error!("vend failed: {e:#}");
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Some(PaymentStatus::Failed) => {
                display.show_message("Payment cancelled");
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Some(PaymentStatus::Pending) => unreachable!(),
            None => {
                info!("Payment session timed out");
                display.show_message("Session expired");
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }
}
