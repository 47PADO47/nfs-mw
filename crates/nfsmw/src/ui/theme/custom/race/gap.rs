//! The rival gap in words (`docs/specs/custom-hud/race.md` section 5): the size of the checkpoint split with two
//! decimals and the side as a word, never a signed number: `1.42 s AHEAD`, `0.85 s BEHIND`, or `LEVEL` when the two
//! passed the checkpoint at the same time to the shown precision. The strip adds an up or a down chevron, so the side
//! reads without colour.

use crate::hud::RivalGap;
use crate::i18n::t;

/// Below this the two times are the same at two decimals.
pub const LEVEL_BELOW: f32 = 0.005;

/// Which side of the rival the player is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Ahead,
    Behind,
    Level,
}

impl Side {
    /// The side of `gap`.
    pub fn of(gap: &RivalGap) -> Self {
        if gap.seconds.is_nan() || gap.seconds.abs() < LEVEL_BELOW {
            return Side::Level;
        }
        match gap.ahead {
            true => Side::Ahead,
            false => Side::Behind,
        }
    }
}

/// The gap in words, from the catalogue: the unsigned seconds and the side.
pub fn gap_words(gap: &RivalGap) -> String {
    let seconds = format!("{:.2}", gap.seconds.abs());
    match Side::of(gap) {
        Side::Ahead => t!("hud-gap-ahead", seconds = seconds),
        Side::Behind => t!("hud-gap-behind", seconds = seconds),
        Side::Level => t!("hud-gap-level"),
    }
}
