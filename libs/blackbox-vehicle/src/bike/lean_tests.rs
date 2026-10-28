use super::*;
use crate::bike::BikeSpec;

const DT: f32 = 1.0 / 60.0;

fn input(stick: f32, lean: f32, speed: f32) -> LeanInput {
    LeanInput {
        stick,
        speed,
        lean,
        lean_rate: 0.0,
        lateral_grip: 1.3,
        lateral_g: 0.0,
        heading_error: 0.0,
        pitch_cut: (0.0, 0.0),
        roll_inertia: 47.8,
        mass: 183.0,
        cg_height: 0.8,
        front_share: 0.5,
        dt: DT,
    }
}

#[test]
fn the_lean_limit_follows_the_grip_and_is_capped_by_the_spec() {
    let steering = BikeSpec::example().steering;
    let weak = lean_limit(&steering, &LeanInput { lateral_grip: 0.3, ..input(1.0, 0.0, 30.0) });
    assert!((weak - 0.3_f32.atan()).abs() < 1e-5);
    let strong = lean_limit(&steering, &LeanInput { lateral_grip: 50.0, ..input(1.0, 0.0, 30.0) });
    assert!(strong <= steering.max_lean + 1e-6);
}

#[test]
fn the_lean_limit_falls_at_a_standstill_with_the_lateral_g_and_in_a_skid() {
    let steering = BikeSpec::example().steering;
    let base = lean_limit(&steering, &input(1.0, 0.0, 30.0));
    assert!((lean_limit(&steering, &input(1.0, 0.0, 0.0)) - steering.lean.stopped_limit).abs() < 1e-6);
    assert!(lean_limit(&steering, &input(1.0, 0.0, 4.0)) < base);
    assert!(lean_limit(&steering, &LeanInput { lateral_g: 1.1, ..input(1.0, 0.0, 30.0) }) < base * 0.95);
    assert!(lean_limit(&steering, &LeanInput { heading_error: 0.9, ..input(1.0, 0.0, 30.0) }) < base);
}

#[test]
fn a_wheelie_pulls_the_lean_limit_down() {
    let steering = BikeSpec::example().steering;
    let base = lean_limit(&steering, &input(1.0, 0.0, 30.0));
    let cut = lean_limit(&steering, &LeanInput { pitch_cut: (0.02, 1.0), ..input(1.0, 0.0, 30.0) });
    assert!((cut - 0.02).abs() < 1e-6 && cut < base);
}

#[test]
fn an_upright_bike_with_the_stick_centred_asks_for_nothing() {
    let steering = BikeSpec::example().steering;
    let plan = Lean::new().plan(&steering, &input(0.0, 0.0, 20.0));
    assert!(plan.acceleration.abs() < 1e-4 && plan.front_force.abs() < 1e-2, "{plan:?}");
}

#[test]
fn leaning_right_asks_first_for_a_push_to_the_left_and_then_for_the_force_of_the_turn() {
    let steering = BikeSpec::example().steering;
    let mut lean = Lean::new();
    let first = lean.plan(&steering, &input(1.0, 0.0, 20.0));
    assert!(first.acceleration > 0.0 && first.front_force < 0.0, "{first:?}");
    // Held at the target lean, the acceleration is gone and the front wants its share of the centripetal force.
    let mut settled = first;
    for _ in 0..600 {
        let at = settled.target;
        settled = lean.plan(&steering, &input(1.0, at, 20.0));
    }
    assert!(settled.acceleration.abs() < 0.5, "{settled:?}");
    assert!(settled.front_force > 0.0, "{settled:?}");
    let expected = 0.5 * 183.0 * 9.81 * settled.target.tan();
    assert!((settled.front_force - expected).abs() < 0.1 * expected, "{} {expected}", settled.front_force);
}

#[test]
fn a_bike_leaning_past_its_target_is_pushed_back() {
    let steering = BikeSpec::example().steering;
    let plan = Lean::new().plan(&steering, &input(0.0, 0.3, 20.0));
    assert!(plan.acceleration < 0.0 && plan.front_force > 0.0, "{plan:?}");
}

#[test]
fn the_slip_for_a_force_has_the_sign_of_the_countersteer() {
    let tyre = BikeSpec::example().front.tyre;
    let left = slip_for_force(&tyre, 1.0, 0.0, -300.0);
    let right = slip_for_force(&tyre, 1.0, 0.0, 300.0);
    assert!(left > 0.0 && right < 0.0, "{left} {right}");
    // The formula gives back the force asked for.
    let back = tyre.lateral(1.0, left.to_degrees(), 0.0);
    assert!((back - 300.0).abs() < 5.0, "{back}");
}

#[test]
fn the_slip_for_a_force_beyond_the_peak_is_the_slip_of_the_peak() {
    let tyre = BikeSpec::example().front.tyre;
    let a = slip_for_force(&tyre, 1.0, 0.0, -1.0e6);
    let b = slip_for_force(&tyre, 1.0, 0.0, -2.0e6);
    assert!((a - b).abs() < 1e-4 && a > 0.0);
}

#[test]
fn the_steer_solve_finds_the_angle_of_a_linear_geometry() {
    let steer = solve_steer(0.45, 0.7, |delta| 0.9 * delta);
    assert!((steer - 0.5).abs() < 1e-3);
    assert_eq!(solve_steer(5.0, 0.7, |delta| delta), 0.7);
}

#[test]
fn the_handlebar_goes_to_the_stick_at_a_walking_pace_and_to_the_controller_at_speed() {
    let steering = BikeSpec::example().steering;
    assert_eq!(blend_to_stick(&steering, 0.5, 0.5, -0.3), 0.5 * steering.max_angle);
    assert!((blend_to_stick(&steering, 10.0, 0.5, -0.3) + 0.3).abs() < 1e-6);
    let half = blend_to_stick(&steering, 3.0, 0.0, 0.4);
    assert!((half - 0.2).abs() < 1e-6);
}
