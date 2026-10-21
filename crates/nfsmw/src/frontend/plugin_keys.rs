//! The keys the front end reads each frame: the pad mask the real input makes, the typed text of a screen that takes it,
//! and the key a remap slot waits for (`plugin.rs`).

use bevy_ecs::prelude::*;
use bevy_input::ButtonInput;
use bevy_input::ButtonState;
use bevy_input::keyboard::{Key, KeyCode, KeyboardInput};
use blackbox_feng::runtime::pad;

use super::name_entry::TextKey;
use crate::input::{Action, ActionState, Bindings};
use crate::settings::Settings;

/// The keys typed since the front end last looked, for the screens that take text (`name_entry.rs`).
#[derive(Resource, Default)]
pub(super) struct TypedKeys(pub(super) Vec<TextKey>);

/// Collects the typed characters and Backspace of this frame.
pub(super) fn collect_typed(mut events: MessageReader<KeyboardInput>, mut typed: ResMut<TypedKeys>) {
    for event in events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match &event.logical_key {
            Key::Character(text) => typed.0.extend(text.chars().map(TextKey::Char)),
            Key::Backspace => typed.0.push(TextKey::Backspace),
            _ => {}
        }
    }
}

/// Puts a key in a slot and writes the bindings to the config file. A missing config path only keeps the change for
/// this run.
pub(super) fn remember_key(bindings: &mut Bindings, control: crate::input::Control, slot: usize, key: KeyCode) {
    bindings.set_keyboard_slot(control, slot, key);
    let Some(path) = Settings::config_path() else { return };
    if let Err(e) = bindings.save_to(&path) {
        log::warn!("the key could not be saved to {}: {e:#}", path.display());
    }
}

/// The pad mask the real input makes this frame.
pub(super) fn pad_mask(actions: &ActionState) -> u32 {
    [
        (Action::MenuUp, pad::UP),
        (Action::MenuDown, pad::DOWN),
        (Action::MenuLeft, pad::LEFT),
        (Action::MenuRight, pad::RIGHT),
        (Action::MenuAccept, pad::ACCEPT),
        (Action::MenuBack, pad::BACK),
        (Action::MenuStart, pad::START),
        // Key 1 is Restart on both post-race screens: the lap table's button 1 and the photo's button 5.
        (Action::MenuButton1, (pad::BUTTON0 << 1) | (pad::BUTTON0 << 5)),
        (Action::MenuButton2, pad::BUTTON0 << 4),
        (Action::MenuButton3, pad::BUTTON0),
        (Action::MenuTriggerLeft, pad::LTRIGGER),
        (Action::MenuTriggerRight, pad::RTRIGGER),
    ]
    .into_iter()
    .filter(|(action, _)| actions.pressed(*action))
    .fold(0, |mask, (_, bit)| mask | bit)
}

/// The pad mask while a screen takes text: Enter accepts and Escape goes back; the letters and Backspace are text.
pub(super) fn text_mask(keys: &ButtonInput<KeyCode>) -> u32 {
    let mut mask = 0;
    if keys.pressed(KeyCode::Enter) || keys.pressed(KeyCode::NumpadEnter) {
        mask |= pad::ACCEPT;
    }
    if keys.pressed(KeyCode::Escape) {
        mask |= pad::BACK;
    }
    mask
}
