//! Tuning of the restored line sparks and the speed trail, the options of the Xenon Effects restoration mod's INI that
//! the rewrite can honour (`docs/mods/nfsmw_xenoneffects.md`, `docs/specs/xenon-particle-motion.md`). Every default keeps
//! the effects as they are without it, so a config file without the table changes nothing.
//!
//! Config only: the `[xenon_effects]` table with `contrail_speed`, `contrail_min_intensity`, `contrail_max_intensity`,
//! `spark_intensity`, `nos_contrails`, `nis_contrails`, `contrail_rate`, `spark_rate`, `max_sparks`, `max_trails` and
//! `shadows`. A value of the wrong kind is reported and ignored; a number outside its range is clamped.

use toml::{Table, Value};

/// The speed in m/s from which the trail shows. The restoration's `ContrailSpeed`.
pub const DEFAULT_CONTRAIL_SPEED: f32 = 44.0;
/// The trail's strength at that speed, and from twice that speed up (`ContrailMinIntensity`, `ContrailMaxIntensity`).
pub const DEFAULT_CONTRAIL_MIN: f32 = 0.1;
pub const DEFAULT_CONTRAIL_MAX: f32 = 0.75;
/// The intensity the line sparks emit at (`SparkIntensity`): one keeps the authored colour and count.
pub const DEFAULT_SPARK_INTENSITY: f32 = 1.0;
/// How often the trail and the sparks emit, per second. The restoration limits them to 30 and 60 frames per second;
/// the rewrite emits both from its fixed 60 Hz step (`docs/specs/xenon-particle-motion.md`).
pub const DEFAULT_RATE: f32 = 60.0;
/// The live particle caps of the restored sparks and the trail.
pub const DEFAULT_MAX_SPARKS: usize = 2048;
pub const DEFAULT_MAX_TRAILS: usize = 256;

const SPEED_RANGE: (f32, f32) = (1.0, 200.0);
const SPARK_INTENSITY_RANGE: (f32, f32) = (0.0, 4.0);
const RATE_RANGE: (f32, f32) = (1.0, 240.0);
const CAP_RANGE: (usize, usize) = (16, 10_000);

/// The options as the settings hold them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XenonEffects {
    /// The speed in m/s from which the trail shows (`contrail_speed`, 1 to 200).
    pub contrail_speed: f32,
    /// The trail's strength at that speed (`contrail_min_intensity`, 0 to 1).
    pub contrail_min: f32,
    /// The trail's strength from twice that speed up (`contrail_max_intensity`, from the minimum to 1).
    pub contrail_max: f32,
    /// The intensity the line sparks emit at (`spark_intensity`, 0 to 4).
    pub spark_intensity: f32,
    /// The trail also shows while the nitrous burns, at its full strength from the first moment (`nos_contrails`).
    pub nos_contrails: bool,
    /// The trail also shows on the player's car in a cutscene (`nis_contrails`).
    pub nis_contrails: bool,
    /// Trail emissions per second (`contrail_rate`, 1 to 240).
    pub contrail_rate: f32,
    /// Spark emissions per second while scraping (`spark_rate`, 1 to 240).
    pub spark_rate: f32,
    /// The most restored sparks alive at once (`max_sparks`, 16 to 10000).
    pub max_sparks: usize,
    /// The most trail particles alive at once (`max_trails`, 16 to 10000).
    pub max_trails: usize,
    /// The sparks and the trail take the sun's shadow (`shadows`).
    pub shadows: bool,
}

impl Default for XenonEffects {
    fn default() -> Self {
        Self::from_layer(XenonEffectsLayer::default())
    }
}

impl XenonEffects {
    /// The values of a merged layer, with the defaults where no layer set one.
    pub fn from_layer(layer: XenonEffectsLayer) -> Self {
        let min = layer.contrail_min.unwrap_or(DEFAULT_CONTRAIL_MIN).clamp(0.0, 1.0);
        Self {
            contrail_speed: clamped(layer.contrail_speed, DEFAULT_CONTRAIL_SPEED, SPEED_RANGE),
            contrail_min: min,
            contrail_max: layer.contrail_max.unwrap_or(DEFAULT_CONTRAIL_MAX).clamp(min, 1.0),
            spark_intensity: clamped(layer.spark_intensity, DEFAULT_SPARK_INTENSITY, SPARK_INTENSITY_RANGE),
            nos_contrails: layer.nos_contrails.unwrap_or(false),
            nis_contrails: layer.nis_contrails.unwrap_or(false),
            contrail_rate: clamped(layer.contrail_rate, DEFAULT_RATE, RATE_RANGE),
            spark_rate: clamped(layer.spark_rate, DEFAULT_RATE, RATE_RANGE),
            max_sparks: layer.max_sparks.unwrap_or(DEFAULT_MAX_SPARKS).clamp(CAP_RANGE.0, CAP_RANGE.1),
            max_trails: layer.max_trails.unwrap_or(DEFAULT_MAX_TRAILS).clamp(CAP_RANGE.0, CAP_RANGE.1),
            shadows: layer.shadows.unwrap_or(false),
        }
    }

    /// How strong the trail is at `speed` (m/s), or `None` when it does not show. From the activation speed the strength
    /// rises from the minimum to the maximum at twice that speed. While the nitrous burns and `nos_contrails` is on the
    /// trail shows at any speed and at the maximum, with no rise.
    pub fn trail_strength(&self, speed: f32, nitrous: bool) -> Option<f32> {
        if nitrous && self.nos_contrails {
            return Some(self.contrail_max);
        }
        if speed < self.contrail_speed {
            return None;
        }
        let rise = (speed / self.contrail_speed - 1.0).clamp(0.0, 1.0);
        Some(self.contrail_min + (self.contrail_max - self.contrail_min) * rise)
    }
}

fn clamped(value: Option<f32>, default: f32, (low, high): (f32, f32)) -> f32 {
    value.unwrap_or(default).clamp(low, high)
}

/// One layer of the options: what a config file set.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct XenonEffectsLayer {
    pub contrail_speed: Option<f32>,
    pub contrail_min: Option<f32>,
    pub contrail_max: Option<f32>,
    pub spark_intensity: Option<f32>,
    pub nos_contrails: Option<bool>,
    pub nis_contrails: Option<bool>,
    pub contrail_rate: Option<f32>,
    pub spark_rate: Option<f32>,
    pub max_sparks: Option<usize>,
    pub max_trails: Option<usize>,
    pub shadows: Option<bool>,
}

impl XenonEffectsLayer {
    /// This layer, with `lower` filling in what it leaves unset.
    pub fn or(self, lower: Self) -> Self {
        Self {
            contrail_speed: self.contrail_speed.or(lower.contrail_speed),
            contrail_min: self.contrail_min.or(lower.contrail_min),
            contrail_max: self.contrail_max.or(lower.contrail_max),
            spark_intensity: self.spark_intensity.or(lower.spark_intensity),
            nos_contrails: self.nos_contrails.or(lower.nos_contrails),
            nis_contrails: self.nis_contrails.or(lower.nis_contrails),
            contrail_rate: self.contrail_rate.or(lower.contrail_rate),
            spark_rate: self.spark_rate.or(lower.spark_rate),
            max_sparks: self.max_sparks.or(lower.max_sparks),
            max_trails: self.max_trails.or(lower.max_trails),
            shadows: self.shadows.or(lower.shadows),
        }
    }
}

/// The `[xenon_effects]` table of the config file.
pub fn read(table: &Table) -> XenonEffectsLayer {
    let Some(section) = table.get("xenon_effects").and_then(Value::as_table) else {
        return XenonEffectsLayer::default();
    };
    XenonEffectsLayer {
        contrail_speed: float(section, "contrail_speed"),
        contrail_min: float(section, "contrail_min_intensity"),
        contrail_max: float(section, "contrail_max_intensity"),
        spark_intensity: float(section, "spark_intensity"),
        nos_contrails: boolean(section, "nos_contrails"),
        nis_contrails: boolean(section, "nis_contrails"),
        contrail_rate: float(section, "contrail_rate"),
        spark_rate: float(section, "spark_rate"),
        max_sparks: count(section, "max_sparks"),
        max_trails: count(section, "max_trails"),
        shadows: boolean(section, "shadows"),
    }
}

fn float(section: &Table, name: &str) -> Option<f32> {
    let value = section.get(name)?;
    let parsed = match value {
        Value::Float(f) => Some(*f as f32),
        Value::Integer(n) => Some(*n as f32),
        _ => None,
    };
    let parsed = parsed.filter(|f| f.is_finite());
    if parsed.is_none() {
        log::warn!("config [xenon_effects] {name}: expected a number");
    }
    parsed
}

fn boolean(section: &Table, name: &str) -> Option<bool> {
    let value = section.get(name)?;
    let parsed = value.as_bool();
    if parsed.is_none() {
        log::warn!("config [xenon_effects] {name}: expected true or false");
    }
    parsed
}

fn count(section: &Table, name: &str) -> Option<usize> {
    let value = section.get(name)?;
    let parsed = value.as_integer().and_then(|n| usize::try_from(n).ok());
    if parsed.is_none() {
        log::warn!("config [xenon_effects] {name}: expected a whole number, zero or more");
    }
    parsed
}
