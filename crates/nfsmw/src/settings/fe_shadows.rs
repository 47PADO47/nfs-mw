//! Front-end shadows follow the selected renderer's shadow settings. `fe_shadows = false` preserves the original
//! unshadowed backdrop; the default shares the driving scene's shadows (`docs/specs/fe-shadows.md`).
//!
//! Config (top-level keys, next to `shadows`): `fe_shadows` (on or off), `fe_shadows_cars` (the car casts and receives),
//! `fe_shadows_bias` (0.25 to 4.0) and `fe_shadows_divider` (1, 2 or 4). Environment: `NFSMW_FE_SHADOWS`. Command line:
//! `--fe-shadows` / `--no-fe-shadows`. Console: `fe_shadows`, `fe_shadows_cars`, `fe_shadows_bias`, `fe_shadows_divider`.

use std::sync::RwLock;

use bevy_ecs::prelude::Res;
use blackbox_gfx::ShadowQuality;
use blackbox_gfx::settings::ShadowTuning;
use clap::Args;
use toml::{Table, Value};

use super::Settings;
use super::partial::parse_bool;

/// The menu follows the driving scene's shadow settings unless explicitly disabled.
pub const DEFAULT_ENABLED: bool = true;
/// The car casts and receives when the front-end shadows are on (the spec's proposal; see `docs/specs/fe-shadows.md`).
pub const DEFAULT_CARS: bool = true;
/// The bias multiplier: 1 keeps the Bevy values.
pub const DEFAULT_BIAS: f32 = 1.0;
/// The lowest and highest bias multiplier a setting may hold. Values outside are clamped.
pub const BIAS_RANGE: (f32, f32) = (0.25, 4.0);
/// The map edge divider: the front-end map is the quality's edge over this.
pub const DEFAULT_DIVIDER: u32 = 1;
/// The dividers a setting may hold.
pub const DIVIDERS: [u32; 3] = [1, 2, 4];

/// The config key, and the environment variable and console key of the switch.
pub const KEY: &str = "fe_shadows";
pub const ENV: &str = "NFSMW_FE_SHADOWS";
/// The console and config keys of the other options.
pub const CARS_KEY: &str = "fe_shadows_cars";
pub const BIAS_KEY: &str = "fe_shadows_bias";
pub const DIVIDER_KEY: &str = "fe_shadows_divider";
/// Every console key of the front-end shadows.
pub const KEYS: [&str; 4] = [KEY, CARS_KEY, BIAS_KEY, DIVIDER_KEY];

/// The options as the settings hold them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeShadows {
    /// The front-end scenes draw a sun shadow pass (`fe_shadows`).
    pub enabled: bool,
    /// The car casts and receives the shadows (`fe_shadows_cars`).
    pub cars: bool,
    /// The bias multiplier on the sun's depth and normal bias (`fe_shadows_bias`).
    pub bias: f32,
    /// The map edge divider, 1, 2 or 4 (`fe_shadows_divider`).
    pub divider: u32,
}

impl Default for FeShadows {
    fn default() -> Self {
        Self { enabled: DEFAULT_ENABLED, cars: DEFAULT_CARS, bias: DEFAULT_BIAS, divider: DEFAULT_DIVIDER }
    }
}

impl FeShadows {
    /// The values of a merged layer, with the defaults where no layer set one.
    pub fn from_layer(layer: FeShadowsLayer) -> Self {
        Self {
            enabled: layer.enabled.unwrap_or(DEFAULT_ENABLED),
            cars: layer.cars.unwrap_or(DEFAULT_CARS),
            bias: layer.bias.map_or(DEFAULT_BIAS, clamp_bias),
            divider: layer.divider.unwrap_or(DEFAULT_DIVIDER),
        }
    }

    /// These options as the renderer runs them. Both native caster flags and Bevy soft shadows are supported.
    pub fn on_renderer(self, shadows: bool) -> Self {
        Self { enabled: self.enabled && shadows, ..self }
    }
}

/// One layer of the options: what a config file, the environment or the command line set.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FeShadowsLayer {
    pub enabled: Option<bool>,
    pub cars: Option<bool>,
    pub bias: Option<f32>,
    pub divider: Option<u32>,
}

impl FeShadowsLayer {
    /// This layer, with `lower` filling in what it leaves unset.
    pub fn or(self, lower: Self) -> Self {
        Self {
            enabled: self.enabled.or(lower.enabled),
            cars: self.cars.or(lower.cars),
            bias: self.bias.or(lower.bias),
            divider: self.divider.or(lower.divider),
        }
    }
}

impl From<FeShadows> for FeShadowsLayer {
    /// Every option set, for writing the whole set of values to the config file.
    fn from(value: FeShadows) -> Self {
        Self {
            enabled: Some(value.enabled),
            cars: Some(value.cars),
            bias: Some(value.bias),
            divider: Some(value.divider),
        }
    }
}

/// What the menu backdrop reads each frame: the front-end options and the world's shadow quality, as the settings
/// hold them now. Kept in step by [`publish`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub options: FeShadows,
    pub world_quality: ShadowQuality,
}

const OFF: Snapshot = Snapshot {
    options: FeShadows { enabled: false, cars: DEFAULT_CARS, bias: DEFAULT_BIAS, divider: DEFAULT_DIVIDER },
    world_quality: ShadowQuality::Off,
};

/// The state in force now (see [`Snapshot`]).
static ACTIVE: RwLock<Snapshot> = RwLock::new(OFF);

/// The front-end shadow state in force now.
pub fn active() -> Snapshot {
    match ACTIVE.read() {
        Ok(value) => *value,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Copies the options and the world's quality to [`active`]. Runs every frame, so a menu or console change takes effect
/// at once.
pub fn publish(settings: Res<Settings>) {
    let _trace = blackbox_trace::scope("Settings: publish frontend shadows");
    let value = Snapshot { options: settings.fe_shadows, world_quality: settings.shadows };
    let Ok(mut active) = ACTIVE.write() else { return };
    *active = value;
}

/// The sun's tuning follows the global quality, including Off. Native world-space shadows use the renderer's
/// shared sun-shadow options; this frame override also carries the global quality to the Bevy backend.
pub fn tuning(snapshot: Snapshot) -> Option<ShadowTuning> {
    if !snapshot.options.enabled {
        return None;
    }
    let quality = snapshot.world_quality;
    Some(ShadowTuning { quality, bias: snapshot.options.bias, divider: snapshot.options.divider as usize })
}

/// Clamps a bias multiplier into [`BIAS_RANGE`]. A value that is not a number gives the default.
pub fn clamp_bias(bias: f32) -> f32 {
    if !bias.is_finite() {
        return DEFAULT_BIAS;
    }
    bias.clamp(BIAS_RANGE.0, BIAS_RANGE.1)
}

/// Whether `divider` is one a setting may hold.
pub fn is_divider(divider: u32) -> bool {
    DIVIDERS.contains(&divider)
}

/// The bias from a console value. Out-of-range numbers are clamped; text that is not a number is an error.
pub fn parse_bias(text: &str) -> Result<f32, String> {
    let bias = text.trim().parse::<f32>().map_err(|_| format!("expected a number, got {text:?}"))?;
    if !bias.is_finite() {
        return Err(format!("expected a number, got {text:?}"));
    }
    Ok(clamp_bias(bias))
}

/// The divider from a console value: 1, 2 or 4.
pub fn parse_divider(text: &str) -> Result<u32, String> {
    let divider = text.trim().parse::<u32>().map_err(|_| format!("expected 1, 2 or 4, got {text:?}"))?;
    if !is_divider(divider) {
        return Err(format!("expected 1, 2 or 4, got {text:?}"));
    }
    Ok(divider)
}

/// The `fe_shadows*` keys of the config file. A value of the wrong kind, or an unknown divider, is reported and left
/// unset.
pub fn read(table: &Table, origin: &str) -> FeShadowsLayer {
    FeShadowsLayer {
        enabled: toml_value(table, origin, KEY, toml_bool),
        cars: toml_value(table, origin, CARS_KEY, toml_bool),
        bias: toml_value(table, origin, BIAS_KEY, toml_bias),
        divider: toml_value(table, origin, DIVIDER_KEY, toml_divider),
    }
}

/// Read `key` through `convert`; a value of the wrong kind is reported and ignored.
fn toml_value<T>(table: &Table, origin: &str, key: &str, convert: fn(&Value) -> Result<T, String>) -> Option<T> {
    convert(table.get(key)?).map_err(|e| log::warn!("{origin}: ignoring `{key}`: {e}")).ok()
}

fn toml_bool(value: &Value) -> Result<bool, String> {
    value.as_bool().ok_or_else(|| "expected true or false".to_owned())
}

fn toml_bias(value: &Value) -> Result<f32, String> {
    let number = value.as_float().or_else(|| value.as_integer().map(|i| i as f64));
    let number = number.ok_or_else(|| "expected a number".to_owned())?;
    Ok(clamp_bias(number as f32))
}

fn toml_divider(value: &Value) -> Result<u32, String> {
    let divider = value.as_integer().and_then(|i| u32::try_from(i).ok());
    divider.filter(|d| is_divider(*d)).ok_or_else(|| "expected 1, 2 or 4".to_owned())
}

/// The environment variable of the switch. A value that does not parse is ignored.
pub fn from_env(get: impl Fn(&str) -> Option<String>) -> FeShadowsLayer {
    FeShadowsLayer { enabled: get(ENV).and_then(|text| parse_bool(&text).ok()), ..Default::default() }
}

/// Puts the options `changes` sets into the config file's top-level keys, through `put` (the writer of
/// `write::merge`), so the rest of the file stays as it is.
pub fn put_into(changes: &FeShadowsLayer, put: &mut impl FnMut(&str, Value)) {
    if let Some(v) = changes.enabled {
        put(KEY, Value::Boolean(v));
    }
    if let Some(v) = changes.cars {
        put(CARS_KEY, Value::Boolean(v));
    }
    if let Some(v) = changes.bias {
        put(BIAS_KEY, Value::Float(f64::from(v)));
    }
    if let Some(v) = changes.divider {
        put(DIVIDER_KEY, Value::Integer(i64::from(v)));
    }
}

/// The command-line switch. Unset switches leave the config file or the environment in charge.
#[derive(Debug, Clone, Default, Args)]
pub struct FeShadowsArgs {
    /// Share driving shadows with the front-end scenes [env NFSMW_FE_SHADOWS; config `fe_shadows`; default on].
    #[arg(long, conflicts_with = "no_fe_shadows")]
    pub fe_shadows: bool,
    /// Keep the front-end scenes without shadows.
    #[arg(long)]
    pub no_fe_shadows: bool,
}

impl FeShadowsArgs {
    pub fn layer(&self) -> FeShadowsLayer {
        let enabled = match (self.fe_shadows, self.no_fe_shadows) {
            (true, _) => Some(true),
            (false, true) => Some(false),
            (false, false) => None,
        };
        FeShadowsLayer { enabled, ..Default::default() }
    }
}

/// Whether the console key names one of the options.
pub fn is_key(key: &str) -> bool {
    KEYS.contains(&key)
}

/// The switch a console key names, for a switch with no value. The bias and the divider take a value.
pub fn flag<'a>(settings: &'a mut FeShadows, key: &str) -> Option<&'a mut bool> {
    match key {
        KEY => Some(&mut settings.enabled),
        CARS_KEY => Some(&mut settings.cars),
        _ => None,
    }
}

/// The value of the option as the console shows it.
pub fn get(settings: &FeShadows, key: &str) -> String {
    match key {
        BIAS_KEY => settings.bias.to_string(),
        DIVIDER_KEY => settings.divider.to_string(),
        CARS_KEY => on_off(settings.cars),
        _ => on_off(settings.enabled),
    }
}

fn on_off(value: bool) -> String {
    match value {
        true => "on".to_owned(),
        false => "off".to_owned(),
    }
}

/// Sets the option from a console value: `on` or `off` for the switches, a number for the bias, 1, 2 or 4 for the
/// divider.
pub fn set(settings: &mut FeShadows, key: &str, value: &str) -> Result<(), String> {
    match key {
        BIAS_KEY => settings.bias = parse_bias(value)?,
        DIVIDER_KEY => settings.divider = parse_divider(value)?,
        _ => {
            let Some(option) = flag(settings, key) else { return Err(format!("unknown setting {key}")) };
            *option = parse_bool(value)?;
        }
    }
    Ok(())
}
