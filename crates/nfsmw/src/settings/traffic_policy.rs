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
    /// The preset's English name, for the F8 graphics lab (which stays in English).
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reference => "Reference",
            Self::Extended => "Extended",
            Self::Custom => "Custom",
        }
    }
    /// The preset's name in the current language, as the options show it.
    pub fn menu_label(self) -> String {
        let id = match self {
            Self::Reference => "options-traffic-reference",
            Self::Extended => "options-traffic-extended",
            Self::Custom => "options-custom",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopulationPreset {
    Normal,
    High,
    VeryHigh,
    Custom,
}
names!(
    PopulationPreset,
    "normal, high, very-high or custom",
    [(Self::Normal, "normal"), (Self::High, "high"), (Self::VeryHigh, "very-high"), (Self::Custom, "custom")]
);
impl PopulationPreset {
    pub const ALL: [Self; 4] = [Self::Normal, Self::High, Self::VeryHigh, Self::Custom];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::High => "High",
            Self::VeryHigh => "Very High",
            Self::Custom => "Custom population",
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
    pub common_fill: bool,
    pub common_rate_percent: u16,
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
            common_fill: false,
            common_rate_percent: 100,
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
            common_fill: false,
            common_rate_percent: 100,
        }
    }
    pub fn range_only(self) -> Self {
        Self { density_percent: 100, common_fill: false, common_rate_percent: 100, ..self }
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
            common_fill: self.common_fill,
            common_rate_percent: self.common_rate_percent.clamp(100, 300),
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
    pub fn population_preset(&self) -> PopulationPreset {
        let p = self.traffic_policy;
        match (self.traffic, p.common_fill, p.common_rate_percent) {
            (20, false, 100) => PopulationPreset::Normal,
            (40, true, 150) => PopulationPreset::High,
            (64, true, 200) => PopulationPreset::VeryHigh,
            _ => PopulationPreset::Custom,
        }
    }
    pub fn select_population_preset(&mut self, preset: PopulationPreset) {
        let (budget, fill, rate) = match preset {
            PopulationPreset::Normal => (20, false, 100),
            PopulationPreset::High => (40, true, 150),
            PopulationPreset::VeryHigh => (64, true, 200),
            PopulationPreset::Custom => return,
        };
        self.traffic = budget;
        self.traffic_policy.common_fill = fill;
        self.traffic_policy.common_rate_percent = rate;
    }
    pub fn traffic_preset(&self) -> TrafficPreset {
        let p = self.traffic_policy.range_only();
        match p {
            p if p == TrafficPolicy::default() => TrafficPreset::Reference,
            p if p == TrafficPolicy::extended() => TrafficPreset::Extended,
            _ => TrafficPreset::Custom,
        }
    }
    pub fn select_traffic_preset(&mut self, preset: TrafficPreset) {
        let saved = self.traffic_policy;
        match preset {
            TrafficPreset::Reference => {
                self.traffic_policy = TrafficPolicy::default();
            }
            TrafficPreset::Extended => {
                self.traffic_policy = TrafficPolicy::extended();
            }
            TrafficPreset::Custom => self.traffic_policy.custom = true,
        }
        self.traffic_policy.density_percent = saved.density_percent;
        self.traffic_policy.common_fill = saved.common_fill;
        self.traffic_policy.common_rate_percent = saved.common_rate_percent;
    }
}

#[cfg(test)]
mod tests;
