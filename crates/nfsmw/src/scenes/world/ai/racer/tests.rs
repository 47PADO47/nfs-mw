use super::*;

#[test]
fn a_stand_in_speeds_up_by_skill_and_brakes_hard() {
    assert!((stand_in_speed(10.0, 50.0, 4.0, 80.0, 1.0, 0.5) - 12.0).abs() < 1e-5);
    assert!((stand_in_speed(10.0, 50.0, 4.0, 80.0, 0.0, 0.5) - 11.0).abs() < 1e-5, "half the pull");
    assert_eq!(stand_in_speed(49.0, 50.0, 4.0, 80.0, 1.0, 1.0), 50.0, "never past the wanted speed");
    assert_eq!(stand_in_speed(50.0, 20.0, 4.0, 80.0, 1.0, 0.5), 35.0, "30 m/s² of braking");
    assert_eq!(stand_in_speed(50.0, 49.0, 4.0, 80.0, 1.0, 0.5), 49.0);
}

#[test]
fn a_stand_in_never_beats_its_skills_share_of_the_top_speed() {
    assert_eq!(stand_in_speed(70.0, 90.0, 4.0, 80.0, 0.0, 1.0), 60.0);
    assert_eq!(stand_in_speed(70.0, 90.0, 40.0, 80.0, 1.0, 1.0), 80.0);
    assert_eq!(stand_in_speed(0.0, -5.0, 4.0, 80.0, 1.0, 1.0), 0.0, "it does not reverse");
}

fn performance(top: f32, grip: f32) -> Performance {
    Performance {
        top_speed: top,
        acceleration: [5.0; blackbox_vehicle::performance::TABLE_SIZE],
        start_grip: grip,
        end_grip: grip,
    }
}

#[test]
fn a_bias_of_zero_matches_the_racer_down_to_the_player() {
    let racer = performance(90.0, 1.4);
    let player = performance(70.0, 1.0);
    let limits = matched_limits(&racer, &AiVehicle::default(), &player, 0.0);
    assert_eq!((limits.top_speed, limits.start_grip), (70.0, 1.0));
}

#[test]
fn a_bias_of_one_keeps_the_racers_own_numbers() {
    let racer = performance(90.0, 1.4);
    let player = performance(70.0, 1.0);
    let limits = matched_limits(&racer, &AiVehicle::default(), &player, 1.0);
    assert_eq!((limits.top_speed, limits.start_grip), (90.0, 1.4));
}

#[test]
fn a_weaker_racer_is_never_raised() {
    let racer = performance(60.0, 0.9);
    let player = performance(70.0, 1.0);
    for bias in [0.0, 0.5, 1.0] {
        let limits = matched_limits(&racer, &AiVehicle::default(), &player, bias);
        assert_eq!((limits.top_speed, limits.start_grip), (60.0, 0.9), "bias {bias}");
    }
}

#[test]
fn the_skill_adds_the_catch_up_and_stays_in_range() {
    let mut state =
        RacerState::new(1, performance(80.0, 1.2), AiVehicle::default(), performance(80.0, 1.2), 1.0, 0.8, 0.0);
    state.glue = 0.5;
    assert_eq!(state.skill(), 1.0);
    assert!((state.cheat() - 0.15).abs() < 1e-6);
    state.human = Some(Default::default());
    assert_eq!(state.cheat(), 0.0);
    state.human = None;
    state.glue = -1.0;
    assert_eq!(state.skill(), 0.0);
    assert_eq!(state.cheat(), 0.0);
}

#[test]
fn catch_up_increases_level_road_acceleration_and_does_not_become_a_grade() {
    let mut state =
        RacerState::new(1, performance(80.0, 1.2), AiVehicle::default(), performance(80.0, 1.2), 0.0, 1.0, 0.0);
    let ordinary = state.action_acceleration(40.0, 0.0, false);
    state.glue = 0.75;
    let boosted = state.action_acceleration(40.0, 0.0, false);
    assert!(boosted > ordinary);
    assert!((boosted / ordinary - 1.37125).abs() < 1e-5);
    assert!(state.action_acceleration(40.0, 0.1, false) < boosted, "only the actual road slope subtracts gravity");
    state.human = Some(Default::default());
    assert_eq!(state.action_acceleration(40.0, 0.0, false), ordinary);
}

#[test]
fn native_steering_drag_applies_to_non_drag_computer_racers_only() {
    let mut state =
        RacerState::new(1, performance(80.0, 1.2), AiVehicle::default(), performance(80.0, 1.2), 0.0, 1.0, 0.0);
    assert_eq!(state.steering_drag(), Some(0.23));
    state.glue = 0.75;
    assert!((state.steering_drag().unwrap() - 0.33125).abs() < 1e-6);
    state.drag = true;
    assert_eq!(state.steering_drag(), None);
    state.drag = false;
    state.human = Some(Default::default());
    assert_eq!(state.steering_drag(), None);
}
