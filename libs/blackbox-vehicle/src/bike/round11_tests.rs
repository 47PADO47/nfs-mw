//! Round 11 (`docs/specs/ride6-feel-round11.md`): the stick flicked from side to side with the throttle open, the lean help of a bike
//! on its rear wheel, and the g-force the lean limit reads.

use super::lean::{LeanInput, lean_limit};
use super::{Bike, BikeInput, BikeSpec, choices};
use crate::FIXED_STEP;
use crate::ground::FlatGround;

/// The stick at time `t`: it moves at `1 / rise` per second towards +1 and -1 in turn, each for `half` seconds, from the middle at `begin`.
fn flick(t: f32, begin: f32, rise: f32, half: f32) -> f32 {
    if t < begin {
        return 0.0;
    }
    let (mut value, mut clock, mut side, rate) = (0.0_f32, begin, 1.0_f32, 1.0 / rise);
    while clock + half < t {
        value = (value + side * rate * half).clamp(-1.0, 1.0);
        clock += half;
        side = -side;
    }
    (value + side * rate * (t - clock)).clamp(-1.0, 1.0)
}

/// A bike with the power of a big sports bike: its front wheel comes up in first gear at full throttle, as the imported ones do.
fn strong_bike() -> Bike {
    let mut spec = BikeSpec::example();
    spec.engine.peak_torque *= 2.2;
    Bike::new(spec)
}

/// Ride from `start` m/s at full throttle with the stick flicked; the time of the fall, if any, and the longest time the front wheel was up.
fn ride_flicked(start: f32, rise: f32, half: f32, seconds: f32) -> (Option<f32>, f32) {
    let ground = FlatGround::new(0.0);
    let mut bike = strong_bike();
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, start));
    let (mut up, mut longest) = (0.0_f32, 0.0_f32);
    for step in 0..(seconds / FIXED_STEP) as usize {
        let t = step as f32 * FIXED_STEP;
        let share = choices::stick_share(bike.forward_speed());
        let input = BikeInput { throttle: 1.0, steer: flick(t, 0.3, rise, half) * share, ..Default::default() };
        bike.step(FIXED_STEP, &input, &ground);
        match bike.wheel(0).on_ground {
            true => up = 0.0,
            false => up += FIXED_STEP,
        }
        longest = longest.max(up);
        if bike.fallen() {
            return (Some(t), longest);
        }
    }
    (None, longest)
}

#[test]
fn the_stick_flicked_from_side_to_side_with_full_throttle_does_not_throw_a_bike_from_a_standstill_or_at_speed() {
    let mut front_up = 0.0_f32;
    for start in [0.0_f32, 4.0, 8.0, 12.0, 20.0] {
        for (rise, half) in [(0.1, 0.2), (0.1, 0.45), (0.2, 0.4), (0.3, 0.85)] {
            let (fell, up) = ride_flicked(start, rise, half, 6.0);
            assert_eq!(fell, None, "from {start} m/s, rise {rise} s, half period {half} s");
            front_up = front_up.max(up);
        }
    }
    // The check means something only if the front wheel does come up in some of the rides.
    assert!(front_up > 0.2, "the front wheel was up for {front_up} s at most");
}

#[test]
fn a_bike_on_its_rear_wheel_gets_the_lean_help_as_a_couple_about_its_centre_of_gravity() {
    let ground = FlatGround::new(0.0);
    let mut bike = strong_bike();
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 4.0));
    // A wheelie with the weight back and a right-hand lean asked for: the roll torque the help gives is to the left (negative about
    // the forward axis' right-handed sense, as the controller's plan) and is not limited by the load of a front tyre that is not there.
    bike.set_weight(-1.0);
    let mut peak_help = 0.0_f32;
    for _ in 0..(3.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &BikeInput { throttle: 1.0, steer: 0.25, ..Default::default() }, &ground);
        if bike.wheel(0).on_ground || !bike.wheel(1).on_ground {
            continue;
        }
        peak_help = peak_help.max(bike.wanted_front_force().abs());
        assert!(!bike.fallen(), "a bike on its rear wheel fell at lean {}", bike.lean());
    }
    assert!(peak_help > 0.0, "the front wheel never left the road");
}

fn input_at(long_g: f32) -> LeanInput {
    LeanInput {
        stick: 1.0,
        speed: 30.0,
        lean: 0.0,
        lean_rate: 0.0,
        lateral_grip: 1.3,
        terrain_slope: 0.0,
        long_g,
        heading_error: 0.0,
        pitch_cut: (0.0, 0.0),
        roll_inertia: 47.8,
        cg_height: 0.8,
        front_arm: 1.0,
        target_cap: f32::MAX,
        dt: FIXED_STEP,
    }
}

#[test]
fn the_lean_limit_falls_with_the_acceleration_along_the_bike_not_with_the_cornering_g() {
    let steering = BikeSpec::example().steering;
    let steady = lean_limit(&steering, &input_at(0.0));
    let braking = lean_limit(&steering, &input_at(-1.5));
    let driving = lean_limit(&steering, &input_at(1.5));
    assert!(braking < steady && driving < steady, "{braking} {driving} {steady}");
    assert_eq!(braking, driving, "the sign does not matter");
}

#[test]
fn a_steady_full_stick_turn_leaves_the_lean_limit_whole() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_moving(&ground, 0.0, 0.0, 2.0, 0.0, 20.0));
    let mut limits = Vec::new();
    for step in 0..(8.0 / FIXED_STEP) as usize {
        let v = bike.body().linear_velocity.length();
        let throttle = (0.25 + 0.2 * (20.0 - v)).clamp(0.0, 1.0);
        bike.step(FIXED_STEP, &BikeInput { throttle, steer: 1.0, ..Default::default() }, &ground);
        if step as f32 * FIXED_STEP > 5.0 {
            limits.push(bike.lean_limit());
        }
    }
    assert!(!bike.fallen());
    let (low, high) = limits.iter().fold((f32::MAX, 0.0_f32), |(l, h), &x| (l.min(x), h.max(x)));
    // A turn at a steady speed is a cornering g of 1 and an acceleration along the bike of about 0: the limit stays within a few
    // percent (the curve of the example bike falls by 10 percent over 1.1 g of acceleration along the bike).
    assert!(low > 0.95 * high, "limit between {low} and {high}");
}
