//! Screen layout shared by the renderer (`display.rs`) and the touch
//! mapper (`main.rs`): one source of truth for where each button lives.
//!
//! 480x320 landscape, both screens:
//!   y  12..112  HH:MM digits (seven-segment, see display.rs)
//!   y     140   status line
//!   y     166   detail line
//!   y 182..238  main: SET CLOCK / SCHEDULE    editor: HOUR -/+  MIN -/+
//!   y 250..308  main: TURN LAMP ON/OFF        editor: CANCEL / NEXT|SAVE

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    SetClock,
    SetSchedule,
    LampToggle,
    HourDown,
    HourUp,
    MinuteDown,
    MinuteUp,
    Cancel,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Main,
    Editor,
}

pub const DIGITS_TOP: i32 = 12;
pub const DIGITS_HEIGHT: u32 = 100;
pub const STATUS_BASELINE: i32 = 140;
pub const DETAIL_BASELINE: i32 = 166;

const MARGIN: i32 = 16;
const GAP: i32 = 12;
const ROW1_TOP: i32 = 182;
const ROW1_HEIGHT: u32 = 56;
const ROW2_TOP: i32 = 250;
const ROW2_HEIGHT: u32 = 58;

impl Layout {
    pub fn buttons(self) -> &'static [Button] {
        match self {
            Layout::Main => &[Button::SetClock, Button::SetSchedule, Button::LampToggle],
            Layout::Editor => &[
                Button::HourDown,
                Button::HourUp,
                Button::MinuteDown,
                Button::MinuteUp,
                Button::Cancel,
                Button::Confirm,
            ],
        }
    }

    /// Which of this screen's buttons, if any, contains a touch point.
    pub fn button_at(self, point: Point, screen_width: i32) -> Option<Button> {
        self.buttons()
            .iter()
            .copied()
            .find(|b| b.rect(screen_width).contains(point))
    }
}

impl Button {
    pub fn rect(self, screen_width: i32) -> Rectangle {
        // Cell `col` of a row split into `cols` equal buttons.
        let cell = |top: i32, height: u32, col: i32, cols: i32| {
            let w = (screen_width - 2 * MARGIN - (cols - 1) * GAP) / cols;
            Rectangle::new(
                Point::new(MARGIN + col * (w + GAP), top),
                Size::new(w as u32, height),
            )
        };
        match self {
            Button::SetClock => cell(ROW1_TOP, ROW1_HEIGHT, 0, 2),
            Button::SetSchedule => cell(ROW1_TOP, ROW1_HEIGHT, 1, 2),
            Button::LampToggle => cell(ROW2_TOP, ROW2_HEIGHT, 0, 1),
            Button::HourDown => cell(ROW1_TOP, ROW1_HEIGHT, 0, 4),
            Button::HourUp => cell(ROW1_TOP, ROW1_HEIGHT, 1, 4),
            Button::MinuteDown => cell(ROW1_TOP, ROW1_HEIGHT, 2, 4),
            Button::MinuteUp => cell(ROW1_TOP, ROW1_HEIGHT, 3, 4),
            Button::Cancel => cell(ROW2_TOP, ROW2_HEIGHT, 0, 2),
            Button::Confirm => cell(ROW2_TOP, ROW2_HEIGHT, 1, 2),
        }
    }

    /// Whether holding the button down repeats it.
    pub fn repeats(self) -> bool {
        matches!(
            self,
            Button::HourDown | Button::HourUp | Button::MinuteDown | Button::MinuteUp
        )
    }
}
