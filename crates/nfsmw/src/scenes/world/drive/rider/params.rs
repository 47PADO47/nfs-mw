//! The numbers that drive the rider's animation, from the bike's state (`docs/specs/ride6-rider-animation.md`, section 5,
//! and `ride6-rider-round6.md`, section 4). The graph's parameters are `lean`, `speed_kmh`, `tuck`, `wheelie`, `stoppie` and
//! three more for the wheelie and stoppie; the rider's body also shifts forward and back with the bike's acceleration.
//!
//! The signals are the ones the animation blueprint computes from the bike's inputs, with its constants (`Rider_Lean_Polish`,
//! `TuckedIn_Alpha`, `Wheelie/ Stoppie`), in the script language of `script.rs`.

use glam::Vec3;

use super::attach::Inertia;
use super::script::{Constant, Flashback, Interpolate, defuzzify, fuzzy_not, fuzzy_sum, ramp};

/// A slower step than this (s) does not move the filters: it is a pause, not time.
const MAX_STEP: f32 = 0.1;

/// `Rider_Lean_Polish(250, 400, 0.3, 0.4, 6, 0.5, 4, 0.4, 2, 3, 4, BikeLeanWeight, ...)`: above 250 km/h the lean fades to 0.3
/// of itself at 400, the lean is smoothed at 4 (the default; 2, 3 or 4 for a bike weight class 1 to 3, a variable the data
/// leaves at 0), and the multiplier at 2.2.
const LEAN_DECAY_SPEED: (f32, f32) = (250.0, 400.0);
const LEAN_DECAY: f32 = 0.3;
const LEAN_RATE: f32 = 4.0;
const LEAN_MULTIPLIER_RATE: f32 = 2.2;
/// The rider's own lean and the bike's, as the blueprint mixes them (0.3 and 0.7), and the mix at which the polished lean is
/// full (0.85); the polished lean is also faded in with the speed over -10 to 50 km/h.
const LEAN_MIX: (f32, f32) = (0.3, 0.7);
const LEAN_FULL: f32 = 0.85;
const LEAN_SPEED_FADE: (f32, f32) = (-10.0, 50.0);
/// `RiderLean_TerrainType_Multiplier` on tarmac (the surface the game rides on): the default of the rule, 1.
const TERRAIN_MULTIPLIER: f32 = 1.0;
/// `TuckedIn_Alpha`: the rates of the smoothers (the tuck input 6, the held throttle 1, the terrain 3, the brake 4) and the
/// time the throttle must have been held (0.2 s).
const TUCK_RATES: [f32; 4] = [6.0, 1.0, 3.0, 4.0];
const TUCK_THROTTLE_HOLD: f32 = 0.2;
/// `Wheelie/ Stoppie(0.15, 0.5, 0.12, 0.2, 0.2, 2.5, 3, 3)`: the range of the wheelie pose, its delay and smoothing, and the
/// stoppie smoothing; inside, the entry takes 8 per second, the stoppie clip's time is delayed 0.15 s and smoothed at 3.
const WHEELIE_RANGE: (f32, f32) = (0.15, 0.5);
const WHEELIE_DELAY: f32 = 0.2;
const WHEELIE_RATE: f32 = 2.5;
const WHEELIE_ENTRY_RATE: f32 = 8.0;
const STOPPIE_RATE: f32 = 3.0;
const STOPPIE_CLIP_DELAY: f32 = 0.15;
/// `BikeStoppie` above which the stoppie value goes to 1 (it goes to 0 at or below 0; between them it holds).
const STOPPIE_ON: f32 = 0.18;
/// `Wheelie_Direction`: the bike's lean beyond which the wheelie pose leans to a side (0.05 to 0.2).
const WHEELIE_SIDE: (f32, f32) = (0.05, 0.2);

/// What the animation needs of the bike at one moment. The bike's physics make it (`CarSim::rider_sample`); the fields
/// are in radians, metres per second and units of 0 to 1.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RiderSample {
    /// The rider's lean from upright (the bike's plus his body's own against it), positive to the right.
    pub lean: f32,
    /// The steepest lean the bike model allows.
    pub max_lean: f32,
    /// The bike's roll and the controller's target lean, over `max_lean` (-1 to 1, positive to the right): the blueprint's
    /// `BikeLean` and `RiderLean` inputs.
    pub bike_lean: f32,
    pub rider_lean: f32,
    /// Speed along the bike.
    pub speed: f32,
    pub throttle: f32,
    pub front_brake: f32,
    pub rear_brake: f32,
    /// The pitch of the bike's nose above the horizon, and the height of its forward axis above the horizontal (the sine of
    /// the pitch): the blueprint's `BikePitch` is the second.
    pub pitch_up: f32,
    pub elevation: f32,
    /// The tuck level of the rider (`Bike::tuck_level`): the blueprint's `TuckedIn`.
    pub tuck_level: f32,
    pub front_down: bool,
    pub rear_down: bool,
    /// Where the rider's centre of mass is from his rest place, in the bike's physics axes (x right, y up, z forward), metres:
    /// the place the weight, the lean and the speed ask for plus how far his spring has let him move.
    pub offset: Vec3,
    /// His body against the bike's attitude: roll to the right, pitch nose up and yaw to the right, radians.
    pub rotation: Vec3,
}

impl RiderSample {
    pub fn speed_kmh(&self) -> f32 {
        self.speed.max(0.0) * 3.6
    }

    /// The harder of the two brake levers, 0 to 1.
    pub fn braking(&self) -> f32 {
        self.front_brake.max(self.rear_brake).clamp(0.0, 1.0)
    }

    /// The blueprint's `BikeWheelie`: the height of the nose (the sine of the pitch) while the front tyre is off the ground
    /// and the rear one is on it, else 0.
    pub fn wheelie(&self) -> f32 {
        match !self.front_down && self.rear_down {
            true => self.elevation.max(0.0),
            false => 0.0,
        }
    }

    /// The blueprint's `BikeStoppie`: the depth of the nose while the rear tyre is off the ground and the front one is on it.
    pub fn stoppie(&self) -> f32 {
        match self.front_down && !self.rear_down {
            true => (-self.elevation).max(0.0),
            false => 0.0,
        }
    }
}

/// The values the graph's parameters take.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Values {
    /// `RiderLeaning`: -1 (left) to 1 (right).
    pub lean: f32,
    pub speed_kmh: f32,
    /// `RiderTuckedIn_Alpha`.
    pub tuck: f32,
    /// `BikeWheelie` held to 8 per second (the entry of the wheelie pose), and `Wheelie_Alpha` (the minimum-to-maximum pose).
    pub wheelie: f32,
    pub wheelie_alpha: f32,
    /// `Wheelie_Direction`, -1 to 1: which side the wheelie pose leans to.
    pub wheelie_direction: f32,
    /// `Stoppie_Alpha`, and the clip's time (`Stoppie_Explicit` over 1.27 s).
    pub stoppie: f32,
    pub stoppie_time: f32,
}

/// The state of the blueprint's smoothers and delay lines.
#[derive(Debug, Clone, Default, PartialEq)]
struct Script {
    lean: Interpolate,
    lean_multiplier: Interpolate,
    tuck_input: Interpolate,
    tuck_throttle: Interpolate,
    tuck_terrain: Interpolate,
    tuck_brake: Interpolate,
    throttle_before: Flashback,
    wheelie_entry: Constant,
    wheelie_delay: Flashback,
    wheelie_alpha: Interpolate,
    stoppie_value: f32,
    stoppie_alpha: Constant,
    stoppie_delay: Flashback,
    stoppie_time: Constant,
}

impl Script {
    /// `Rider_Lean_Polish`: the lean of the rider's animation, -1 to 1.
    fn lean(&mut self, sample: &RiderSample, speed: f32, dt: f32) -> f32 {
        let mix = LEAN_MIX.0 * sample.rider_lean + LEAN_MIX.1 * sample.bike_lean;
        let fade = ramp(speed, LEAN_SPEED_FADE.0, LEAN_SPEED_FADE.1);
        let polished = defuzzify(&[(ramp(mix, 0.0, LEAN_FULL), fade), (ramp(mix, 0.0, -LEAN_FULL), -fade)], 0.0);
        let smooth = self.lean.step(polished, LEAN_RATE, dt);
        let decay = ramp(speed, LEAN_DECAY_SPEED.0, LEAN_DECAY_SPEED.1);
        let wanted = TERRAIN_MULTIPLIER * defuzzify(&[(decay, LEAN_DECAY)], 1.0);
        smooth * self.lean_multiplier.step(wanted, LEAN_MULTIPLIER_RATE, dt)
    }

    /// `TuckedIn_Alpha`: the tuck of the rider's animation, 0 to 1.
    fn tuck(&mut self, sample: &RiderSample, wheelie_alpha: f32, dt: f32) -> f32 {
        let held = self.throttle_before.step(sample.throttle.clamp(0.0, 1.0), TUCK_THROTTLE_HOLD, dt);
        let on = f32::from(sample.throttle > 0.0) * f32::from(held > 0.0);
        let level = sample.tuck_level.clamp(0.0, 1.0);
        let by_input = self.tuck_input.step(level, TUCK_RATES[0], dt);
        let by_throttle = self.tuck_throttle.step(fuzzy_sum(level, on), TUCK_RATES[1], dt);
        let terrain = self.tuck_terrain.step(TERRAIN_MULTIPLIER.clamp(0.0, 1.0), TUCK_RATES[2], dt);
        let not_braking = self.tuck_brake.step(fuzzy_not(sample.braking()), TUCK_RATES[3], dt);
        by_input.max(by_throttle) * terrain * not_braking * fuzzy_not(wheelie_alpha.clamp(0.0, 1.0))
    }
}

/// `Wheelie_Direction`: the bike's lean to either side beyond a dead band, weighted by itself.
fn wheelie_direction(bike_lean: f32) -> f32 {
    let right = ramp(bike_lean, WHEELIE_SIDE.0, WHEELIE_SIDE.1);
    let left = ramp(bike_lean, -WHEELIE_SIDE.0, -WHEELIE_SIDE.1);
    defuzzify(&[(right, right), (left, -left)], 0.0)
}

/// The inputs after the blueprint's script, and the rider's shift along the bike.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Smoothed {
    values: Values,
    script: Script,
    /// The rider's displacement from his rest place in the car's axes (x forward, y left, z up), metres, after the clamp to the
    /// ranges of the animation and the smoothing.
    shift: Vec3,
    /// His body's turn against the bike's: roll to the right, pitch nose up, yaw to the right (rad).
    rotation: Vec3,
}

impl Smoothed {
    /// Advance by `dt` seconds with the bike in state `sample`; the script carries the blueprint's own rates.
    pub fn step(&mut self, sample: &RiderSample, inertia: &Inertia, dt: f32) {
        if !(dt > 0.0 && dt.is_finite()) {
            return;
        }
        let dt = dt.min(MAX_STEP);
        let speed = sample.speed_kmh();
        let (wheelie, stoppie) = (sample.wheelie(), sample.stoppie());
        let s = &mut self.script;
        let lean = s.lean(sample, speed, dt);
        // `Wheelie_Alpha`: the ramp of the wheelie, delayed and smoothed.
        let ramped = ramp(wheelie, WHEELIE_RANGE.0, WHEELIE_RANGE.1);
        let delayed = s.wheelie_delay.step(ramped, WHEELIE_DELAY, dt);
        let wheelie_alpha = s.wheelie_alpha.step(delayed, WHEELIE_RATE, dt);
        let tuck = s.tuck(sample, wheelie_alpha, dt);
        // `StoppieValue`: 1 above 0.18, 0 at or below 0, and the value it had in between.
        let on = f32::from(stoppie > STOPPIE_ON);
        let off = f32::from(stoppie <= 0.0);
        s.stoppie_value = defuzzify(&[(on, 1.0), (off, 0.0)], s.stoppie_value);
        let delayed = s.stoppie_delay.step(s.stoppie_value, STOPPIE_CLIP_DELAY, dt);
        self.values = Values {
            lean,
            speed_kmh: speed,
            tuck,
            wheelie: s.wheelie_entry.step(wheelie, WHEELIE_ENTRY_RATE, dt),
            wheelie_alpha,
            wheelie_direction: wheelie_direction(sample.bike_lean),
            stoppie: s.stoppie_alpha.step(s.stoppie_value, STOPPIE_RATE, dt),
            stoppie_time: s.stoppie_time.step(delayed, STOPPIE_RATE, dt),
        };
        // The shift: the rider's displacement from the physics, held to the ranges of the animation's inertia input.
        let target = shift_target(sample.offset, inertia);
        self.shift += (target - self.shift) * (inertia.rate * dt).clamp(0.0, 1.0);
        self.rotation = match sample.rotation.is_finite() {
            true => sample.rotation,
            false => Vec3::ZERO,
        };
    }

    pub fn values(&self) -> Values {
        self.values
    }

    /// The rider's displacement from his rest place in the car's axes (x forward, y left, z up), metres.
    pub fn shift(&self) -> Vec3 {
        self.shift
    }

    /// His body's turn against the bike's: roll to the right, pitch nose up, yaw to the right (rad). The physics already
    /// filters it (the rider's body is a spring and a damper), so it is not smoothed again.
    pub fn rotation(&self) -> Vec3 {
        self.rotation
    }
}

/// Where the rider's shift goes for the displacement `offset` of his centre of mass (the bike's physics axes: x right, y up,
/// z forward, metres): the same vector in the car's axes (x forward, y left, z up), held to the range of the animation's
/// inertia input (`inertia.min` to `inertia.max`, the blueprint's `InertiaDisplacement_RidingMin/Max`).
pub fn shift_target(offset: Vec3, inertia: &Inertia) -> Vec3 {
    if !offset.is_finite() {
        return Vec3::ZERO;
    }
    let car = Vec3::new(offset.z, -offset.x, offset.y);
    car.clamp(Vec3::from_array(inertia.min), Vec3::from_array(inertia.max))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inertia() -> Inertia {
        Inertia {
            bone: "pelvis".into(),
            min: [-0.06, -0.15, -0.04],
            max: [0.0, 0.15, 0.2],
            braking_m: 0.0,
            full_acceleration: 0.0,
            rate: 4.0,
        }
    }

    fn sample() -> RiderSample {
        RiderSample { max_lean: 1.0, front_down: true, rear_down: true, ..RiderSample::default() }
    }

    fn run(sample: &RiderSample, seconds: f32) -> Smoothed {
        let mut smoothed = Smoothed::default();
        for _ in 0..(seconds * 60.0) as usize {
            smoothed.step(sample, &inertia(), 1.0 / 60.0);
        }
        smoothed
    }

    #[test]
    fn a_wheelie_needs_the_front_tyre_up_and_the_rear_one_down_and_is_the_height_of_the_nose() {
        let up = RiderSample { elevation: 0.3, ..sample() };
        assert_eq!(up.wheelie(), 0.0, "the front tyre is down");
        let lifted = RiderSample { front_down: false, ..up };
        assert_eq!(lifted.wheelie(), 0.3);
        assert_eq!(RiderSample { elevation: -0.2, ..lifted }.wheelie(), 0.0);
        assert_eq!(RiderSample { rear_down: false, ..lifted }.wheelie(), 0.0, "both off the ground is neither");
        assert_eq!(lifted.stoppie(), 0.0);
    }

    #[test]
    fn a_stoppie_needs_the_rear_tyre_up_and_the_front_one_down_and_the_nose_down() {
        let nose_down = RiderSample { elevation: -0.25, rear_down: false, ..sample() };
        assert_eq!(nose_down.stoppie(), 0.25);
        assert_eq!(RiderSample { rear_down: true, ..nose_down }.stoppie(), 0.0);
        assert_eq!(nose_down.wheelie(), 0.0);
    }

    #[test]
    fn the_speed_is_in_kilometres_an_hour_and_never_negative() {
        assert!((RiderSample { speed: 10.0, ..sample() }.speed_kmh() - 36.0).abs() < 1e-4);
        assert_eq!(RiderSample { speed: -3.0, ..sample() }.speed_kmh(), 0.0);
    }

    #[test]
    fn the_lean_is_the_mix_of_the_riders_and_the_bikes_over_85_percent_faded_in_with_the_speed() {
        // At 50 km/h and more the fade is full: a mix of 0.85 or more is the whole lean (times the multiplier of 1).
        let lean = |rider_lean: f32, bike_lean: f32, speed: f32| {
            let s = RiderSample { rider_lean, bike_lean, speed, ..sample() };
            run(&s, 8.0).values().lean
        };
        assert!((lean(1.0, 1.0, 20.0) - 1.0).abs() < 0.01);
        assert!((lean(-1.0, -1.0, 20.0) + 1.0).abs() < 0.01);
        // Half the lean at 0.425 of the mix: 0.3 * 0.5 + 0.7 * 0.5 = 0.5 is 0.588 of the full.
        assert!((lean(0.5, 0.5, 20.0) - 0.5 / 0.85).abs() < 0.01);
        // The bike's lean weighs 0.7 and the rider's 0.3.
        assert!((lean(0.0, 0.85, 20.0) - 0.7).abs() < 0.01);
        assert!((lean(0.85, 0.0, 20.0) - 0.3).abs() < 0.01);
        // Slower, less: the fade is 1/6 at a standstill and full at 50 km/h.
        assert!((lean(1.0, 1.0, 0.0) - 1.0 / 6.0).abs() < 0.01);
        // Above 250 km/h the lean decays towards 0.3 of itself at 400.
        assert!((lean(1.0, 1.0, 400.0 / 3.6) - 0.3).abs() < 0.01);
    }

    #[test]
    fn the_lean_is_smoothed_and_starts_from_nothing() {
        let s = RiderSample { rider_lean: 1.0, bike_lean: 1.0, speed: 20.0, ..sample() };
        let first = run(&s, 1.0 / 60.0).values().lean;
        assert!(first > 0.0 && first < 0.05, "{first}");
        let later = run(&s, 3.0).values().lean;
        assert!(later > 0.9, "{later}");
    }

    #[test]
    fn the_tuck_follows_the_riders_tuck_level_unless_he_brakes_or_lifts_the_front() {
        let tucked = RiderSample { tuck_level: 1.0, speed: 50.0, ..sample() };
        assert!(run(&tucked, 3.0).values().tuck > 0.95);
        assert!(run(&RiderSample { tuck_level: 0.0, ..tucked }, 3.0).values().tuck < 0.01, "no input, no throttle");
        assert!(run(&RiderSample { front_brake: 1.0, ..tucked }, 3.0).values().tuck < 0.01);
        // The throttle held for a moment tucks him slowly (the sum of the input and the held throttle, smoothed at 1).
        let throttle = RiderSample { throttle: 1.0, speed: 50.0, ..sample() };
        let quick = run(&throttle, 0.5).values().tuck;
        let slow = run(&throttle, 4.0).values().tuck;
        assert!(quick > 0.0 && quick < 0.5 && slow > 0.95, "{quick} {slow}");
        // A wheelie in progress lifts him out of the tuck.
        let wheelie = RiderSample { elevation: 0.6, front_down: false, ..tucked };
        assert!(run(&wheelie, 5.0).values().tuck < 0.05);
    }

    #[test]
    fn the_wheelie_pose_follows_the_height_of_the_nose_with_a_delay() {
        let wheelie = RiderSample { elevation: 0.5, front_down: false, ..sample() };
        let early = run(&wheelie, 0.15).values();
        assert_eq!(early.wheelie_alpha, 0.0, "the delay of 0.2 s has not passed");
        assert!(early.wheelie > 0.45, "the entry follows at 8 per second: {}", early.wheelie);
        let late = run(&wheelie, 4.0).values();
        assert!(late.wheelie_alpha > 0.99 && (late.wheelie - 0.5).abs() < 1e-4, "{late:?}");
        // Under 0.15 there is no pose.
        let low = RiderSample { elevation: 0.1, front_down: false, ..sample() };
        assert_eq!(run(&low, 4.0).values().wheelie_alpha, 0.0);
    }

    #[test]
    fn the_wheelie_leans_to_the_side_of_the_bikes_lean() {
        let side = |bike_lean: f32| wheelie_direction(bike_lean);
        assert_eq!(side(0.03), 0.0);
        assert_eq!(side(0.5), 1.0);
        assert_eq!(side(-0.5), -1.0);
        // Between the dead band and the full range the weight is under 1: the value is squared away from the default.
        assert!((side(0.125) - 0.25).abs() < 1e-6, "{}", side(0.125));
        assert!((side(-0.125) + 0.25).abs() < 1e-6);
    }

    #[test]
    fn the_stoppie_latches_above_a_fifth_of_the_nose_and_lets_go_when_it_is_level() {
        let mut smoothed = Smoothed::default();
        let step = |smoothed: &mut Smoothed, dive: f32| {
            let s = RiderSample { elevation: -dive, rear_down: false, ..sample() };
            for _ in 0..180 {
                smoothed.step(&s, &inertia(), 1.0 / 60.0);
            }
            smoothed.values()
        };
        assert_eq!(step(&mut smoothed, 0.1).stoppie, 0.0, "under 0.18 and never above: still 0");
        assert!(step(&mut smoothed, 0.25).stoppie > 0.99);
        assert!(step(&mut smoothed, 0.1).stoppie > 0.99, "between 0 and 0.18 it holds what it had");
        let level = RiderSample { rear_down: true, ..sample() };
        for _ in 0..180 {
            smoothed.step(&level, &inertia(), 1.0 / 60.0);
        }
        assert!(smoothed.values().stoppie < 0.01 && smoothed.values().stoppie_time < 0.01);
    }

    #[test]
    fn the_displacement_is_taken_into_the_cars_axes_and_held_to_the_ranges_of_the_data() {
        let inertia = inertia();
        assert_eq!(shift_target(Vec3::ZERO, &inertia), Vec3::ZERO);
        // The physics axes are x right, y up, z forward: back 3 cm, 10 cm to the right, 5 cm up.
        let shift = shift_target(Vec3::new(0.10, 0.05, -0.03), &inertia);
        assert!(shift.distance(Vec3::new(-0.03, -0.10, 0.05)) < 1e-6, "{shift:?}");
        // The ranges: forward from -6 cm to 0, sideways 15 cm either way, up from -4 cm to 20 cm.
        assert_eq!(shift_target(Vec3::new(0.0, 0.0, 0.2), &inertia).x, 0.0, "forward is not asked for");
        assert_eq!(shift_target(Vec3::new(0.0, 0.0, -0.2), &inertia).x, -0.06);
        assert_eq!(shift_target(Vec3::new(-0.5, 0.0, 0.0), &inertia).y, 0.15);
        assert_eq!(shift_target(Vec3::new(0.5, 0.0, 0.0), &inertia).y, -0.15);
        assert_eq!(shift_target(Vec3::new(0.0, -0.3, 0.0), &inertia).z, -0.04);
        assert_eq!(shift_target(Vec3::new(0.0, 0.9, 0.0), &inertia).z, 0.2);
        assert_eq!(shift_target(Vec3::splat(f32::NAN), &inertia), Vec3::ZERO);
    }

    #[test]
    fn the_turn_of_the_body_is_passed_on_as_the_physics_has_it() {
        let turned = RiderSample { rotation: Vec3::new(0.1, -0.05, 0.02), ..sample() };
        assert_eq!(run(&turned, 0.1).rotation(), Vec3::new(0.1, -0.05, 0.02));
        let bad = RiderSample { rotation: Vec3::splat(f32::NAN), ..sample() };
        assert_eq!(run(&bad, 0.1).rotation(), Vec3::ZERO);
    }

    #[test]
    fn the_shift_is_the_mirror_image_for_the_other_side() {
        let inertia = inertia();
        let right = shift_target(Vec3::new(0.08, -0.02, -0.01), &inertia);
        let left = shift_target(Vec3::new(-0.08, -0.02, -0.01), &inertia);
        assert!((right.y + left.y).abs() < 1e-7);
        assert_eq!((right.x, right.z), (left.x, left.z));
    }

    #[test]
    fn the_shift_follows_the_riders_place_at_the_rate_of_the_data() {
        let hanging = RiderSample { offset: Vec3::new(0.1, -0.02, -0.04), ..sample() };
        let after_a_tick = run(&hanging, 1.0 / 60.0).shift();
        let settled = run(&hanging, 3.0).shift();
        assert!(settled.distance(Vec3::new(-0.04, -0.1, -0.02)) < 1e-4, "{settled:?}");
        assert!(after_a_tick.length() > 0.0 && after_a_tick.length() < settled.length() * 0.2, "{after_a_tick:?}");
        // Back in its place he comes home.
        let mut smoothed = run(&hanging, 3.0);
        for _ in 0..180 {
            smoothed.step(&sample(), &inertia(), 1.0 / 60.0);
        }
        assert!(smoothed.shift().length() < 1e-3);
    }

    #[test]
    fn a_step_of_no_time_or_a_bad_time_changes_nothing() {
        let mut smoothed = run(&RiderSample { rider_lean: 0.5, bike_lean: 0.5, speed: 10.0, ..sample() }, 1.0);
        let before = smoothed.clone();
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            smoothed.step(&RiderSample { bike_lean: -1.0, ..sample() }, &inertia(), dt);
            assert_eq!(smoothed, before);
        }
    }
}
