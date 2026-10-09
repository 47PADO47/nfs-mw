//! Stable text for physical inputs in the config file and console.

use bevy_input::{
    gamepad::{GamepadAxis, GamepadButton},
    keyboard::KeyCode,
    mouse::MouseButton,
};
use serde::de::DeserializeOwned;
use toml::{Table, Value};

use super::{
    Action,
    bindings::{Binding, Gate, Source},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Keyboard,
    Mouse,
    Gamepad,
}

pub fn family(source: Source) -> Family {
    match source {
        Source::Key(_) => Family::Keyboard,
        Source::MouseMotion { .. } | Source::Scroll | Source::MouseButton(_) => Family::Mouse,
        Source::PadAxis(_) | Source::PadButton(_) | Source::PadTrigger(_) => Family::Gamepad,
    }
}

fn decode<T: DeserializeOwned>(name: &str) -> Result<T, String> {
    if let Some(code) = name.strip_prefix("Other(").and_then(|s| s.strip_suffix(')')) {
        let code = code.parse::<i64>().map_err(|_| "Other requires a whole button/axis code".to_owned())?;
        let mut table = Table::new();
        table.insert("Other".into(), Value::Integer(code));
        return Value::Table(table).try_into().map_err(|e| format!("invalid input {name:?}: {e}"));
    }
    Value::String(name.to_owned()).try_into().map_err(|e| format!("invalid input {name:?}: {e}"))
}

fn key(name: &str) -> Result<KeyCode, String> {
    if name.len() == 1 && name.as_bytes()[0].is_ascii_alphabetic() {
        return decode(&format!("Key{}", name.to_ascii_uppercase()));
    }
    if name.len() == 1 && name.as_bytes()[0].is_ascii_digit() {
        return decode(&format!("Digit{name}"));
    }
    decode(name)
}

fn button(name: &str) -> Result<GamepadButton, String> {
    let alias = match name.to_ascii_lowercase().as_str() {
        "a" => "South",
        "b" => "East",
        "x" => "West",
        "y" => "North",
        "lb" => "LeftTrigger",
        "rb" => "RightTrigger",
        "lt" => "LeftTrigger2",
        "rt" => "RightTrigger2",
        "back" => "Select",
        "start" => "Start",
        "ls" => "LeftThumb",
        "rs" => "RightThumb",
        _ => name,
    };
    decode(alias)
}

/// `kind:name[:scale[:per_second]]`; the defaults for scale and clock are 1 and per-frame.
pub fn parse(action: Action, text: &str) -> Result<Binding, String> {
    let parts: Vec<_> = text.split(':').collect();
    if !(2..=4).contains(&parts.len()) {
        return Err("input syntax: kind:name[:scale[:per_second]]".into());
    }
    let source = match parts[0] {
        "key" => Source::Key(key(parts[1])?),
        "button" => Source::PadButton(button(parts[1])?),
        "trigger" => Source::PadTrigger(button(parts[1])?),
        "axis" => Source::PadAxis(decode::<GamepadAxis>(parts[1])?),
        "mouse" => match parts[1] {
            "scroll" => Source::Scroll,
            "look_x" => Source::MouseMotion { y: false, gate: Gate::Look },
            "look_y" => Source::MouseMotion { y: true, gate: Gate::Look },
            "orbit_x" => Source::MouseMotion { y: false, gate: Gate::Drag },
            "orbit_y" => Source::MouseMotion { y: true, gate: Gate::Drag },
            name => Source::MouseButton(decode::<MouseButton>(name)?),
        },
        _ => return Err("input kind is key, button, trigger, axis or mouse".into()),
    };
    let scale: f32 = parts.get(2).map_or(Ok(1.0), |s| s.parse().map_err(|_| "invalid input scale".to_owned()))?;
    if !scale.is_finite() || scale.abs() > 10000.0 || scale == 0.0 {
        return Err("input scale must be finite, nonzero and between -10000 and 10000".into());
    }
    let per_second = match parts.get(3) {
        None => false,
        Some(&"per_second") => true,
        _ => return Err("input clock is per_second, or omit it for per-frame input".into()),
    };
    if per_second && matches!(source, Source::MouseMotion { .. } | Source::Scroll) {
        return Err("mouse deltas and scroll are already per frame".into());
    }
    Ok(Binding { action, source, scale, per_second })
}

pub fn encode(binding: &Binding) -> String {
    let source = match binding.source {
        Source::Key(k) => format!("key:{k:?}"),
        Source::PadButton(b) => format!("button:{b:?}"),
        Source::PadTrigger(b) => format!("trigger:{b:?}"),
        Source::PadAxis(a) => format!("axis:{a:?}"),
        Source::MouseButton(b) => format!("mouse:{b:?}"),
        Source::Scroll => "mouse:scroll".into(),
        Source::MouseMotion { y, gate } => {
            let axis = match y {
                true => "y",
                false => "x",
            };
            let mode = match gate {
                Gate::Look => "look",
                Gate::Drag => "orbit",
            };
            format!("mouse:{mode}_{axis}")
        }
    };
    if binding.per_second {
        return format!("{source}:{}:per_second", binding.scale);
    }
    if binding.scale != 1.0 {
        return format!("{source}:{}", binding.scale);
    }
    source
}
