use super::*;

#[test]
fn a_human_grid_launch_uses_staged_revs_and_resets_on_placement_configuration() {
    let ground = FlatGround::new(5.0);
    let mut car = sim();
    assert!(car.place_moving(&ground, spawn_at(0.0), 0.0));
    car.configure_player_launch(false);
    car.set_race_staging(true);
    let input = DriveInput { throttle: 1.0, ..Default::default() };
    let position = car.vehicle.position();
    let mut launched = false;
    for _ in 0..1200 {
        car.step(&input, &ground, None);
        assert_eq!(car.vehicle.position().x, position.x);
        assert_eq!(car.vehicle.position().z, position.z);
        assert_eq!(car.vehicle.gear(), blackbox_vehicle::drivetrain::GEAR_FIRST, "staging blocks automatic shifts");
        assert_eq!(car.vehicle.launch_strength(), 0.0, "the boost is unavailable before GO");
        let mut candidate = car.vehicle.clone();
        candidate.launch();
        candidate.set_staging(false);
        if candidate.launch_strength() > 0.0 {
            car.launch_from_grid();
            launched = true;
            break;
        }
    }
    assert!(launched, "revving must cross the independently verified launch band");
    assert!(!car.vehicle.staging());
    assert!(car.vehicle.launch_strength() > 0.5 && car.vehicle.launch_strength() < 1.0);
    car.configure_player_launch(false);
    assert_eq!(car.vehicle.launch_strength(), 0.0, "a new placement cannot reuse a launch");
    assert!(!car.vehicle.staging());
}

#[test]
fn unconfigured_ai_does_not_stage_or_launch() {
    let mut car = sim();
    car.set_race_staging(true);
    assert!(!car.vehicle.staging());
    car.launch_from_grid();
    assert_eq!(car.vehicle.launch_strength(), 0.0);
}
