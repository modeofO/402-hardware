//! Lamp timer: the lamp follows a daily on/off schedule against a clock
//! set on the touchscreen (no network, see clock.rs), with a manual
//! override button. Single-threaded loop: poll touch, work out what the
//! lamp should be doing, redraw when the visible state changes (a
//! full-frame flush costs ~100ms at 26MHz, so never more than that).

mod clock;
mod display;
mod editor;
mod pins;
mod relay;
mod schedule;
mod store;
mod touch;
mod ui;

use anyhow::Result;
use clock::Trust;
use display::{Display, MainView};
use editor::Editor;
use esp_idf_svc::hal::gpio::{OutputPin, PinDriver};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi::{config as spi_config, SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use log::{error, info, LevelFilter};
use schedule::{Lamp, Minutes, Schedule};
use std::time::{Duration, Instant};
use store::Store;
use touch::Touch;
use ui::{Button, Layout};

/// Schedule used until one is saved: 17:00 to midnight.
const DEFAULT_SCHEDULE: Schedule = Schedule {
    on: 17 * 60,
    off: 0,
};
/// SET CLOCK opens here when there is no time to start from.
const DEFAULT_CLOCK: Minutes = 12 * 60;

/// Holding an editor +/- button repeats it after this long...
const REPEAT_DELAY: Duration = Duration::from_millis(500);
/// ...at this pace (each repeat also pays for a redraw).
const REPEAT_INTERVAL: Duration = Duration::from_millis(100);
/// Consecutive empty polls that end a press, so one noisy sample in the
/// middle of a touch does not register as a second tap.
const RELEASE_POLLS: u8 = 2;

/// A touch in progress. `button` is what it landed on, if anything.
struct Press {
    button: Option<Button>,
    repeat_at: Instant,
    misses: u8,
}

#[derive(PartialEq)]
enum View {
    Main(MainView),
    Editor(Editor),
}

fn minutes_now() -> Option<Minutes> {
    clock::secs_of_day().map(|s| (s / 60) as Minutes)
}

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    // The touch driver re-roles its pins on every poll; the GPIO driver
    // logs each change at info, which would drown everything else.
    esp_idf_svc::log::set_target_level("gpio", LevelFilter::Warn)?;

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

    let store = Store::new()?;
    match (clock::trust(), store.clock()) {
        (Trust::Unset, Some(secs)) => {
            clock::set(secs, true);
            info!("Clock: restored checkpoint, late by however long power was out");
        }
        (Trust::Unset, None) => info!("Clock: not set"),
        (trust, _) => info!("Clock: kept across reset ({trust:?})"),
    }
    let mut lamp = Lamp::new(store.schedule().unwrap_or(DEFAULT_SCHEDULE));
    info!("Schedule: {:?}", lamp.schedule());

    // None = main screen
    let mut editor: Option<Editor> = None;
    let mut press: Option<Press> = None;
    let mut checkpointed: Option<Minutes> = None;
    let mut shown: Option<View> = None;

    loop {
        let now = Instant::now();

        // One tap fires once; held editor +/- buttons repeat.
        let mut fired = None;
        if let Some(point) = touch.poll() {
            if let Some(held) = press.as_mut() {
                held.misses = 0;
                if held.button.is_some_and(Button::repeats) && now >= held.repeat_at {
                    fired = held.button;
                    held.repeat_at = now + REPEAT_INTERVAL;
                }
            } else {
                let layout = if editor.is_some() {
                    Layout::Editor
                } else {
                    Layout::Main
                };
                fired = layout.button_at(point, display::WIDTH as i32);
                press = Some(Press {
                    button: fired,
                    repeat_at: now + REPEAT_DELAY,
                    misses: 0,
                });
            }
        } else if press.as_mut().is_some_and(|held| {
            held.misses += 1;
            held.misses >= RELEASE_POLLS
        }) {
            press = None;
        }

        if let Some(button) = fired {
            info!("Button: {button:?}");
            editor = match (editor, button) {
                (None, Button::SetClock) => {
                    Some(Editor::Clock(minutes_now().unwrap_or(DEFAULT_CLOCK)))
                }
                (None, Button::SetSchedule) => Some(Editor::LampOn(lamp.schedule().on)),
                (None, Button::LampToggle) => {
                    lamp.toggle(minutes_now());
                    None
                }
                (Some(mut e), Button::HourDown) => {
                    e.adjust_hours(-1);
                    Some(e)
                }
                (Some(mut e), Button::HourUp) => {
                    e.adjust_hours(1);
                    Some(e)
                }
                (Some(mut e), Button::MinuteDown) => {
                    e.adjust_minutes(-1);
                    Some(e)
                }
                (Some(mut e), Button::MinuteUp) => {
                    e.adjust_minutes(1);
                    Some(e)
                }
                (Some(_), Button::Cancel) => None,
                (Some(Editor::Clock(time)), Button::Confirm) => {
                    clock::set(u32::from(time) * 60, false);
                    info!("Clock: set to {:02}:{:02}", time / 60, time % 60);
                    checkpointed = None;
                    None
                }
                (Some(Editor::LampOn(on)), Button::Confirm) => Some(Editor::LampOff {
                    on,
                    off: lamp.schedule().off,
                }),
                (Some(Editor::LampOff { on, off }), Button::Confirm) => {
                    let schedule = Schedule { on, off };
                    lamp.set_schedule(schedule);
                    info!("Schedule: {schedule:?}");
                    if let Err(e) = store.save_schedule(schedule) {
                        error!("saving schedule failed: {e:#}");
                    }
                    None
                }
                (unchanged, _) => unchanged,
            };
        }

        let minutes = minutes_now();
        let lamp_on = lamp.update(minutes);

        // Relay state always follows the lamp logic, whatever changed it.
        if relay.is_on() != lamp_on {
            let res = if lamp_on { relay.on() } else { relay.off() };
            if let Err(e) = res {
                error!("relay sync failed: {e:#}");
            }
        }

        // Checkpoint the clock once a minute so a power cut only costs
        // the length of the outage (see clock.rs).
        if let Some(secs) = clock::secs_of_day().filter(|_| minutes != checkpointed) {
            if let Err(e) = store.save_clock(secs) {
                error!("clock checkpoint failed: {e:#}");
            }
            checkpointed = minutes;
        }

        let view = match editor {
            Some(e) => View::Editor(e),
            None => View::Main(MainView {
                time: minutes,
                restored: clock::trust() == Trust::Restored,
                lamp_on,
                manual: lamp.is_manual(),
                schedule: lamp.schedule(),
            }),
        };
        if shown.as_ref() != Some(&view) {
            match &view {
                View::Main(main) => display.show_main(main),
                View::Editor(e) => display.show_editor(e),
            }
            shown = Some(view);
        }

        std::thread::sleep(Duration::from_millis(30));
    }
}
