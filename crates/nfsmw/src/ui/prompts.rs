//! A prompt describes an action's current physical binding, never a fixed button assignment. The names of keys and
//! buttons are words of the current language (`keys-*` in the catalogue); letters, digits and function keys read the same
//! in every language.

use bevy_input::{
    gamepad::{GamepadAxis, GamepadButton},
    keyboard::KeyCode,
    mouse::MouseButton,
};

use super::input_icons::Glyph;
use crate::i18n::{self, Args, t};
use crate::input::{Action, Bindings, InputDevice, Source};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    Icon(Glyph),
    Key(String),
}

pub fn for_action(bindings: &Bindings, action: Action, device: InputDevice) -> Prompt {
    let Some(binding) = bindings.prompt_binding(action, device) else { return Prompt::Key(t!("keys-unbound")) };
    match binding.source {
        Source::PadButton(button) | Source::PadTrigger(button) => button_prompt(button),
        Source::PadAxis(axis) => axis_prompt(axis, binding.scale > 0.0),
        Source::PedalAxis { axis, .. } => Prompt::Key(t!("keys-pedal", axis = axis_name(axis))),
        Source::Key(key) => Prompt::Key(key_name(key)),
        Source::MouseButton(button) => Prompt::Key(mouse_name(button)),
        Source::Scroll => Prompt::Key(t!("keys-wheel")),
        Source::MouseMotion { .. } => Prompt::Key(t!("keys-mouse")),
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
        // The pads' C and Z buttons are letters, the same in every language.
        C => return Prompt::Key("C".into()),
        Z => return Prompt::Key("Z".into()),
        Mode => return Prompt::Key(t!("keys-pad-mode")),
        Other(code) => return Prompt::Key(t!("keys-pad-button", code = code)),
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
            let sign = match positive {
                true => "+",
                false => "-",
            };
            return Prompt::Key(format!("{}{sign}", axis_name(axis)));
        }
    };
    Prompt::Icon(icon)
}

/// The name of a pad axis, for a prompt that has no picture for it.
fn axis_name(axis: GamepadAxis) -> String {
    let id = match axis {
        GamepadAxis::LeftStickX => "keys-axis-left-stick-x",
        GamepadAxis::LeftStickY => "keys-axis-left-stick-y",
        GamepadAxis::LeftZ => "keys-axis-left-z",
        GamepadAxis::RightStickX => "keys-axis-right-stick-x",
        GamepadAxis::RightStickY => "keys-axis-right-stick-y",
        GamepadAxis::RightZ => "keys-axis-right-z",
        GamepadAxis::Other(n) => return t!("keys-axis-other", n = n),
    };
    i18n::text(id, &Args::new())
}

fn mouse_name(button: MouseButton) -> String {
    let id = match button {
        MouseButton::Left => "keys-mouse-left",
        MouseButton::Right => "keys-mouse-right",
        MouseButton::Middle => "keys-mouse-middle",
        MouseButton::Back => "keys-mouse-back",
        MouseButton::Forward => "keys-mouse-forward",
        MouseButton::Other(n) => return t!("keys-mouse-other", n = n),
    };
    i18n::text(id, &Args::new())
}

/// The name of a key in a footer prompt: the short words of the footer (`Esc`, `Enter`, `Left`), else [`key_word`].
fn key_name(key: KeyCode) -> String {
    let id = match key {
        KeyCode::Escape => "keys-esc",
        KeyCode::NumpadEnter | KeyCode::Enter => "keys-enter",
        KeyCode::ArrowLeft => "keys-left",
        KeyCode::ArrowRight => "keys-right",
        KeyCode::ArrowUp => "keys-up",
        KeyCode::ArrowDown => "keys-down",
        _ => return key_word(key),
    };
    i18n::text(id, &Args::new())
}

/// The name of a key in the current language. Letters, digits and function keys keep their own label (`W`, `7`, `F5`).
pub fn key_word(key: KeyCode) -> String {
    use KeyCode as K;
    let id = match key {
        K::Unidentified(_) => "keys-unidentified",
        K::Backquote => "keys-backquote",
        K::Backslash => "keys-backslash",
        K::BracketLeft => "keys-bracket-left",
        K::BracketRight => "keys-bracket-right",
        K::Comma => "keys-comma",
        K::Equal => "keys-equal",
        K::IntlBackslash => "keys-intl-backslash",
        K::IntlRo => "keys-intl-ro",
        K::IntlYen => "keys-intl-yen",
        K::Minus => "keys-minus",
        K::Period => "keys-period",
        K::Quote => "keys-quote",
        K::Semicolon => "keys-semicolon",
        K::Slash => "keys-slash",
        K::AltLeft => "keys-alt-left",
        K::AltRight => "keys-alt-right",
        K::Backspace => "keys-backspace",
        K::CapsLock => "keys-caps-lock",
        K::ContextMenu => "keys-context-menu",
        K::ControlLeft => "keys-control-left",
        K::ControlRight => "keys-control-right",
        K::Enter => "keys-enter",
        K::SuperLeft => "keys-super-left",
        K::SuperRight => "keys-super-right",
        K::ShiftLeft => "keys-shift-left",
        K::ShiftRight => "keys-shift-right",
        K::Space => "keys-space",
        K::Tab => "keys-tab",
        K::Convert => "keys-convert",
        K::KanaMode => "keys-kana-mode",
        K::Lang1 => return t!("keys-lang", n = 1),
        K::Lang2 => return t!("keys-lang", n = 2),
        K::Lang3 => return t!("keys-lang", n = 3),
        K::Lang4 => return t!("keys-lang", n = 4),
        K::Lang5 => return t!("keys-lang", n = 5),
        K::NonConvert => "keys-non-convert",
        K::Delete => "keys-delete",
        K::End => "keys-end",
        K::Help => "keys-help",
        K::Home => "keys-home",
        K::Insert => "keys-insert",
        K::PageDown => "keys-page-down",
        K::PageUp => "keys-page-up",
        K::ArrowDown => "keys-arrow-down",
        K::ArrowLeft => "keys-arrow-left",
        K::ArrowRight => "keys-arrow-right",
        K::ArrowUp => "keys-arrow-up",
        K::NumLock => "keys-num-lock",
        K::Numpad0 => return t!("keys-numpad-digit", digit = 0),
        K::Numpad1 => return t!("keys-numpad-digit", digit = 1),
        K::Numpad2 => return t!("keys-numpad-digit", digit = 2),
        K::Numpad3 => return t!("keys-numpad-digit", digit = 3),
        K::Numpad4 => return t!("keys-numpad-digit", digit = 4),
        K::Numpad5 => return t!("keys-numpad-digit", digit = 5),
        K::Numpad6 => return t!("keys-numpad-digit", digit = 6),
        K::Numpad7 => return t!("keys-numpad-digit", digit = 7),
        K::Numpad8 => return t!("keys-numpad-digit", digit = 8),
        K::Numpad9 => return t!("keys-numpad-digit", digit = 9),
        K::NumpadAdd => "keys-numpad-add",
        K::NumpadBackspace => "keys-numpad-backspace",
        K::NumpadClear => "keys-numpad-clear",
        K::NumpadClearEntry => "keys-numpad-clear-entry",
        K::NumpadComma => "keys-numpad-comma",
        K::NumpadDecimal => "keys-numpad-decimal",
        K::NumpadDivide => "keys-numpad-divide",
        K::NumpadEnter => "keys-numpad-enter",
        K::NumpadEqual => "keys-numpad-equal",
        K::NumpadHash => "keys-numpad-hash",
        K::NumpadMemoryAdd => "keys-numpad-memory-add",
        K::NumpadMemoryClear => "keys-numpad-memory-clear",
        K::NumpadMemoryRecall => "keys-numpad-memory-recall",
        K::NumpadMemoryStore => "keys-numpad-memory-store",
        K::NumpadMemorySubtract => "keys-numpad-memory-subtract",
        K::NumpadMultiply => "keys-numpad-multiply",
        K::NumpadParenLeft => "keys-numpad-paren-left",
        K::NumpadParenRight => "keys-numpad-paren-right",
        K::NumpadStar => "keys-numpad-star",
        K::NumpadSubtract => "keys-numpad-subtract",
        K::Escape => "keys-escape",
        K::Fn => "keys-fn",
        K::FnLock => "keys-fn-lock",
        K::PrintScreen => "keys-print-screen",
        K::ScrollLock => "keys-scroll-lock",
        K::Pause => "keys-pause",
        K::BrowserBack => "keys-browser-back",
        K::BrowserFavorites => "keys-browser-favorites",
        K::BrowserForward => "keys-browser-forward",
        K::BrowserHome => "keys-browser-home",
        K::BrowserRefresh => "keys-browser-refresh",
        K::BrowserSearch => "keys-browser-search",
        K::BrowserStop => "keys-browser-stop",
        K::Eject => "keys-eject",
        K::LaunchApp1 => "keys-launch-app1",
        K::LaunchApp2 => "keys-launch-app2",
        K::LaunchMail => "keys-launch-mail",
        K::MediaPlayPause => "keys-media-play-pause",
        K::MediaSelect => "keys-media-select",
        K::MediaStop => "keys-media-stop",
        K::MediaTrackNext => "keys-media-track-next",
        K::MediaTrackPrevious => "keys-media-track-previous",
        K::Power => "keys-power",
        K::Sleep => "keys-sleep",
        K::AudioVolumeDown => "keys-audio-volume-down",
        K::AudioVolumeMute => "keys-audio-volume-mute",
        K::AudioVolumeUp => "keys-audio-volume-up",
        K::WakeUp => "keys-wake-up",
        K::Meta => "keys-meta",
        K::Hyper => "keys-hyper",
        K::Turbo => "keys-turbo",
        K::Abort => "keys-abort",
        K::Resume => "keys-resume",
        K::Suspend => "keys-suspend",
        K::Again => "keys-again",
        K::Copy => "keys-copy",
        K::Cut => "keys-cut",
        K::Find => "keys-find",
        K::Open => "keys-open",
        K::Paste => "keys-paste",
        K::Props => "keys-props",
        K::Select => "keys-select",
        K::Undo => "keys-undo",
        K::Hiragana => "keys-hiragana",
        K::Katakana => "keys-katakana",
        // What is left are the letters (`KeyW`), the digits (`Digit7`) and the function keys (`F5`): the code's name
        // without its prefix is the key's own label.
        _ => return format!("{key:?}").trim_start_matches("Key").trim_start_matches("Digit").to_owned(),
    };
    i18n::text(id, &Args::new())
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
    #[test]
    fn key_names_are_words_and_letters_stay_letters() {
        assert_eq!(key_name(KeyCode::Escape), "Esc");
        assert_eq!(key_word(KeyCode::Escape), "Escape");
        assert_eq!(key_word(KeyCode::KeyW), "W");
        assert_eq!(key_word(KeyCode::Digit7), "7");
        assert_eq!(key_word(KeyCode::F5), "F5");
        assert_eq!(key_word(KeyCode::Backspace), "Backspace");
        assert_eq!(key_word(KeyCode::Numpad3), "Numpad3");
        assert_eq!(mouse_name(MouseButton::Left), "Mouse Left");
    }
}
