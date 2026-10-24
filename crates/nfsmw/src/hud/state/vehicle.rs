//! What the HUD knows of the car beyond the gauges: the nitrous burn, the shift point, the engine load, the lateral
//! acceleration, and the condition of the tyres and the body. Plain data the scene fills from the simulation
//! (`scenes/world/drive/sim/hud.rs`); a metric the simulation does not compute is absent, never a placeholder.

use blackbox_damage::{Damage, MAX_LEVEL};

/// The body damage from which the condition warns of heavy damage: half of the panel and paint levels the zones can
/// take. A presentation threshold, not a game value.
pub const HEAVY_DAMAGE: f32 = 0.5;

/// The extra telemetry of the car the HUD follows.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HudTelemetry {
    /// The nitrous burns (held, in gear, on the throttle, fast enough), not only asked for.
    pub nos_burning: bool,
    /// The rpm at which the gearbox wants the next gear from the current one (`Powertrain::shift_points`). `None` in
    /// reverse, in neutral and in the top gear, where no gear follows.
    pub shift_up_rpm: Option<f32>,
    /// The engine load: the throttle the powertrain applies, 0 to 1.
    pub throttle: f32,
    /// The sideways acceleration of the car, in g, positive to the car's right. Zero with every wheel off the ground:
    /// a car in the air has no sideways grip (see `lateral_g` in `scenes/world/drive/sim/hud.rs`).
    pub lateral_g: f32,
}

/// The state of one tyre (`blackbox_vehicle::tires::TireState`, without the fuse).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TireCondition {
    #[default]
    Sound,
    /// Punctured: it blows when its fuse runs out (0.5 s, or 3 minutes on run-flat tyres).
    Punctured,
    /// Blown: it brakes and keeps 30 % of its grip.
    Blown,
}

/// The condition of the car: the body, the tyres and whether it still drives. Only what the simulation models: there is
/// no engine temperature, cooling or brake damage to report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleCondition {
    /// The body damage, 0 (as new) to 1 (every zone at its worst), from [`cosmetic_level`]. `None` when the car has no
    /// damage model.
    pub cosmetic: Option<f32>,
    /// The tyres, in wheel order: front left, front right, rear left, rear right.
    pub tires: [TireCondition; 4],
    /// The car no longer drives (`Vehicle::disabled`: the engine blew or the car is wrecked): no throttle, brakes on.
    pub disabled: bool,
    /// The car's health ran out: it is wrecked.
    pub destroyed: bool,
}

impl Default for VehicleCondition {
    fn default() -> Self {
        Self { cosmetic: None, tires: [TireCondition::Sound; 4], disabled: false, destroyed: false }
    }
}

/// The warning a condition calls for, the most serious first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConditionWarning {
    /// The car is wrecked or disabled.
    Critical,
    /// A tyre has blown.
    TireBlown,
    /// A tyre is punctured and will blow.
    TirePunctured,
    /// The body damage passed [`HEAVY_DAMAGE`].
    HeavyDamage,
}

impl VehicleCondition {
    /// The most serious warning the condition calls for; `None` for a car with nothing to warn of.
    #[allow(dead_code)] // read by the HUD events, not built yet
    pub fn warning(&self) -> Option<ConditionWarning> {
        if self.destroyed || self.disabled {
            return Some(ConditionWarning::Critical);
        }
        if self.tires.contains(&TireCondition::Blown) {
            return Some(ConditionWarning::TireBlown);
        }
        if self.tires.contains(&TireCondition::Punctured) {
            return Some(ConditionWarning::TirePunctured);
        }
        if self.cosmetic.is_some_and(|level| level >= HEAVY_DAMAGE) {
            return Some(ConditionWarning::HeavyDamage);
        }
        None
    }

    /// Whether any tyre is punctured or blown.
    #[allow(dead_code)] // read by the HUD events, not built yet
    pub fn tire_damaged(&self) -> bool {
        self.tires.iter().any(|tire| *tire != TireCondition::Sound)
    }
}

/// The body damage of `damage`, 0 to 1: the panel and paint levels of every zone over the most they can reach. Glass is
/// left out, since only the glazed zones have any. A car without zones reads 0.
pub fn cosmetic_level(damage: &Damage) -> f32 {
    let zones = damage.zone_count();
    if zones == 0 {
        return 0.0;
    }
    let sum: u32 = (0..zones).map(|i| damage.zone(i)).map(|zone| u32::from(zone.panel) + u32::from(zone.paint)).sum();
    let most = (zones as u32) * 2 * u32::from(MAX_LEVEL);
    (sum as f32 / most as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
#[path = "vehicle_tests.rs"]
mod tests;
