//! The drift assist on a car built from its stats: an all-wheel-drive coupe with a rear bias, whose ordinary physics keeps
//! it from sliding (a handbrake and a turned wheel at 100 km/h bring its slip angle to a few degrees and no further).

use glam::Vec3;

use super::*;
use crate::DriftTuning;
use crate::FIXED_STEP;
use crate::ground::FlatGround;
use crate::input::InputState;
use crate::tires::TireIntegration;
use crate::vehicle::Vehicle;

const BOX: Vec3 = Vec3::new(0.9, 0.55, 2.2);

/// A 1600 kg coupe with 600 N m, drive to all four wheels with a quarter at the front and both differentials half locked,
/// and rear tyres that hold a little less than the front (the build raises them just above the front).
fn awd_coupe() -> CarStats {
    CarStats {
        mass_kg: 1598.0,
        front_weight_pct: 50.0,
        cg_height_m: 0.387,
        wheelbase_m: 2.66,
        track_m: [1.618, 1.63],
        tire_radius_m: [0.343, 0.353],
        tire_width_m: [0.256, 0.296],
        ride_height_m: 0.131,
        idle_rpm: 1000.0,
        redline_rpm: 9200.0,
        torque_curve: vec![
            [1000.0, 300.0],
            [2000.0, 520.0],
            [3000.0, 600.0],
            [4000.0, 598.0],
            [5000.0, 580.0],
            [6100.0, 562.0],
            [7000.0, 520.0],
            [8000.0, 480.0],
            [9200.0, 450.0],
        ],
        gear_ratios: vec![3.82, 2.26, 1.64, 1.29, 1.06, 0.88],
        reverse_ratio: 2.86,
        final_drive: 3.89,
        drive: Drive::All,
        front_torque_share: 0.25,
        diff_lock: [0.5, 0.5],
        top_speed: Some(87.2),
        tire_grip: [1.35, 1.25],
        brake_torque_nm: [1750.0, 1600.0],
        drift: DriftStats { enter_angle_deg: 8.5, yaw_torque_nm: 37_500.0, ..DriftStats::default() },
        ..CarStats::default()
    }
}

/// What a run measured.
struct Run {
    /// The largest slip angle, degrees.
    peak: f32,
    /// The assist counted the car as drifting at some step.
    drifted: bool,
    /// The slip angle at the end, degrees.
    last: f32,
    /// The state of the car at the end, for comparing two runs.
    end: (Vec3, Vec3),
}

/// Drive `stats` at `speed` m/s through: a turn with the handbrake held, the turn held, then the wheel straight.
fn run(stats: &CarStats, speed: f32, steer: f32, assist: Option<bool>) -> Run {
    let mut spec = VehicleSpec::from_stats(stats, BOX);
    spec.rest_at(0.13);
    let ground = FlatGround::new(0.0);
    let mut car = Vehicle::new(spec);
    car.set_tire_integration(TireIntegration::Reference);
    car.config.auto_reverse = false;
    assert!(car.place_on_ground_moving(&ground, 0.0, 0.0, 5.0, 0.0, speed));
    if let Some(on) = assist {
        car.set_drift_assist(on);
    }
    for _ in 0..90 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
    }
    let script = [
        (0.4, InputState { throttle: 1.0, steer, ..Default::default() }),
        (1.2, InputState { throttle: 1.0, steer, handbrake: 1.0, ..Default::default() }),
        (2.5, InputState { throttle: 1.0, steer, ..Default::default() }),
        (1.5, InputState { throttle: 1.0, ..Default::default() }),
    ];
    let (mut peak, mut drifted) = (0.0_f32, false);
    for (seconds, input) in &script {
        for _ in 0..(seconds / FIXED_STEP).round() as usize {
            car.step(FIXED_STEP, input, &ground);
            peak = peak.max(car.slip_angle().to_degrees().abs());
            drifted |= car.assist_drifting();
        }
    }
    Run { peak, drifted, last: car.slip_angle().to_degrees().abs(), end: (car.position(), car.linear_velocity()) }
}

#[test]
fn the_coupe_does_not_slide_on_its_own() {
    let off = run(&awd_coupe(), 28.0, 0.5, None);
    assert!(off.peak < 7.5, "peak slip {} deg", off.peak);
    assert!(!off.drifted);
}

#[test]
fn with_the_assist_the_handbrake_and_the_wheel_start_a_drift_that_the_wheel_holds() {
    for speed in [20.0, 28.0] {
        let on = run(&awd_coupe(), speed, 0.5, Some(true));
        assert!(on.drifted, "no drift at {speed} m/s");
        assert!(on.peak > 8.5, "peak slip {} deg at {speed} m/s", on.peak);
        assert!(on.peak < 25.0, "half lock must not spin the car: {} deg", on.peak);
        assert!(on.last < 3.0, "the straight wheel ends the drift: {} deg", on.last);
    }
}

#[test]
fn full_lock_holds_a_bigger_angle_than_half_lock() {
    let half = run(&awd_coupe(), 28.0, 0.5, Some(true));
    let full = run(&awd_coupe(), 28.0, 1.0, Some(true));
    assert!(full.peak > half.peak + 3.0, "{} {}", full.peak, half.peak);
}

#[test]
fn the_assist_does_nothing_without_the_handbrake() {
    let mut spec = VehicleSpec::from_stats(&awd_coupe(), BOX);
    spec.rest_at(0.13);
    let ground = FlatGround::new(0.0);
    let mut car = Vehicle::new(spec);
    car.set_tire_integration(TireIntegration::Reference);
    car.place_on_ground_moving(&ground, 0.0, 0.0, 5.0, 0.0, 28.0);
    car.set_drift_assist(true);
    for _ in 0..300 {
        car.step(FIXED_STEP, &InputState { throttle: 1.0, steer: 0.6, ..Default::default() }, &ground);
        assert!(!car.assist_drifting());
    }
}

#[test]
fn an_assist_turned_off_again_leaves_the_car_as_it_was() {
    let never = run(&awd_coupe(), 28.0, 0.5, None);
    let off = run(&awd_coupe(), 28.0, 0.5, Some(false));
    assert_eq!(never.end, off.end);
    let mut toggled = Vehicle::new(VehicleSpec::from_stats(&awd_coupe(), BOX));
    toggled.set_drift_assist(true);
    assert!(toggled.drift_assist());
    toggled.set_drift_assist(false);
    assert!(!toggled.drift_assist() && !toggled.assist_drifting());
}

#[test]
fn the_stats_make_the_tuning() {
    let spec = VehicleSpec::from_stats(&awd_coupe(), BOX);
    let tuning = spec.drift;
    assert!((tuning.enter_angle.to_degrees() - 8.5).abs() < 1e-3);
    assert!((tuning.exit_angle.to_degrees() - 3.0).abs() < 1e-3);
    assert_eq!(tuning.enter_speed, 10.0);
    // 37.5 kN m on this body's inertia is a few radians a second squared, inside the range a car may have.
    assert!(tuning.yaw_accel > 3.0 && tuning.yaw_accel < 10.0, "{}", tuning.yaw_accel);
    // A car with no torque in its stats gets the generic strength; a huge torque is held to the range.
    let generic = VehicleSpec::from_stats(&CarStats::default(), BOX).drift;
    let typical = DriftTuning::GENERIC;
    assert_eq!(
        (generic.yaw_accel, generic.enter_speed, generic.rear_grip),
        (typical.yaw_accel, 10.0, typical.rear_grip)
    );
    assert!(
        (generic.max_angle - typical.max_angle).abs() < 1e-5
            && (generic.enter_angle - typical.enter_angle).abs() < 1e-5
    );
    let strong =
        CarStats { drift: DriftStats { yaw_torque_nm: 1.0e7, ..DriftStats::default() }, ..CarStats::default() };
    assert_eq!(VehicleSpec::from_stats(&strong, BOX).drift.yaw_accel, 10.0);
}
