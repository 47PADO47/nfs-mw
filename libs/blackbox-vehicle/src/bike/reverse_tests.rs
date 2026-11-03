use crate::{FIXED_STEP, FlatGround};

use super::super::{Bike, BikeInput, BikeSpec};
use super::*;

fn spec() -> ReverseSpec {
    ReverseSpec { force: 1000.0, max_speed: 2.5, hold_time: 0.5 }
}

fn held(front_brake: f32) -> BikeInput {
    BikeInput { front_brake, rear_brake: front_brake, ..BikeInput::default() }
}

#[test]
fn nothing_happens_until_the_brake_has_been_held_for_the_hold_time() {
    let mut state = ReverseState::default();
    let mut drive = ReverseDrive::default();
    for _ in 0..40 {
        drive = state.step(&spec(), 0.01, 0.0, &held(1.0));
    }
    assert_eq!(drive, ReverseDrive::default(), "0.4 s of 0.5 s");
    assert!(!state.pushing());
    for _ in 0..20 {
        drive = state.step(&spec(), 0.01, 0.0, &held(1.0));
    }
    assert!(state.pushing());
    assert!(drive.release);
    assert!((drive.force + 1000.0).abs() < 1e-3, "full force, backwards: {drive:?}");
}

#[test]
fn the_force_follows_the_lever_and_dies_away_towards_the_speed_limit() {
    let mut state = ReverseState::default();
    for _ in 0..60 {
        state.step(&spec(), 0.01, 0.0, &held(1.0));
    }
    let force = |state: &mut ReverseState, speed: f32, lever: f32| state.step(&spec(), 0.01, speed, &held(lever)).force;
    assert!((force(&mut state, 0.0, 0.5) + 500.0).abs() < 1e-3, "half the lever, half the force");
    assert!((force(&mut state, -1.25, 1.0) + 500.0).abs() < 1e-3, "half the limit, half the force");
    assert!(force(&mut state, -2.5, 1.0).abs() < 1e-3, "no force at the limit");
    assert!(force(&mut state, -3.75, 1.0) > 0.0, "past the limit it pushes back towards it");
}

#[test]
fn a_throttle_a_moving_bike_or_a_light_lever_is_no_request_and_resets_the_timer() {
    for (speed, throttle, lever) in [(3.0, 0.0, 1.0), (0.0, 0.5, 1.0), (0.0, 0.0, 0.05)] {
        let mut state = ReverseState::default();
        for _ in 0..40 {
            state.step(&spec(), 0.01, 0.0, &held(1.0));
        }
        let input = BikeInput { throttle, front_brake: lever, ..BikeInput::default() };
        assert_eq!(state.step(&spec(), 0.01, speed, &input), ReverseDrive::default());
        // The timer started again from 0: 0.4 s more of the brake is not enough.
        for _ in 0..40 {
            assert_eq!(state.step(&spec(), 0.01, 0.0, &held(1.0)), ReverseDrive::default());
        }
    }
}

#[test]
fn the_levers_are_let_go_only_while_the_rider_pushes() {
    assert_eq!(ReverseDrive::default().levers(0.7, 0.4), (0.7, 0.4));
    assert_eq!(ReverseDrive { force: -10.0, release: true }.levers(0.7, 0.4), (0.0, 0.0));
}

#[test]
fn a_spec_with_a_zero_or_missing_number_is_no_reverse() {
    let mut state = ReverseState::default();
    let broken = ReverseSpec { max_speed: 0.0, ..spec() };
    assert!(!broken.is_valid());
    for _ in 0..200 {
        assert_eq!(state.step(&broken, 0.01, 0.0, &held(1.0)), ReverseDrive::default());
    }
}

fn run(spec: Option<ReverseSpec>, seconds: f32, input: &BikeInput) -> Bike {
    let ground = FlatGround::new(0.0);
    let mut bike = Bike::new(BikeSpec { reverse: spec, ..BikeSpec::example() });
    bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0);
    for _ in 0..(seconds / FIXED_STEP) as usize {
        bike.step(FIXED_STEP, input, &ground);
    }
    bike
}

#[test]
fn a_bike_with_the_brake_held_rolls_back_after_the_hold_time_and_settles_near_the_limit() {
    let spec = ReverseSpec { force: 1000.0, max_speed: 2.5, hold_time: 0.5 };
    let early = run(Some(spec), 0.4, &held(1.0));
    assert!(early.forward_speed().abs() < 0.05, "still held: {}", early.forward_speed());
    assert!(!early.reversing());
    let later = run(Some(spec), 4.0, &held(1.0));
    assert!(later.reversing());
    let speed = later.forward_speed();
    assert!(speed < -1.0 && speed > -2.8, "walking pace backwards: {speed}");
    assert!(later.lean().abs() < 0.2, "the rider keeps it up: {}", later.lean());
    // The bike went backwards, along its own axis.
    assert!(later.position().z < -1.0 && later.position().x.abs() < 0.5, "{:?}", later.position());
}

#[test]
fn a_bike_without_a_reverse_stays_where_the_brake_holds_it() {
    let bike = run(None, 4.0, &held(1.0));
    assert!(bike.forward_speed().abs() < 0.05);
    assert!(!bike.reversing());
}
