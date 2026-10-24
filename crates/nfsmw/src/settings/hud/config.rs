//! The `[hud]` table of the config file.
//!
//! The key `hud` was a plain switch (`hud = false` hides the HUD), and TOML cannot hold a switch and a table under one
//! key. So the switch is read from either place: `hud = false`, or `enabled = false` inside the table. A write keeps the
//! plain switch while no option of the table is set, and moves it into the table as `enabled` once one is.

use std::str::FromStr;

use toml::{Table, Value};

use super::HudLayer;

/// The key of the table, and of the old switch.
const KEY: &str = "hud";
/// The switch's key inside the table.
const ENABLED: &str = "enabled";

/// The HUD on/off switch: `hud = true/false`, or `enabled` in the `[hud]` table. A value of the wrong kind is reported
/// and left unset.
pub fn read_switch(table: &Table, origin: &str) -> Option<bool> {
    let value = table.get(KEY)?;
    let value = match value.as_table() {
        Some(section) => section.get(ENABLED)?,
        None => value,
    };
    let parsed = value.as_bool();
    if parsed.is_none() {
        log::warn!("{origin}: ignoring `hud`: expected true or false, or a [hud] table");
    }
    parsed
}

/// The options of the `[hud]` table. Each key is read on its own: one of the wrong kind is reported and left unset, the
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
    convert(section.get(key)?).map_err(|e| log::warn!("{origin}: ignoring `[hud] {key}`: {e}")).ok()
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

/// Puts the switch and the options `changes` sets into the file's table, keeping its other keys. With no option set the
/// switch is written where it already is (the plain key, or `enabled` in an existing table).
pub fn merge_into(table: &mut Table, switch: Option<bool>, changes: &HudLayer) {
    if *changes == HudLayer::default() {
        put_switch(table, switch);
        return;
    }
    let entry = table.entry(KEY).or_insert_with(|| Value::Table(Table::new()));
    if !entry.is_table() {
        // The old plain switch becomes the table's `enabled`; a value of any other kind was ignored when read.
        let mut section = Table::new();
        if let Some(on) = entry.as_bool() {
            section.insert(ENABLED.to_owned(), Value::Boolean(on));
        }
        *entry = Value::Table(section);
    }
    let Some(section) = entry.as_table_mut() else { return };
    if let Some(on) = switch {
        section.insert(ENABLED.to_owned(), Value::Boolean(on));
    }
    for (key, value) in entries(changes) {
        section.insert(key.to_owned(), value);
    }
}

fn put_switch(table: &mut Table, switch: Option<bool>) {
    let Some(on) = switch else { return };
    if let Some(section) = table.get_mut(KEY).and_then(Value::as_table_mut) {
        section.insert(ENABLED.to_owned(), Value::Boolean(on));
        return;
    }
    table.insert(KEY.to_owned(), Value::Boolean(on));
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
