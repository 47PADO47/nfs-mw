use chameleon_engine_state::EngineParams;

use super::{Action, Controller, Sample};
use crate::ignition;

const DT: f32 = 0.1;

fn controller() -> Controller {
    Controller::new(EngineParams { idle_seconds: 1.0, still_speed: 0.5, restart_throttle: 0.1, crank_seconds: 0.45 })
}

fn idle() -> Sample {
    Sample { dt: DT, speed: 0.0, throttle: 0.0, free_roam: true }
}

fn own(car: &str) -> bool {
    car == "POR911CARR12"
}

/// Step `count` times and collect the actions.
fn run(controller: &mut Controller, enabled: bool, sample: Sample, car: &str, count: usize) -> Vec<Action> {
    (0..count).filter_map(|_| controller.step(enabled, sample, car, own)).collect()
}

#[test]
fn a_standing_car_stops_and_the_throttle_starts_it_with_its_own_sound() {
    let mut controller = controller();
    assert!(run(&mut controller, true, idle(), "POR911CARR12", 9).is_empty());
    assert_eq!(run(&mut controller, true, idle(), "POR911CARR12", 3), [Action::Stop]);
    let pressed = Sample { throttle: 0.7, ..idle() };
    assert_eq!(
        run(&mut controller, true, pressed, "POR911CARR12", 1),
        [Action::Start { sound: "SOUND/MODENGINE/POR911CARR12/IGNITION.WAV".to_owned(), seconds: 0.45 }]
    );
}

#[test]
fn a_car_without_a_recording_of_its_own_uses_the_shared_one() {
    let mut controller = controller();
    run(&mut controller, true, idle(), "BMWM3GTR", 12);
    let pressed = Sample { throttle: 0.7, ..idle() };
    assert_eq!(
        run(&mut controller, true, pressed, "BMWM3GTR", 1),
        [Action::Start { sound: ignition::SHARED_PATH.to_owned(), seconds: 0.45 }]
    );
}

#[test]
fn nothing_happens_while_the_setting_is_off() {
    let mut controller = controller();
    assert!(run(&mut controller, false, idle(), "POR911CARR12", 100).is_empty());
}

#[test]
fn turning_the_setting_off_starts_a_stopped_engine_without_a_sound() {
    let mut controller = controller();
    run(&mut controller, true, idle(), "POR911CARR12", 12);
    assert_eq!(run(&mut controller, false, idle(), "POR911CARR12", 3), [Action::Resume], "once");
}

#[test]
fn a_race_or_a_chase_starts_a_stopped_engine_and_keeps_it_running() {
    let mut controller = controller();
    run(&mut controller, true, idle(), "POR911CARR12", 12);
    let event = Sample { free_roam: false, ..idle() };
    assert_eq!(run(&mut controller, true, event, "POR911CARR12", 50), [Action::Resume]);
}

#[test]
fn a_new_car_starts_with_a_running_engine() {
    let mut controller = controller();
    run(&mut controller, true, idle(), "POR911CARR12", 12);
    assert_eq!(run(&mut controller, true, idle(), "BMWM3GTR", 1), [Action::Resume]);
    assert!(run(&mut controller, true, idle(), "BMWM3GTR", 5).is_empty(), "and its clock starts again");
}
