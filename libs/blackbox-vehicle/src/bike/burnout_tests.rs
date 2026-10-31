use crate::{FIXED_STEP, FlatGround};

use super::super::{Bike, BikeInput, BikeSpec, FRONT, REAR};
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
    assert!(!BurnoutSpec { lean_limit: 2.0, ..spec }.is_valid(), "a lean limit past a right angle");
    assert!(!BurnoutSpec { help: HelpSpec { reduction: 2.0, ..spec.help }, ..spec }.is_valid());
    assert!(!BurnoutSpec { help: HelpSpec { yaw_rate: f32::NAN, ..spec.help }, ..spec }.is_valid());
}

#[test]
fn the_defaults_carry_the_handlers_lean_limit_the_tcs_flag_and_the_help_force() {
    let spec = BurnoutSpec::default();
    assert!((spec.lean_limit - 18.0_f32.to_radians()).abs() < 1e-6, "BurnoutLeanLimit of 436 of 445 handler objects");
    assert!(spec.tcs_modulation, "the constructor of the traction control element sets the flag");
    let help = spec.help;
    assert!(help.enabled && help.factor == 1.0 && help.steer_filter == 1.0);
    assert!((help.yaw_rate - 45.0_f32.to_radians()).abs() < 1e-6, "45 degrees per second");
    assert_eq!(help.rpm_range, [0.75, 0.95]);
    assert_eq!((help.reduction, help.reduction_duration, help.raise_rate), (1.0, 0.1, 10.0));
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

#[test]
fn the_stick_filter_closes_the_gap_in_the_square_root_domain_in_the_filter_time() {
    let step = 1.0 / 480.0;
    let at = |seconds: f32, time: f32, stick: f32| {
        (0..(seconds / step).round() as usize).fold(0.0, |filtered, _| filter_stick(filtered, stick, time, step))
    };
    // From rest to a full stick: 1 - (1 - t)^2 with the filter time 1 s, so a quarter of the time gets 44 % and half of it 75 %.
    assert!((at(0.25, 1.0, 1.0) - 0.4375).abs() < 0.01, "{}", at(0.25, 1.0, 1.0));
    assert!((at(0.5, 1.0, 1.0) - 0.75).abs() < 0.01);
    assert_eq!(at(1.05, 1.0, 1.0), 1.0, "it arrives, and does not pass");
    assert!((at(0.5, 2.0, 1.0) - 1.0).abs() < 0.01, "twice the rate: half the time");
    assert!((at(0.5, 1.0, -1.0) + 0.75).abs() < 0.01, "the same to the left");
    assert_eq!(filter_stick(0.3, 0.3, 1.0, step), 0.3);
    assert_eq!(filter_stick(0.0, 1.0, 0.0, step), 0.0, "a filter time of 0 never moves");
}

#[test]
fn the_help_force_comes_in_over_the_rpm_range_of_the_rev_limit() {
    let range = [0.75, 0.95];
    assert_eq!(rpm_share(range, 0.7 * 10000.0, 10000.0), 0.0);
    assert!((rpm_share(range, 0.85 * 10000.0, 10000.0) - 0.5).abs() < 1e-4);
    assert_eq!(rpm_share(range, 12000.0, 10000.0), 1.0);
    assert_eq!(rpm_share([-1.0, 0.95], 100.0, 10000.0), 1.0, "an unset range is a full share");
    assert_eq!(rpm_share([0.9, 0.9], 8999.0, 10000.0), 0.0);
    assert_eq!(rpm_share([0.9, 0.9], 9000.0, 10000.0), 1.0);
}

/// A bike with the pedal and the throttle held and the stick over, ridden for `seconds`: the bike, the mean yaw rate over the last second
/// (positive to the right) and how far the front contact moved.
fn donut(spec: BikeSpec, steer: f32, seconds: f32) -> (Bike, f32, f32) {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(spec);
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    let input = BikeInput { throttle: 1.0, front_brake: 1.0, rear_brake: 1.0, steer };
    // (The contact point is read from the first step on.)
    bike.step(FIXED_STEP, &input, &ground);
    let start = bike.wheel(FRONT).contact;
    let steps = (seconds / FIXED_STEP) as usize;
    let (mut yaw, mut count, mut farthest) = (0.0, 0, 0.0_f32);
    for step in 0..steps {
        bike.step(FIXED_STEP, &input, &ground);
        let moved = bike.wheel(FRONT).contact - start;
        farthest = farthest.max(moved.x.hypot(moved.z));
        if step as f32 * FIXED_STEP >= seconds - 1.0 {
            yaw += bike.body().angular_velocity.y;
            count += 1;
        }
    }
    (bike, yaw / count as f32, farthest)
}

#[test]
fn with_the_stick_over_the_rear_swings_round_the_front_wheel_at_the_data_yaw_rate() {
    let wanted = BurnoutSpec::default().help.yaw_rate;
    let (bike, yaw, front) = donut(BikeSpec::example(), 1.0, 4.0);
    assert!(bike.burnout() && !bike.fallen());
    assert!((yaw / wanted - 1.0).abs() < 0.1, "the yaw rate {yaw} rad/s against the data's {wanted}");
    assert!((bike.burnout_yaw_target() - wanted).abs() < 1e-4, "the filter has arrived");
    assert!(front < 0.3, "the front wheel stays put: it moved {front} m");
    assert!(bike.wheel(REAR).slip_ratio > 1.0, "the rear wheel is still spinning");
    assert!(bike.pitch().abs() < 0.2, "pitch {}", bike.pitch());
    // The bike has turned: well round after 4 s at 45 degrees per second (the filter takes the first second).
    let forward = bike.body().rotation() * glam::Vec3::Z;
    let heading = forward.x.atan2(forward.z);
    assert!(heading > 1.5, "the bike turned {heading} rad to the right");
}

#[test]
fn the_left_stick_swings_it_to_the_left_and_half_the_stick_is_half_the_yaw_rate() {
    let wanted = BurnoutSpec::default().help.yaw_rate;
    let (left, yaw, _) = donut(BikeSpec::example(), -1.0, 4.0);
    assert!(!left.fallen() && (yaw / -wanted - 1.0).abs() < 0.1, "{yaw}");
    let (half, yaw, _) = donut(BikeSpec::example(), 0.5, 4.0);
    assert!(!half.fallen() && (yaw / (0.5 * wanted) - 1.0).abs() < 0.1, "{yaw}");
}

#[test]
fn a_centred_stick_keeps_the_yaw_at_zero_and_a_handler_without_the_help_force_does_not_swing() {
    let (bike, yaw, front) = donut(BikeSpec::example(), 0.0, 3.0);
    assert!(bike.burnout() && yaw.abs() < 0.02 && front < 0.1, "yaw {yaw} front {front}");
    let help = HelpSpec { enabled: false, ..HelpSpec::default() };
    let off = BikeSpec { burnout: BurnoutSpec { help, ..BurnoutSpec::default() }, ..BikeSpec::example() };
    let (bike, yaw, _) = donut(off, 1.0, 4.0);
    assert!(bike.burnout() && bike.burnout_yaw_target() == 0.0);
    // The bike swings on the steered front wheel by the rear's own grip alone (what the supermotos, whose handler has no help force, do):
    // at a rate of its own, not the data's.
    let wanted = BurnoutSpec::default().help.yaw_rate;
    assert!(yaw > 0.0 && (yaw / wanted - 1.0).abs() > 0.15, "yaw {yaw} rad/s against the data's {wanted}");
}

#[test]
fn the_help_force_ends_with_the_burnout_and_its_stick_filter_starts_from_rest() {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    let held = BikeInput { throttle: 1.0, front_brake: 1.0, rear_brake: 1.0, steer: 1.0 };
    for _ in 0..(2.0 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &held, &ground);
    }
    assert!(bike.burnout_yaw_target() > 0.5);
    // The brake let go: no burnout, no help force.
    bike.step(FIXED_STEP, &BikeInput { front_brake: 0.0, rear_brake: 0.0, ..held }, &ground);
    assert!(!bike.burnout() && bike.burnout_yaw_target() == 0.0);
    // Held again, the stick filter starts from rest.
    bike.step(FIXED_STEP, &held, &ground);
    assert!(bike.burnout_yaw_target() < 0.1, "{}", bike.burnout_yaw_target());
}

#[test]
fn a_collision_cuts_the_help_force_for_a_moment_and_gives_it_back_at_the_raise_rate() {
    let help = HelpSpec::default();
    let mut state = HelpState { reduction: help.reduction, hold: help.reduction_duration, ..HelpState::default() };
    let step = 1.0 / 480.0;
    for _ in 0..(0.09 / step) as usize {
        state.run_down(&help, step);
    }
    assert_eq!(state.reduction, 1.0, "held for 0.1 s");
    for _ in 0..(0.05 / step) as usize {
        state.run_down(&help, step);
    }
    // 0.14 s in: 0.1 s held, 0.04 s of the fall at 10 per second.
    assert!((state.reduction - 0.6).abs() < 0.05, "falling at 10 per second: {}", state.reduction);
    for _ in 0..(0.1 / step) as usize {
        state.run_down(&help, step);
    }
    assert_eq!(state.reduction, 0.0);
    // On a bike: the force stays out while the cut is held.
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec::example());
    assert!(bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    let held = BikeInput { throttle: 1.0, front_brake: 1.0, rear_brake: 1.0, steer: 1.0 };
    bike.step(FIXED_STEP, &held, &ground);
    bike.notify_collision(false);
    assert_eq!((bike.burnout_help.reduction, bike.burnout_help.hold), (1.0, 0.1));
    for _ in 0..(0.09 / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, &held, &ground);
    }
    let yaw = bike.body().angular_velocity.y;
    assert!(yaw.abs() < 0.1, "no yaw while the force is out: {yaw}");
}

#[test]
fn the_lean_limit_of_a_burnout_is_the_handlers_and_replaces_the_rest() {
    let (bike, _, _) = donut(BikeSpec::example(), 0.0, 1.0);
    assert!((bike.plan.limit - 18.0_f32.to_radians()).abs() < 1e-5, "{}", bike.plan.limit);
    let motard = BurnoutSpec { lean_limit: 10.0_f32.to_radians(), ..BurnoutSpec::default() };
    let (bike, _, _) = donut(BikeSpec { burnout: motard, ..BikeSpec::example() }, 0.0, 1.0);
    assert!((bike.plan.limit - 10.0_f32.to_radians()).abs() < 1e-5, "{}", bike.plan.limit);
    // Out of a burnout the stopped bike's own (small) limit stands.
    let ground = FlatGround::new(0.0);
    let mut idle = Bike::new(BikeSpec::example());
    assert!(idle.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0));
    for _ in 0..60 {
        idle.step(FIXED_STEP, &BikeInput::default(), &ground);
    }
    assert!(idle.plan.limit < 18.0_f32.to_radians() - 0.01, "{}", idle.plan.limit);
    let none = BurnoutSpec { lean_limit: 0.0, ..BurnoutSpec::default() };
    let (bike, _, _) = donut(BikeSpec { burnout: none, ..BikeSpec::example() }, 0.0, 1.0);
    assert!(bike.burnout() && bike.plan.limit < 18.0_f32.to_radians() - 0.01, "no limit of its own: the usual one");
}

#[test]
fn the_traction_control_keeps_its_cut_in_a_burnout_unless_the_flag_is_cleared() {
    let mut bike = Bike::new(BikeSpec::example());
    assert_eq!(bike.burnout_traction_cut(0.6), 0.6, "no burnout");
    bike.burnout = true;
    assert_eq!(bike.burnout_traction_cut(0.6), 0.6, "the constructor's flag: the cut follows the control's window");
    bike.spec.burnout.tcs_modulation = false;
    assert_eq!(bike.burnout_traction_cut(0.6), 0.0, "the flag cleared: no cut in a burnout");
    bike.burnout = false;
    assert_eq!(bike.burnout_traction_cut(0.6), 0.6);
}
