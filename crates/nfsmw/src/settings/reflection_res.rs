//! The reflection size and mirror tint settings as the menu and the config file read them: the sizes the Video
//! options step through, their labels, and the config file's forms. The values and their text rules are in
//! `blackbox_gfx::reflection`.

use std::str::FromStr;

use blackbox_gfx::reflection::{MirrorTint, ReflectionSize};
use toml::Value;

/// The same rear-view choices in the Video menu and F8. Fixed values are picture heights in pixels.
pub const MENU_SIZES: [ReflectionSize; 5] = [
    ReflectionSize::Auto,
    ReflectionSize::Fixed(128),
    ReflectionSize::Fixed(256),
    ReflectionSize::Fixed(360),
    ReflectionSize::Fixed(512),
];

/// Explicit square face sizes offered by the car-cubemap UI. Saved legacy sizes still load unchanged.
pub const CUBE_SIZES: [u32; 7] = [16, 32, 64, 128, 256, 512, 1024];

/// Effective face edge, including quality-relative legacy values and power-of-two rounding.
pub fn cube_pixels(settings: &super::Settings, output_height: u32) -> u32 {
    settings.graphics().reflection_resolution.cube_face(
        settings.reflections.size(),
        output_height,
        blackbox_gfx::reflection::PLANAR_MAX,
    )
}

/// The current capture size, with the saved legacy rule spelled out when applicable.
pub fn cube_label(settings: &super::Settings, output_height: u32) -> String {
    let pixels = cube_pixels(settings, output_height);
    let dimensions = format!("{pixels} × {pixels} per face");
    match settings.reflection_cube_size {
        ReflectionSize::Fixed(_) => dimensions,
        ReflectionSize::Auto => format!("{dimensions} (saved: follow output height, capped at 1024)"),
        ReflectionSize::Scale(_) => format!("{dimensions} (saved: {} of quality size)", settings.reflection_cube_size),
    }
}

/// Frontend values have no live surface dimensions. Spell out Auto rather than invent a native size.
pub fn cube_menu_label(settings: &super::Settings) -> String {
    if settings.reflection_cube_size == ReflectionSize::Auto {
        return "Output height (maximum 1024 per face)".into();
    }
    cube_label(settings, 0)
}

/// Step to the next explicit face edge from the current effective size; wrap at either end.
pub fn step_cube(settings: &super::Settings, forward: bool) -> ReflectionSize {
    let pixels = cube_pixels(settings, blackbox_gfx::reflection::CUBE_FACE_MAX);
    let next = match forward {
        true => CUBE_SIZES.into_iter().find(|px| *px > pixels).unwrap_or(CUBE_SIZES[0]),
        false => CUBE_SIZES.into_iter().rev().find(|px| *px < pixels).unwrap_or(CUBE_SIZES[CUBE_SIZES.len() - 1]),
    };
    ReflectionSize::Fixed(next)
}

/// Step through the mirror choices. An older custom value returns to Auto in either direction.
pub fn step_size(current: ReflectionSize, forward: bool) -> ReflectionSize {
    let Some(at) = MENU_SIZES.iter().position(|size| *size == current) else {
        return ReflectionSize::Auto;
    };
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

/// Label the mirror choices and preserve the meaning of saved custom values.
pub fn size_label(size: ReflectionSize) -> String {
    match size {
        ReflectionSize::Auto => "Auto".to_owned(),
        ReflectionSize::Fixed(128) => "128 px".to_owned(),
        ReflectionSize::Fixed(256) => "256 px".to_owned(),
        ReflectionSize::Fixed(360) => "360 px".to_owned(),
        ReflectionSize::Fixed(512) => "512 px".to_owned(),
        ReflectionSize::Fixed(px) => format!("Custom ({px} px)"),
        ReflectionSize::Scale(_) => format!("Custom ({size} of base)"),
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
