use super::*;
use crate::bike::BikeSpec;

fn input(weight: f32, pitch: f32, rate: f32) -> PitchInput {
    PitchInput {
        weight,
        throttle: 1.0,
        front_brake: 0.0,
        pitch,
        pitch_rate: rate,
        terrain: 0.0,
        speed: 10.0,
        normalised_lean: 0.0,
        front_on_ground: true,
        rear_on_ground: true,
        inertia: 80.0,
        dt: 1.0 / 60.0,
    }
}

/// A stoppie asks with the front brake and the throttle shut.
fn stoppie(weight: f32, pitch: f32, rate: f32) -> PitchInput {
    PitchInput { throttle: 0.0, front_brake: 1.0, ..input(weight, pitch, rate) }
}

fn spec() -> WheelieSpec {
    BikeSpec::example().wheelie.unwrap()
}

#[test]
fn the_dead_zones_swallow_a_small_lever_travel() {
    let s = spec();
    assert_eq!(target_position(&s, 0.0), 0.0);
    assert_eq!(target_position(&s, -0.2), 0.0);
    assert!(target_position(&s, -0.65) < 0.0 && target_position(&s, -0.65) > -1.0);
    assert_eq!(target_position(&s, 0.9), 0.0);
    assert_eq!(target_position(&s, 1.0), 1.0);
    assert_eq!(target_position(&s, -1.0), -1.0);
}

#[test]
fn weight_back_with_the_throttle_raises_the_nose_and_weight_forward_with_the_brake_lowers_it() {
    let s = spec();
    assert!(torque(&s, &input(-1.0, 0.0, 0.0)) > 0.0);
    assert!(torque(&s, &stoppie(1.0, 0.0, 0.0)) < 0.0);
}

#[test]
fn the_pedal_sets_how_far_the_nose_is_asked_to_go() {
    let s = spec();
    // At the angle half the throttle asks for the assist neither pushes nor pulls; below it pushes, above it pulls.
    let half = PitchInput { throttle: 0.5, ..input(-1.0, 0.0, 0.0) };
    let target = 0.5 * s.wheelie.max_angle;
    assert!(torque(&s, &PitchInput { pitch: target + 0.1, ..half }) < 0.0);
    assert!(torque(&s, &PitchInput { pitch: target - 0.1, ..half }) > 0.0);
    assert!(torque(&s, &PitchInput { pitch: target, ..half }).abs() < 1.0);
    // With the throttle shut the target is level and the assist holds the nose down.
    assert!(torque(&s, &PitchInput { throttle: 0.0, pitch: 0.2, ..half }) < 0.0);
}

#[test]
fn the_assist_holds_the_angle_it_was_asked_for() {
    let s = spec();
    let target = s.wheelie.max_angle;
    assert!(torque(&s, &input(-1.0, target, 0.0)).abs() < 1.0);
    assert!(torque(&s, &input(-1.0, target + 0.2, 0.0)) < 0.0);
}

#[test]
fn the_assist_needs_the_right_wheel_on_the_road_and_the_other_pedal_free() {
    let s = spec();
    assert_eq!(torque(&s, &PitchInput { rear_on_ground: false, ..input(-1.0, 0.0, 0.0) }), 0.0);
    assert_eq!(torque(&s, &PitchInput { front_brake: 0.5, ..input(-1.0, 0.0, 0.0) }), 0.0);
    assert_eq!(torque(&s, &PitchInput { throttle: 0.5, ..stoppie(1.0, 0.0, 0.0) }), 0.0);
    assert_eq!(torque(&s, &PitchInput { front_on_ground: false, ..stoppie(1.0, 0.0, 0.0) }), 0.0);
}

#[test]
fn the_assist_fades_when_leaned_over_and_is_off_at_a_crawl() {
    let s = spec();
    let upright = torque(&s, &input(-1.0, 0.0, 0.0));
    assert!(torque(&s, &PitchInput { normalised_lean: 1.0, ..input(-1.0, 0.0, 0.0) }) < upright * 0.01);
    assert_eq!(torque(&s, &PitchInput { speed: 0.0, ..input(-1.0, 0.0, 0.0) }), 0.0);
}

#[test]
fn the_torque_is_bounded_by_the_accelerations_the_tracker_allows() {
    let s = spec();
    let t = torque(&s, &input(-1.0, -1.0, -5.0));
    assert!(t.abs() <= 80.0 * s.wheelie.acceleration + 1e-3);
}

#[test]
fn the_road_slope_is_taken_off_the_pitch_of_a_wheelie_but_not_of_a_stoppie() {
    let s = spec();
    // Uphill the body pitch includes the slope; the wheelie target is measured from it.
    let slope = 0.2;
    let level = torque(&s, &input(-1.0, 0.0, 0.0));
    let uphill = torque(&s, &PitchInput { terrain: slope, pitch: slope, ..input(-1.0, 0.0, 0.0) });
    assert!((level - uphill).abs() < 1e-3, "{level} {uphill}");
    let flat = torque(&s, &stoppie(1.0, 0.0, 0.0));
    let tilted = torque(&s, &PitchInput { terrain: slope, ..stoppie(1.0, 0.0, 0.0) });
    assert!((flat - tilted).abs() < 1e-3);
}

#[test]
fn the_tracker_follows_a_time_optimal_plan() {
    // From rest, 1 rad to go at 10 rad/s^2: accelerate half way then decelerate; the first command is the full limit.
    let a = track(0.0, 0.0, 1.0, 10.0, 0.005);
    assert!((a - 10.0).abs() < 1e-3, "{a}");
    // Past the target and still moving away: the command is the full limit the other way.
    assert!((track(1.2, 1.0, 1.0, 10.0, 0.005) + 10.0).abs() < 1e-3);
    // On the target at rest nothing is commanded.
    assert_eq!(track(1.0, 0.0, 1.0, 10.0, 0.005), 0.0);
    // Half way there with the speed to stop at the target: the command is the braking one.
    let half = (2.0 * 10.0 * 0.5_f32).sqrt();
    assert!(track(0.5, half, 1.0, 10.0, 0.005) < -9.0);
    // The window smooths the switch: a plan that switches inside it commands a mean between the two limits.
    let mean = track(0.4998, (2.0 * 10.0 * 0.4998_f32).sqrt(), 1.0, 10.0, 0.1);
    assert!(mean.abs() < 10.0 - 1e-3, "{mean}");
}

#[test]
fn the_tracker_brings_a_double_integrator_to_rest_on_the_target() {
    let (mut x, mut v) = (0.0_f32, 0.0_f32);
    let dt = 1.0 / 240.0;
    for _ in 0..(3.0 / dt) as usize {
        let a = track(x, v, 0.6, 10.0, 0.1);
        v += a * dt;
        x += v * dt;
    }
    assert!((x - 0.6).abs() < 0.01 && v.abs() < 0.05, "{x} {v}");
}

#[test]
fn the_square_root_approach_arrives() {
    assert_eq!(approach(0.0, 1.0, 2.0), 1.0);
    let mut x = 0.0;
    for _ in 0..200 {
        x = approach(x, 1.0, 0.02);
    }
    assert_eq!(x, 1.0);
    assert!(approach(0.0, 1.0, 0.1) > 0.0 && approach(0.0, 1.0, 0.1) < 1.0);
    assert!(approach(2.0, 1.0, 0.1) < 2.0 && approach(2.0, 1.0, 0.1) > 1.0);
}

#[test]
fn the_stoppie_target_is_filtered_towards_its_value() {
    let s = spec();
    assert!(s.stoppie.filter_timescale > 0.0 && s.wheelie.filter_timescale == 0.0);
}
