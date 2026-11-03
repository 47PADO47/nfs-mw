//! The pad's weight lever: the left stick's Y turned into the rider's longitudinal weight, -1 (back) to 1 (forward)
//! (`docs/specs/ride6-weight-round9.md`, section 4).
//!
//! The reference's physics reads the lever as it is (`Actions.WeightLongitudinal`); how a device reaches it is the game's input
//! layer, which the physics module does not contain. The rider help of a bike tests the lever against fixed numbers (0.75 for
//! the auto-rider, 0.95 for the stoppie help), so a stick that cannot report them is a rider who cannot do the trick. Three
//! steps, all of this game:
//!
//! 1. the stick's circular gate is opened to a square, so a stick at the rim on a diagonal (steering and leaning at once) gives
//!    the lever its end;
//! 2. the pad's usual dead zone is taken off and the rest rescaled;
//! 3. the lever reads its end ([`REACH`]) before the stick is at the end of its travel, because a thumb rarely holds a stick
//!    at exactly 1.0 and the stoppie help starts at 0.95 of the lever.

use crate::settings::Deadzone;

/// The share of the stick's travel past its dead zone at which the lever reads 1 (a modelling choice of this game: the largest
/// share a thumb holds steadily on common pads; 0.9 puts the lever's 0.95 at 0.86 of the raw travel and its 0.75 at 0.72).
pub(super) const REACH: f32 = 0.9;

/// The lever for a stick at (`x`, `y`): `y` is the axis bound to the weight, `x` the other axis of the same stick.
pub(super) fn lever(y: f32, x: f32) -> f32 {
    (Deadzone::default().apply(square_gate(y, x)) / REACH).clamp(-1.0, 1.0)
}

/// A stick's `y` with its circular gate opened to a square: a stick pushed to the rim at 45 degrees reads `x` and `y` of
/// 0.71 each, and this gives that `y` the full 1, so the weight lever reaches its end while the stick also steers. The
/// vector keeps its direction and its length is divided by the larger of its two components; on an axis it changes
/// nothing, and a stick that is not at the rim is scaled by its radius (never past 1).
fn square_gate(y: f32, x: f32) -> f32 {
    if !y.is_finite() || !x.is_finite() {
        return 0.0;
    }
    let (x, y) = (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0));
    let larger = x.abs().max(y.abs());
    if larger < 1e-6 {
        return 0.0;
    }
    (y * x.hypot(y).min(1.0) / larger).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lever_reaches_its_end_before_the_stick_does_and_keeps_its_sign() {
        assert_eq!(lever(0.0, 0.0), 0.0);
        assert_eq!(lever(1.0, 0.0), 1.0);
        assert_eq!(lever(-1.0, 0.0), -1.0);
        let dead = f32::from(Deadzone::default().percent()) / 100.0;
        let at_reach = dead + REACH * (1.0 - dead);
        assert!((lever(at_reach, 0.0) - 1.0).abs() < 1e-5, "{}", lever(at_reach, 0.0));
        assert!((lever(-at_reach, 0.0) + 1.0).abs() < 1e-5);
        assert!(lever(at_reach - 0.02, 0.0) < 1.0);
    }

    #[test]
    fn the_help_thresholds_are_reachable_without_a_full_push() {
        // The stoppie help starts at 0.95 of the lever and the auto-rider lets go at 0.75.
        let dead = f32::from(Deadzone::default().percent()) / 100.0;
        let stick_for = |lever_value: f32| dead + lever_value * REACH * (1.0 - dead);
        assert!(stick_for(0.95) < 0.9, "{}", stick_for(0.95));
        assert!(stick_for(0.75) < 0.75, "{}", stick_for(0.75));
        assert!(lever(stick_for(0.95) + 0.001, 0.0) > 0.95);
        assert!(lever(stick_for(0.75) + 0.001, 0.0) > 0.75);
    }

    #[test]
    fn a_stick_that_is_not_a_number_gives_nothing() {
        assert_eq!(lever(f32::NAN, 0.0), 0.0);
        assert_eq!(lever(0.5, f32::INFINITY), 0.0);
    }
}
