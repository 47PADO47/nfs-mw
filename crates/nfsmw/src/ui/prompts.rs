//! A prompt describes an action's current physical binding, never a fixed button assignment.

use bevy_input::{
    gamepad::{GamepadAxis, GamepadButton},
    keyboard::KeyCode,
};

use super::input_icons::Glyph;
use crate::input::{Action, Bindings, InputDevice, Source};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    Icon(Glyph),
    Key(String),
}

pub fn for_action(bindings: &Bindings, action: Action, device: InputDevice) -> Prompt {
    let Some(binding) = bindings.prompt_binding(action, device) else { return Prompt::Key("Unbound".into()) };
    match binding.source {
        Source::PadButton(button) | Source::PadTrigger(button) => button_prompt(button),
        Source::PadAxis(axis) => axis_prompt(axis, binding.scale > 0.0),
        Source::Key(key) => Prompt::Key(key_name(key)),
        Source::MouseButton(button) => Prompt::Key(format!("Mouse {button:?}")),
        Source::Scroll => Prompt::Key("Wheel".into()),
        Source::MouseMotion { .. } => Prompt::Key("Mouse".into()),
    }
}

pub fn pair(bindings: &Bindings, a: Action, b: Action, device: InputDevice) -> Vec<Prompt> {
    let prompts = [for_action(bindings, a, device), for_action(bindings, b, device)];
    match prompts {
        [Prompt::Icon(Glyph::Up), Prompt::Icon(Glyph::Down)] => vec![Prompt::Icon(Glyph::Vertical)],
        [Prompt::Icon(Glyph::Left), Prompt::Icon(Glyph::Right)] => vec![Prompt::Icon(Glyph::Horizontal)],
        [a, b] if a == b => vec![a],
        [a, b] => vec![a, b],
    }
}

fn button_prompt(button: GamepadButton) -> Prompt {
    use GamepadButton::*;
    let icon = match button {
        South => Glyph::A,
        East => Glyph::B,
        West => Glyph::X,
        North => Glyph::Y,
        LeftTrigger => Glyph::Lb,
        RightTrigger => Glyph::Rb,
        LeftTrigger2 => Glyph::Lt,
        RightTrigger2 => Glyph::Rt,
        Start => Glyph::Menu,
        Select => Glyph::View,
        LeftThumb => Glyph::Ls,
        RightThumb => Glyph::Rs,
        DPadUp => Glyph::Up,
        DPadDown => Glyph::Down,
        DPadLeft => Glyph::Left,
        DPadRight => Glyph::Right,
        Other(code) => return Prompt::Key(format!("Button {code}")),
        _ => return Prompt::Key(format!("{button:?}")),
    };
    Prompt::Icon(icon)
}

fn axis_prompt(axis: GamepadAxis, positive: bool) -> Prompt {
    use GamepadAxis::*;
    let icon = match (axis, positive) {
        (LeftStickX, false) => Glyph::LeftLeft,
        (LeftStickX, true) => Glyph::LeftRight,
        (LeftStickY, false) => Glyph::LeftDown,
        (LeftStickY, true) => Glyph::LeftUp,
        (RightStickX, false) => Glyph::RightLeft,
        (RightStickX, true) => Glyph::RightRight,
        (RightStickY, false) => Glyph::RightDown,
        (RightStickY, true) => Glyph::RightUp,
        _ => {
            return Prompt::Key(format!(
                "{axis:?}{}",
                match positive {
                    true => "+",
                    false => "-",
                }
            ));
        }
    };
    Prompt::Icon(icon)
}

fn key_name(key: KeyCode) -> String {
    match key {
        KeyCode::Escape => "Esc".into(),
        KeyCode::NumpadEnter | KeyCode::Enter => "Enter".into(),
        KeyCode::ArrowLeft => "Left".into(),
        KeyCode::ArrowRight => "Right".into(),
        KeyCode::ArrowUp => "Up".into(),
        KeyCode::ArrowDown => "Down".into(),
        _ => format!("{key:?}").trim_start_matches("Key").trim_start_matches("Digit").to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompts_follow_rebinding_and_switching_without_stale_glyphs() {
        let mut bindings = Bindings::default();
        assert_eq!(for_action(&bindings, Action::MenuAccept, InputDevice::Xbox), Prompt::Icon(Glyph::A));
        bindings.bind(Action::MenuAccept, "button:West", false).unwrap();
        assert_eq!(for_action(&bindings, Action::MenuAccept, InputDevice::Xbox), Prompt::Icon(Glyph::X));
        assert_eq!(for_action(&bindings, Action::MenuAccept, InputDevice::Keyboard), Prompt::Key("Enter".into()));
        bindings.bind(Action::MenuAccept, "trigger:RightTrigger2", false).unwrap();
        assert_eq!(for_action(&bindings, Action::MenuAccept, InputDevice::Xbox), Prompt::Icon(Glyph::Rt));
        bindings.bind(Action::MenuAccept, "button:Other(7)", false).unwrap();
        assert_eq!(for_action(&bindings, Action::MenuAccept, InputDevice::Xbox), Prompt::Key("Button 7".into()));
    }
    #[test]
    fn directional_remaps_do_not_keep_a_misleading_dpad_pair() {
        let mut bindings = Bindings::default();
        assert_eq!(
            pair(&bindings, Action::MenuLeft, Action::MenuRight, InputDevice::Xbox),
            vec![Prompt::Icon(Glyph::Horizontal)]
        );
        bindings.bind(Action::MenuRight, "button:North", false).unwrap();
        assert_eq!(
            pair(&bindings, Action::MenuLeft, Action::MenuRight, InputDevice::Xbox),
            vec![Prompt::Icon(Glyph::Left), Prompt::Icon(Glyph::Y)]
        );
    }
}
