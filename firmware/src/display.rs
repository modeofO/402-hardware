//! HX8357D (Adafruit 2050, 480x320) driver over SPI.
//!
//! mipidsi has no HX8357D model, so this is a minimal driver: the init
//! sequence follows Adafruit's C driver. Rendering goes through a full
//! RGB565 framebuffer (307KB, lands in PSRAM via CONFIG_SPIRAM_USE_MALLOC)
//! that `flush()` streams to the panel in one RAMWR burst — per-pixel SPI
//! transactions would be orders of magnitude slower.
//!
//! Framing is standard 4-wire 8-bit SPI with the D/C pin (IM2 jumper
//! closed on the breakout). Two hard-won constraints from bring-up:
//! CS is driven manually and held low across a full command+data window,
//! and the SPI bus must be created WITHOUT a MISO pin — routing GPIO13 as
//! MISO made the panel ignore everything (probe-verified; root cause in
//! the esp-idf full-duplex path unclear). The panel is write-only here.
//! Pixel bytes go out big-endian (RGB565 high byte first) — see
//! docs/hardware-notes.md for the full bring-up story.

use anyhow::Result;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyOutputPin, Output, PinDriver};
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver};
use log::info;

use crate::timer::Phase;
use crate::ui;

pub const WIDTH: usize = 480;
pub const HEIGHT: usize = 320;

// HX8357D commands
const SWRESET: u8 = 0x01;
const SLPOUT: u8 = 0x11;
const TEON: u8 = 0x35;
const MADCTL: u8 = 0x36;
const COLMOD: u8 = 0x3A;
const DISPON: u8 = 0x29;
const CASET: u8 = 0x2A;
const PASET: u8 = 0x2B;
const RAMWR: u8 = 0x2C;
const TEARLINE: u8 = 0x44;
const SETOSC: u8 = 0xB0;
const SETPWR1: u8 = 0xB1;
const SETRGB: u8 = 0xB3;
const SETCYC: u8 = 0xB4;
const SETCOM: u8 = 0xB6;
const SETC: u8 = 0xB9;
const SETSTBA: u8 = 0xC0;
const SETPANEL: u8 = 0xCC;
const SETGAMMA: u8 = 0xE0;

// MADCTL_MY | MADCTL_MV — landscape 480x320, USB port on the left
const MADCTL_LANDSCAPE: u8 = 0xA0;

pub struct Display {
    spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
    // Manual CS held low across a full command+data window, matching
    // Adafruit's driver — hardware CS would release between the command
    // byte and its parameters.
    cs: PinDriver<'static, AnyOutputPin, Output>,
    dc: PinDriver<'static, AnyOutputPin, Output>,
    #[allow(dead_code)] // held so the pin stays high
    rst: PinDriver<'static, AnyOutputPin, Output>,
    fb: Vec<u8>,
}

impl Display {
    pub fn new(
        spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
        mut cs: PinDriver<'static, AnyOutputPin, Output>,
        mut dc: PinDriver<'static, AnyOutputPin, Output>,
        mut rst: PinDriver<'static, AnyOutputPin, Output>,
    ) -> Result<Self> {
        cs.set_high()?;
        dc.set_low()?;
        rst.set_high()?;
        FreeRtos::delay_ms(10);
        rst.set_low()?;
        FreeRtos::delay_ms(10);
        rst.set_high()?;
        FreeRtos::delay_ms(150);

        let mut d = Self {
            spi,
            cs,
            dc,
            rst,
            fb: vec![0u8; WIDTH * HEIGHT * 2],
        };
        d.init_panel()?;
        info!("Display: HX8357D initialized (4-wire SPI, write-only)");
        Ok(d)
    }

    fn init_panel(&mut self) -> Result<()> {
        self.cmd(SWRESET, &[])?;
        FreeRtos::delay_ms(10);
        self.cmd(SETC, &[0xFF, 0x83, 0x57])?;
        FreeRtos::delay_ms(300);
        self.cmd(SETRGB, &[0x80, 0x00, 0x06, 0x06])?;
        self.cmd(SETCOM, &[0x25])?;
        self.cmd(SETOSC, &[0x68])?;
        self.cmd(SETPANEL, &[0x05])?;
        self.cmd(SETPWR1, &[0x00, 0x15, 0x1C, 0x1C, 0x83, 0xAA])?;
        self.cmd(SETSTBA, &[0x50, 0x50, 0x01, 0x3C, 0x1E, 0x08])?;
        self.cmd(SETCYC, &[0x02, 0x40, 0x00, 0x2A, 0x2A, 0x0D, 0x78])?;
        self.cmd(
            SETGAMMA,
            &[
                0x02, 0x0A, 0x11, 0x1D, 0x23, 0x35, 0x41, 0x4B, 0x4B, 0x42, 0x3A, 0x27, 0x1B, 0x08,
                0x09, 0x03, 0x02, 0x0A, 0x11, 0x1D, 0x23, 0x35, 0x41, 0x4B, 0x4B, 0x42, 0x3A, 0x27,
                0x1B, 0x08, 0x09, 0x03, 0x00, 0x01,
            ],
        )?;
        self.cmd(COLMOD, &[0x55])?; // RGB565
        self.cmd(MADCTL, &[MADCTL_LANDSCAPE])?;
        self.cmd(TEON, &[0x00])?;
        self.cmd(TEARLINE, &[0x00, 0x02])?;
        self.cmd(SLPOUT, &[])?;
        FreeRtos::delay_ms(150);
        self.cmd(DISPON, &[])?;
        FreeRtos::delay_ms(50);
        Ok(())
    }

    fn cmd(&mut self, cmd: u8, data: &[u8]) -> Result<()> {
        self.cs.set_low()?;
        self.dc.set_low()?;
        self.spi.write(&[cmd])?;
        if !data.is_empty() {
            self.dc.set_high()?;
            self.spi.write(data)?;
        }
        self.cs.set_high()?;
        Ok(())
    }

    /// Push the framebuffer to the panel.
    pub fn flush(&mut self) -> Result<()> {
        let w = (WIDTH - 1) as u16;
        let h = (HEIGHT - 1) as u16;
        self.cmd(CASET, &[0, 0, (w >> 8) as u8, (w & 0xFF) as u8])?;
        self.cmd(PASET, &[0, 0, (h >> 8) as u8, (h & 0xFF) as u8])?;
        self.cs.set_low()?;
        self.dc.set_low()?;
        self.spi.write(&[RAMWR])?;
        self.dc.set_high()?;
        // esp-idf-hal chunks this into max-transfer-sized transactions
        let fb = std::mem::take(&mut self.fb);
        let res = self.spi.write(&fb);
        self.fb = fb;
        res?;
        self.cs.set_high()?;
        Ok(())
    }

    pub fn clear(&mut self, color: Rgb565) {
        let raw = RawU16::from(color).into_inner().to_be_bytes();
        for px in self.fb.chunks_exact_mut(2) {
            px.copy_from_slice(&raw);
        }
    }

    pub fn show_message(&mut self, msg: &str) {
        info!("Display: {}", msg);
        self.clear(Rgb565::BLACK);
        let style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
        let _ = Text::with_alignment(
            msg,
            Point::new(WIDTH as i32 / 2, HEIGHT as i32 / 2),
            style,
            Alignment::Center,
        )
        .draw(self);
        let _ = self.flush();
    }

    /// Full timer screen: countdown digits, status line, buttons.
    pub fn show_timer(&mut self, remaining_secs: u64, phase: Phase) {
        self.clear(Rgb565::BLACK);

        let (digit_color, status, status_color) = match phase {
            Phase::Running => (Rgb565::GREEN, "LAMP ON", Rgb565::GREEN),
            Phase::Idle if remaining_secs > 0 => (Rgb565::WHITE, "LAMP OFF", Rgb565::CSS_GRAY),
            Phase::Idle => (Rgb565::CSS_DIM_GRAY, "SET A TIME", Rgb565::CSS_GRAY),
            Phase::Done => (Rgb565::CSS_ORANGE, "DONE - LAMP OFF", Rgb565::CSS_ORANGE),
        };
        self.draw_countdown(remaining_secs, digit_color);

        let _ = Text::with_alignment(
            status,
            Point::new(WIDTH as i32 / 2, ui::STATUS_BASELINE),
            MonoTextStyle::new(&FONT_10X20, status_color),
            Alignment::Center,
        )
        .draw(self);

        let running = phase == Phase::Running;
        for button in ui::Button::ALL {
            let fill = match button {
                ui::Button::StartStop if running => Rgb565::new(20, 8, 4),
                ui::Button::StartStop if remaining_secs > 0 => Rgb565::new(2, 28, 6),
                ui::Button::StartStop => Rgb565::new(4, 8, 4),
                _ => Rgb565::new(6, 12, 12),
            };
            self.draw_button(button.rect(WIDTH as i32), button.label(running), fill);
        }
        let _ = self.flush();
    }

    fn draw_button(&mut self, rect: Rectangle, label: &str, fill: Rgb565) {
        let _ = rect.into_styled(PrimitiveStyle::with_fill(fill)).draw(self);
        let center = rect.center();
        let _ = Text::with_alignment(
            label,
            Point::new(center.x, center.y + 7),
            MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE),
            Alignment::Center,
        )
        .draw(self);
    }

    /// Seven-segment countdown, centred: MM:SS under an hour, else H:MM:SS.
    fn draw_countdown(&mut self, secs: u64, color: Rgb565) {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        // None = colon
        let mut glyphs: Vec<Option<u8>> = Vec::with_capacity(7);
        if h > 0 {
            glyphs.push(Some((h % 10) as u8));
            glyphs.push(None);
        }
        glyphs.extend([
            Some((m / 10) as u8),
            Some((m % 10) as u8),
            None,
            Some((s / 10) as u8),
            Some((s % 10) as u8),
        ]);

        const DIGIT_W: i32 = 64;
        const COLON_W: i32 = 24;
        const GAP: i32 = 12;
        const THICK: i32 = 12;
        let height = ui::DIGITS_HEIGHT as i32;
        let total: i32 = glyphs
            .iter()
            .map(|g| if g.is_some() { DIGIT_W } else { COLON_W })
            .sum::<i32>()
            + GAP * (glyphs.len() as i32 - 1);
        let mut x = (WIDTH as i32 - total) / 2;
        let y = ui::DIGITS_TOP;
        for g in glyphs {
            match g {
                Some(d) => {
                    self.draw_seven_seg(x, y, DIGIT_W, height, THICK, d, color);
                    x += DIGIT_W + GAP;
                }
                None => {
                    let cx = x + (COLON_W - THICK) / 2;
                    for cy in [y + height / 4, y + 3 * height / 4] {
                        self.fill_rect(cx, cy - THICK / 2, THICK, THICK, color);
                    }
                    x += COLON_W + GAP;
                }
            }
        }
    }

    /// One digit as lit segments. Bits: a=top, b=top-right, c=bottom-right,
    /// d=bottom, e=bottom-left, f=top-left, g=middle.
    fn draw_seven_seg(&mut self, x: i32, y: i32, w: i32, h: i32, t: i32, digit: u8, color: Rgb565) {
        const SEGMENTS: [u8; 10] = [
            0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111,
            0b1111111, 0b1101111,
        ];
        let lit = SEGMENTS[(digit % 10) as usize];
        let mid = y + h / 2;
        let upper = (y + t, mid - t / 2 - (y + t));
        let lower = (mid + t / 2, (y + h - t) - (mid + t / 2));
        let segs = [
            (x + t, y, w - 2 * t, t),           // a
            (x + w - t, upper.0, t, upper.1),   // b
            (x + w - t, lower.0, t, lower.1),   // c
            (x + t, y + h - t, w - 2 * t, t),   // d
            (x, lower.0, t, lower.1),           // e
            (x, upper.0, t, upper.1),           // f
            (x + t, mid - t / 2, w - 2 * t, t), // g
        ];
        for (i, (sx, sy, sw, sh)) in segs.into_iter().enumerate() {
            if lit & (1 << i) != 0 {
                self.fill_rect(sx, sy, sw, sh, color);
            }
        }
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgb565) {
        let _ = Rectangle::new(Point::new(x, y), Size::new(w as u32, h as u32))
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(self);
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl DrawTarget for Display {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(p, color) in pixels {
            if (0..WIDTH as i32).contains(&p.x) && (0..HEIGHT as i32).contains(&p.y) {
                let idx = (p.y as usize * WIDTH + p.x as usize) * 2;
                let raw = RawU16::from(color).into_inner().to_be_bytes();
                self.fb[idx..idx + 2].copy_from_slice(&raw);
            }
        }
        Ok(())
    }
}
