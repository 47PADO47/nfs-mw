//! How the car feels: stopping, response, jumps, tire marks and forced induction.

use super::*;
use crate::induction::InductionSpec;

fn throttle() -> InputState {
    InputState { throttle: 1.0, ..Default::default() }
}

fn moving(spec: VehicleSpec, speed: f32) -> Vehicle {
    let mut v = Vehicle::new(spec);
    v.config.auto_reverse = false;
    v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), speed);
    // Let the springs settle before the test starts.
    let g = flat();
    for _ in 0..30 {
        v.step(FIXED_STEP, &InputState { throttle: 0.3, ..Default::default() }, &g);
    }
    v
}

#[test]
fn braking_from_100_kmh_takes_about_a_second_and_a_half() {
    let g = flat();
    let mut v = moving(VehicleSpec::example(), 27.8);
    let (z0, v0) = (v.position().z, v.forward_speed());
    let mut t = 0.0;
    while v.forward_speed() > 0.3 && t < 10.0 {
        v.step(FIXED_STEP, &InputState { brake: 1.0, ..Default::default() }, &g);
        t += FIXED_STEP;
    }
    let distance = v.position().z - z0;
    assert!(v0 > 26.0, "started at {v0}");
    assert!(t > 1.2 && t < 3.0, "100 to 0 km/h in {t} s");
    assert!(distance > 15.0 && distance < 45.0, "stopping distance {distance} m");
}

#[test]
fn a_brake_tap_slows_less_than_a_full_press() {
    let g = flat();
    let mut tap = moving(VehicleSpec::example(), 30.0);
    let mut full = moving(VehicleSpec::example(), 30.0);
    for _ in 0..30 {
        tap.step(FIXED_STEP, &InputState { brake: 0.3, ..Default::default() }, &g);
        full.step(FIXED_STEP, &InputState { brake: 1.0, ..Default::default() }, &g);
    }
    assert!(tap.forward_speed() > full.forward_speed() + 2.0, "{} {}", tap.forward_speed(), full.forward_speed());
    assert!(tap.forward_speed() < 29.5, "a light press still slows: {}", tap.forward_speed());
}

#[test]
fn the_car_turns_in_quickly() {
    let g = flat();
    let mut v = moving(VehicleSpec::example(), 20.0);
    let mut settled = 0.0;
    let mut samples = Vec::new();
    for i in 0..(4.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &InputState { throttle: 0.4, steer: 0.3, ..Default::default() }, &g);
        samples.push(v.angular_velocity().y);
        if i as f32 * FIXED_STEP > 3.0 {
            settled = v.angular_velocity().y;
        }
    }
    assert!(settled > 0.1, "settled yaw rate {settled}");
    let reached = samples.iter().position(|w| *w > 0.8 * settled).expect("never reached 80%");
    assert!(reached as f32 * FIXED_STEP < 0.8, "took {} s to reach 80% of the yaw rate", reached as f32 * FIXED_STEP);
}

#[test]
fn the_throttle_pulls_at_once() {
    let g = flat();
    let mut v = moving(VehicleSpec::example(), 25.0);
    for _ in 0..30 {
        v.step(FIXED_STEP, &InputState::default(), &g);
    }
    let v0 = v.forward_speed();
    for _ in 0..30 {
        v.step(FIXED_STEP, &throttle(), &g);
    }
    assert!(v.forward_speed() - v0 > 1.0, "gained only {} m/s in half a second", v.forward_speed() - v0);
}

#[test]
fn a_jump_floats_instead_of_slamming_down() {
    // The real data have a big aero coefficient; with the original airborne downforce a car fell about twice
    // as fast as gravity.
    let mut spec = VehicleSpec::example();
    spec.aero.aero_coefficient = 0.24;
    let mut v = Vehicle::new(spec);
    v.place_moving(Vec3::new(0.0, 2.3, 0.0), identity(), 50.0);
    let g = flat();
    let mut t = 0.0;
    while v.wheels_on_ground() == 0 && t < 5.0 {
        v.step(FIXED_STEP, &throttle(), &g);
        t += FIXED_STEP;
    }
    let ballistic = (2.0 * 1.5 / 9.81_f32).sqrt();
    assert!(t > 0.8 * ballistic, "fell in {t} s, free fall takes {ballistic} s");
}

#[test]
fn a_burnout_smokes_the_driven_wheels_only() {
    let g = flat();
    let mut v = parked();
    let (mut rear, mut front, mut rear_skid) = (0.0_f32, 0.0_f32, 0.0_f32);
    for _ in 0..90 {
        v.step(FIXED_STEP, &InputState { throttle: 1.0, handbrake: 0.0, ..Default::default() }, &g);
        rear = rear.max(v.wheel(2).smoke).max(v.wheel(3).smoke);
        rear_skid = rear_skid.max(v.wheel(2).skid);
        front = front.max(v.wheel(0).smoke).max(v.wheel(1).smoke);
    }
    assert!(rear_skid > 0.0 || rear > 0.0, "no marks at all: skid {rear_skid}, smoke {rear}");
    assert_eq!(front, 0.0, "front wheels smoked");
}

#[test]
fn cruising_in_a_straight_line_leaves_no_marks() {
    let g = flat();
    let mut v = moving(VehicleSpec::example(), 25.0);
    for _ in 0..180 {
        v.step(FIXED_STEP, &InputState { throttle: 0.4, ..Default::default() }, &g);
        for i in 0..4 {
            assert_eq!(v.wheel(i).skid, 0.0, "wheel {i} skidded");
            assert_eq!(v.wheel(i).smoke, 0.0);
        }
    }
}

#[test]
fn a_handbrake_turn_and_a_lock_up_leave_marks() {
    let g = flat();
    let mut v = moving(VehicleSpec::example(), 25.0);
    let mut marked = 0.0_f32;
    for _ in 0..120 {
        v.step(FIXED_STEP, &InputState { handbrake: 1.0, steer: 0.6, ..Default::default() }, &g);
        marked = marked.max(v.wheel(2).skid).max(v.wheel(3).skid);
    }
    assert!(marked > 0.5, "rear skid only {marked}");

    let mut hard = moving(VehicleSpec::example(), 35.0);
    let mut cornering = 0.0_f32;
    for _ in 0..180 {
        hard.step(FIXED_STEP, &InputState { throttle: 0.6, steer: 1.0, ..Default::default() }, &g);
        for i in 0..4 {
            cornering = cornering.max(hard.wheel(i).skid);
        }
    }
    assert!(cornering > 0.2, "hard corner skid only {cornering}");
}

fn turbo_spec() -> VehicleSpec {
    let mut spec = VehicleSpec::example();
    spec.induction = InductionSpec {
        low_boost: 0.3,
        high_boost: 0.6,
        spool: 0.3,
        spool_time_up: 1.0,
        spool_time_down: 0.5,
        vacuum: 0.0,
        psi: 18.0,
    };
    spec
}

#[test]
fn forced_induction_spools_and_adds_power() {
    let g = flat();
    let mut plain = moving(VehicleSpec::example(), 22.0);
    let mut turbo = moving(turbo_spec(), 22.0);
    for _ in 0..(4.0 / FIXED_STEP) as usize {
        plain.step(FIXED_STEP, &throttle(), &g);
        turbo.step(FIXED_STEP, &throttle(), &g);
    }
    assert!(turbo.boost_psi() > 5.0, "gauge {}", turbo.boost_psi());
    assert_eq!(plain.boost_psi(), 0.0);
    assert!(
        turbo.forward_speed() > plain.forward_speed() + 4.0,
        "{} vs {}",
        turbo.forward_speed(),
        plain.forward_speed()
    );
    // Lifting off lets the turbo spool down again.
    for _ in 0..120 {
        turbo.step(FIXED_STEP, &InputState::default(), &g);
    }
    assert!(turbo.boost_psi() < 2.0, "gauge after lifting: {}", turbo.boost_psi());
}

#[test]
fn nitrous_follows_the_button() {
    let g = flat();
    let mut v = moving(VehicleSpec::example(), 30.0);
    for _ in 0..30 {
        v.step(FIXED_STEP, &InputState { nos: true, ..throttle() }, &g);
    }
    assert!(v.nos_level() < 1.0, "tank untouched");
    assert!(v.powertrain().nos.is_engaged());
    for _ in 0..90 {
        v.step(FIXED_STEP, &throttle(), &g);
    }
    assert!(!v.powertrain().nos.is_engaged());
}

#[test]
fn acceleration_stays_smooth_through_every_shift() {
    let g = flat();
    let mut v = parked();
    let mut last = v.forward_speed();
    let mut last_a = 0.0;
    let mut worst_jump = 0.0_f32;
    let mut peak = 0.0_f32;
    for i in 0..(20.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &throttle(), &g);
        let a = (v.forward_speed() - last) / FIXED_STEP;
        last = v.forward_speed();
        // Skip the launch, where the wheels spin on purpose.
        if i as f32 * FIXED_STEP > 2.5 {
            worst_jump = worst_jump.max((a - last_a).abs());
            peak = peak.max(a);
        }
        last_a = a;
    }
    assert!(worst_jump < 6.0, "acceleration jumped by {worst_jump} m/s^2 in one step");
    assert!(peak < 8.0, "peak acceleration {peak}");
}
