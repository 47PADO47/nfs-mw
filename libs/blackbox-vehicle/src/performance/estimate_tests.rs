use super::*;

fn car() -> VehicleSpec {
    let mut spec = VehicleSpec::example();
    spec.mass = 1000.0;
    spec.engine.torque = vec![100.0; 5];
    spec.engine.idle = 1000.0;
    spec.engine.max_rpm = 5000.0;
    spec.engine.red_line = 4000.0;
    spec.induction = Default::default();
    spec.transmission.gear_ratio = vec![3.0, 0.0, 4.0, 2.0, 1.0];
    spec.transmission.gear_efficiency = vec![1.0; 5];
    spec.transmission.final_gear = 1.0;
    spec.tires.rim_size = [1.0 / 0.0254; 2];
    spec.tires.section_width = [0.0; 2];
    spec.aero.drag_coefficient = 0.0;
    spec.aero.aero_coefficient = 0.0;
    spec
}

#[test]
fn gear_knots_use_force_over_mass_and_clamp_below_the_first_gear_speed() {
    let spec = car();
    let p = estimate_ai(&spec).unwrap();
    let radius = spec.tires.radius(1);
    let first_speed = 4000.0 * std::f32::consts::TAU / 60.0 * radius / 4.0;
    let first_acceleration = 100.0 * FT_LB_TO_NM * 4.0 / radius / 1000.0;
    assert!((p.top_speed - first_speed * 4.0).abs() < 1e-4);
    assert!((p.acceleration[0] - first_acceleration).abs() < 1e-5);
    assert!((p.acceleration[1] - first_acceleration).abs() < 1e-5);
    assert!((p.acceleration[TABLE_SIZE - 1] - first_acceleration / 4.0).abs() < 1e-5);
}

#[test]
fn zero_acceleration_crossing_limits_speed_and_the_last_table_sample() {
    let mut spec = car();
    spec.aero.drag_coefficient = 0.1;
    let p = estimate_ai(&spec).unwrap();
    let no_drag = estimate_ai(&car()).unwrap();
    let first_speed = no_drag.top_speed / 4.0;
    let second_speed = first_speed * 2.0;
    let first_acceleration = no_drag.acceleration[0] - 0.1 * first_speed.powi(2) / spec.mass;
    let second_acceleration = no_drag.acceleration[0] * 0.5 - 0.1 * second_speed.powi(2) / spec.mass;
    let weight = 1.0 - first_acceleration / (first_acceleration - second_acceleration);
    let expected = first_speed + (second_speed - first_speed) * weight;
    assert!((p.top_speed - expected).abs() < 1e-4, "retain the shipped complementary interpolation");
    assert!(p.top_speed > no_drag.top_speed / 4.0 && p.top_speed < no_drag.top_speed);
    assert_eq!(p.acceleration[TABLE_SIZE - 1], 0.0);
    assert!(p.acceleration.iter().all(|a| a.is_finite() && *a >= 0.0));
}

#[test]
fn average_from_peak_uses_full_induction_and_excludes_redline_samples() {
    let mut spec = car();
    spec.engine.torque = vec![100.0, 200.0, 150.0, 125.0, 50.0];
    assert!((average_inducted_torque(&spec).unwrap() / FT_LB_TO_NM - 175.0).abs() < 1e-4);
    spec.induction.low_boost = 1.0;
    spec.induction.high_boost = 1.0;
    assert!((average_inducted_torque(&spec).unwrap() / FT_LB_TO_NM - 350.0).abs() < 1e-4);
}

#[test]
fn estimator_uses_the_rear_radius_and_is_independent_of_physical_shift_delays() {
    let mut spec = car();
    let original = estimate_ai(&spec).unwrap();
    spec.tires.rim_size[0] *= 1.5;
    spec.transmission.shift_speed = 100.0;
    spec.engine.flywheel_mass = 100.0;
    assert_eq!(estimate_ai(&spec), Some(original));
    spec.tires.rim_size[1] *= 2.0;
    let rear = estimate_ai(&spec).unwrap();
    assert!((rear.top_speed / original.top_speed - 2.0).abs() < 1e-5);
    assert!((rear.acceleration[0] / original.acceleration[0] - 0.5).abs() < 1e-5);
}

#[test]
fn invalid_or_non_propulsive_data_requests_a_measured_fallback() {
    let mut spec = car();
    spec.mass = 0.0;
    assert!(estimate_ai(&spec).is_none());
    spec = car();
    spec.engine.torque.clear();
    assert!(estimate_ai(&spec).is_none());
    spec = car();
    spec.transmission.gear_ratio.truncate(GEAR_FIRST);
    assert!(estimate_ai(&spec).is_none());
    spec = car();
    spec.aero.drag_coefficient = f32::NAN;
    assert!(estimate_ai(&spec).is_none());
}
