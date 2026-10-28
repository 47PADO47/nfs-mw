use super::*;
use crate::bike::BikeSpec;

const DT: f32 = 1.0 / 60.0;

fn input(throttle: f32, wheel_spin: f32) -> DriveInput {
    DriveInput { throttle, wheel_spin, speed: wheel_spin * 0.32, dt: DT }
}

/// One step with the wheel held at `spin` (a stand-in for a wheel the bike's mass keeps turning).
fn step(train: &mut Powertrain, spec: &BikeSpec, throttle: f32, spin: f32) -> Drive {
    let drive = train.drive(&spec.engine, &spec.transmission, &input(throttle, spin));
    train.finish(&spec.engine, spin, DT);
    drive
}

#[test]
fn a_standing_start_slips_the_clutch_and_drives_the_wheel_in_first_gear() {
    let spec = BikeSpec::example();
    let mut train = Powertrain::new(&spec.engine);
    let mut torque = 0.0;
    for _ in 0..30 {
        torque = step(&mut train, &spec, 1.0, 0.0).torque;
    }
    // The engine revs against the slipping clutch, which passes a torque the engine's own torque holds in balance.
    assert!(torque > 100.0, "{torque}");
    assert_eq!(train.gear(), 1);
    assert!(train.rpm() > spec.engine.idle_rpm && train.rpm() < spec.engine.max_rpm);
}

#[test]
fn a_closed_throttle_at_a_standstill_does_nothing_and_idles() {
    let spec = BikeSpec::example();
    let mut train = Powertrain::new(&spec.engine);
    for _ in 0..60 {
        assert!(step(&mut train, &spec, 0.0, 0.0).torque.abs() < 1.0);
    }
    assert!((train.rpm() - spec.engine.idle_rpm).abs() < 0.2 * spec.engine.idle_rpm, "{}", train.rpm());
}

#[test]
fn a_closed_throttle_at_speed_brakes_with_the_engine() {
    let spec = BikeSpec::example();
    let mut train = Powertrain::new(&spec.engine);
    train.match_speed(&spec.engine, &spec.transmission, 60.0);
    let drive = step(&mut train, &spec, 0.0, 60.0);
    assert!(drive.torque < 0.0, "{drive:?}");
    // The clutch is locked, so the engine's inertia is part of what the wheel turns.
    assert!(drive.extra_inertia > 0.0);
}

#[test]
fn a_locked_engine_turns_with_the_wheel() {
    let spec = BikeSpec::example();
    let mut train = Powertrain::new(&spec.engine);
    train.match_speed(&spec.engine, &spec.transmission, 80.0);
    let gear = train.gear();
    step(&mut train, &spec, 0.5, 80.0);
    let ratio = spec.transmission.gears[gear - 1] * spec.transmission.final_ratio;
    let expected = 80.0 * ratio * 60.0 / TAU;
    assert!((train.rpm() - expected).abs() < 1.0, "{} {expected}", train.rpm());
}

#[test]
fn the_box_changes_up_as_the_wheel_speeds_up_and_the_drive_is_cut_for_the_change() {
    let spec = BikeSpec::example();
    let mut train = Powertrain::new(&spec.engine);
    train.match_speed(&spec.engine, &spec.transmission, 5.0);
    let mut spin = 5.0;
    let mut cut = false;
    for _ in 0..1200 {
        spin += 0.2;
        let drive = step(&mut train, &spec, 1.0, spin);
        cut |= train.shifting() && drive.torque == 0.0;
    }
    assert!(train.gear() > 1);
    assert!(cut);
}

#[test]
fn the_box_changes_down_when_the_wheel_slows() {
    let spec = BikeSpec::example();
    let mut train = Powertrain::new(&spec.engine);
    train.match_speed(&spec.engine, &spec.transmission, 70.0);
    let high = train.gear();
    let mut spin: f32 = 70.0;
    for _ in 0..600 {
        spin = (spin - 0.2).max(8.0);
        step(&mut train, &spec, 0.0, spin);
    }
    assert!(train.gear() < high, "{} {high}", train.gear());
}

#[test]
fn the_rev_limiter_cuts_the_throttle_in_turn() {
    let spec = BikeSpec::example();
    let mut train = Powertrain::new(&spec.engine);
    let top = spec.transmission.gears.len() - 1;
    let spin = spec.engine.max_rpm * TAU / 60.0 / (spec.transmission.gears[top] * spec.transmission.final_ratio);
    train.match_speed(&spec.engine, &spec.transmission, spin * 1.05);
    let torques: Vec<f32> = (0..12).map(|_| step(&mut train, &spec, 1.0, spin * 1.05).torque).collect();
    assert!(torques.iter().any(|&t| t < 0.0), "{torques:?}");
    assert!(torques.iter().any(|&t| t > 0.0), "{torques:?}");
}

#[test]
fn the_clutch_closes_again_after_a_change_instead_of_at_once() {
    let spec = BikeSpec::example();
    let mut train = Powertrain::new(&spec.engine);
    train.match_speed(&spec.engine, &spec.transmission, 5.0);
    let mut spin = 5.0;
    let mut worst: f32 = 0.0;
    for _ in 0..1200 {
        spin += 0.2;
        let drive = step(&mut train, &spec, 1.0, spin);
        worst = worst.max(drive.torque.abs());
    }
    // Never more than the clutch's capacity at the wheel in the first gear.
    let limit = spec.transmission.clutch_max_factor
        * spec.engine.peak_torque
        * spec.transmission.gears[0]
        * spec.transmission.final_ratio;
    assert!(worst <= limit * 1.01, "{worst} {limit}");
}

#[test]
fn the_smooth_sign_is_odd_and_bounded() {
    assert_eq!(smooth_sign(0.0, 1.0), 0.0);
    assert!(smooth_sign(5.0, 1.0) < 1.0 && smooth_sign(5.0, 1.0) > 0.8);
    assert_eq!(smooth_sign(-2.0, 1.0), -smooth_sign(2.0, 1.0));
}
