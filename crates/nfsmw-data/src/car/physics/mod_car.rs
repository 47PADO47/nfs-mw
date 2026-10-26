//! The collision and impact data of a mod car, from its own `BOUNDS.BIN` and nothing a stock car holds.

use std::sync::Arc;

use anyhow::{Context, Result};
use game_install::GameDir;

use super::{CarBounds, CarEventSpec, CarGeometry, mod_car_bounds};

/// What a mod car's physics takes in place of the stock car's vehicle record: the box its rigid body is built from, the
/// contact shapes of its body, and its damage numbers.
#[derive(Debug, Clone)]
pub struct ModCarCollision {
    /// The root box of the car's bounds: also `VehicleSpec::dimension` (its half dimensions) and the offset of the
    /// body from the model's origin (its pivot).
    pub bounds: CarBounds,
    /// The boxes, spheres and ground mesh of the car's body, from its own set.
    pub geometry: Arc<CarGeometry>,
    /// The damage numbers: [`CarEventSpec::racing_defaults`].
    pub events: Arc<CarEventSpec>,
}

/// The collision data of the mod car in folder `folder`: [`mod_car_bounds`] for the box and the set, [`CarGeometry::for_mod_car`]
/// for the contact shapes and [`CarEventSpec::racing_defaults`] for the damage. Fails, naming the car, when the car has no
/// valid `BOUNDS.BIN`: a mod car cannot drive without its own bounds.
pub fn mod_car_collision(dir: &GameDir, folder: &str) -> Result<ModCarCollision> {
    let (bounds, set) = mod_car_bounds(dir, folder)?;
    let geometry = CarGeometry::for_mod_car(Arc::new(set))
        .with_context(|| format!("{folder}: the bounds have no shape to collide with"))?;
    Ok(ModCarCollision { bounds, geometry: Arc::new(geometry), events: Arc::new(CarEventSpec::racing_defaults()) })
}

#[cfg(test)]
mod tests;
