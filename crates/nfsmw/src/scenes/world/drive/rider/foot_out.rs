//! The inside foot comes off the peg in a lean (`docs/specs/ride6-rider-knee-foot.md`, section 4): the blueprint's
//! `IK Overrides and Smoothing` function.
//!
//! With the old-school riding style (`RidingStyle == 2`) on a supermoto or an enduro bike (`BikeType` 2 or 4) the function sets
//! `Leaning_FootDown_L` while the bike's lean is beyond -0.3 and `Leaning_FootDown_R` beyond +0.3 (a share of its steepest lean),
//! and follows each at `LeanFootDwn_Speed` (4 a second) with `SmoothFloat_Constant` into `OverrideFootIK_Leaning_L/R`. The foot IK
//! output (`LeftFoot_IK`, `RightFoot_IK`) is multiplied by `1 - OverrideFootIK_Leaning`: the inside foot is no longer pinned to the
//! peg and stays where the lean clip puts it, which in the old-school clips (`motard_Old_School_*`) is 19 cm out and 12 to 17 cm up.

use super::script::Constant;

/// `Leaning_FootDown_*`: the share of the bike's steepest lean beyond which the inside foot is put down.
const FOOT_DOWN_LEAN: f32 = 0.3;
/// `LeanFootDwn_Speed` for every type but the scooter (6): the rate at which the override follows, a second.
const FOOT_DOWN_RATE: f32 = 4.0;
/// The animation blueprint's `RidingStyle` of the old-school clips.
const OLD_SCHOOL: u8 = 2;
/// `BikeType` of the supermoto and of the enduro bike.
const FOOT_OUT_TYPES: [u8; 2] = [2, 4];

/// Whether a bike of this type and riding style takes its inside foot off the peg. Unknown (a graph that does not say) is no.
pub fn applies(bike_type: Option<u8>, riding_style: Option<u8>) -> bool {
    riding_style == Some(OLD_SCHOOL) && bike_type.is_some_and(|t| FOOT_OUT_TYPES.contains(&t))
}

/// The two overrides, left and right.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FootOut {
    enabled: bool,
    left: Constant,
    right: Constant,
}

impl FootOut {
    /// Turn the inside foot's release on or off (the player changed the riding style); a foot already off its peg goes back.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Advance by `dt` seconds with the bike's lean `bike_lean` (-1 to 1, to the right): how far each foot is let off its peg,
    /// left and right, 0 to 1.
    pub fn step(&mut self, bike_lean: f32, dt: f32) -> [f32; 2] {
        if !self.enabled {
            return [0.0; 2];
        }
        let left = self.left.step(f32::from(bike_lean < -FOOT_DOWN_LEAN), FOOT_DOWN_RATE, dt);
        let right = self.right.step(f32::from(bike_lean > FOOT_DOWN_LEAN), FOOT_DOWN_RATE, dt);
        [left, right]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn enabled() -> FootOut {
        let mut foot = FootOut::default();
        foot.set_enabled(true);
        foot
    }

    fn run(foot: &mut FootOut, lean: f32, seconds: f32) -> [f32; 2] {
        (0..(seconds / DT).round() as usize).map(|_| foot.step(lean, DT)).last().unwrap()
    }

    #[test]
    fn only_the_old_school_style_of_a_supermoto_or_an_enduro_bike_puts_the_foot_out() {
        assert!(applies(Some(2), Some(2)) && applies(Some(4), Some(2)));
        assert!(
            !applies(Some(2), Some(0)) && !applies(Some(2), Some(1)),
            "the knee and the plain styles keep the foot"
        );
        assert!(!applies(Some(0), Some(2)) && !applies(Some(1), Some(2)), "a road bike keeps it in every style");
        assert!(!applies(None, Some(2)) && !applies(Some(2), None));
    }

    #[test]
    fn the_inside_foot_comes_off_beyond_30_percent_of_the_lean_and_goes_back_at_4_a_second() {
        let mut foot = enabled();
        assert_eq!(run(&mut foot, 0.29, 1.0), [0.0, 0.0]);
        // To the right: the right foot, in a quarter of a second.
        let [left, right] = run(&mut foot, 0.31, 0.125);
        assert_eq!(left, 0.0);
        assert!((right - 0.5).abs() < 0.07, "{right}");
        assert_eq!(run(&mut foot, 0.6, 1.0), [0.0, 1.0]);
        // The bike straightens: the foot goes back to the peg at the same rate, not at once.
        let [_, back] = run(&mut foot, 0.1, 0.125);
        assert!((back - 0.5).abs() < 0.07, "{back}");
        let [left, right] = run(&mut foot, 0.1, 1.0);
        assert!(left == 0.0 && right < 1e-5, "{right}");
        // To the left: the left foot.
        let [left, right] = run(&mut foot, -0.8, 1.0);
        assert!(left == 1.0 && right < 1e-5, "{left} {right}");
    }

    #[test]
    fn a_bike_that_does_not_take_the_foot_out_never_releases_it() {
        let mut foot = FootOut::default();
        assert_eq!(run(&mut foot, 1.0, 1.0), [0.0, 0.0]);
    }

    #[test]
    fn the_override_never_moves_faster_than_its_rate() {
        let mut foot = enabled();
        let mut before = [0.0; 2];
        for lean in (0..600).map(|n| if (n / 100) % 2 == 0 { 0.9 } else { -0.9 }) {
            let now = foot.step(lean, DT);
            assert!((now[0] - before[0]).abs() <= FOOT_DOWN_RATE * DT + 1e-6);
            assert!((now[1] - before[1]).abs() <= FOOT_DOWN_RATE * DT + 1e-6);
            before = now;
        }
    }
}
