//! Geometry choices and explicitly separate draw-range enhancements.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorldDetail {
    Low,
    Medium,
    High,
    #[default]
    Maximum,
}
names!(
    WorldDetail,
    "low, medium, high or maximum",
    [(Self::Low, "low"), (Self::Medium, "medium"), (Self::High, "high"), (Self::Maximum, "maximum")]
);
impl WorldDetail {
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Maximum];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Maximum => "Maximum",
        }
    }
    pub const fn mip_discard(self) -> u32 {
        match self {
            Self::Low => 2,
            Self::Medium => 1,
            _ => 0,
        }
    }
    pub const fn coarse(self) -> bool {
        matches!(self, Self::Low | Self::Medium)
    }
    pub const fn admits(self, flags: u32) -> bool {
        match self {
            Self::Maximum => flags & 0x40000 == 0 || flags & 0x100 != 0,
            _ => flags & 0x80000 == 0,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetailRange {
    #[default]
    #[serde(rename = "1x")]
    Original,
    #[serde(rename = "1.5x")]
    Extended,
    #[serde(rename = "2x")]
    Double,
}
names!(DetailRange, "1x, 1.5x or 2x", [(Self::Original, "1x"), (Self::Extended, "1.5x"), (Self::Double, "2x")]);
impl DetailRange {
    pub const ALL: [Self; 3] = [Self::Original, Self::Extended, Self::Double];
    pub const fn factor(self) -> f32 {
        match self {
            Self::Original => 1.0,
            Self::Extended => 1.5,
            Self::Double => 2.0,
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Original => "1×",
            Self::Extended => "1.5×",
            Self::Double => "2×",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CarGeometry {
    Low,
    High,
    #[default]
    Highest,
}
names!(CarGeometry, "low, high or highest", [(Self::Low, "low"), (Self::High, "high"), (Self::Highest, "highest")]);
impl CarGeometry {
    pub const ALL: [Self; 3] = [Self::Low, Self::High, Self::Highest];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "Low (adaptive)",
            Self::High => "High (adaptive)",
            Self::Highest => "Always highest",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DetailSettings {
    pub world: WorldDetail,
    pub geometry_range: DetailRange,
    pub visibility_range: DetailRange,
    pub highest_world: bool,
    pub car: CarGeometry,
    pub highest_player: bool,
}
impl DetailSettings {
    pub(crate) fn read(value: &toml::Value, origin: &str) -> Option<Self> {
        match value.clone().try_into() {
            Ok(v) => Some(v),
            Err(e) => {
                log::warn!("{origin}: invalid detail settings: {e}");
                None
            }
        }
    }
    pub(crate) fn merge(self, old: Option<&toml::Value>) -> anyhow::Result<toml::Value> {
        let mut table = old.and_then(toml::Value::as_table).cloned().unwrap_or_default();
        let toml::Value::Table(update) = toml::Value::try_from(self)? else { unreachable!() };
        table.extend(update);
        Ok(toml::Value::Table(table))
    }
}
#[cfg(test)]
mod tests;
