//! Everything the physics needs from the install, loaded once: the gameplay database, the car bounds
//! and the road surfaces.

use anyhow::{Context, Result};
use blackbox_attrib::Database;
use blackbox_collision::BoundsSet;
use game_install::GameDir;

use super::{CarPhysics, SurfaceTable, car_bounds, car_physics, read_car_bounds};

pub struct PhysicsData {
    db: Database,
    bounds: Vec<BoundsSet>,
    /// Grip of each road surface, by name hash (collision hits carry the hash).
    pub surfaces: SurfaceTable,
}

impl PhysicsData {
    pub fn load(dir: &GameDir) -> Result<Self> {
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").context("reading attributes.bin")?)
            .context("parsing attributes.bin")?;
        let bounds = read_car_bounds(dir)?;
        let surfaces = SurfaceTable::from_database(&db);
        log::info!("physics data: {} car bounds, {} road surfaces", bounds.len(), surfaces.len());
        Ok(Self { db, bounds, surfaces })
    }

    /// The physics of car type `type_name` (e.g. `BMWM3GTR`).
    pub fn car(&self, type_name: &str) -> Result<CarPhysics> {
        let bounds = car_bounds(&self.bounds, type_name)?;
        car_physics(&self.db, type_name, bounds)
    }
}
