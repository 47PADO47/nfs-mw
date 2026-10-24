//! The Remastered pursuit panel: the chase's time in large numbers at the top right on a dark right trapezoid, with the minutes
//! and seconds in the accent colour and the hundredths in white; under it the cop cars the chase has called, how many of them
//! are damaged and how many are wrecked; and the two bars of the chase's end: the meter of being busted or escaping, and the
//! cool-down once the player has got away.

mod draw;

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use crate::hud::{LABEL_BUSTED, game_word};
use crate::i18n::t;
use crate::ui::theme::custom::DriveFeedCell;

/// The signed meter is shown from this much of it, and is full at one.
const METER_SHOWN: f32 = 0.01;

pub struct PursuitTimer {
    feed: DriveFeedCell,
    parts: Vec<Box<dyn Component>>,
}

impl PursuitTimer {
    pub fn new(feed: DriveFeedCell) -> Self {
        Self { feed, parts: Vec::new() }
    }
}

impl Component for PursuitTimer {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let feed = self.feed.snapshot();
        self.parts = match feed.pursuit {
            Some(pursuit) => draw::parts(&feed, &pursuit, cx),
            None => Vec::new(),
        };
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// The time as the board writes it: minutes without a leading zero, then seconds and hundredths.
pub(super) fn time_parts(secs: f32) -> (String, String) {
    // A little over, so a time that is 137.43 s in the sum (and 137.42999 in a float) still reads .43.
    let hundredths = (secs.max(0.0) * 100.0 + 0.01) as u64;
    let main = format!("{}:{:02}.", hundredths / 6000, (hundredths / 100) % 60);
    (main, format!("{:02}", hundredths % 100))
}

/// Which side of the signed meter the chase is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    /// The cops are closing in (the red bar).
    Busted,
    /// The player is getting away (the accent).
    Escaping,
}

impl Side {
    /// The side's label, in capitals as the panel writes it: the game's `BUSTED`, the rewrite's "escaping".
    pub(super) fn label(self) -> String {
        match self {
            Side::Busted => game_word(LABEL_BUSTED, "BUSTED").to_uppercase(),
            Side::Escaping => t!("hud-pursuit-escaping").to_uppercase(),
        }
    }
}

/// The signed meter as the panel shows it: the side it is on and the fill of that side, or `None` while it rests at zero. A
/// positive value is the busted side, a negative one the escaping side.
pub(super) fn meter(busted: f32) -> Option<(Side, f32)> {
    let fill = busted.abs().clamp(0.0, 1.0);
    match busted {
        t if t >= 1.0 => Some((Side::Busted, 1.0)),
        t if t >= METER_SHOWN => Some((Side::Busted, fill)),
        t if t <= -METER_SHOWN => Some((Side::Escaping, fill)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_drops_the_leading_zero_of_the_minutes() {
        assert_eq!(time_parts(137.43), ("2:17.".into(), "43".into()));
        assert_eq!(time_parts(0.0), ("0:00.".into(), "00".into()));
        assert_eq!(time_parts(605.07), ("10:05.".into(), "07".into()));
        assert_eq!(time_parts(-3.0), ("0:00.".into(), "00".into()));
    }

    #[test]
    fn the_signed_meter_is_busted_above_zero_escaping_below_it_and_hidden_at_rest() {
        assert_eq!(meter(0.0), None);
        assert_eq!(meter(0.004), None);
        assert_eq!(meter(0.4), Some((Side::Busted, 0.4)));
        assert_eq!(meter(1.0), Some((Side::Busted, 1.0)));
        assert_eq!(meter(-0.25), Some((Side::Escaping, 0.25)));
        assert_eq!(meter(-3.0), Some((Side::Escaping, 1.0)), "a value past the end is a full bar");
    }
}
