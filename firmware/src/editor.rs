//! Time-entry flow behind the SET CLOCK and SCHEDULE buttons. Pure logic:
//! `main.rs` feeds it button presses and acts on the result of CONFIRM.
//!
//! The schedule is entered as two steps on the same screen:
//! LampOn --NEXT--> LampOff --SAVE--> done.

use crate::schedule::{wrapping_add, Minutes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editor {
    Clock(Minutes),
    LampOn(Minutes),
    LampOff { on: Minutes, off: Minutes },
}

impl Editor {
    /// The time being edited.
    pub fn value(&self) -> Minutes {
        match *self {
            Editor::Clock(t) | Editor::LampOn(t) | Editor::LampOff { off: t, .. } => t,
        }
    }

    fn value_mut(&mut self) -> &mut Minutes {
        match self {
            Editor::Clock(t) | Editor::LampOn(t) | Editor::LampOff { off: t, .. } => t,
        }
    }

    pub fn adjust_hours(&mut self, hours: i32) {
        let t = self.value_mut();
        *t = wrapping_add(*t, hours * 60);
    }

    /// The clock moves a minute at a time; schedule times move in 5s,
    /// which is plenty for a lamp and far fewer taps.
    pub fn adjust_minutes(&mut self, direction: i32) {
        let step = match self {
            Editor::Clock(_) => 1,
            Editor::LampOn(_) | Editor::LampOff { .. } => 5,
        };
        let t = self.value_mut();
        *t = wrapping_add(*t, direction * step);
    }
}
