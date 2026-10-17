//! Population controls independent of geometry LOD. Spec: `traffic-presets.md`.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficPreset {
    Reference,
    Extended,
    Custom,
}
names!(
    TrafficPreset,
    "reference, extended or custom",
    [(Self::Reference, "reference"), (Self::Extended, "extended"), (Self::Custom, "custom")]
);
impl TrafficPreset {
    pub const ALL: [Self; 3] = [Self::Reference, Self::Extended, Self::Custom];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reference => "Reference",
            Self::Extended => "Extended",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrafficPolicy {
    pub custom: bool,
    pub spawn_min: u32,
    pub spawn_max: u32,
    pub retire_distance: u32,
    pub offscreen_seconds: u32,
    pub density_percent: u8,
}
impl Default for TrafficPolicy {
    fn default() -> Self {
        Self {
            custom: false,
            spawn_min: 300,
            spawn_max: 400,
            retire_distance: 650,
            offscreen_seconds: 4,
            density_percent: 100,
        }
    }
}
impl TrafficPolicy {
    pub fn extended() -> Self {
        Self {
            custom: false,
            spawn_min: 450,
            spawn_max: 600,
            retire_distance: 1000,
            offscreen_seconds: 8,
            density_percent: 100,
        }
    }
    pub fn sanitized(self) -> Self {
        let spawn_min = self.spawn_min.clamp(150, 1200);
        let spawn_max = self.spawn_max.clamp(spawn_min + 50, 1400);
        Self {
            custom: self.custom,
            spawn_min,
            spawn_max,
            retire_distance: self.retire_distance.clamp(spawn_max + 200, 2400),
            offscreen_seconds: self.offscreen_seconds.clamp(4, 30),
            density_percent: self.density_percent.min(100),
        }
    }
    pub fn density(self, context: f32) -> f32 {
        (context * f32::from(self.density_percent) / 100.0).clamp(0.0, 1.0)
    }
    pub(crate) fn read(value: &toml::Value, origin: &str) -> Option<Self> {
        match value.clone().try_into::<Self>() {
            Ok(v) => Some(v.sanitized()),
            Err(e) => {
                log::warn!("{origin}: invalid traffic tuning: {e}");
                None
            }
        }
    }
    pub(crate) fn merge(self, old: Option<&toml::Value>) -> anyhow::Result<toml::Value> {
        let mut table = old.and_then(toml::Value::as_table).cloned().unwrap_or_default();
        let toml::Value::Table(update) = toml::Value::try_from(self.sanitized())? else { unreachable!() };
        table.extend(update);
        Ok(toml::Value::Table(table))
    }
}

impl super::Settings {
    pub fn traffic_preset(&self) -> TrafficPreset {
        let p = TrafficPolicy { density_percent: 100, ..self.traffic_policy };
        match p {
            p if p == TrafficPolicy::default() => TrafficPreset::Reference,
            p if p == TrafficPolicy::extended() => TrafficPreset::Extended,
            _ => TrafficPreset::Custom,
        }
    }
    pub fn select_traffic_preset(&mut self, preset: TrafficPreset) {
        let density = self.traffic_policy.density_percent;
        match preset {
            TrafficPreset::Reference => {
                self.traffic_policy = TrafficPolicy::default();
            }
            TrafficPreset::Extended => {
                self.traffic_policy = TrafficPolicy::extended();
            }
            TrafficPreset::Custom => self.traffic_policy.custom = true,
        }
        self.traffic_policy.density_percent = density;
    }
}

#[cfg(test)]
mod tests;
