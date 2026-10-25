//! The console's `custom_hud.<key>` settings: `get custom_hud.scale`, `set custom_hud.map_mode compact`,
//! `set custom_hud.haptics` (a switch flips). A console change lasts for the run. `set custom_hud.preset` applies the
//! preset's values at once.

use std::str::FromStr;

use super::{
    HudPreset, HudSettings, MAP_OPACITY_RANGE, MAP_ZOOM_RANGE, OPACITY_RANGE, SCALE_RANGE, TEXT_SIZE_RANGE, UNIT_RANGE,
};
use crate::settings::parse_bool;

/// The console keys, in the order `get` lists them.
pub const KEYS: [&str; 20] = [
    "custom_hud.preset",
    "custom_hud.scale",
    "custom_hud.opacity",
    "custom_hud.text_size",
    "custom_hud.map_mode",
    "custom_hud.map_opacity",
    "custom_hud.map_zoom",
    "custom_hud.advanced_telemetry",
    "custom_hud.world_markers",
    "custom_hud.race_info",
    "custom_hud.pursuit_info",
    "custom_hud.free_roam_extras",
    "custom_hud.notifications",
    "custom_hud.high_contrast",
    "custom_hud.palette",
    "custom_hud.motion_reduction",
    "custom_hud.camera_shake",
    "custom_hud.fov_intensity",
    "custom_hud.haptics",
    "custom_hud.hud_volume",
];

/// Whether the console key names one of the options.
pub fn is_key(key: &str) -> bool {
    KEYS.contains(&key)
}

/// The on/off option a console key names, for a switch with no value.
pub fn flag<'a>(s: &'a mut HudSettings, key: &str) -> Option<&'a mut bool> {
    match key {
        "custom_hud.advanced_telemetry" => Some(&mut s.advanced_telemetry),
        "custom_hud.world_markers" => Some(&mut s.world_markers),
        "custom_hud.race_info" => Some(&mut s.race_info),
        "custom_hud.pursuit_info" => Some(&mut s.pursuit_info),
        "custom_hud.free_roam_extras" => Some(&mut s.free_roam_extras),
        "custom_hud.high_contrast" => Some(&mut s.high_contrast),
        "custom_hud.motion_reduction" => Some(&mut s.motion_reduction),
        "custom_hud.haptics" => Some(&mut s.haptics),
        _ => None,
    }
}

/// The number option a console key names, with its range.
fn number<'a>(s: &'a mut HudSettings, key: &str) -> Option<(&'a mut f32, (f32, f32))> {
    match key {
        "custom_hud.scale" => Some((&mut s.scale, SCALE_RANGE)),
        "custom_hud.opacity" => Some((&mut s.opacity, OPACITY_RANGE)),
        "custom_hud.text_size" => Some((&mut s.text_size, TEXT_SIZE_RANGE)),
        "custom_hud.map_opacity" => Some((&mut s.map_opacity, MAP_OPACITY_RANGE)),
        "custom_hud.map_zoom" => Some((&mut s.map_zoom, MAP_ZOOM_RANGE)),
        "custom_hud.camera_shake" => Some((&mut s.camera_shake, UNIT_RANGE)),
        "custom_hud.fov_intensity" => Some((&mut s.fov_intensity, UNIT_RANGE)),
        "custom_hud.hud_volume" => Some((&mut s.hud_volume, UNIT_RANGE)),
        _ => None,
    }
}

/// The value of an option as the console shows it.
pub fn get(s: &HudSettings, key: &str) -> String {
    let mut copy = *s;
    if let Some(on) = flag(&mut copy, key) {
        return on_off(*on).to_owned();
    }
    if let Some((value, _)) = number(&mut copy, key) {
        return value.to_string();
    }
    match key {
        "custom_hud.preset" => s.preset.to_string(),
        "custom_hud.map_mode" => s.map_mode.to_string(),
        "custom_hud.notifications" => s.notifications.to_string(),
        _ => s.palette.to_string(),
    }
}

/// Sets an option from a console value. A number outside its range is clamped; a value that does not parse changes
/// nothing.
pub fn set(s: &mut HudSettings, key: &str, value: &str) -> Result<(), String> {
    if let Some(on) = flag(s, key) {
        *on = parse_bool(value)?;
        return Ok(());
    }
    if let Some((option, (low, high))) = number(s, key) {
        let parsed: f32 =
            value.trim().parse().map_err(|_| format!("expected a number from {low} to {high}, got {value:?}"))?;
        if !parsed.is_finite() {
            return Err(format!("expected a number from {low} to {high}, got {value:?}"));
        }
        *option = parsed.clamp(low, high);
        return Ok(());
    }
    match key {
        "custom_hud.preset" => s.apply_preset(HudPreset::from_str(value)?),
        "custom_hud.map_mode" => s.map_mode = value.parse()?,
        "custom_hud.notifications" => s.notifications = value.parse()?,
        "custom_hud.palette" => s.palette = value.parse()?,
        _ => return Err(format!("unknown setting {key}")),
    }
    Ok(())
}

/// What an option accepts, for the usage line of a `set` with no value. Switches have none: they flip.
pub fn syntax(key: &str) -> Option<&'static str> {
    Some(match key {
        "custom_hud.preset" => "<minimal|balanced|advanced>",
        "custom_hud.scale" => "<0.75-1.5>",
        "custom_hud.opacity" => "<0.2-1>",
        "custom_hud.text_size" => "<0.8-1.5>",
        "custom_hud.map_mode" => "<off|compact|centered|expanded>",
        "custom_hud.map_opacity" => "<0.1-0.9>",
        "custom_hud.map_zoom" => "<0.5-2>",
        "custom_hud.notifications" => "<all|important|off>",
        "custom_hud.palette" => "<default|deuteranopia|tritanopia>",
        "custom_hud.camera_shake" | "custom_hud.fov_intensity" | "custom_hud.hud_volume" => "<0-1>",
        _ => return None,
    })
}

fn on_off(on: bool) -> &'static str {
    match on {
        true => "on",
        false => "off",
    }
}
