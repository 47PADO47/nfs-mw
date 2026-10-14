use super::speedbreaker::{SpeedbreakerTuning, parse, vanilla};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn defaults_are_the_verified_vanilla_values() {
    let d = SpeedbreakerTuning::default();
    assert_eq!(d.min_speed, vanilla::MINIMUM_SPEED_MPS);
    assert_eq!(d.max_duration, Some(10.0));
    assert_eq!(d.passive.map(|p| (p.min_speed, p.seconds)), Some((vanilla::RECHARGE_ABOVE_MPS, 25.0)));
    assert_eq!((d.drift.scale, d.dilation.target), (1.0, 0.25));
    assert_eq!((d.dilation.enter_rate, d.dilation.exit_rate), (2.0, 0.5));
    assert_eq!(d.physics.car_mass_scale, 2.0);
    assert_eq!(d.physics.gravity_scale, 3.0, "three times gravity: two extra g of downforce");
    assert_eq!(d.physics.front_grip_gain, 0.75);
    assert_eq!(d.physics.max_steering_degrees, 60.0);
    assert_eq!(d.physics.aero_drag_scale, 0.25);
    assert_eq!(d.physics.slip_drag, 1.0);
}

#[test]
fn a_file_that_sets_the_vanilla_values_reads_back_the_vanilla_tuning() {
    let text = "[Activation]\nminCarSpeed = 48.2803\nmaxDuration = 10.0\n\
                [Recharging]\nminCarSpeed = 160.9344\npassiveTime = 25.0\nminDriftSpeed = 56.3265\n\
                minSlipAngle = 30.0\nactiveScale = 1.0\n\
                [Physics]\ntimeScale = 4.0\ncarMassScale = 2.0\ngravityScale = 3.0\ntoDilationScale = 1.0\n\
                fromDilationScale = 1.0\nfrontFrictionBoost = 75.0\nmaxSteeringAngle = 60.0\n\
                aeroDragReduction = 75.0\nslipGripReduction = 100.0\n";
    let t = parse(text);
    let d = SpeedbreakerTuning::default();
    assert!(close(t.min_speed, d.min_speed), "{} vs {}", t.min_speed, d.min_speed);
    assert!(close(t.dilation.target, d.dilation.target));
    assert!(close(t.drift.min_speed, d.drift.min_speed));
    assert!(close(t.drift.min_slip, d.drift.min_slip));
    assert!(close(t.physics.front_grip_gain, d.physics.front_grip_gain));
    assert!(close(t.physics.aero_drag_scale, d.physics.aero_drag_scale));
    assert!(close(t.physics.slip_drag, d.physics.slip_drag));
    assert_eq!(t.max_duration, Some(10.0));
}

#[test]
fn speeds_are_kilometres_per_hour_in_the_file_and_metres_per_second_in_the_tuning() {
    let t = parse("[Activation]\nminCarSpeed = 36.0\n");
    assert!(close(t.min_speed, 10.0), "36 km/h is 10 m/s, got {}", t.min_speed);
}

#[test]
fn an_empty_file_follows_the_mods_omission_rules() {
    let t = parse("");
    assert_eq!(t.max_duration, None, "an absent maxDuration is unlimited");
    assert_eq!(t.passive, None, "passive recharge is off when both of its keys are absent");
    assert_eq!(t.min_speed, vanilla::MINIMUM_SPEED_MPS, "other absent keys are vanilla");
}

#[test]
fn a_single_passive_key_keeps_the_other_at_vanilla() {
    let t = parse("[Recharging]\npassiveTime = 40\n");
    let passive = t.passive.expect("passive is on when one key is set");
    assert_eq!(passive.seconds, 40.0);
    assert_eq!(passive.min_speed, vanilla::RECHARGE_ABOVE_MPS);
}

#[test]
fn out_of_range_values_clamp_to_the_nearest_bound() {
    let t = parse(
        "[Physics]\ntimeScale = 0.5\nslipGripReduction = 5\naeroDragReduction = 150\n[Recharging]\nminSlipAngle = 200\n",
    );
    assert_eq!(t.dilation.target, 1.0, "timeScale clamps to 1, so the target is normal speed");
    assert!(close(t.physics.slip_drag, 0.15), "slipGripReduction clamps to 15 percent");
    assert_eq!(t.physics.aero_drag_scale, 0.0, "aeroDragReduction clamps to 100 percent");
    assert!(close(t.drift.min_slip, 90f32.to_radians()), "slip clamps to 90 degrees");
}

#[test]
fn malformed_values_fall_back_to_vanilla() {
    let t = parse("[Physics]\ncarMassScale = lots\n[Activation]\nmaxDuration = soon\n");
    assert_eq!(t.physics.car_mass_scale, 2.0);
    assert_eq!(t.max_duration, Some(10.0), "a malformed duration is vanilla, not unlimited");
}

#[test]
fn a_zero_duration_is_clamped_so_the_drain_stays_finite() {
    let t = parse("[Activation]\nmaxDuration = 0\n");
    let secs = t.max_duration.expect("set");
    assert!(secs > 0.0 && secs.is_finite(), "{secs}");
}

#[test]
fn gravity_scale_below_one_is_clamped_so_the_downforce_never_lifts_the_car() {
    assert_eq!(parse("[Physics]\ngravityScale = 0.2\n").physics.gravity_scale, 1.0);
}

#[test]
fn a_timescale_of_eight_halves_the_dilation_again() {
    assert!(close(parse("[Physics]\ntimeScale = 8\n").dilation.target, 0.125));
}

#[test]
fn a_slower_entry_scale_slows_only_the_entry_rate() {
    let t = parse("[Physics]\ntoDilationScale = 2\n");
    assert!(close(t.dilation.enter_rate, 1.0));
    assert_eq!(t.dilation.exit_rate, 0.5);
}
