//! The numbers that drive the rider's animation, from the bike's state (`docs/specs/ride6-rider-animation.md`, section 5
//! and the table "Mapping from this project's bike state"). The graph's parameters are `lean`, `speed_kmh`, `tuck`,
//! `wheelie` and `stoppie`; the rider's body also shifts forward and back with the bike's acceleration.
//!
//! Every constant that is not in `attach.json` is one of the named limits below; the numbers of the data (the speed from
//! which the rider tucks, the smoothing rates, the shift ranges) come from the file.

use glam::Vec3;

use super::attach::{Inertia, Inputs};

/// A slower step than this (s) does not move the filters: it is a pause, not time.
const MAX_STEP: f32 = 0.1;

/// What the animation needs of the bike at one moment. The bike's physics make it (`CarSim::rider_sample`); the fields
/// are in radians, metres per second and units of 0 to 1.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RiderSample {
    /// The rider's lean from upright (the bike's plus his body's own against it), positive to the right.
    pub lean: f32,
    /// The steepest lean the bike model allows: the rider's lean is the bike's over this.
    pub max_lean: f32,
    /// Speed along the bike.
    pub speed: f32,
    pub throttle: f32,
    pub front_brake: f32,
    pub rear_brake: f32,
    /// The pitch of the bike's nose above the horizon.
    pub pitch_up: f32,
    /// The pitch at which the bike's centre of gravity is above its rear tyre's patch (a wheelie balanced), and above the
    /// front one (a stoppie balanced).
    pub wheelie_limit: f32,
    pub stoppie_limit: f32,
    pub front_down: bool,
    pub rear_down: bool,
    /// Where the rider's centre of mass is from his rest place, in the bike's physics axes (x right, y up, z forward), metres:
    /// the place the weight, the lean and the speed ask for plus how far his spring has let him move.
    pub offset: Vec3,
    /// His body against the bike's attitude: roll to the right, pitch nose up and yaw to the right, radians.
    pub rotation: Vec3,
}

impl RiderSample {
    /// The lean as the graph takes it: -1 (full left) to 1 (full right). **Choice**: the bike's lean over the steepest lean
    /// its model allows.
    pub fn lean_unit(&self) -> f32 {
        if self.max_lean <= f32::EPSILON {
            return 0.0;
        }
        (self.lean / self.max_lean).clamp(-1.0, 1.0)
    }

    pub fn speed_kmh(&self) -> f32 {
        self.speed.max(0.0) * 3.6
    }

    /// The harder of the two brake levers, 0 to 1.
    pub fn braking(&self) -> f32 {
        self.front_brake.max(self.rear_brake).clamp(0.0, 1.0)
    }

    /// The front wheel's lift, 0 to 1: the pitch of the nose over the pitch of a balanced wheelie, while the front tyre is
    /// off the ground. **Choice** (the spec's table): the data's `BikeWheelie` comes from native code.
    pub fn wheelie(&self) -> f32 {
        if self.front_down || self.wheelie_limit <= f32::EPSILON {
            return 0.0;
        }
        (self.pitch_up / self.wheelie_limit).clamp(0.0, 1.0)
    }

    /// The rear wheel's lift, 0 to 1, likewise.
    pub fn stoppie(&self) -> f32 {
        if self.rear_down || self.stoppie_limit <= f32::EPSILON {
            return 0.0;
        }
        (-self.pitch_up / self.stoppie_limit).clamp(0.0, 1.0)
    }
}

/// `value` moved towards `target` at `rate` per second for `dt` seconds, as a first-order filter does: the same
/// whatever the step.
pub fn follow(value: f32, target: f32, rate: f32, dt: f32) -> f32 {
    if !(dt > 0.0 && rate > 0.0) {
        return value;
    }
    value + (target - value) * (1.0 - (-rate * dt).exp())
}

/// The values the graph's parameters take.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Values {
    pub lean: f32,
    pub speed_kmh: f32,
    pub tuck: f32,
    pub wheelie: f32,
    pub stoppie: f32,
}

/// The inputs after smoothing, and the rider's shift along the bike.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Smoothed {
    values: Values,
    /// The rider's displacement from his rest place in the car's axes (x forward, y left, z up), metres, after the clamp to the
    /// ranges of the animation and the smoothing.
    shift: Vec3,
    /// His body's turn against the bike's: roll to the right, pitch nose up, yaw to the right (rad).
    rotation: Vec3,
}

impl Smoothed {
    /// Advance by `dt` seconds with the bike in state `sample`.
    pub fn step(&mut self, sample: &RiderSample, inputs: &Inputs, inertia: &Inertia, dt: f32) {
        if !(dt > 0.0 && dt.is_finite()) {
            return;
        }
        let dt = dt.min(MAX_STEP);
        let speed = sample.speed_kmh();
        // Tuck (the shape of the blueprint's `TuckedIn_Alpha`: throttle held, brake released, no wheelie, from the speed of
        // the data on). **Choice**: the native `TuckedIn` input is not read.
        let tuck = match speed > inputs.tuck_speed_kmh {
            true => sample.throttle.clamp(0.0, 1.0) * (1.0 - sample.braking()) * (1.0 - self.values.wheelie),
            false => 0.0,
        };
        let v = &mut self.values;
        v.lean = follow(v.lean, sample.lean_unit(), inputs.lean_rate, dt);
        v.tuck = follow(v.tuck, tuck, inputs.tuck_rate, dt);
        v.wheelie = follow(v.wheelie, sample.wheelie(), inputs.wheelie_rate, dt);
        v.stoppie = follow(v.stoppie, sample.stoppie(), inputs.stoppie_rate, dt);
        v.speed_kmh = speed;
        // The shift: the rider's displacement from the physics, held to the ranges of the animation's inertia input.
        let target = shift_target(sample.offset, inertia);
        self.shift += (target - self.shift) * (1.0 - (-inertia.rate * dt).exp());
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

    fn inputs() -> Inputs {
        Inputs { tuck_speed_kmh: 140.0, tuck_rate: 6.0, lean_rate: 4.0, wheelie_rate: 3.0, stoppie_rate: 3.0 }
    }

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
        RiderSample {
            max_lean: 1.0,
            wheelie_limit: 0.8,
            stoppie_limit: 0.9,
            front_down: true,
            rear_down: true,
            ..RiderSample::default()
        }
    }

    #[test]
    fn the_lean_is_the_bikes_over_the_steepest_and_keeps_its_side() {
        let s = |lean: f32| RiderSample { lean, ..sample() };
        assert_eq!(s(0.5).lean_unit(), 0.5);
        assert_eq!(s(-0.25).lean_unit(), -0.25);
        assert_eq!(s(3.0).lean_unit(), 1.0);
        assert_eq!(RiderSample { max_lean: 0.0, lean: 0.5, ..sample() }.lean_unit(), 0.0);
    }

    #[test]
    fn a_wheelie_needs_the_front_wheel_in_the_air_and_scales_with_the_pitch() {
        let up = RiderSample { pitch_up: 0.4, ..sample() };
        assert_eq!(up.wheelie(), 0.0, "the front tyre is down");
        let lifted = RiderSample { front_down: false, ..up };
        assert!((lifted.wheelie() - 0.5).abs() < 1e-6);
        assert_eq!(RiderSample { pitch_up: 2.0, ..lifted }.wheelie(), 1.0);
        assert_eq!(RiderSample { pitch_up: -0.2, ..lifted }.wheelie(), 0.0);
        assert_eq!(lifted.stoppie(), 0.0);
    }

    #[test]
    fn a_stoppie_needs_the_rear_wheel_in_the_air_and_the_nose_down() {
        let nose_down = RiderSample { pitch_up: -0.45, rear_down: false, ..sample() };
        assert!((nose_down.stoppie() - 0.5).abs() < 1e-6);
        assert_eq!(RiderSample { rear_down: true, ..nose_down }.stoppie(), 0.0);
        assert_eq!(nose_down.wheelie(), 0.0);
    }

    #[test]
    fn the_speed_is_in_kilometres_an_hour_and_never_negative() {
        assert!((RiderSample { speed: 10.0, ..sample() }.speed_kmh() - 36.0).abs() < 1e-4);
        assert_eq!(RiderSample { speed: -3.0, ..sample() }.speed_kmh(), 0.0);
    }

    #[test]
    fn the_filter_is_the_same_whatever_the_step_and_does_not_overshoot() {
        let once = follow(0.0, 1.0, 4.0, 0.2);
        let twice = follow(follow(0.0, 1.0, 4.0, 0.1), 1.0, 4.0, 0.1);
        assert!((once - twice).abs() < 1e-6);
        assert!(once > 0.0 && once < 1.0);
        assert_eq!(follow(0.3, 1.0, 4.0, 0.0), 0.3);
        assert_eq!(follow(0.3, 1.0, 0.0, 1.0), 0.3);
        assert!(follow(0.0, 1.0, 100.0, 1.0) <= 1.0);
    }

    fn run(sample: &RiderSample, seconds: f32) -> Smoothed {
        let mut smoothed = Smoothed::default();
        for _ in 0..(seconds * 60.0) as usize {
            smoothed.step(sample, &inputs(), &inertia(), 1.0 / 60.0);
        }
        smoothed
    }

    #[test]
    fn the_rider_tucks_above_the_threshold_with_the_throttle_held_and_the_brake_released() {
        let fast = RiderSample { speed: 50.0, throttle: 1.0, ..sample() };
        assert!(run(&fast, 2.0).values().tuck > 0.95);
        assert_eq!(
            run(&RiderSample { speed: 30.0, ..fast }, 2.0).values().tuck,
            0.0,
            "108 km/h is under the threshold"
        );
        assert!(run(&RiderSample { front_brake: 1.0, ..fast }, 2.0).values().tuck < 0.01);
        assert!(run(&RiderSample { throttle: 0.0, ..fast }, 2.0).values().tuck < 0.01);
        let half = run(&fast, 0.1).values().tuck;
        assert!(half > 0.0 && half < 0.95, "the tuck is smoothed: {half}");
    }

    #[test]
    fn the_lean_follows_the_bike_to_its_side() {
        let right = run(&RiderSample { lean: 0.5, speed: 10.0, ..sample() }, 3.0).values();
        assert!((right.lean - 0.5).abs() < 0.01);
        let left = run(&RiderSample { lean: -1.0, speed: 10.0, ..sample() }, 3.0).values();
        assert!((left.lean + 1.0).abs() < 0.01);
        assert!((right.speed_kmh - 36.0).abs() < 1e-3);
    }

    #[test]
    fn a_wheelie_and_a_stoppie_ramp_to_their_amounts() {
        let wheelie = RiderSample { pitch_up: 0.8, front_down: false, ..sample() };
        let values = run(&wheelie, 3.0).values();
        assert!(values.wheelie > 0.99 && values.stoppie == 0.0);
        let stoppie = RiderSample { pitch_up: -0.9, rear_down: false, ..sample() };
        assert!(run(&stoppie, 3.0).values().stoppie > 0.99);
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
            smoothed.step(&sample(), &inputs(), &inertia(), 1.0 / 60.0);
        }
        assert!(smoothed.shift().length() < 1e-3);
    }

    #[test]
    fn a_step_of_no_time_or_a_bad_time_changes_nothing() {
        let mut smoothed = run(&RiderSample { lean: 0.5, speed: 10.0, ..sample() }, 1.0);
        let before = smoothed;
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            smoothed.step(&RiderSample { lean: -1.0, ..sample() }, &inputs(), &inertia(), dt);
            assert_eq!(smoothed, before);
        }
    }
}
