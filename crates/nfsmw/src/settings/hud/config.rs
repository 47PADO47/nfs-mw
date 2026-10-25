//! The `[custom_hud]` table of the config file. The HUD's on/off switch stays the separate top-level `hud` key.

use std::str::FromStr;

use toml::{Table, Value};

use super::HudLayer;

/// The key of the table.
const KEY: &str = "custom_hud";

/// The options of the `[custom_hud]` table. Each key is read on its own: one of the wrong kind is reported and left unset, the
/// others still apply. Numbers are clamped later, by `HudSettings::from_layer`.
pub fn read(table: &Table, origin: &str) -> HudLayer {
    let Some(section) = table.get(KEY).and_then(Value::as_table) else {
        return HudLayer::default();
    };
    HudLayer {
        preset: choice(section, origin, "preset"),
        scale: number(section, origin, "scale"),
        opacity: number(section, origin, "opacity"),
        text_size: number(section, origin, "text_size"),
        map_mode: choice(section, origin, "map_mode"),
        map_opacity: number(section, origin, "map_opacity"),
        map_zoom: number(section, origin, "map_zoom"),
        advanced_telemetry: boolean(section, origin, "advanced_telemetry"),
        world_markers: boolean(section, origin, "world_markers"),
        race_info: boolean(section, origin, "race_info"),
        pursuit_info: boolean(section, origin, "pursuit_info"),
        free_roam_extras: boolean(section, origin, "free_roam_extras"),
        notifications: choice(section, origin, "notifications"),
        high_contrast: boolean(section, origin, "high_contrast"),
        palette: choice(section, origin, "palette"),
        motion_reduction: boolean(section, origin, "motion_reduction"),
        camera_shake: number(section, origin, "camera_shake"),
        fov_intensity: number(section, origin, "fov_intensity"),
        haptics: boolean(section, origin, "haptics"),
        hud_volume: number(section, origin, "hud_volume"),
    }
}

/// Reads `key` through `convert`; a value of the wrong kind is reported and ignored.
fn field<T>(section: &Table, origin: &str, key: &str, convert: impl Fn(&Value) -> Result<T, String>) -> Option<T> {
    convert(section.get(key)?).map_err(|e| log::warn!("{origin}: ignoring `[custom_hud] {key}`: {e}")).ok()
}

fn number(section: &Table, origin: &str, key: &str) -> Option<f32> {
    field(section, origin, key, |v| {
        let n = match v {
            Value::Integer(n) => *n as f64,
            Value::Float(f) => *f,
            _ => return Err("expected a number".to_owned()),
        };
        match n.is_finite() {
            true => Ok(n as f32),
            false => Err("expected a finite number".to_owned()),
        }
    })
}

fn boolean(section: &Table, origin: &str, key: &str) -> Option<bool> {
    field(section, origin, key, |v| v.as_bool().ok_or_else(|| "expected true or false".to_owned()))
}

fn choice<T: FromStr<Err = String>>(section: &Table, origin: &str, key: &str) -> Option<T> {
    field(section, origin, key, |v| T::from_str(v.as_str().ok_or_else(|| "expected a string".to_owned())?))
}

/// Puts the options `changes` sets into the `[custom_hud]` table of `table`, keeping the rest of the file. A value under
/// the key that is not a table is left alone rather than overwritten.
pub fn merge_into(table: &mut Table, changes: &HudLayer) {
    if *changes == HudLayer::default() {
        return;
    }
    let entry = table.entry(KEY).or_insert_with(|| Value::Table(Table::new()));
    let Some(section) = entry.as_table_mut() else { return };
    for (key, value) in entries(changes) {
        section.insert(key.to_owned(), value);
    }
}

/// The keys and values a layer sets, in the order the table lists them.
fn entries(l: &HudLayer) -> Vec<(&'static str, Value)> {
    let text = |v: Option<String>| v.map(Value::String);
    let float = |v: Option<f32>| v.map(|v| Value::Float(f64::from(v)));
    let flag = |v: Option<bool>| v.map(Value::Boolean);
    let all = [
        ("preset", text(l.preset.map(|v| v.to_string()))),
        ("scale", float(l.scale)),
        ("opacity", float(l.opacity)),
        ("text_size", float(l.text_size)),
        ("map_mode", text(l.map_mode.map(|v| v.to_string()))),
        ("map_opacity", float(l.map_opacity)),
        ("map_zoom", float(l.map_zoom)),
        ("advanced_telemetry", flag(l.advanced_telemetry)),
        ("world_markers", flag(l.world_markers)),
        ("race_info", flag(l.race_info)),
        ("pursuit_info", flag(l.pursuit_info)),
        ("free_roam_extras", flag(l.free_roam_extras)),
        ("notifications", text(l.notifications.map(|v| v.to_string()))),
        ("high_contrast", flag(l.high_contrast)),
        ("palette", text(l.palette.map(|v| v.to_string()))),
        ("motion_reduction", flag(l.motion_reduction)),
        ("camera_shake", float(l.camera_shake)),
        ("fov_intensity", float(l.fov_intensity)),
        ("haptics", flag(l.haptics)),
        ("hud_volume", float(l.hud_volume)),
    ];
    all.into_iter().filter_map(|(key, value)| Some((key, value?))).collect()
}
