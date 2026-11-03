//! The menus' default bindings (`bindings::defaults`).

use bevy_input::gamepad::{GamepadAxis, GamepadButton};
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::MouseButton;

use super::{Binding, Source};
use crate::input::Action;

/// Menus: the arrows or WASD and the pad's D-pad or left stick move, Enter or Space accepts, Esc goes back,
/// P or the pad's Start is Start, Q quits from the main menu. The pad's extra buttons follow NFS-XtendedInput:
/// `PAD_BUTTON0`/`1` are the right/left bumper, `PAD_BUTTON4` is X, and the triggers are `PAD_RTRIGGER`/`LTRIGGER`.
pub(super) fn menus() -> Vec<Binding> {
    use Action::*;
    let key = Source::Key;
    let button = Source::PadButton;
    let stick = Source::PadAxis;
    vec![
        Binding::new(MenuUp, key(KeyCode::ArrowUp), 1.0),
        Binding::new(MenuUp, key(KeyCode::KeyW), 1.0),
        Binding::new(MenuUp, button(GamepadButton::DPadUp), 1.0),
        Binding::new(MenuUp, stick(GamepadAxis::LeftStickY), 1.0),
        Binding::new(MenuDown, key(KeyCode::ArrowDown), 1.0),
        Binding::new(MenuDown, key(KeyCode::KeyS), 1.0),
        Binding::new(MenuDown, button(GamepadButton::DPadDown), 1.0),
        Binding::new(MenuDown, stick(GamepadAxis::LeftStickY), -1.0),
        Binding::new(MenuLeft, key(KeyCode::ArrowLeft), 1.0),
        Binding::new(MenuLeft, key(KeyCode::KeyA), 1.0),
        Binding::new(MenuLeft, button(GamepadButton::DPadLeft), 1.0),
        Binding::new(MenuLeft, stick(GamepadAxis::LeftStickX), -1.0),
        Binding::new(MenuRight, key(KeyCode::ArrowRight), 1.0),
        Binding::new(MenuRight, key(KeyCode::KeyD), 1.0),
        Binding::new(MenuRight, button(GamepadButton::DPadRight), 1.0),
        Binding::new(MenuRight, stick(GamepadAxis::LeftStickX), 1.0),
        Binding::new(MenuAccept, key(KeyCode::Enter), 1.0),
        Binding::new(MenuAccept, key(KeyCode::NumpadEnter), 1.0),
        Binding::new(MenuAccept, key(KeyCode::Space), 1.0),
        Binding::new(MenuAccept, button(GamepadButton::South), 1.0),
        Binding::new(MenuBack, key(KeyCode::Escape), 1.0),
        Binding::new(MenuBack, key(KeyCode::Backspace), 1.0),
        Binding::new(MenuBack, button(GamepadButton::East), 1.0),
        Binding::new(MenuStart, key(KeyCode::KeyP), 1.0),
        Binding::new(MenuStart, button(GamepadButton::Start), 1.0),
        Binding::new(MenuButton1, key(KeyCode::Digit1), 1.0),
        Binding::new(MenuButton1, button(GamepadButton::LeftTrigger), 1.0),
        Binding::new(MenuButton2, key(KeyCode::Digit2), 1.0),
        Binding::new(MenuButton2, button(GamepadButton::West), 1.0),
        Binding::new(MenuButton3, key(KeyCode::Digit3), 1.0),
        Binding::new(MenuButton3, button(GamepadButton::RightTrigger), 1.0),
        Binding::new(JukeboxGrab, key(KeyCode::Digit1), 1.0),
        Binding::new(JukeboxGrab, button(GamepadButton::North), 1.0),
        Binding::new(MenuZoomIn, key(KeyCode::Digit9), 1.0),
        Binding::new(MenuZoomOut, key(KeyCode::Digit0), 1.0),
        Binding::new(MenuZoomIn, button(GamepadButton::RightTrigger2), 1.0),
        Binding::new(MenuZoomOut, button(GamepadButton::LeftTrigger2), 1.0),
        Binding::new(MenuTriggerLeft, key(KeyCode::PageUp), 1.0),
        Binding::new(MenuTriggerLeft, button(GamepadButton::LeftTrigger2), 1.0),
        Binding::new(MenuTriggerRight, key(KeyCode::PageDown), 1.0),
        Binding::new(MenuTriggerRight, button(GamepadButton::RightTrigger2), 1.0),
        Binding::new(MenuQuit, key(KeyCode::KeyQ), 1.0),
        Binding::new(MenuQuit, button(GamepadButton::Select), 1.0),
        Binding::new(Click, Source::MouseButton(MouseButton::Left), 1.0),
    ]
}
