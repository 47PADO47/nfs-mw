//! The Extra Options headlights hotkey replaces only the keyboard binding of `ToggleHeadlights`.

use bevy_input::keyboard::KeyCode;

use super::action::Action;
use super::bindings::Source;
use super::state::Bindings;

fn keys_for(bindings: &Bindings, action: Action) -> Vec<Source> {
    bindings.0.iter().filter(|b| b.action == action).map(|b| b.source).collect()
}

#[test]
fn the_default_headlights_key_is_h() {
    assert_eq!(keys_for(&Bindings::default(), Action::ToggleHeadlights), vec![Source::Key(KeyCode::KeyH)]);
}

#[test]
fn rebinding_replaces_the_key_and_keeps_the_pad_button() {
    let mut bindings = Bindings::default();
    let pad = keys_for(&bindings, Action::ToggleCamera).len();
    bindings.rebind_key(Action::ToggleHeadlights, KeyCode::KeyJ);
    assert_eq!(keys_for(&bindings, Action::ToggleHeadlights), vec![Source::Key(KeyCode::KeyJ)]);
    assert_eq!(keys_for(&bindings, Action::ToggleCamera).len(), pad, "other actions are untouched");
}
