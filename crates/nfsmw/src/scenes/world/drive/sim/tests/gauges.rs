use super::*;

#[test]
fn gauges_use_engine_display_rpm_and_physical_speed_instead_of_raw_telemetry() {
    let ground = FlatGround::new(0.0);
    let mut car = sim();
    assert!(car.place(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }));
    car.configure_player_launch(false);
    car.set_race_staging(true);
    assert!(car.staged_launch_band().is_some());
    let mut distinct = false;
    for tick in 0..240 {
        let input = match tick < 144 {
            true => DriveInput { throttle: 1.0, ..Default::default() },
            false => DriveInput { brake: 1.0, ..Default::default() },
        };
        car.step_with_delta(1.0 / 144.0, &input, &ground, None);
        let gauge = car.gauge_sample();
        assert_eq!(gauge.rpm, car.vehicle.powertrain().rpm_display());
        assert_eq!(gauge.speed, car.vehicle.speed());
        distinct |= gauge.rpm != car.telemetry().rpm;
    }
    assert!(distinct, "the test must exercise different raw and displayed RPM");
    // A held brake can select reverse in this fixture; only first gear has a launch band.
    assert_eq!(car.staged_launch_band().is_some(), car.vehicle.gear() == blackbox_vehicle::drivetrain::GEAR_FIRST);
    car.set_race_staging(false);
    assert!(car.staged_launch_band().is_none());
}
