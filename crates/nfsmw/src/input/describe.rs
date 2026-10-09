//! The bindings as text, for the `keys` console command and the `nfsmw keys` subcommand.
//!
//! The listing is built from the live [`Bindings`], so it can never disagree with what the game does.

use bevy_input::gamepad::{GamepadAxis, GamepadButton};
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::MouseButton;

use super::Action;
use super::bindings::{Binding, Gate, Source};
use super::state::Bindings;

/// Where an action is used; the listing is grouped by it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Driving,
    FreeCamera,
    Menus,
    General,
}

const SECTIONS: [(Section, &str); 4] = [
    (Section::Driving, "Driving"),
    (Section::FreeCamera, "Free camera (view-world, view-car)"),
    (Section::Menus, "Menus"),
    (Section::General, "Anywhere"),
];

/// Keys that are not actions (they are handled by the window itself).
const FIXED_KEYS: [(&str, &str); 1] = [("Alt+Enter", "toggle fullscreen")];

fn section(action: Action) -> Section {
    use Action::*;
    match action {
        Throttle | Brake | Steer | Handbrake | ShiftUp | ShiftDown | Nos | ResetCar | ToggleCamera => Section::Driving,
        MoveForward | MoveRight | MoveUp | LookX | LookY | OrbitX | OrbitY | Zoom | Boost => Section::FreeCamera,
        MenuUp | MenuDown | MenuLeft | MenuRight | MenuAccept | MenuBack | MenuStart | MenuQuit | Click => {
            Section::Menus
        }
        Cancel | Console | RadioToggle | RadioNext | RadioPrevious => Section::General,
    }
}

/// The action's name, and the name of its negative half for a two-way axis (empty for a one-way action).
fn labels(action: Action) -> (&'static str, &'static str) {
    use Action::*;
    match action {
        MoveForward => ("Move forward", "Move back"),
        MoveRight => ("Move right", "Move left"),
        MoveUp => ("Move up", "Move down"),
        LookX => ("Look right", "Look left"),
        LookY => ("Look down", "Look up"),
        OrbitX => ("Orbit right", "Orbit left"),
        OrbitY => ("Orbit down", "Orbit up"),
        Zoom => ("Zoom out", "Zoom in"),
        Boost => ("Fly fast", ""),
        Cancel => ("Back out (release mouse, pause, quit)", ""),
        Console => ("Developer console", ""),
        Throttle => ("Accelerate", ""),
        Brake => ("Brake, then reverse", ""),
        Steer => ("Steer right", "Steer left"),
        Handbrake => ("Handbrake", ""),
        ShiftUp => ("Shift up (manual transmission)", ""),
        ShiftDown => ("Shift down (manual transmission)", ""),
        Nos => ("Nitrous", ""),
        ResetCar => ("Reset the car onto the road", ""),
        ToggleCamera => ("Chase camera / free camera", ""),
        MenuUp => ("Menu up", ""),
        MenuDown => ("Menu down", ""),
        MenuLeft => ("Menu left", ""),
        MenuRight => ("Menu right", ""),
        MenuAccept => ("Menu accept", ""),
        MenuBack => ("Menu back", ""),
        MenuStart => ("Start (pause, resume)", ""),
        MenuQuit => ("Quit from the main menu", ""),
        Click => ("Skip a movie, continue at the title screen", ""),
        RadioToggle => ("Radio: pause / resume", ""),
        RadioNext => ("Radio: next song", ""),
        RadioPrevious => ("Radio: previous song", ""),
    }
}

fn key_name(key: KeyCode) -> String {
    let name = format!("{key:?}");
    if let Some(letter) = name.strip_prefix("Key").filter(|rest| rest.len() == 1) {
        return letter.to_owned();
    }
    if let Some(digit) = name.strip_prefix("Digit") {
        return digit.to_owned();
    }
    if let Some(direction) = name.strip_prefix("Arrow") {
        return direction.to_owned();
    }
    for (modifier, shown) in [("Shift", "Shift"), ("Control", "Ctrl"), ("Alt", "Alt"), ("Super", "Super")] {
        if let Some(side) = name.strip_prefix(modifier).filter(|side| matches!(*side, "Left" | "Right")) {
            return format!("{side} {shown}");
        }
    }
    name
}

fn mouse_button_name(button: MouseButton) -> String {
    match button {
        MouseButton::Left => "Left click".to_owned(),
        MouseButton::Right => "Right click".to_owned(),
        MouseButton::Middle => "Middle click".to_owned(),
        other => format!("Mouse {other:?}"),
    }
}

fn button_name(button: GamepadButton) -> String {
    let name = match button {
        GamepadButton::South => "A",
        GamepadButton::East => "B",
        GamepadButton::West => "X",
        GamepadButton::North => "Y",
        GamepadButton::LeftTrigger => "LB",
        GamepadButton::RightTrigger => "RB",
        GamepadButton::LeftTrigger2 => "LT",
        GamepadButton::RightTrigger2 => "RT",
        GamepadButton::Select => "Back",
        GamepadButton::Start => "Start",
        GamepadButton::LeftThumb => "left stick click",
        GamepadButton::RightThumb => "right stick click",
        GamepadButton::DPadUp => "D-pad up",
        GamepadButton::DPadDown => "D-pad down",
        GamepadButton::DPadLeft => "D-pad left",
        GamepadButton::DPadRight => "D-pad right",
        GamepadButton::Other(code) => return format!("Pad button {code}"),
        other => return format!("Pad {other:?}"),
    };
    format!("Pad {name}")
}

/// A stick pushed along `axis` towards its positive or negative end.
fn stick_name(axis: GamepadAxis, positive: bool) -> String {
    let (stick, plus, minus) = match axis {
        GamepadAxis::LeftStickX => ("left stick", "right", "left"),
        GamepadAxis::LeftStickY => ("left stick", "up", "down"),
        GamepadAxis::RightStickX => ("right stick", "right", "left"),
        GamepadAxis::RightStickY => ("right stick", "up", "down"),
        other => return format!("Pad {other:?}{}", if positive { "+" } else { "-" }),
    };
    format!("Pad {stick} {}", if positive { plus } else { minus })
}

/// An analog source moved towards `positive` (the direction on its own axis).
fn analog_name(source: Source, positive: bool) -> String {
    match source {
        Source::PadAxis(axis) => stick_name(axis, positive),
        Source::Scroll => format!("Wheel {}", if positive { "up" } else { "down" }),
        Source::MouseMotion { y, gate } => {
            let direction = match (y, positive) {
                (false, true) => "right",
                (false, false) => "left",
                (true, true) => "down",
                (true, false) => "up",
            };
            let when = match gate {
                Gate::Look => "right button held or cursor captured",
                Gate::Drag => "a button held",
            };
            format!("Mouse {direction} ({when})")
        }
        Source::Key(_) | Source::MouseButton(_) | Source::PadButton(_) | Source::PadTrigger(_) => String::new(),
    }
}

/// Whether `action` has a negative half of its own.
fn is_two_way(action: Action) -> bool {
    !labels(action).1.is_empty()
}

/// What the player presses (or moves) to give `binding`'s action a positive (`false`) or negative (`true`) value.
fn input_for(binding: &Binding, negative: bool) -> Option<String> {
    let pushes_negative = binding.scale < 0.0;
    match binding.source {
        Source::Key(key) => (negative == pushes_negative).then(|| key_name(key)),
        Source::MouseButton(button) => (negative == pushes_negative).then(|| mouse_button_name(button)),
        Source::PadButton(button) => (negative == pushes_negative).then(|| button_name(button)),
        Source::PadTrigger(button) => (negative == pushes_negative).then(|| button_name(button)),
        analog => {
            // An analog source moves both ways: the row of a one-way action only lists the push that fills it.
            if !is_two_way(binding.action) && negative {
                return None;
            }
            // The direction on the source's own axis whose product with the scale has the row's sign.
            Some(analog_name(analog, negative == pushes_negative))
        }
    }
}

impl Bindings {
    /// Every binding by section and action, one row per action, for a human.
    pub fn describe(&self) -> String {
        let mut text = String::new();
        let width = Action::ALL.iter().flat_map(|a| [labels(*a).0, labels(*a).1]).map(str::len).max().unwrap_or(0);
        for (which, title) in SECTIONS {
            text.push_str(title);
            text.push('\n');
            for action in Action::ALL.into_iter().filter(|a| section(*a) == which) {
                let (positive, negative) = labels(action);
                for (is_negative, label) in [(false, positive), (true, negative)] {
                    let mut inputs: Vec<String> = self
                        .0
                        .iter()
                        .filter(|b| b.action == action)
                        .filter_map(|b| input_for(b, is_negative))
                        .collect();
                    inputs.dedup();
                    if label.is_empty() || inputs.is_empty() {
                        continue;
                    }
                    text.push_str(&format!("  {label:<width$}  {}\n", inputs.join(", ")));
                }
            }
        }
        text.push_str("Other\n");
        for (keys, what) in FIXED_KEYS {
            text.push_str(&format!("  {keys:<width$}  {what}\n"));
        }
        text.push_str("Action names for bind/addbind/unbind:\n  ");
        text.push_str(&Action::ALL.map(Action::name).join(", "));
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str, label: &str) -> String {
        let line = text.lines().find(|l| l.trim_start().starts_with(label)).unwrap_or_else(|| panic!("no row {label}"));
        line.trim_start()[label.len()..].trim().to_owned()
    }

    #[test]
    fn nitrous_is_left_shift_and_the_gear_keys_leave_it_alone() {
        let text = Bindings::default().describe();
        assert!(row(&text, "Nitrous").contains("Left Shift"));
        assert!(!row(&text, "Nitrous").contains(", N"), "N no longer fires the nitrous");
        assert!(!row(&text, "Shift up (manual transmission)").contains("Left Shift"));
        assert_eq!(row(&text, "Shift up (manual transmission)"), "E, Right Shift, Pad RB");
    }

    #[test]
    fn two_way_axes_get_a_row_per_direction() {
        let text = Bindings::default().describe();
        assert_eq!(row(&text, "Steer right"), "D, Right, Pad left stick right");
        assert_eq!(row(&text, "Steer left"), "A, Left, Pad left stick left");
        assert_eq!(row(&text, "Move forward"), "W, Pad left stick up");
        assert_eq!(row(&text, "Move back"), "S, Pad left stick down");
    }

    #[test]
    fn menus_use_the_stick_direction_that_fills_them() {
        let text = Bindings::default().describe();
        assert!(row(&text, "Menu down").contains("Pad left stick down"));
        assert!(row(&text, "Menu up").contains("Pad left stick up"));
    }

    #[test]
    fn every_action_is_listed_and_paddles_show_up() {
        let text = Bindings::with_paddles(Some(7), None).describe();
        for action in Action::ALL {
            assert!(text.contains(labels(action).0), "{action:?} is missing");
        }
        assert!(row(&text, "Shift up (manual transmission)").contains("Pad button 7"));
        assert!(text.contains("Alt+Enter"));
    }

    #[test]
    fn the_radio_keys_are_listed() {
        let text = Bindings::default().describe();
        assert_eq!(row(&text, "Radio: pause / resume"), "M, MediaPlayPause, Pad right stick click");
        assert_eq!(row(&text, "Radio: next song"), "Period, MediaTrackNext, Pad D-pad right");
        assert_eq!(row(&text, "Radio: previous song"), "Comma, MediaTrackPrevious, Pad D-pad left");
    }

    #[test]
    fn key_names_are_readable() {
        assert_eq!(key_name(KeyCode::KeyW), "W");
        assert_eq!(key_name(KeyCode::ArrowUp), "Up");
        assert_eq!(key_name(KeyCode::ControlLeft), "Left Ctrl");
        assert_eq!(key_name(KeyCode::F12), "F12");
        assert_eq!(key_name(KeyCode::Space), "Space");
    }
}
