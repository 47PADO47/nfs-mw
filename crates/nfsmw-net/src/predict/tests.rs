use blackbox_vehicle::InputState;
use nfsmw_sim::toy::Car;

use super::*;
use crate::input::InputCode;

fn toy() -> ToyCar {
    ToyCar::new(Car::new([0.0, 0.0], [0.0, 1.0], 1.0, 60.0), 500.0)
}

fn drive(tick: u32) -> InputState {
    let t = tick as f32 / 60.0;
    InputState {
        throttle: 0.7 + 0.3 * (t * 1.3).sin(),
        steer: 0.5 * (t * 0.7).sin(),
        nos: (t * 0.4).sin() > 0.8,
        handbrake: f32::from(u8::from((t * 0.9).sin() > 0.95)),
        ..InputState::default()
    }
}

#[test]
fn a_step_quantises_the_input_before_using_it() {
    let mut predictor = Predictor::new(toy(), 10);
    let input = InputState { throttle: 0.123_456, steer: -0.333, ..InputState::default() };
    let code = predictor.step(&input).unwrap();
    assert_eq!(code, InputCode::quantise(&input));
    let mut reference = toy();
    reference.step(&code.to_input());
    assert_eq!(predictor.car().car, reference.car);
    let entry = predictor.entry(10).unwrap();
    assert_eq!(entry.input, code);
    assert_eq!(entry.state, reference.state());
    assert_eq!(predictor.next_tick(), 11);
}

#[test]
fn confirm_drops_older_ticks() {
    let mut predictor = Predictor::new(toy(), 0);
    for tick in 0..20 {
        predictor.step(&drive(tick));
    }
    predictor.confirm(12);
    assert_eq!(predictor.oldest(), Some(12));
    assert_eq!(predictor.unacknowledged(), 8);
    assert_eq!(predictor.acked(), Some(12));
    predictor.confirm(5);
    assert_eq!(predictor.acked(), Some(12), "an older confirmation never moves the ack back");
}

#[test]
fn a_full_ring_stops_prediction() {
    let mut predictor = Predictor::with_capacity(toy(), 0, 4);
    for tick in 0..4 {
        assert!(predictor.step(&drive(tick)).is_some());
    }
    assert!(predictor.is_stalled());
    assert!(predictor.step(&drive(4)).is_none());
    assert_eq!(predictor.next_tick(), 4);
    predictor.confirm(2);
    assert!(predictor.step(&drive(4)).is_some());
}

#[test]
fn frozen_inputs_are_neutral() {
    let mut predictor = Predictor::new(toy(), 0);
    predictor.set_frozen(true);
    let code = predictor.step(&InputState { throttle: 1.0, ..InputState::default() }).unwrap();
    assert_eq!(code, InputCode::NEUTRAL);
    assert_eq!(predictor.car().car.speed, 0.0);
}

/// sync-model.md section 10, test 1: rewind and replay equals a straight run, bit for bit.
#[test]
fn rewind_and_replay_to_the_same_state_equals_a_straight_run() {
    let mut straight = Predictor::with_capacity(toy(), 0, 1024);
    for tick in 0..600 {
        straight.step(&drive(tick));
    }
    let mut replayed = Predictor::with_capacity(toy(), 0, 1024);
    for tick in 0..600 {
        replayed.step(&drive(tick));
        if tick % 7 == 0 && tick > 20 {
            let at = tick - 13;
            let state = replayed.entry(at).unwrap().state;
            replayed.rewind_replay(at, &state);
        }
    }
    assert_eq!(straight.car().car, replayed.car().car);
    for (a, b) in straight.entries().zip(replayed.entries()) {
        assert_eq!((a.tick, a.state), (b.tick, b.state));
    }
}

#[test]
fn a_rewind_to_a_moved_state_carries_the_move_forward() {
    let mut predictor = Predictor::new(toy(), 100);
    for tick in 100..130 {
        predictor.step(&drive(tick));
    }
    let mut moved = predictor.entry(110).unwrap().state;
    moved.position[0] += 1.0;
    let before = predictor.car().car.pos;
    predictor.rewind_replay(110, &moved);
    let after = predictor.car().car.pos;
    assert!((after[0] - before[0] - 1.0).abs() < 0.05, "{before:?} -> {after:?}");
    assert_eq!(predictor.entry(110).unwrap().state.position[0], moved.position[0]);
    assert_eq!(predictor.next_tick(), 130);
}

#[test]
fn a_rewind_past_the_newest_tick_restarts_after_it() {
    let mut predictor = Predictor::new(toy(), 0);
    for tick in 0..5 {
        predictor.step(&drive(tick));
    }
    let state = CarState { position: [3.0, 0.0, 4.0], ..CarState::default() };
    predictor.rewind_replay(9, &state);
    assert_eq!(predictor.next_tick(), 10);
    assert_eq!(predictor.unacknowledged(), 0);
    assert_eq!(predictor.car().car.pos, [3.0, 4.0]);
}

#[test]
fn dropped_ticks_are_replayed_with_the_held_input() {
    let mut predictor = Predictor::new(toy(), 0);
    let throttle = InputState { throttle: 1.0, ..InputState::default() };
    for _ in 0..10 {
        predictor.step(&throttle);
    }
    predictor.skip_to(15);
    for _ in 0..5 {
        predictor.step(&throttle);
    }
    let start = predictor.entry(3).unwrap().state;
    predictor.rewind_replay(3, &start);
    let mut straight = toy();
    for _ in 0..20 {
        straight.step(&InputCode::quantise(&throttle).to_input());
    }
    assert_eq!(predictor.car().car, straight.car);
}

#[test]
fn the_toy_state_round_trips() {
    let mut car = toy();
    for tick in 0..300 {
        car.step(&drive(tick));
    }
    let state = car.state();
    let mut copy = toy();
    copy.set_state(&state);
    let back = copy.state();
    for axis in 0..3 {
        assert!((back.position[axis] - state.position[axis]).abs() < 1.0e-4);
        assert!((back.linear_velocity[axis] - state.linear_velocity[axis]).abs() < 1.0e-3);
    }
    assert!(crate::geom::angle_between(back.orientation, state.orientation) < 1.0e-3);
    assert_eq!((back.nitrous, back.boost), (state.nitrous, state.boost));
}
