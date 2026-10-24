//! The client mode's pieces that need no world: the car handle outside a lend, the input edges, the state mapping and
//! which entities are drawn as cars. The predicted car against the real world is tested in `crate::net::tests`.

use nfsmw_net::predict::{CarState, InputState, PredictedCar};
use nfsmw_sim::{EntityKind, EntityView};

use super::frame::{car_pose, with_edges, without_edges};
use super::remote::is_car;
use super::{GameCar, motion_of, state_of};
use crate::scenes::world::net::flags::FLAG_ROADBLOCK_CAR;

fn state(x: f32) -> CarState {
    CarState {
        position: [x, 2.0, 3.0],
        orientation: [0.0, 0.0, 0.382_683_43, 0.923_879_5],
        linear_velocity: [10.0, -1.0, 0.5],
        angular_velocity: [0.0, 0.0, 0.25],
        rpm: 4200.0,
        gear: 3,
        nitrous: 0.5,
        boost: 0.0,
    }
}

#[test]
fn outside_a_lend_the_car_keeps_its_state_and_a_step_does_nothing() {
    let mut car = GameCar::new();
    assert!(!car.is_lent());
    car.set_state(&state(1.0));
    assert!(car.has_pending(), "a state set with no world waits for the next lend");
    car.step(&InputState { throttle: 1.0, ..InputState::default() });
    assert_eq!(car.state(), state(1.0));
    assert_eq!(car.steps(), 0);
}

#[test]
fn save_and_restore_outside_a_lend_bring_the_state_back() {
    let mut car = GameCar::new();
    car.set_state(&state(1.0));
    let saved = car.save();
    car.set_state(&state(9.0));
    car.restore(&saved);
    assert_eq!(car.state(), state(1.0));
}

#[test]
fn clones_share_one_car() {
    let mut car = GameCar::new();
    let template = car.clone();
    car.set_state(&state(4.0));
    assert_eq!(template.state(), state(4.0), "the session's predictor and the scene see the same car");
}

#[test]
fn a_state_survives_the_trip_through_a_motion() {
    let state = state(5.0);
    assert_eq!(state_of(&motion_of(&state), state.boost), state);
}

#[test]
fn edges_go_with_the_first_tick_and_wait_when_no_tick_is_due() {
    let held = InputState { shift_up: true, ..InputState::default() };
    let frame = InputState { throttle: 0.8, gear_select: Some(2), ..InputState::default() };
    let first = with_edges(frame, &held);
    assert!(first.shift_up && first.gear_select == Some(2) && first.throttle == 0.8);
    let next = without_edges(first);
    assert!(!next.shift_up && !next.shift_down && next.gear_select.is_none());
    assert_eq!(next.throttle, 0.8, "the axes go with every tick");
}

#[test]
fn a_session_pose_draws_at_its_place_and_turn() {
    let pose = car_pose([1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 2.0]);
    assert_eq!(pose.position, glam::Vec3::new(1.0, 2.0, 3.0));
    assert!((pose.rotation.length() - 1.0).abs() < 1e-6, "the turn is normalised");
}

#[test]
fn cars_are_drawn_with_rigs_and_the_rest_is_not() {
    let view = |kind, flags| EntityView {
        id: 20,
        kind,
        owner: None,
        position: [0.0; 3],
        orientation: [0.0, 0.0, 0.0, 1.0],
        linear_velocity: [0.0; 3],
        angular_velocity: [0.0; 3],
        type_id: 1,
        flags,
    };
    for kind in [EntityKind::PlayerCar, EntityKind::TrafficCar, EntityKind::PoliceCar] {
        assert!(is_car(&view(kind, 0)));
    }
    assert!(is_car(&view(EntityKind::Roadblock, FLAG_ROADBLOCK_CAR)));
    assert!(!is_car(&view(EntityKind::Roadblock, 0)), "a spike strip");
    assert!(!is_car(&view(EntityKind::Helicopter, 0)) && !is_car(&view(EntityKind::MovingProp, 0)));
}

#[test]
fn the_clock_follows_the_server_only_when_it_has_strayed_also_across_midnight() {
    use super::hud::strayed;
    assert!(!strayed(720.0, 724.0));
    assert!(strayed(720.0, 730.0));
    assert!(!strayed(1438.0, 2.0), "four minutes across midnight");
    assert!(strayed(1430.0, 20.0), "thirty minutes across midnight");
}
