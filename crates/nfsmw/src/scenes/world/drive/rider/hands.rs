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
/// The blueprint's `BikeType` of a scooter: its left hand works the rear brake lever.
pub const SCOOTER_TYPE: u8 = 3;
/// The lever from which a pull of the clutch is a pull the rider's hand makes (a gear change's, a start's), and how long such a pull is
/// shown at least (s). A choice: the physics' gear change lets the lever go again within a tenth of a second, which the left hand cannot
/// be seen to do (a user's playtest: the fingers showed at a start in first, not at a change); the hold is the length of the blueprint's
/// own clutch window, `Hand_Clutch_Explicit` of `Hands_FingersOverride(0, 0.28, ..)`. A pull that lasts longer is not held on.
const CLUTCH_PULL: f32 = 0.3;
const CLUTCH_HOLD_S: f32 = 0.28;

/// The pull of a clutch lever that is held for [`CLUTCH_HOLD_S`] when it is shorter: the first frame the lever passes [`CLUTCH_PULL`] starts
/// the hold, the lever shown is the highest it reached until the hold is over, then the input itself.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Pull {
    age: f32,
    peak: f32,
    on: bool,
}

impl Pull {
    fn step(&mut self, lever: f32, dt: f32) -> f32 {
        if !self.on && lever < CLUTCH_PULL {
            return lever;
        }
        if !self.on {
            *self = Self { age: 0.0, peak: lever, on: true };
        }
        self.age += dt;
        self.peak = self.peak.max(lever);
        let held = self.age < CLUTCH_HOLD_S;
        self.on = held || lever >= CLUTCH_PULL;
        match held {
            true => self.peak,
            false => lever,
        }
    }
}

/// What the rider's hands are asked for: the levers and the throttle, each 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Levers {
    /// The clutch lever (the left hand).
    pub clutch: f32,
    /// The front brake (the right hand).
    pub front: f32,
    /// The rear brake lever of a bike that has one (a scooter's left hand); 0 for a bike whose rear brake is a pedal.
    pub rear: f32,
    pub throttle: f32,
}

/// What the graph and the right grip read.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Signals {
    /// The clutch lever the left fingers show (0 let go, 1 pulled in) and the weight of their layer.
    pub clutch: f32,
    pub clutch_alpha: f32,
    /// The front brake lever the right fingers show and the weight of their layer.
    pub brake: f32,
    pub brake_alpha: f32,
    /// The rear brake lever the left fingers show on a bike that has one, and the weight of their layer.
    pub rear: f32,
    pub rear_alpha: f32,
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
    rear: Constant,
    rear_alpha: Constant,
    pull: Pull,
    wrist: Interpolate,
}

impl Hands {
    /// Advance by `dt` seconds with the levers and the throttle at `levers`. Nothing here depends on the gear, the speed or whether the bike
    /// is moving: each hand follows its own lever (a user's playtest: the fingers must follow the inputs at any time).
    pub fn step(&mut self, levers: Levers, dt: f32) -> Signals {
        let pulled = self.pull.step(levers.clutch.clamp(0.0, 1.0), dt);
        let clutch = self.clutch.step(pulled, CLUTCH_RATE, dt);
        let brake = self.brake.step(levers.front.clamp(0.0, 1.0), BRAKE_RATE, dt);
        let rear = self.rear.step(levers.rear.clamp(0.0, 1.0), BRAKE_RATE, dt);
        Signals {
            clutch,
            clutch_alpha: self.clutch_alpha.step(f32::from(clutch > 0.0), CLUTCH_ALPHA_RATE, dt),
            brake,
            brake_alpha: self.brake_alpha.step(f32::from(brake > 0.0), BRAKE_ALPHA_RATE, dt),
            rear,
            rear_alpha: self.rear_alpha.step(f32::from(rear > 0.0), BRAKE_ALPHA_RATE, dt),
            wrist: self.wrist.step(levers.throttle.clamp(0.0, 1.0), WRIST_RATE, dt),
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
            out = hands.step(Levers { clutch: inputs[0], front: inputs[1], rear: 0.0, throttle: inputs[2] }, DT);
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
        // The lever's clip time is 0.27 s when pulled, so the lever goes at 9.3 a second: three frames (0.05 s) take it 0.46 of the way.
        let out = run(&mut hands, [1.0, 0.0, 0.0], 0.05);
        assert!((out.clutch - 0.463).abs() < 0.01, "{out:?}");
        let out = run(&mut Hands::default(), [1.0, 0.0, 0.0], 0.12);
        assert_eq!(out.clutch, 1.0, "a gear change's tenth of a second pulls the lever all the way in");
        let out = run(&mut hands, [1.0, 0.0, 0.0], 0.2);
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
    fn a_short_pull_of_the_clutch_is_shown_for_the_blueprints_window_and_a_long_one_is_not_held_on() {
        // A gear change: the lever is in for 0.05 s. The fingers close all the way and stay closed for the hold, then open.
        let mut hands = Hands::default();
        let pulled = run(&mut hands, [1.0, 0.0, 0.0], 0.05);
        assert!(pulled.clutch > 0.4, "{pulled:?}");
        let held = run(&mut hands, [0.0; 3], 0.15);
        assert_eq!(held.clutch, 1.0, "still closed 0.2 s after the change began: {held:?}");
        assert!(held.clutch_alpha > 0.95, "{held:?}");
        let open = run(&mut hands, [0.0; 3], 0.5);
        assert_eq!((open.clutch, open.clutch_alpha), (0.0, 0.0), "and open again: {open:?}");
        // Another change later is shown again.
        assert!(run(&mut hands, [1.0, 0.0, 0.0], 0.05).clutch > 0.4);
        // A launch's long pull lets go with the lever, not 0.28 s after it.
        let mut launch = Hands::default();
        run(&mut launch, [1.0, 0.0, 0.0], 1.0);
        run(&mut launch, [0.0; 3], 0.01);
        let after = run(&mut launch, [0.0; 3], 0.3);
        assert_eq!(after.clutch, 0.0, "{after:?}");
        // A light touch of the lever (under the pull) is shown as it is, and is not held.
        let mut light = Hands::default();
        run(&mut light, [0.2, 0.0, 0.0], 0.1);
        assert_eq!(run(&mut light, [0.0; 3], 0.3).clutch, 0.0);
    }

    #[test]
    fn each_lever_has_its_own_hand_and_none_waits_for_another() {
        let mut hands = Hands::default();
        // The rear lever (a scooter's left hand) and the front one (the right), pulled together, and the throttle open.
        let both = (0..120)
            .map(|_| hands.step(Levers { clutch: 0.0, front: 0.6, rear: 1.0, throttle: 0.4 }, DT))
            .last()
            .unwrap();
        assert!((both.brake - 0.6).abs() < 1e-6 && both.brake_alpha == 1.0, "{both:?}");
        assert!((both.rear - 1.0).abs() < 1e-6 && both.rear_alpha == 1.0, "{both:?}");
        assert!((both.wrist - 0.4).abs() < 0.001, "{both:?}");
        assert_eq!((both.clutch, both.clutch_alpha), (0.0, 0.0));
        let rear_only = run_levers(&mut Hands::default(), Levers { rear: 1.0, ..Levers::default() }, 1.0);
        assert_eq!((rear_only.brake_alpha, rear_only.clutch_alpha, rear_only.rear_alpha), (0.0, 0.0, 1.0));
    }

    fn run_levers(hands: &mut Hands, levers: Levers, seconds: f32) -> Signals {
        (0..(seconds / DT).round() as usize).map(|_| hands.step(levers, DT)).last().unwrap_or_default()
    }

    #[test]
    fn inputs_out_of_range_are_held_to_the_levers() {
        let out = run(&mut Hands::default(), [5.0, -3.0, 9.0], 5.0);
        assert_eq!((out.clutch, out.brake), (1.0, 0.0));
        assert!(out.wrist <= 1.0);
    }
}
