use crate::{FIXED_STEP, FlatGround};

use super::super::{Bike, BikeInput, BikeSpec, REAR};
use super::*;

fn request(throttle: f32, front_brake: f32, rear_brake: f32, speed: f32) -> Request {
    Request { throttle, front_brake, rear_brake, speed }
}

#[test]
fn the_flag_needs_the_throttle_the_front_brake_no_handbrake_and_a_crawl() {
    let spec = BurnoutSpec::default();
    assert!(request(1.0, 1.0, 0.0, 0.0).is_burnout(&spec), "separate levers: the front one alone");
    assert!(request(1.0, 1.0, 1.0, 0.0).is_burnout(&spec), "the brake pedal pulls both levers by the same amount");
    assert!(request(0.5, 0.3, 0.3, 1.0).is_burnout(&spec));
    assert!(!request(0.05, 1.0, 1.0, 0.0).is_burnout(&spec), "no throttle");
    assert!(!request(1.0, 0.05, 0.0, 0.0).is_burnout(&spec), "no front brake");
    assert!(!request(1.0, 0.0, 1.0, 0.0).is_burnout(&spec), "the handbrake alone");
    assert!(!request(1.0, 0.4, 1.0, 0.0).is_burnout(&spec), "the handbrake beyond the pedal");
    assert!(!request(1.0, 1.0, 0.5, 0.0).is_burnout(&spec), "separate levers with the rear one pressed");
    assert!(!request(1.0, 1.0, 0.0, 5.0 / 3.6 + 0.01).is_burnout(&spec), "faster than 5 km/h");
    assert!(request(1.0, 1.0, 0.0, -1.0).is_burnout(&spec), "a bike rolling back is slow too");
}

#[test]
fn the_defaults_are_the_road_bike_handlers_and_a_bad_spec_is_refused() {
    let spec = BurnoutSpec::default();
    assert_eq!((spec.grip, spec.slip_correction), (0.05, 0.0));
    assert!((spec.speed_limit - 5.0 / 3.6).abs() < 1e-6);
    assert!(spec.is_valid());
    assert!(!BurnoutSpec { grip: 1.5, ..spec }.is_valid());
    assert!(!BurnoutSpec { grip: f32::NAN, ..spec }.is_valid());
    assert!(!BurnoutSpec { speed_limit: 0.0, ..spec }.is_valid());
}

/// A bike standing on flat ground, ridden for `seconds` with `input`; the path is `(slip ratio of the rear wheel, distance from the start)`.
fn ride(spec: BikeSpec, seconds: f32, input: &BikeInput) -> (Bike, Vec<(f32, f32)>) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    let start = bike.body().position;
    let mut path = Vec::new();
    for _ in 0..(seconds / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, input, &ground);
        let moved = bike.body().position - start;
        path.push((bike.wheel(REAR).slip_ratio, moved.x.hypot(moved.z)));
    }
    (bike, path)
}

fn pedal(throttle: f32, lever: f32) -> BikeInput {
    BikeInput { throttle, front_brake: lever, rear_brake: lever, steer: 0.0 }
}

#[test]
fn the_brake_pedal_and_the_throttle_spin_the_rear_wheel_on_the_spot() {
    let (bike, path) = ride(BikeSpec::example(), 3.0, &pedal(1.0, 1.0));
    let spun = path.iter().position(|(slip, _)| *slip > 1.0).expect("the rear wheel never spun");
    assert!(spun as f32 * FIXED_STEP < 1.0, "spun after {} s", spun as f32 * FIXED_STEP);
    let far = path.iter().map(|(_, d)| *d).fold(0.0, f32::max);
    assert!(far < 1.0, "the bike moved {far} m");
    assert!(!bike.fallen() && bike.burnout());
    assert!(bike.lean().abs() < 0.1 && bike.pitch().abs() < 0.2, "lean {} pitch {}", bike.lean(), bike.pitch());
    assert!(bike.wheel(REAR).spin * bike.spec().rear.radius > 10.0, "the wheel turns fast");
}

#[test]
fn the_front_brake_alone_does_the_same_and_the_handbrake_or_no_brake_does_not() {
    let front_only = BikeInput { throttle: 1.0, front_brake: 1.0, ..BikeInput::default() };
    let (bike, path) = ride(BikeSpec::example(), 2.0, &front_only);
    assert!(path.iter().any(|(slip, _)| *slip > 1.0) && bike.burnout());
    let handbrake = BikeInput { throttle: 1.0, rear_brake: 1.0, ..BikeInput::default() };
    let (bike, path) = ride(BikeSpec::example(), 2.0, &handbrake);
    assert!(!bike.burnout() && path.iter().all(|(slip, _)| *slip < 1.0), "a locked rear brake holds the wheel");
    let (bike, _) = ride(BikeSpec::example(), 2.0, &BikeInput { throttle: 1.0, ..BikeInput::default() });
    assert!(!bike.burnout(), "no brake is a launch");
    assert!(bike.forward_speed() > 8.0, "{}", bike.forward_speed());
}

#[test]
fn the_rear_tyre_has_a_twentieth_of_its_grip_in_a_burnout_and_all_of_it_out_of_one() {
    let (bike, _) = ride(BikeSpec::example(), 1.5, &pedal(1.0, 1.0));
    let peak = bike.spec().rear.tyre.longitudinal_peak(bike.wheel(REAR).load * 1e-3);
    let force = bike.wheel(REAR).traction;
    assert!(force > 0.0 && force < 0.1 * peak, "force {force} N of a peak of {peak} N");
    let loose = BurnoutSpec { grip: 1.0, ..BurnoutSpec::default() };
    let (grippy, _) = ride(BikeSpec { burnout: loose, ..BikeSpec::example() }, 1.5, &pedal(1.0, 1.0));
    assert!(grippy.wheel(REAR).traction > 5.0 * force, "{} against {force}", grippy.wheel(REAR).traction);
}

#[test]
fn letting_the_brake_go_launches_the_bike_with_the_rear_still_spinning_and_does_not_flip_it() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    for _ in 0..(2.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &pedal(1.0, 1.0), &ground);
    }
    assert!(bike.burnout());
    let (mut spinning, mut worst_pitch) = (0.0_f32, 0.0_f32);
    for step in 0..(4.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &pedal(1.0, 0.0), &ground);
        if step < 30 {
            spinning = spinning.max(bike.wheel(REAR).slip_ratio);
        }
        worst_pitch = worst_pitch.max(bike.pitch().abs());
    }
    assert!(!bike.burnout() && !bike.fallen());
    assert!(spinning > 1.0, "the rear wheel keeps spinning as the bike goes off: {spinning}");
    assert!(bike.forward_speed() > 15.0, "{}", bike.forward_speed());
    assert!(worst_pitch < 1.0, "the nose came up to {worst_pitch} rad");
}

#[test]
fn a_motard_handler_keeps_more_grip_and_a_slip_correction() {
    let motard = BurnoutSpec { grip: 0.15, slip_correction: 0.9, ..BurnoutSpec::default() };
    let (bike, path) = ride(BikeSpec { burnout: motard, ..BikeSpec::example() }, 3.0, &pedal(1.0, 1.0));
    assert!(path.iter().any(|(slip, _)| *slip > 1.0), "it still spins");
    assert!(!bike.fallen() && path.iter().map(|(_, d)| *d).fold(0.0, f32::max) < 1.0);
}

#[test]
fn a_bad_spec_has_no_burnout() {
    let bad = BurnoutSpec { grip: -1.0, ..BurnoutSpec::default() };
    let (bike, _) = ride(BikeSpec { burnout: bad, ..BikeSpec::example() }, 1.0, &pedal(1.0, 1.0));
    assert!(!bike.burnout());
}
