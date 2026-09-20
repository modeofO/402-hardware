//! Screen layout shared by the renderer (`display.rs`) and the touch
//! mapper (`main.rs`): one source of truth for where each button lives.
//!
//! 480x320 landscape:
//!   y  16..126  countdown digits (seven-segment, see display.rs)
//!   y     150   status line ("LAMP ON" / "LAMP OFF" / "DONE")
//!   y 165..225  +1 / +5 / +15 / CLEAR
//!   y 240..304  START / STOP

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Add1,
    Add5,
    Add15,
    Clear,
    StartStop,
}

pub const DIGITS_TOP: i32 = 16;
pub const DIGITS_HEIGHT: u32 = 110;
pub const STATUS_BASELINE: i32 = 150;

const MARGIN: i32 = 16;
const GAP: i32 = 12;
const ROW1_TOP: i32 = 165;
const ROW1_HEIGHT: u32 = 60;
const ROW2_TOP: i32 = 240;
const ROW2_HEIGHT: u32 = 64;

impl Button {
    pub const ALL: [Button; 5] = [
        Button::Add1,
        Button::Add5,
        Button::Add15,
        Button::Clear,
        Button::StartStop,
    ];

    pub fn rect(self, screen_width: i32) -> Rectangle {
        let row1_w = (screen_width - 2 * MARGIN - 3 * GAP) / 4;
        let row1 = |col: i32| {
            Rectangle::new(
                Point::new(MARGIN + col * (row1_w + GAP), ROW1_TOP),
                Size::new(row1_w as u32, ROW1_HEIGHT),
            )
        };
        match self {
            Button::Add1 => row1(0),
            Button::Add5 => row1(1),
            Button::Add15 => row1(2),
            Button::Clear => row1(3),
            Button::StartStop => Rectangle::new(
                Point::new(MARGIN, ROW2_TOP),
                Size::new((screen_width - 2 * MARGIN) as u32, ROW2_HEIGHT),
            ),
        }
    }

    pub fn label(self, running: bool) -> &'static str {
        match self {
            Button::Add1 => "+1 MIN",
            Button::Add5 => "+5 MIN",
            Button::Add15 => "+15 MIN",
            Button::Clear => "CLEAR",
            Button::StartStop if running => "STOP",
            Button::StartStop => "START",
        }
    }

    /// Which button, if any, contains a touch point.
    pub fn at(point: Point, screen_width: i32) -> Option<Button> {
        Button::ALL
            .into_iter()
            .find(|b| b.rect(screen_width).contains(point))
    }
}
