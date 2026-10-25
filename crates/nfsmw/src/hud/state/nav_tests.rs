use super::*;

#[test]
fn a_junction_to_the_left_is_a_left_turn_at_its_distance() {
    // Render space: x forward, y left. The route runs 100 m along x, then turns towards +y at the junction.
    let path = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    let turn = next_turn(&path, &[1], [10.0, 0.0]).unwrap();
    assert_eq!(turn.direction, TurnDirection::Left);
    assert!((turn.distance_m - 90.0).abs() < 1e-3, "{turn:?}");
}

#[test]
fn a_junction_to_the_right_is_a_right_turn() {
    let path = [[0.0, 0.0], [100.0, 0.0], [100.0, -100.0]];
    assert_eq!(next_turn(&path, &[1], [0.0, 0.0]).unwrap().direction, TurnDirection::Right);
}

#[test]
fn a_route_that_doubles_back_is_a_u_turn() {
    let path = [[0.0, 0.0], [100.0, 0.0], [0.0, 2.0]];
    assert_eq!(next_turn(&path, &[1], [0.0, 0.0]).unwrap().direction, TurnDirection::UTurn);
}

#[test]
fn a_shallow_junction_and_a_bend_inside_a_segment_are_not_turns() {
    // A shallow fork at the junction: straight on to the end.
    let path = [[0.0, 0.0], [100.0, 0.0], [200.0, 20.0]];
    let turn = next_turn(&path, &[1], [0.0, 0.0]).unwrap();
    assert_eq!(turn.direction, TurnDirection::Straight);
    assert!((turn.distance_m - (100.0 + 100.0f32.hypot(20.0))).abs() < 1e-3);
    // A sharp bend that is not at a junction is the road's own curve.
    let bend = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    assert_eq!(next_turn(&bend, &[], [0.0, 0.0]).unwrap().direction, TurnDirection::Straight);
}

#[test]
fn a_junction_behind_the_car_is_passed() {
    let path = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    // The car is on the second piece, past the left turn at point 1: the next one is the left turn at point 2.
    let turn = next_turn(&path, &[1, 2], [100.0, 30.0]).unwrap();
    assert_eq!(turn.direction, TurnDirection::Left);
    assert!((turn.distance_m - 70.0).abs() < 1e-3, "{turn:?}");
}

#[test]
fn a_route_too_short_has_no_cue() {
    assert_eq!(next_turn(&[], &[], [0.0, 0.0]), None);
    assert_eq!(next_turn(&[[1.0, 1.0]], &[], [0.0, 0.0]), None);
}

#[test]
fn without_a_route_the_hud_has_none() {
    let state = crate::hud::HudState::default();
    assert_eq!(state.route, None);
    assert!(state.markers.is_empty());
}
