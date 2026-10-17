//! The Extra Options pursuit keys: the heat range of a forced chase, and the speeds over which the Speeding and the
//! Reckless Driving infractions are recorded. Each is set only when the file sets it. Absent, a chase keeps the
//! vanilla heat range, Speeding keeps the port's own threshold, and no Reckless Driving is recorded.

use blackbox_pursuit::heat::MAX_LEVEL;

use super::ExtraOptions;

/// The range of an infraction speed the file may set, m/s.
const MAX_SPEED_MPS: f32 = 1000.0;

/// The heat range a chase is held to: the heat it starts at, and the most it may reach.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatBounds {
    pub min: f32,
    pub max: f32,
}

impl Default for HeatBounds {
    /// The vanilla range: from heat 1 up to the top level.
    fn default() -> Self {
        Self { min: 1.0, max: MAX_LEVEL as f32 }
    }
}

impl ExtraOptions {
    /// The forced heat range, when the file turns the override on. Absent min or max keep the vanilla end.
    pub fn heat_bounds(&self) -> Option<HeatBounds> {
        if self.heat_override != Some(true) {
            return None;
        }
        let vanilla = HeatBounds::default();
        Some(HeatBounds {
            min: self.heat_min.map_or(vanilla.min, level),
            max: self.heat_max.map_or(vanilla.max, level),
        })
    }

    /// The speed (m/s) over which the Speeding infraction is recorded, when the file sets it.
    pub fn speeding_limit(&self) -> Option<f32> {
        self.speeding_limit_mps.map(speed)
    }

    /// The speed (m/s) over which the Reckless Driving infraction is recorded, when the file sets it.
    pub fn reckless_limit(&self) -> Option<f32> {
        self.reckless_limit_mps.map(speed)
    }
}

/// A heat level: the nearest whole number, from 1 to the top level.
fn level(value: f32) -> f32 {
    value.round().clamp(1.0, MAX_LEVEL as f32)
}

/// An infraction speed, clamped to the range.
fn speed(value: f32) -> f32 {
    value.clamp(0.0, MAX_SPEED_MPS)
}
