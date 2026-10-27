use crate::{EngineMachine, EngineParams, Event, Input, Phase};

const DT: f32 = 0.1;

fn machine() -> EngineMachine {
    EngineMachine::new(EngineParams { idle_seconds: 2.0, still_speed: 0.5, restart_throttle: 0.1, crank_seconds: 0.45 })
}

fn idle() -> Input {
    Input { dt: DT, speed: 0.0, throttle: 0.0, active: true }
}

/// Step `count` times with `input`, collecting the events that are not `None`.
fn run(machine: &mut EngineMachine, input: Input, count: usize) -> Vec<Event> {
    (0..count).map(|_| machine.step(input)).filter(|event| *event != Event::None).collect()
}

#[test]
fn a_car_that_stands_still_stops_after_the_idle_time() {
    let mut engine = machine();
    assert!(run(&mut engine, idle(), 19).is_empty(), "1.9 s is not enough");
    assert!(engine.is_running());
    assert_eq!(run(&mut engine, idle(), 2), [Event::Stopped]);
    assert_eq!(engine.phase(), Phase::Off);
    assert!(!engine.is_running());
}

#[test]
fn moving_or_pressing_the_throttle_restarts_the_clock() {
    let mut engine = machine();
    run(&mut engine, idle(), 15);
    run(&mut engine, Input { speed: 3.0, ..idle() }, 1);
    assert_eq!(engine.phase(), Phase::Running { still: 0.0 });
    run(&mut engine, idle(), 15);
    run(&mut engine, Input { throttle: 0.5, ..idle() }, 1);
    assert_eq!(engine.phase(), Phase::Running { still: 0.0 });
    assert!(run(&mut engine, idle(), 15).is_empty());
}

#[test]
fn reversing_is_moving_and_the_brake_does_not_count() {
    let mut engine = machine();
    assert!(run(&mut engine, Input { speed: -2.0, ..idle() }, 40).is_empty());
    assert_eq!(run(&mut engine, idle(), 21), [Event::Stopped], "the brake is not an input of the rule");
}

#[test]
fn the_throttle_starts_a_stopped_engine_after_the_crank() {
    let mut engine = machine();
    run(&mut engine, idle(), 21);
    assert!(run(&mut engine, idle(), 10).is_empty(), "an engine that is off stays off with no pedal");
    let pressed = Input { throttle: 0.6, ..idle() };
    assert_eq!(run(&mut engine, pressed, 1), [Event::Cranking { seconds: 0.45 }]);
    assert!(!engine.is_running(), "cranking is not running");
    assert!(run(&mut engine, pressed, 3).is_empty());
    assert_eq!(run(&mut engine, pressed, 2), [Event::Running]);
    assert!(engine.is_running());
}

#[test]
fn a_light_touch_below_the_threshold_does_not_start_it() {
    let mut engine = machine();
    run(&mut engine, idle(), 21);
    assert!(run(&mut engine, Input { throttle: 0.05, ..idle() }, 10).is_empty());
    assert_eq!(engine.phase(), Phase::Off);
}

#[test]
fn a_started_engine_waits_the_full_idle_time_again() {
    let mut engine = machine();
    run(&mut engine, idle(), 21);
    run(&mut engine, Input { throttle: 1.0, ..idle() }, 8);
    assert!(engine.is_running());
    assert!(run(&mut engine, idle(), 15).is_empty());
    assert_eq!(run(&mut engine, idle(), 6), [Event::Stopped]);
}

#[test]
fn the_rule_lets_go_when_it_stops_applying() {
    let mut engine = machine();
    run(&mut engine, idle(), 21);
    let race = Input { active: false, ..idle() };
    assert_eq!(run(&mut engine, race, 1), [Event::Resumed]);
    assert!(engine.is_running());
    assert!(run(&mut engine, race, 100).is_empty(), "a race never stops the engine");
    let mut starting = machine();
    run(&mut starting, idle(), 21);
    run(&mut starting, Input { throttle: 1.0, ..idle() }, 1);
    assert_eq!(run(&mut starting, race, 1), [Event::Resumed]);
}

#[test]
fn bad_numbers_fall_back_to_the_defaults() {
    let params =
        EngineParams { idle_seconds: f32::NAN, still_speed: -1.0, restart_throttle: 7.0, crank_seconds: f32::INFINITY };
    let clean = params.sanitised();
    let defaults = EngineParams::default();
    assert_eq!(clean.idle_seconds, defaults.idle_seconds);
    assert_eq!(clean.still_speed, defaults.still_speed);
    assert_eq!(clean.restart_throttle, 1.0);
    assert_eq!(clean.crank_seconds, defaults.crank_seconds);
    let mut engine = EngineMachine::new(params);
    assert_eq!(engine.step(Input { dt: f32::NAN, ..idle() }), Event::None);
}
