//! The reflection size and mirror tint settings as the menu and the config file read them: the sizes the Video
//! options step through, their labels, and the config file's forms. The values and their text rules are in
//! `blackbox_gfx::reflection`.

use std::str::FromStr;

use blackbox_gfx::reflection::{MirrorTint, ReflectionSize};
use toml::Value;

/// The sizes the Video options step through, in order. A size from the config file that is not in this list steps
/// from the first entry.
pub const MENU_SIZES: [ReflectionSize; 5] = [
    ReflectionSize::Auto,
    ReflectionSize::Scale(0.5),
    ReflectionSize::Scale(1.0),
    ReflectionSize::Scale(1.5),
    ReflectionSize::Scale(2.0),
];

/// The next (or previous) size in [`MENU_SIZES`], wrapping.
pub fn step_size(current: ReflectionSize, forward: bool) -> ReflectionSize {
    let at = MENU_SIZES.iter().position(|size| *size == current).unwrap_or(0);
    let next = match forward {
        true => (at + 1) % MENU_SIZES.len(),
        false => (at + MENU_SIZES.len() - 1) % MENU_SIZES.len(),
    };
    MENU_SIZES[next]
}

/// Off goes to the stock tint and anything else goes to off.
pub fn step_tint(current: MirrorTint) -> MirrorTint {
    match current.is_none() {
        true => MirrorTint::STOCK,
        false => MirrorTint::NONE,
    }
}

/// The row's text: `Auto`, `150%` or `256 px`.
pub fn size_label(size: ReflectionSize) -> String {
    match size {
        ReflectionSize::Auto => "Auto".to_owned(),
        ReflectionSize::Fixed(px) => format!("{px} px"),
        ReflectionSize::Scale(_) => size.to_string(),
    }
}

/// The row's text: `Off`, `Stock` or `Custom`.
pub fn tint_label(tint: MirrorTint) -> &'static str {
    if tint.is_none() {
        return "Off";
    }
    match tint == MirrorTint::STOCK {
        true => "Stock",
        false => "Custom",
    }
}

/// A size in the config file: text (`auto`, `150%`, `256`) or a whole number of pixels.
pub fn size_from_toml(value: &Value) -> Result<ReflectionSize, String> {
    match value {
        Value::Integer(px) => ReflectionSize::from_str(&px.to_string()),
        other => ReflectionSize::from_str(other.as_str().ok_or("expected a string or a whole number")?),
    }
}

/// A mirror tint in the config file: text (`off`, `on`, `#rrggbb`, `r g b`) or an array of three channels.
pub fn tint_from_toml(value: &Value) -> Result<MirrorTint, String> {
    match value {
        Value::Array(values) => {
            let channels: Option<Vec<u8>> =
                values.iter().map(|v| v.as_integer().and_then(|n| u8::try_from(n).ok())).collect();
            match channels.as_deref() {
                Some([r, g, b]) => Ok(MirrorTint([*r, *g, *b])),
                _ => Err("expected three channels from 0 to 255".to_owned()),
            }
        }
        other => MirrorTint::from_str(other.as_str().ok_or("expected a string or three channels")?),
    }
}
