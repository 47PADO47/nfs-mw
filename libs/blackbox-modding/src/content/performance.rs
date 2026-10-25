//! The `[performance]` table of a car manifest: how the car drives compared with the stock car it clones.
//!
//! Every number is a multiplier of the clone's own value, so `1.0` (or leaving the key out) keeps the clone as it is. The
//! game applies them after the upgrades the player has bought, so a car that is 10 percent stronger stays 10 percent
//! stronger at every stage.

use serde::Deserialize;

use super::ContentError;

/// How a mod car differs from its clone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Performance {
    /// Engine torque, at every rpm.
    pub power: f32,
    /// Body mass.
    pub mass: f32,
    /// Tyre grip, lateral and longitudinal.
    pub grip: f32,
    /// Brake torque of both axles.
    pub brakes: f32,
    /// The car has nitrous from the factory: the clone's first nitrous kit, as if the player had bought it. A car whose
    /// clone has no kit gets a typical one. Without it the car has nitrous only when the player buys the kit.
    pub nitrous: bool,
    /// Torque the nitrous adds, as a multiplier of the kit's own boost. `0.0` turns a car's nitrous off.
    pub nitrous_power: f32,
}

impl Default for Performance {
    fn default() -> Self {
        Self { power: 1.0, mass: 1.0, grip: 1.0, brakes: 1.0, nitrous: false, nitrous_power: 1.0 }
    }
}

impl Performance {
    /// Whether the car drives exactly as its clone does.
    pub fn is_stock(&self) -> bool {
        *self == Self::default()
    }
}

/// The table as serde reads it. Unknown keys are an error, like everywhere in a car manifest.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct Raw {
    power: Option<f32>,
    mass: Option<f32>,
    grip: Option<f32>,
    brakes: Option<f32>,
    nitrous: Option<bool>,
    nitrous_power: Option<f32>,
}

/// Check the table: every multiplier is a finite number inside the range that keeps the physics stable.
pub(super) fn check(raw: Option<Raw>) -> Result<Performance, ContentError> {
    let Some(raw) = raw else { return Ok(Performance::default()) };
    let stock = Performance::default();
    Ok(Performance {
        power: multiplier(raw.power, stock.power, "performance.power", 0.25, 4.0)?,
        mass: multiplier(raw.mass, stock.mass, "performance.mass", 0.25, 4.0)?,
        grip: multiplier(raw.grip, stock.grip, "performance.grip", 0.25, 3.0)?,
        brakes: multiplier(raw.brakes, stock.brakes, "performance.brakes", 0.25, 4.0)?,
        nitrous: raw.nitrous.unwrap_or(stock.nitrous),
        nitrous_power: multiplier(raw.nitrous_power, stock.nitrous_power, "performance.nitrous_power", 0.0, 4.0)?,
    })
}

fn multiplier(value: Option<f32>, stock: f32, field: &'static str, low: f32, high: f32) -> Result<f32, ContentError> {
    let Some(value) = value else { return Ok(stock) };
    if !value.is_finite() || value < low || value > high {
        return Err(ContentError::Field { field, reason: "must be a number inside the range the docs give" });
    }
    Ok(value)
}
