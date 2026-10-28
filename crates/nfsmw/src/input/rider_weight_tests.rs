//! The rider's weight of a bike (wheelies and stoppies): G and V (or Numpad 8 and 2) and the pad's right stick. A car
//! reads none of it; the axis shares the stick with the car's gear shifts, which a bike's automatic gearbox ignores.

use bevy_input::{gamepad::GamepadAxis, keyboard::KeyCode};

use super::action::Action;
use super::bindings::Source;
use super::state::{ActionState, Bindings};

fn weight_with(keys: &[KeyCode]) -> f32 {
    let mut state = ActionState::default();
    state.update_for_test(&Bindings::default(), keys, &[], false);
    state.value(Action::RiderWeight)
}

#[test]
fn g_leans_the_rider_forward_and_v_back() {
    assert_eq!(weight_with(&[]), 0.0);
    assert_eq!(weight_with(&[KeyCode::KeyG]), 1.0);
    assert_eq!(weight_with(&[KeyCode::KeyV]), -1.0);
    assert_eq!(weight_with(&[KeyCode::Numpad8]), 1.0);
    assert_eq!(weight_with(&[KeyCode::Numpad2]), -1.0);
    assert_eq!(weight_with(&[KeyCode::KeyG, KeyCode::KeyV]), 0.0, "both cancel");
}

#[test]
fn the_pads_right_stick_moves_the_weight_and_still_shifts() {
    let bindings = Bindings::default();
    let sources = |action: Action| -> Vec<Source> {
        bindings.0.iter().filter(|b| b.action == action).map(|b| b.source).collect()
    };
    assert!(sources(Action::RiderWeight).contains(&Source::PadAxis(GamepadAxis::RightStickY)));
    assert!(sources(Action::ShiftUp).contains(&Source::PadAxis(GamepadAxis::RightStickY)));
}

#[test]
fn the_weight_axis_round_trips_by_name_and_no_key_is_shared_with_another_driving_action() {
    assert_eq!(Action::parse("rider_weight"), Ok(Action::RiderWeight));
    assert_eq!(Action::RiderWeight.name(), "rider_weight");
    assert!(Action::ALL.contains(&Action::RiderWeight));
    let bindings = Bindings::default();
    for key in [KeyCode::KeyG, KeyCode::KeyV, KeyCode::Numpad8, KeyCode::Numpad2] {
        let actions: Vec<Action> =
            bindings.0.iter().filter(|b| b.source == Source::Key(key)).map(|b| b.action).collect();
        assert_eq!(actions, vec![Action::RiderWeight], "{key:?}");
    }
}
