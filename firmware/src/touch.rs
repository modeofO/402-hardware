//! 4-wire resistive touch, read Adafruit-TouchScreen style:
//! drive one axis, ADC-sample the other, plus a Z (pressure) phase for
//! touch detection. Pins swap roles between GPIO-output and ADC-input on
//! every read, so short-lived drivers borrow the owned pins per phase.
//!
//! YP = GPIO4 (ADC1_CH3), XP = GPIO5 (ADC1_CH4),
//! YM = GPIO6 (drive),    XM = GPIO7 (ADC1_CH6 / drive)

use anyhow::Result;
use embedded_graphics::prelude::Point;
use esp_idf_svc::hal::adc::attenuation::DB_11;
use esp_idf_svc::hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_svc::hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_svc::hal::adc::ADC1;
use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{ADCPin, Gpio4, Gpio5, Gpio6, Gpio7, PinDriver, Pull};
use log::info;

/// Raw ADC range seen at the panel edges. Rough defaults for a 12-bit
/// read; calibrate on device (log raw values and adjust).
const RAW_X_MIN: i32 = 300;
const RAW_X_MAX: i32 = 3800;
const RAW_Y_MIN: i32 = 300;
const RAW_Y_MAX: i32 = 3800;
/// Z threshold — readings below this mean "not touched".
const Z_THRESHOLD: i32 = 100;

/// Panel raw axes vs. our landscape orientation (MADCTL 0xA0).
/// Adjust after on-device calibration.
const SWAP_XY: bool = true;
const INVERT_X: bool = false;
const INVERT_Y: bool = false;

pub struct Touch {
    adc: AdcDriver<'static, ADC1>,
    yp: Gpio4,
    xp: Gpio5,
    ym: Gpio6,
    xm: Gpio7,
}

/// Median-of-3 raw read of one prepared ADC pin.
fn sample<P: ADCPin<Adc = ADC1>>(adc: &AdcDriver<'static, ADC1>, pin: &mut P) -> Result<i32> {
    let cfg = AdcChannelConfig {
        attenuation: DB_11,
        ..Default::default()
    };
    let mut ch = AdcChannelDriver::new(adc, pin, &cfg)?;
    let mut v = [0i32; 3];
    for s in v.iter_mut() {
        *s = adc.read_raw(&mut ch)? as i32;
    }
    v.sort_unstable();
    Ok(v[1])
}

impl Touch {
    pub fn new(adc1: ADC1, yp: Gpio4, xp: Gpio5, ym: Gpio6, xm: Gpio7) -> Result<Self> {
        let adc = AdcDriver::new(adc1)?;
        info!("Touch: 4-wire resistive on YP=4 XP=5 YM=6 XM=7");
        Ok(Self {
            adc,
            yp,
            xp,
            ym,
            xm,
        })
    }

    /// Pressure: XP low, YM high, sense YP/XM. Touched plates connect
    /// the layers; untouched sense lines stay near the rails.
    fn read_z(&mut self) -> Result<i32> {
        let Self {
            adc,
            yp,
            xp,
            ym,
            xm,
        } = self;
        let mut xp = PinDriver::output(&mut *xp)?;
        let mut ym_d = PinDriver::output(&mut *ym)?;
        xp.set_low()?;
        ym_d.set_high()?;
        Ets::delay_us(20);
        let z1 = sample(adc, xm)?;
        let z2 = sample(adc, yp)?;
        Ok(4095 - (z2 - z1))
    }

    /// X axis: drive X plate (XP high, XM low), sense on YP.
    fn read_raw_x(&mut self) -> Result<i32> {
        let Self {
            adc,
            yp,
            xp,
            ym,
            xm,
        } = self;
        let mut xp = PinDriver::output(&mut *xp)?;
        let mut xm = PinDriver::output(&mut *xm)?;
        xp.set_high()?;
        xm.set_low()?;
        let mut ym = PinDriver::input(&mut *ym)?;
        ym.set_pull(Pull::Floating)?;
        Ets::delay_us(20);
        sample(adc, yp)
    }

    /// Y axis: drive Y plate (YP high, YM low), sense on XP.
    fn read_raw_y(&mut self) -> Result<i32> {
        let Self {
            adc,
            yp,
            xp,
            ym,
            xm,
        } = self;
        let mut yp = PinDriver::output(&mut *yp)?;
        let mut ym = PinDriver::output(&mut *ym)?;
        yp.set_high()?;
        ym.set_low()?;
        let mut xm = PinDriver::input(&mut *xm)?;
        xm.set_pull(Pull::Floating)?;
        Ets::delay_us(20);
        sample(adc, xp)
    }

    /// Returns the touched point in screen coordinates, or None.
    pub fn poll(&mut self) -> Option<Point> {
        let z = self.read_z().ok()?;
        if z < Z_THRESHOLD {
            return None;
        }
        let rx = self.read_raw_x().ok()?;
        let ry = self.read_raw_y().ok()?;
        // Confirm still touched so we don't report a release glitch
        if self.read_z().ok()? < Z_THRESHOLD {
            return None;
        }

        let (mut rx, mut ry) = if SWAP_XY { (ry, rx) } else { (rx, ry) };
        if INVERT_X {
            rx = RAW_X_MAX + RAW_X_MIN - rx;
        }
        if INVERT_Y {
            ry = RAW_Y_MAX + RAW_Y_MIN - ry;
        }
        let x = (rx - RAW_X_MIN) * crate::display::WIDTH as i32 / (RAW_X_MAX - RAW_X_MIN);
        let y = (ry - RAW_Y_MIN) * crate::display::HEIGHT as i32 / (RAW_Y_MAX - RAW_Y_MIN);
        log::debug!("Touch: raw=({rx},{ry}) z={z} -> ({x},{y})");
        Some(Point::new(
            x.clamp(0, crate::display::WIDTH as i32 - 1),
            y.clamp(0, crate::display::HEIGHT as i32 - 1),
        ))
    }

    /// Map a touch point to a menu item index.
    pub fn item_at(point: Point, item_count: usize) -> Option<usize> {
        (0..item_count).find(|&i| crate::display::Display::item_hitbox(i).contains(point))
    }
}
