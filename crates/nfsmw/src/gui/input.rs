//! Collecting Bevy's keyboard, mouse and window messages into egui events.

use bevy_ecs::prelude::*;
use bevy_input::ButtonInput;
use bevy_input::ButtonState;
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::mouse::{MouseButton, MouseButtonInput, MouseScrollUnit, MouseWheel};
use bevy_window::CursorMoved;

use super::keys::egui_key;

/// The egui events gathered since the last UI pass.
#[derive(Resource, Default)]
pub struct GuiInput {
    pub events: Vec<egui::Event>,
    pub modifiers: egui::Modifiers,
    pub pointer: Option<egui::Pos2>,
}

fn pointer_button(b: MouseButton) -> Option<egui::PointerButton> {
    match b {
        MouseButton::Left => Some(egui::PointerButton::Primary),
        MouseButton::Right => Some(egui::PointerButton::Secondary),
        MouseButton::Middle => Some(egui::PointerButton::Middle),
        _ => None,
    }
}

/// Text worth typing: printable characters only (no control codes, no Delete).
fn printable(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| !c.is_control())
}

pub fn collect(
    mut input: ResMut<GuiInput>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut buttons: MessageReader<MouseButtonInput>,
    mut wheel: MessageReader<MouseWheel>,
    mut moved: MessageReader<CursorMoved>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let modifiers = egui::Modifiers {
        alt: keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
        ctrl,
        shift: keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        command: ctrl,
        mac_cmd: false,
    };
    input.modifiers = modifiers;

    for m in moved.read() {
        let pos = egui::pos2(m.position.x, m.position.y);
        input.pointer = Some(pos);
        input.events.push(egui::Event::PointerMoved(pos));
    }
    for b in buttons.read() {
        if let (Some(button), Some(pos)) = (pointer_button(b.button), input.pointer) {
            input.events.push(egui::Event::PointerButton {
                pos,
                button,
                pressed: b.state == ButtonState::Pressed,
                modifiers,
            });
        }
    }
    for w in wheel.read() {
        let unit = match w.unit {
            MouseScrollUnit::Line => egui::MouseWheelUnit::Line,
            MouseScrollUnit::Pixel => egui::MouseWheelUnit::Point,
        };
        input.events.push(egui::Event::MouseWheel {
            unit,
            delta: egui::vec2(w.x, w.y),
            phase: egui::TouchPhase::Move,
            modifiers,
        });
    }
    for k in keyboard.read() {
        let pressed = k.state == ButtonState::Pressed;
        if let Some(key) = egui_key(k.key_code) {
            input.events.push(egui::Event::Key { key, physical_key: Some(key), pressed, repeat: k.repeat, modifiers });
        }
        if pressed
            && !ctrl
            && let Some(text) = &k.text
            && printable(text)
        {
            input.events.push(egui::Event::Text(text.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_printable_text_is_typed() {
        assert!(printable("a"));
        assert!(printable("é"));
        assert!(!printable("\r"));
        assert!(!printable("\u{7f}"));
        assert!(!printable(""));
    }
}
