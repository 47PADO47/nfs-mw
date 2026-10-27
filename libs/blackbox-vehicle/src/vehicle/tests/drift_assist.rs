//! The drift assist on the example car with the generic tuning.

use super::*;

fn drive(assist: bool) -> (f32, bool) {
    let g = flat();
    let mut v = Vehicle::new(VehicleSpec::example());
    v.config.auto_reverse = false;
    v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 25.0);
    v.set_drift_assist(assist);
    for _ in 0..60 {
        v.step(FIXED_STEP, &InputState::default(), &g);
    }
    let (mut peak, mut drifted) = (0.0_f32, false);
    for (seconds, input) in [
        (0.4, InputState { throttle: 1.0, steer: 0.6, ..Default::default() }),
        (1.2, InputState { throttle: 1.0, steer: 0.6, handbrake: 1.0, ..Default::default() }),
        (1.5, InputState { throttle: 1.0, steer: 0.6, ..Default::default() }),
    ] {
        for _ in 0..(seconds / FIXED_STEP).round() as usize {
            v.step(FIXED_STEP, &input, &g);
            peak = peak.max(v.slip_angle().to_degrees().abs());
            drifted |= v.assist_drifting();
        }
    }
    (peak, drifted)
}

#[test]
fn the_generic_tuning_holds_a_drift_with_the_handbrake() {
    let (peak, drifted) = drive(true);
    assert!(drifted && peak > 7.5, "{peak}");
}

#[test]
fn the_assist_is_off_by_default_and_reports_no_drift() {
    let v = Vehicle::new(VehicleSpec::example());
    assert!(!v.drift_assist() && !v.assist_drifting());
    let (_, drifted) = drive(false);
    assert!(!drifted);
}
