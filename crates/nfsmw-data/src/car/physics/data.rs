//! Everything the physics needs from the install, loaded once: the gameplay database, the car bounds
//! and the road surfaces.

use anyhow::{Context, Result};
use blackbox_attrib::{Database, vlt_hash};
use blackbox_collision::{BoundsSet, find_bounds};
use game_install::GameDir;
use std::sync::Arc;

use super::{CarEventSpec, CarGeometry, CarPhysics, Fields, SurfaceTable, car_bounds, car_physics, read_car_bounds};
use crate::shared;
use crate::world::sequences::SequenceCatalog;

pub struct PhysicsData {
    db: Arc<Database>,
    bounds: Arc<Vec<BoundsSet>>,
    sequences: SequenceCatalog,
    /// Grip of each road surface, by name hash (collision hits carry the hash).
    pub surfaces: SurfaceTable,
}

impl PhysicsData {
    pub fn load(dir: &GameDir) -> Result<Self> {
        let db = shared::attributes(dir)?;
        let bounds = read_car_bounds(dir)?;
        let surfaces = SurfaceTable::from_database(&db);
        log::info!("physics data: {} car bounds, {} road surfaces", bounds.len(), surfaces.len());
        Ok(Self { db, bounds, surfaces, sequences: SequenceCatalog::default() })
    }

    /// The gameplay database (`attributes.bin`).
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// The track's installed event packages, shared by every vehicle loaded afterwards.
    pub fn set_sequences(&mut self, sequences: SequenceCatalog) {
        self.sequences = sequences;
    }

    pub fn fragment_dynamics(&self, key: u32) -> Option<Arc<crate::world::PropDynamics>> {
        let collection = self.db.collection_by_key(vlt_hash("smackable"), key)?;
        crate::world::PropDynamics::read(collection).map(Arc::new)
    }

    pub fn sequence(&self, key: u32) -> Option<crate::world::sequences::Sequence> {
        self.sequences.get(key).cloned()
    }

    /// The physics of car type `type_name` (e.g. `BMWM3GTR`), or of a mod car's folder: the clone's physics with the
    /// mod's `[performance]` on top. A vehicle without bounds of its own (the semi tractor variants `semib`,
    /// `semicon`, ...) has the bounds of the nearest ancestor that has them.
    pub fn car(&self, type_name: &str) -> Result<CarPhysics> {
        let mut physics = self.stock_car(type_name)?;
        self.tune(type_name, &mut physics);
        Ok(physics)
    }

    /// [`Self::car`] with the performance upgrades `package` names. A mod car's `[performance]` is applied after the
    /// upgrades, so it holds at every stage. Pass a mod car's folder, not its clone's type name, to get its tuning.
    pub fn car_with_performance(&self, type_name: &str, package: super::PerformancePackage) -> Result<CarPhysics> {
        let mut physics = self.stock_car(type_name)?;
        super::performance::apply(&self.db, crate::car::mods::physics_name(type_name), package, &mut physics)?;
        self.tune(type_name, &mut physics);
        Ok(physics)
    }

    /// The mod's multipliers on a mod car's physics, and its factory nitrous. A car that is not a mod car, or whose mod
    /// asked for nothing, is left alone.
    fn tune(&self, name: &str, physics: &mut CarPhysics) {
        let Some(tuning) = crate::car::mods::tuning(name) else { return };
        if tuning.nitrous && !physics.spec.nos.present() {
            physics.spec.nos = self.first_nitrous_kit(crate::car::mods::physics_name(name), physics);
        }
        tuning.apply(&mut physics.spec);
    }

    /// The nitrous of the first kit the car's shop sells, or a typical kit for a car whose record has none.
    fn first_nitrous_kit(&self, type_name: &str, physics: &CarPhysics) -> blackbox_vehicle::nos::NosSpec {
        let mut package = super::PerformancePackage::default();
        package.levels[super::PerformanceCategory::Nitrous as usize] = 1;
        let mut kitted = physics.clone();
        let fitted = super::performance::apply(&self.db, type_name, package, &mut kitted).is_ok();
        match fitted && kitted.spec.nos.present() {
            true => kitted.spec.nos,
            false => crate::car::mods::TYPICAL_NITROUS,
        }
    }

    fn stock_car(&self, type_name: &str) -> Result<CarPhysics> {
        // A mod car names its clone's physics record (car/mods.rs); any other name is unchanged.
        let type_name = crate::car::mods::physics_name(type_name);
        let (bounds, source) = self
            .lineage(type_name)
            .into_iter()
            .find_map(|name| {
                let bounds = car_bounds(&self.bounds, &name).ok()?;
                let source = find_bounds(&self.bounds, vlt_hash(&name.to_ascii_uppercase()))?;
                Some((bounds, source))
            })
            .with_context(|| format!("{type_name} has no collision bounds in its vehicle lineage"))?;
        let mut physics = car_physics(&self.db, type_name, bounds)?;
        let record = self
            .db
            .collection("pvehicle", &type_name.to_ascii_lowercase())
            .and_then(|v| v.follow("rigidbodyspecs"))
            .map(Fields);
        physics.geometry = CarGeometry::read(Arc::new(source.clone()), record).map(Arc::new);
        // The event sequencer and damage specs come from the nearest ancestor that has them.
        physics.events = self
            .lineage(type_name)
            .into_iter()
            .find_map(|name| {
                let vehicle = self.db.collection("pvehicle", &name.to_ascii_lowercase())?;
                CarEventSpec::read(vehicle, &self.sequences)
            })
            .map(Arc::new);
        Ok(physics)
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

    /// The installed rigid-body behaviour, including tractors that have no attached trailer.
    pub fn is_tractor(&self, name: &str) -> bool {
        self.db
            .collection("pvehicle", &name.to_ascii_lowercase())
            .and_then(|vehicle| vehicle.get_string_key("BEHAVIOR_MECHANIC_RIGIDBODY"))
            .is_some_and(|behaviour| behaviour.hash32 == vlt_hash("RBTractor"))
    }
}
