//! The in-world Interact button: Enter on the keyboard and the pad's South (A) face button, which it shares with
//! Handbrake, as Space shares Menu accept and Handbrake.

use bevy_input::{gamepad::GamepadButton, keyboard::KeyCode};

use super::action::Action;
use super::bindings::Source;
use super::state::Bindings;

fn sources_for(bindings: &Bindings, action: Action) -> Vec<Source> {
    bindings.0.iter().filter(|b| b.action == action).map(|b| b.source).collect()
}

#[test]
fn interact_is_enter_and_the_pad_south_button() {
    assert_eq!(
        sources_for(&Bindings::default(), Action::Interact),
        vec![Source::Key(KeyCode::Enter), Source::PadButton(GamepadButton::South)]
    );
}

#[test]
fn the_pad_south_button_also_handbrakes() {
    let bindings = Bindings::default();
    assert!(sources_for(&bindings, Action::Handbrake).contains(&Source::PadButton(GamepadButton::South)));
}

#[test]
fn interact_round_trips_by_name() {
    assert_eq!(Action::parse("interact"), Ok(Action::Interact));
    assert_eq!(Action::Interact.name(), "interact");
    assert!(Action::ALL.contains(&Action::Interact));
}
