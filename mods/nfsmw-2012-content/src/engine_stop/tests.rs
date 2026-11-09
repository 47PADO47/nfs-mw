use chameleon_engine_state::EngineParams;

use super::{Action, Controller, Sample};
use crate::ignition;

const DT: f32 = 0.1;

fn controller() -> Controller {
    Controller::new(EngineParams { idle_seconds: 1.0, crank_seconds: 0.45, ..EngineParams::default() })
}

fn idle() -> Sample {
    Sample { dt: DT, speed: 0.0, throttle: 0.0, brake: 0.0, free_roam: true }
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

#[test]
fn a_fresh_brake_press_from_a_stop_starts_the_engine_like_the_throttle() {
    let mut controller = controller();
    run(&mut controller, true, idle(), "POR911CARR12", 12);
    let brake = Sample { brake: 1.0, ..idle() };
    assert_eq!(
        run(&mut controller, true, brake, "POR911CARR12", 4),
        [Action::Start { sound: "SOUND/MODENGINE/POR911CARR12/IGNITION.WAV".to_owned(), seconds: 0.45 }],
        "one start with the car's ignition sound"
    );
}

#[test]
fn the_delay_setting_picks_how_long_a_standing_vehicle_waits() {
    for &(id, _, seconds) in &super::DELAYS {
        let mut controller = controller();
        controller.set_delay(seconds);
        let steps = (seconds / DT) as usize;
        assert!(run(&mut controller, true, idle(), "KTM450", steps - 3).is_empty(), "{id} s: too early");
        assert_eq!(run(&mut controller, true, idle(), "KTM450", 6), [Action::Stop], "{id} s: one stop");
    }
}

#[test]
fn the_delay_values_are_plain_ids_the_first_is_the_old_five_seconds_and_a_wrong_id_is_the_default() {
    assert_eq!(super::DELAYS[0].2, 5.0);
    assert!(super::DELAYS.windows(2).all(|pair| pair[1].2 > pair[0].2));
    for (id, label, seconds) in super::DELAYS {
        assert!(id.bytes().all(|b| b.is_ascii_digit()), "{id}");
        assert!(label.starts_with("nfsmw2012-engine-delay-"));
        assert_eq!(super::delay_seconds(id), seconds);
    }
    assert_eq!(super::delay_seconds("7"), 5.0);
    assert_eq!(super::delay_seconds(""), 5.0);
}
