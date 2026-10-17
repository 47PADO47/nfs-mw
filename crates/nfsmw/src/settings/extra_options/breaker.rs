//! The Extra Options Speedbreaker keys, as the fallbacks of the SpeedFixer tuning: the passive recharge (its time
//! and the speed above which it refills) and the collision mass scale. Each is set only when the file sets it. The
//! mod's defaults are the vanilla values, so an absent key changes nothing.

use super::ExtraOptions;
use crate::settings::speedbreaker::{Overrides, Recharge, vanilla};

/// The range of the recharge speed, mph (the SpeedFixer range).
const MAX_MPH: f32 = 1000.0;
/// The range of the recharge time, seconds. The floor keeps a zero time from dividing by zero.
const MIN_SECONDS: f32 = 0.001;
const MAX_SECONDS: f32 = 3600.0;
/// The range of the collision mass scale (the SpeedFixer range).
const MIN_MASS: f32 = 0.0;
const MAX_MASS: f32 = 100.0;

impl ExtraOptions {
    /// The Speedbreaker fallbacks these options set. Empty when the file sets none of the keys.
    pub fn speedbreaker(&self) -> Overrides {
        Overrides {
            recharge: recharge(self.sb_recharge_mph, self.sb_recharge_seconds),
            mass_scale: self.sb_mass_multiplier.map(|mass| mass.clamp(MIN_MASS, MAX_MASS)),
        }
    }
}

/// The passive recharge, with the vanilla value for the half the file leaves out. `None` when it sets neither half.
fn recharge(mph: Option<f32>, seconds: Option<f32>) -> Option<Recharge> {
    if mph.is_none() && seconds.is_none() {
        return None;
    }
    Some(Recharge {
        min_speed: mph.map_or(vanilla::RECHARGE_ABOVE_MPS, mph_to_mps),
        seconds: seconds.map_or(vanilla::RECHARGE_SECONDS, |s| s.clamp(MIN_SECONDS, MAX_SECONDS)),
    })
}

/// Miles per hour to metres per second, clamped to the range.
fn mph_to_mps(mph: f32) -> f32 {
    mph.clamp(0.0, MAX_MPH) * vanilla::MPH
}
