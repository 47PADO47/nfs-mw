//! The rider's hands on the controls: the fingers pull the clutch and the front brake levers, and the right wrist rolls the throttle
//! (`docs/specs/ride6-rider-hands.md`).
//!
//! The animation blueprint's `Hands_FingersOverride` makes `Fingers_Clutch_Explicit` (a time in the clutch clip, 0.27 s at a fully pulled
//! lever, followed at `FingersClutchExplicitSpeed`, 2.5 clip seconds a second) and `Fingers_FrontBrake_Explicit` (`BikeFrontBrake`
//! followed at 3.5 a second), and
//! the weights of the finger layers over the riding pose: `Fingers_Clutch_Alpha` and `FrontFingers_Brake_Alpha` are 1 while the
//! lever is off its stop, followed at 7 and 10 a second. `HandsBehaviours_(Leaning,_UpAndDown)` makes the right wrist's weight
//! `Hand_R_WristDown` from `VehicleThrottle` and follows the resulting offset at `Hand_R_TransformSpeed`, 4 a second.

use super::script::{Constant, Interpolate};

/// `FingersClutchExplicitSpeed`: the rate at which the clutch fingers' time in the clip follows the lever, in clip seconds a second. The
/// time of a fully pulled lever is 0.27 s, so the lever (0 to 1) is followed at 2.5 / 0.27 = 9.3 a second: a pull of a tenth of a
/// second, a gear change's, takes the fingers all the way in. (The first version followed the lever at 2.5 a second as if the speed
/// were in lever units: 0.4 s to close, which a gear change's clutch never lasts, so the fingers hardly moved.)
const CLUTCH_SPEED: f32 = 2.5;
const CLUTCH_PULLED_SECONDS: f32 = 0.27;
const CLUTCH_RATE: f32 = CLUTCH_SPEED / CLUTCH_PULLED_SECONDS;
/// The rate of the clutch layer's weight (`SmoothFloat_Constant(.., 7)`) and of the brake layer's (`.., 10`).
const CLUTCH_ALPHA_RATE: f32 = 7.0;
const BRAKE_ALPHA_RATE: f32 = 10.0;
/// The rate at which the brake fingers follow `BikeFrontBrake` (`Fingers_FrontBrake_Condition`, `SmoothFloat_Constant(.., 3.5)`).
const BRAKE_RATE: f32 = 3.5;
/// `Hand_R_TransformSpeed`: the rate at which the wrist offset follows (a second).
const WRIST_RATE: f32 = 4.0;

/// What the graph and the right grip read.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Signals {
    /// The clutch lever the left fingers show (0 let go, 1 pulled in) and the weight of their layer.
    pub clutch: f32,
    pub clutch_alpha: f32,
    /// The front brake lever the right fingers show and the weight of their layer.
    pub brake: f32,
    pub brake_alpha: f32,
    /// How far the right wrist is rolled (0 closed on the grip's rest, 1 the throttle full open).
    pub wrist: f32,
}

/// The state of the hands' smoothers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hands {
    clutch: Constant,
    clutch_alpha: Constant,
    brake: Constant,
    brake_alpha: Constant,
    wrist: Interpolate,
}

impl Hands {
    /// Advance by `dt` seconds with the clutch lever pulled `clutch` (0 to 1), the front brake `front_brake` and the throttle open
    /// `throttle`.
    pub fn step(&mut self, clutch: f32, front_brake: f32, throttle: f32, dt: f32) -> Signals {
        let clutch = self.clutch.step(clutch.clamp(0.0, 1.0), CLUTCH_RATE, dt);
        let brake = self.brake.step(front_brake.clamp(0.0, 1.0), BRAKE_RATE, dt);
        Signals {
            clutch,
            clutch_alpha: self.clutch_alpha.step(f32::from(clutch > 0.0), CLUTCH_ALPHA_RATE, dt),
            brake,
            brake_alpha: self.brake_alpha.step(f32::from(brake > 0.0), BRAKE_ALPHA_RATE, dt),
            wrist: self.wrist.step(throttle.clamp(0.0, 1.0), WRIST_RATE, dt),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn run(hands: &mut Hands, inputs: [f32; 3], seconds: f32) -> Signals {
        let mut out = Signals::default();
        for _ in 0..(seconds / DT).round() as usize {
            out = hands.step(inputs[0], inputs[1], inputs[2], DT);
        }
        out
    }

    #[test]
    fn hands_at_rest_show_nothing() {
        assert_eq!(run(&mut Hands::default(), [0.0; 3], 1.0), Signals::default());
    }

    #[test]
    fn the_clutch_fingers_follow_the_lever_at_two_and_a_half_clip_seconds_a_second_and_the_layer_comes_in_at_once() {
        let mut hands = Hands::default();
        // The lever's clip time is 0.27 s when pulled: half of it in 0.054 s, all of it in 0.108 s.
        let out = run(&mut hands, [1.0, 0.0, 0.0], 0.054);
        assert!((out.clutch - 0.5).abs() < 0.02, "{out:?}");
        let out = run(&mut Hands::default(), [1.0, 0.0, 0.0], 0.12);
        assert_eq!(out.clutch, 1.0, "a gear change's tenth of a second pulls the lever all the way in");
        assert_eq!(out.clutch_alpha, 1.0, "the layer is up while the lever is off its stop");
        assert_eq!(out.brake_alpha, 0.0);
        let out = run(&mut hands, [1.0, 0.0, 0.0], 1.0);
        assert_eq!(out.clutch, 1.0);
        let out = run(&mut hands, [0.0; 3], 1.0);
        assert_eq!((out.clutch, out.clutch_alpha), (0.0, 0.0), "let go: the layer is out again");
    }

    #[test]
    fn the_brake_fingers_are_the_right_hands_and_follow_at_three_and_a_half() {
        let out = run(&mut Hands::default(), [0.0, 1.0, 0.0], 0.1);
        assert!((out.brake - 0.35).abs() < 0.02, "{out:?}");
        assert_eq!((out.brake_alpha, out.clutch_alpha), (1.0, 0.0));
    }

    #[test]
    fn the_wrist_rolls_with_the_throttle_and_follows_it_smoothly() {
        let mut hands = Hands::default();
        let early = run(&mut hands, [0.0, 0.0, 1.0], 0.1);
        assert!(early.wrist > 0.2 && early.wrist < 0.5, "{early:?}");
        let held = run(&mut hands, [0.0, 0.0, 1.0], 3.0);
        assert!(held.wrist > 0.999, "{held:?}");
        let back = run(&mut hands, [0.0; 3], 3.0);
        assert!(back.wrist < 0.001);
    }

    #[test]
    fn inputs_out_of_range_are_held_to_the_levers() {
        let out = run(&mut Hands::default(), [5.0, -3.0, 9.0], 5.0);
        assert_eq!((out.clutch, out.brake), (1.0, 0.0));
        assert!(out.wrist <= 1.0);
    }
}
