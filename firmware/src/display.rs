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

    pub fn show_menu(&mut self, items: &[crate::types::MenuItem]) {
        info!("Display: showing {} menu items", items.len());
        self.clear(Rgb565::BLACK);

        let title = MonoTextStyle::new(&FONT_10X20, Rgb565::CSS_GOLD);
        let _ = Text::with_alignment(
            "TAP AN ITEM TO PAY WITH USDC",
            Point::new(WIDTH as i32 / 2, 34),
            title,
            Alignment::Center,
        )
        .draw(self);

        let label = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
        for (i, item) in items.iter().enumerate() {
            let rect = Self::item_hitbox(i);
            let _ = rect
                .into_styled(PrimitiveStyle::with_fill(Rgb565::new(6, 12, 12)))
                .draw(self);
            let cy = rect.top_left.y + rect.size.height as i32 / 2 + 7;
            let _ = Text::new(&item.name, Point::new(rect.top_left.x + 24, cy), label).draw(self);
            let price = format!("{} USDC", item.price_usdc);
            let _ = Text::with_alignment(
                &price,
                Point::new(rect.top_left.x + rect.size.width as i32 - 24, cy),
                label,
                Alignment::Right,
            )
            .draw(self);
        }
        let _ = self.flush();
    }

    /// Button rect for menu item `i` — shared with touch mapping.
    pub fn item_hitbox(i: usize) -> Rectangle {
        Rectangle::new(
            Point::new(20, 60 + i as i32 * 82),
            Size::new(WIDTH as u32 - 40, 72),
        )
    }

    pub fn show_qr(&mut self, data: &str) {
        info!("Display: QR code for {}", data);
        self.clear(Rgb565::WHITE);

        match qrcode::QrCode::new(data.as_bytes()) {
            Ok(code) => {
                let modules = code.width();
                let quiet = 4;
                let scale = ((HEIGHT - 40) / (modules + 2 * quiet)).max(1);
                let side = (modules + 2 * quiet) * scale;
                let ox = (WIDTH - side) as i32 / 2;
                let oy = (HEIGHT - side) as i32 / 2;
                let colors = code.to_colors();
                for (idx, c) in colors.iter().enumerate() {
                    if *c == qrcode::Color::Dark {
                        let mx = (idx % modules + quiet) * scale;
                        let my = (idx / modules + quiet) * scale;
                        let _ = Rectangle::new(
                            Point::new(ox + mx as i32, oy + my as i32),
                            Size::new(scale as u32, scale as u32),
                        )
                        .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
                        .draw(self);
                    }
                }
                let style = MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK);
                let _ = Text::with_alignment(
                    "SCAN TO PAY",
                    Point::new(WIDTH as i32 / 2, 24),
                    style,
                    Alignment::Center,
                )
                .draw(self);
            }
            Err(e) => {
                log::error!("QR encode failed: {e}");
                self.show_message("QR error");
                return;
            }
        }
        let _ = self.flush();
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
