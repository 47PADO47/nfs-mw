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

    /// The gameplay database (`attributes.bin`).
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// The physics of car type `type_name` (e.g. `BMWM3GTR`). A vehicle without bounds of its own (the
    /// semi tractor variants `semib`, `semicon`, ...) has the bounds of the nearest ancestor that has them.
    pub fn car(&self, type_name: &str) -> Result<CarPhysics> {
        let mut names = self.lineage(type_name).into_iter();
        let first = names.next().unwrap_or_else(|| type_name.to_owned());
        let bounds = match car_bounds(&self.bounds, &first) {
            Ok(bounds) => bounds,
            Err(e) => names.find_map(|name| car_bounds(&self.bounds, &name).ok()).ok_or(e)?,
        };
        car_physics(&self.db, type_name, bounds)
    }

    /// The `pvehicle` named `name` and then its ancestors, by name (`semib`, `semi`, `tractors`); just
    /// `name` when the record is unknown.
    pub fn lineage(&self, name: &str) -> Vec<String> {
        let Some(vehicle) = self.db.collection("pvehicle", &name.to_ascii_lowercase()) else {
            return vec![name.to_owned()];
        };
        vehicle.lineage().filter_map(|c| c.name().map(str::to_owned)).collect()
    }

    /// The trailer a tractor pulls: the `pvehicle` its `Trailer` field refers to (`semib` pulls `trailerb`).
    pub fn trailer_of(&self, tractor: &str) -> Option<String> {
        let vehicle = self.db.collection("pvehicle", &tractor.to_ascii_lowercase())?;
        Some(vehicle.follow("Trailer")?.name()?.to_owned())
    }
}
