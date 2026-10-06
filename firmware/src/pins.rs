//! Pin map for the lamp-timer carrier board (`designer/projects/lamp-timer`),
//! an ESP32-S3-WROOM-1 behind the Adafruit 2050 display.
//!
//! The breadboard prototype on the DEVKITC-1 used SCK 12 / MOSI 11 / CS 10 /
//! DC 9 / RST 14 and touch 4/5/6/7. The carrier's routing wanted the display
//! header and the module's pin row to nest without crossings on two layers,
//! which fixed the assignment below; the numbers are the board's, not a
//! preference. Constraints that still apply: GPIO 22–25 do not exist on the
//! ESP32-S3, GPIO 26–32 are flash, GPIO 33–37 are the octal PSRAM, and the
//! touch pins must be on ADC1 (GPIO 1–10).
//!
//! See docs/superpowers/specs/2026-09-14-lamp-timer-design.md ("Wiring")
//! for the full table and docs/hardware-notes.md for bring-up findings.

// Constants are consumed as peripherals get implemented.
#![allow(dead_code)]

// Display — Adafruit 2050 (HX8357D, 480x320) over SPI, write-only
pub const DISPLAY_SCK: u8 = 10; // display CLK
pub const DISPLAY_MOSI: u8 = 11;
pub const DISPLAY_CS: u8 = 12; // TFT CS, driven manually (see display.rs)
pub const DISPLAY_DC: u8 = 14;
pub const DISPLAY_RST: u8 = 47;
/// Backlight enable/PWM ("Lite"). Pulled up on the 2050, so left as an
/// input the backlight is simply on; drive it low to dim or blank.
pub const DISPLAY_LITE: u8 = 48;
// MISO is not wired: see docs/hardware-notes.md.

// Resistive touch (4-wire), all on ADC1: YP = ADC1_CH6, XP = ADC1_CH5,
// YM = ADC1_CH4, XM = ADC1_CH3.
pub const TOUCH_YP: u8 = 7;
pub const TOUCH_XP: u8 = 6;
pub const TOUCH_YM: u8 = 5;
pub const TOUCH_XM: u8 = 4;

// Digital Loggers IoT Power Relay: trigger "+" (trigger "-" to GND).
// Active high, opto-isolated, 3–60V DC input — 3.3V GPIO drives it directly.
pub const RELAY_IN: u8 = 21;

/// Status LED (green, through 470R to GND). Active high. GPIO 45 is a
/// strapping pin (VDD_SPI select): the LED only ever loads it toward
/// ground, which is its default and the safe boot state.
pub const STATUS_LED: u8 = 45;

// DS3231 RTC on I2C (not driven yet — see the design doc's "Not in scope").
pub const RTC_SDA: u8 = 15;
pub const RTC_SCL: u8 = 16;
