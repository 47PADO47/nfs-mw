use super::*;

const INERTIA: Vec3 = Vec3::new(120.0, 90.0, 40.0);

fn body(omega: Vec3) -> BodyFrame {
    BodyFrame { rotation: Mat3::IDENTITY, angular_velocity: omega, inertia: INERTIA }
}

fn none() -> Attitude {
    Attitude::default()
}

#[test]
fn an_untouched_body_is_only_damped_and_never_pushed_to_spin() {
    let spec = AirControlSpec::BIKE;
    assert_eq!(spec.torque(AirAxes::ZERO, &body(Vec3::ZERO), &none()), Vec3::ZERO);
    let torque = spec.torque(AirAxes::ZERO, &body(Vec3::new(1.0, -1.0, 0.5)), &none());
    assert!(torque.x < 0.0 && torque.y > 0.0 && torque.z < 0.0, "the torque opposes the spin: {torque:?}");
}

#[test]
fn the_stick_forward_pitches_the_nose_down_and_the_stick_right_rolls_the_right_side_down() {
    let spec = AirControlSpec::CAR;
    let forward = spec.torque(AirAxes { pitch: 1.0, ..AirAxes::ZERO }, &body(Vec3::ZERO), &none());
    assert!(forward.x > 0.0 && forward.y == 0.0 && forward.z == 0.0, "{forward:?}");
    let back = spec.torque(AirAxes { pitch: -1.0, ..AirAxes::ZERO }, &body(Vec3::ZERO), &none());
    assert!(back.x < 0.0);
    let right = spec.torque(AirAxes { roll: 1.0, ..AirAxes::ZERO }, &body(Vec3::ZERO), &none());
    assert!(right.z < 0.0 && right.x == 0.0, "{right:?}");
    let spin = spec.torque(AirAxes { yaw: 1.0, ..AirAxes::ZERO }, &body(Vec3::ZERO), &none());
    assert!(spin.y > 0.0 && spin.x == 0.0 && spin.z == 0.0, "{spin:?}");
}

#[test]
fn the_angular_acceleration_is_bounded_whatever_the_gap_and_scales_with_the_inertia() {
    for spec in [AirControlSpec::CAR, AirControlSpec::BIKE] {
        let all = AirAxes { pitch: 1.0, roll: 1.0, yaw: 1.0 };
        let torque = spec.torque(all, &body(Vec3::new(-30.0, -30.0, 30.0)), &none());
        assert!((torque.x / INERTIA.x - spec.pitch.accel).abs() < 1e-3, "{torque:?}");
        assert!((torque.y / INERTIA.y - spec.yaw.accel).abs() < 1e-3);
        assert!((-torque.z / INERTIA.z - spec.roll.accel).abs() < 1e-3);
        let heavy = BodyFrame { inertia: INERTIA * 4.0, ..body(Vec3::new(-30.0, -30.0, 30.0)) };
        let again = spec.torque(all, &heavy, &none());
        assert!((again - torque * 4.0).length() < 1e-2, "the same acceleration for a body four times as heavy");
    }
}

#[test]
fn an_input_does_not_push_an_axis_that_already_turns_faster_than_it_asks() {
    let spec = AirControlSpec::BIKE;
    let fast = spec.pitch.rate * 2.0;
    let torque = spec.torque(AirAxes { pitch: 1.0, ..AirAxes::ZERO }, &body(Vec3::new(fast, 0.0, 0.0)), &none());
    assert!(torque.x < 0.0, "only the damping acts, it never adds to the spin: {torque:?}");
    let against = spec.torque(AirAxes { pitch: -1.0, ..AirAxes::ZERO }, &body(Vec3::new(fast, 0.0, 0.0)), &none());
    assert!(against.x < 0.0, "an input against the spin brakes it");
}

#[test]
fn a_full_input_reaches_its_rate_and_holds_it_without_overshoot() {
    for spec in [AirControlSpec::CAR, AirControlSpec::BIKE] {
        let mut omega = Vec3::ZERO;
        let mut peak = 0.0_f32;
        for _ in 0..240 {
            let torque = spec.torque(AirAxes { pitch: 1.0, ..AirAxes::ZERO }, &body(omega), &none());
            omega += torque / INERTIA / 60.0;
            peak = peak.max(omega.x);
        }
        assert!((omega.x - spec.pitch.rate).abs() < 0.05 * spec.pitch.rate, "{} against {}", omega.x, spec.pitch.rate);
        assert!(peak <= spec.pitch.rate * 1.02, "no overshoot: {peak}");
        // And let go, it settles.
        for _ in 0..600 {
            omega += spec.torque(AirAxes::ZERO, &body(omega), &none()) / INERTIA / 60.0;
        }
        assert!(omega.length() < 0.05, "{omega:?}");
    }
}

#[test]
fn a_half_input_asks_half_the_rate() {
    let spec = AirControlSpec::BIKE;
    let mut omega = Vec3::ZERO;
    for _ in 0..240 {
        omega += spec.torque(AirAxes { yaw: 0.5, ..AirAxes::ZERO }, &body(omega), &none()) / INERTIA / 60.0;
    }
    assert!((omega.y - 0.5 * spec.yaw.rate).abs() < 0.05, "{}", omega.y);
}

#[test]
fn the_push_fades_out_towards_the_attitude_limits_and_not_away_from_them() {
    let spec = AirControlSpec::BIKE;
    let limits = Attitude { pitch_limit: 1.0, lean_limit: 1.0, ..Attitude::default() };
    let at = |pitch_up: f32, lean_right: f32| Attitude { pitch_up, lean_right, ..limits };
    let push = |attitude: &Attitude, pitch: f32, roll: f32| {
        spec.torque(AirAxes { pitch, roll, yaw: 0.0 }, &body(Vec3::ZERO), attitude)
    };
    // Nose down at the limit: the stick forward does nothing, the stick back still lifts the nose.
    assert_eq!(push(&at(-1.0, 0.0), 1.0, 0.0).x, 0.0);
    assert!(push(&at(-1.0, 0.0), -1.0, 0.0).x < 0.0);
    // Nose up at the limit: the opposite.
    assert_eq!(push(&at(1.0, 0.0), -1.0, 0.0).x, 0.0);
    assert!(push(&at(1.0, 0.0), 1.0, 0.0).x > 0.0);
    // Inside half the limit the push is whole; between, it is part.
    let whole = push(&at(0.0, 0.0), -1.0, 0.0).x;
    assert_eq!(push(&at(0.49, 0.0), -1.0, 0.0).x, whole);
    let part = push(&at(0.75, 0.0), -1.0, 0.0).x;
    assert!(part.abs() < whole.abs() && part != 0.0, "{part} {whole}");
    // The same in roll: leaning right at the limit, the stick right adds no roll, the stick left rolls back.
    assert_eq!(push(&at(0.0, 1.0), 0.0, 1.0).z, 0.0);
    assert!(push(&at(0.0, 1.0), 0.0, -1.0).z > 0.0);
}

#[test]
fn a_body_turned_over_takes_the_torque_about_its_own_axes() {
    // Rolled a quarter turn about its forward axis: its right axis points up, so a pitch input turns it about the world's y axis.
    let rotation = Mat3::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let frame = BodyFrame { rotation, angular_velocity: Vec3::ZERO, inertia: INERTIA };
    let torque = AirControlSpec::CAR.torque(AirAxes { pitch: 1.0, ..AirAxes::ZERO }, &frame, &none());
    assert!(torque.y > 0.0 && torque.x.abs() < 1e-3, "{torque:?}");
}

#[test]
fn bad_input_reads_as_none() {
    let axes = AirAxes { pitch: f32::NAN, roll: 9.0, yaw: f32::INFINITY }.clamped();
    assert_eq!(axes, AirAxes { pitch: 0.0, roll: 1.0, yaw: 0.0 });
    assert!(!AirAxes::ZERO.is_active() && axes.is_active());
    let torque = AirControlSpec::CAR.torque(AirAxes { pitch: f32::NAN, ..AirAxes::ZERO }, &body(Vec3::ZERO), &none());
    assert!(torque.is_finite());
}

#[test]
fn a_body_turning_into_an_attitude_limit_is_braked_and_not_left_to_drift_on() {
    let spec = AirControlSpec::BIKE;
    let at_limit = Attitude { pitch_up: -1.0, pitch_limit: 1.0, ..Attitude::default() };
    let turning = body(Vec3::new(1.5, 0.0, 0.0));
    let braked = spec.torque(AirAxes { pitch: 1.0, ..AirAxes::ZERO }, &turning, &at_limit);
    let drifting = spec.torque(AirAxes::ZERO, &turning, &at_limit);
    assert!(braked.x < 0.0 && braked.x < drifting.x, "{} against the damping's {}", braked.x, drifting.x);
}
