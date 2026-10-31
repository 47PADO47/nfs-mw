//! The two signals that pick which lean pose the rider shows (`docs/specs/ride6-rider-knee-foot.md`, section 2): the blueprint's
//! `LeaningAnimations` function ("leaning Out and In turn anim").
//!
//! The lean clips of the animation (`race_Rossi_standard_Left`, `motard_Leaning_Knee_LowSpeed_R`, ...) are **sweeps** of 2 s: the
//! blueprint evaluates the left clip at `1 - RiderLeaning` seconds and the right one at `1 + RiderLeaning` seconds, so frame 0 is
//! the rider hung off to one side with the knee out, frame 30 is upright and the last frame is hung off to the other side. Both
//! clips cover the whole range; they differ in how the body travels between the ends. `RiderLean_LeftOrRight` (here `out`) crosses
//! from the left clip to the right one while the bike rolls to the right fast, and `LeanOrStraight_Alpha` (`straight`) crosses to
//! the plain upright pose while the bike is nearly upright and the stick is neutral.

use super::script::{Interpolate, ramp, trapezoid};

/// `RiderLean_DerivativeReturnTreshold`: the rate of the bike's lean (a share of its steepest lean per second) above which the
/// right clip is the one shown.
const OUT_DERIVATIVE: f32 = 0.15;
/// The rate at which `RiderLean_LeftOrRight` follows (`DefuzzifyFloatEx` of two rules on `BikeType`, default 4): 5 for the race
/// and naked types (0 and 1), 6 for the bagger (5).
const OUT_RATE_DEFAULT: f32 = 4.0;
const OUT_RATE_ROAD: f32 = 5.0;
const OUT_RATE_BAGGER: f32 = 6.0;
/// `FuzzifyTrapezoid(BikeLean, -0.25, -0.12, 0.12, 0.25)`: the bike is upright (1) below 12 percent of its steepest lean and leaning
/// (0) beyond 25 percent.
const UPRIGHT_BAND: (f32, f32, f32, f32) = (-0.25, -0.12, 0.12, 0.25);
/// `RiderLean_AeroPose_Speed`: the rate at which `LeanOrStraight_Alpha` follows.
const STRAIGHT_RATE: f32 = 2.0;
/// `RiderLean_Min_DecaySpeed` and `RiderLean_Max_DecaySpeed` (km/h): `RiderLean_SpeedReducerCondition`, the share of the speed
/// range in which the rider stops leaning in his pose and stays upright and tucked.
const SPEED_REDUCER: (f32, f32) = (250.0, 400.0);

/// What the graph reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Signals {
    /// `RiderLean_LeftOrRight`: 0 the left clip, 1 the right one.
    pub out: f32,
    /// `LeanOrStraight_Alpha`: 1 the upright pose, 0 the lean sweep.
    pub straight: f32,
}

impl Default for Signals {
    /// The variables' defaults in the blueprint: the left clip, and the upright pose (`LeanOrStraight_Alpha = 1`).
    fn default() -> Self {
        Self { out: 0.0, straight: 1.0 }
    }
}

/// The two smoothers and the bike's lean of the step before (for its derivative).
#[derive(Debug, Clone, PartialEq)]
pub struct LeanPose {
    out: Interpolate,
    straight: Interpolate,
    previous: Option<f32>,
    bike_type: Option<u8>,
}

impl Default for LeanPose {
    fn default() -> Self {
        Self { out: Interpolate::default(), straight: Interpolate::at(1.0), previous: None, bike_type: None }
    }
}

impl LeanPose {
    /// The rider's bike type (`BikeType` of the blueprint) sets the rate of `out`; `None` is the default one.
    pub fn set_bike_type(&mut self, bike_type: Option<u8>) {
        self.bike_type = bike_type;
    }

    fn out_rate(&self) -> f32 {
        match self.bike_type {
            Some(0 | 1) => OUT_RATE_ROAD,
            Some(5) => OUT_RATE_BAGGER,
            _ => OUT_RATE_DEFAULT,
        }
    }

    /// Advance by `dt` seconds with the bike's lean `bike_lean` (-1 to 1, to the right), the stick `steer` (-1 to 1) and the
    /// speed in km/h.
    pub fn step(&mut self, bike_lean: f32, steer: f32, speed_kmh: f32, dt: f32) -> Signals {
        let derivative = match self.previous {
            Some(before) if dt > 0.0 => (bike_lean - before) / dt,
            _ => 0.0,
        };
        self.previous = Some(bike_lean);
        let out = self.out.step(f32::from(derivative > OUT_DERIVATIVE), self.out_rate(), dt);
        let upright = trapezoid(bike_lean, UPRIGHT_BAND) * (1.0 - steer.abs().clamp(0.0, 1.0));
        let fast = ramp(speed_kmh, SPEED_REDUCER.0, SPEED_REDUCER.1);
        let straight = self.straight.step(upright.max(fast), STRAIGHT_RATE, dt);
        Signals { out, straight }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn an_upright_bike_with_a_neutral_stick_shows_the_upright_pose_and_a_leaning_one_the_sweep() {
        let mut pose = LeanPose::default();
        let held = (0..120).map(|_| pose.step(0.0, 0.0, 100.0, DT)).last().unwrap();
        assert!(held.straight > 0.99 && held.out == 0.0, "{held:?}");
        let leaning = (0..240).map(|_| pose.step(0.6, 0.0, 100.0, DT)).last().unwrap();
        assert!(leaning.straight < 0.01, "{leaning:?}");
    }

    #[test]
    fn the_stick_pulls_the_rider_out_of_the_upright_pose_before_the_bike_has_rolled() {
        let mut pose = LeanPose::default();
        let pushed = (0..240).map(|_| pose.step(0.0, 1.0, 100.0, DT)).last().unwrap();
        assert!(pushed.straight < 0.01, "{pushed:?}");
    }

    #[test]
    fn a_fast_roll_to_the_right_brings_in_the_right_clip_and_a_steady_lean_lets_it_go() {
        let mut pose = LeanPose::default();
        // 0.5 of the lean in a third of a second is 1.5 per second, over the 0.15 limit.
        let mut lean = 0.0;
        let mut out = 0.0;
        for _ in 0..20 {
            lean += 0.025;
            out = pose.step(lean, 1.0, 100.0, DT).out;
        }
        assert!(out > 0.5, "{out}");
        let held = (0..120).map(|_| pose.step(lean, 1.0, 100.0, DT).out).last().unwrap();
        assert!(held < 0.01, "{held}");
        // Rolling to the left is not the right clip's business.
        let mut pose = LeanPose::default();
        let mut lean = 0.0;
        for _ in 0..20 {
            lean -= 0.025;
            assert_eq!(pose.step(lean, -1.0, 100.0, DT).out, 0.0);
        }
    }

    #[test]
    fn the_signal_follows_at_the_rate_of_the_bike_type() {
        let rise = |bike_type: Option<u8>| {
            let mut pose = LeanPose::default();
            pose.set_bike_type(bike_type);
            let mut lean = 0.0;
            let mut out = 0.0;
            for _ in 0..6 {
                lean += 0.025;
                out = pose.step(lean, 1.0, 100.0, DT).out;
            }
            out
        };
        assert!(rise(Some(0)) > rise(None), "race 5 against 4 a second");
        assert!(rise(Some(5)) > rise(Some(1)), "bagger 6 against 5");
        assert_eq!(rise(Some(1)), rise(Some(0)));
    }

    #[test]
    fn a_fast_bike_stays_upright_in_the_pose() {
        let mut pose = LeanPose::default();
        let fast = (0..240).map(|_| pose.step(0.9, 1.0, 400.0, DT)).last().unwrap();
        assert!(fast.straight > 0.99, "{fast:?}");
    }
}
