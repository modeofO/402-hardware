//! Pin map for the ESP32-S3-DEVKITC-1-N32R16V breadboard prototype.
//!
//! Constraints on this module: GPIO 22–25 do not exist on the ESP32-S3,
//! GPIO 26–32 are used by flash, and GPIO 33–37 by the octal PSRAM.
//! Touch sense pins must be on ADC1 (GPIO 1–10).
//!
//! See docs/superpowers/specs/2026-06-23-x402-vending-terminal-design.md
//! ("Wiring") for the full table.

// Constants are consumed as peripherals get implemented.
#![allow(dead_code)]

// Display — Adafruit 2050 (HX8357D, 480x320) over SPI (S3 FSPI defaults)
pub const DISPLAY_SCK: u8 = 12; // display CLK
pub const DISPLAY_MOSI: u8 = 11;
pub const DISPLAY_MISO: u8 = 13; // SD/debug only, optional
pub const DISPLAY_CS: u8 = 10; // TFT CS (Card CS unused)
pub const DISPLAY_DC: u8 = 9;
pub const DISPLAY_RST: u8 = 14;

// Resistive touch (4-wire). Y+/X+ read via ADC1, Y-/X- driven.
pub const TOUCH_YP: u8 = 4;
pub const TOUCH_XP: u8 = 5;
pub const TOUCH_YM: u8 = 6;
pub const TOUCH_XM: u8 = 7;

// Relay module (Adafruit 2895)
pub const RELAY_IN: u8 = 21;
