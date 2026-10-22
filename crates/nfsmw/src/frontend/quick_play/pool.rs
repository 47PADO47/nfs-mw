//! The pool Quick Play draws from: the cars of the front-end catalogue and the races of the quick race board, limited by the
//! career's stage, and the random pick from it (`docs/specs/quick-play.md` sections 2 and 3).
//!
//! The cars are read on a worker thread when the front end starts (the physics of the whole catalogue are measured for the
//! stat bars, which takes a moment); the races are the quick race board (`quick_race.rs`).

use std::sync::{Arc, OnceLock};

use blackbox_attrib::vlt_hash;
use game_install::GameDir;
use nfsmw_career::carlot::stats::CarStats;
use nfsmw_data::car::physics::PhysicsData;
use nfsmw_data::career_cars::{self, CarClass, CatalogueCar};

use crate::frontend::quick_race::Track;

/// How many Blacklist positions past the current one the pool reaches: "all cars and tracks for the two upcoming Blacklist
/// positions" (label `0x7C3E459E`). **[ours]** for the arithmetic; the label is **[exe]**.
pub const UPCOMING: u8 = 2;

/// The stage with no profile loaded: the prologue, where a career starts.
pub const PROLOGUE_STAGE: u8 = 16;

/// A car Quick Play can pick.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolCar {
    /// The front-end collection's name (`cobaltss`); the model-name picture is named after it.
    pub name: String,
    /// The car type folder under `CARS/` the car is built from.
    pub folder: String,
    /// The unlock rank: the car is available while the stage is at most this (`LotCar::unlock_rank`).
    pub rank: u8,
    /// The handle the car would have in a stable (`vlt_hash` of the name), which the unlock sentences key on.
    pub handle: u32,
    /// The manufacturer index (1 BMW ... 25 Corvette), `None` for a car with none.
    pub manufacturer: Option<u8>,
    /// What the stat bars measure, `None` for a car whose physics cannot be read.
    pub stats: Option<CarStats>,
}

/// A preset ride of the install (`BL10`, `M3GTRCAREERSTART`): a car with its paint and parts, which the custom race's car
/// select lists in its Bonus and Preset lists.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolPreset {
    /// The preset's name, which the car's parts are built from (`LoadOptions::preset`).
    pub name: String,
    /// The car the preset is made of. Its name is the front-end collection the preset points at (`cayman`), which the logos
    /// are named after; the preset's own name when the catalogue has no such car.
    pub car: PoolCar,
}

/// The cars Quick Play can pick from, with the stats of all of them (the stat bars fill relative to that population).
#[derive(Debug, Default)]
pub struct CarPool {
    pub cars: Vec<PoolCar>,
    pub population: Vec<CarStats>,
    pub presets: Vec<PoolPreset>,
}

static CARS: OnceLock<Arc<CarPool>> = OnceLock::new();

/// Starts reading the cars in the background. Does nothing when it was started already.
pub fn load_in_background(dir: &GameDir) {
    if CARS.get().is_some() {
        return;
    }
    let dir = dir.clone();
    let spawned = std::thread::Builder::new().name("quick-play-cars".into()).spawn(move || {
        load_now(&dir);
    });
    if let Err(e) = spawned {
        log::warn!("quick play: cannot read the cars: {e}");
    }
}

/// Reads the cars on this thread and keeps them for [`cars`].
pub fn load_now(dir: &GameDir) -> Arc<CarPool> {
    let _trace = blackbox_trace::scope("Quick Play: load the car pool now");
    if let Some(pool) = CARS.get() {
        return pool.clone();
    }
    let pool = match CarPool::load(dir) {
        Ok(pool) => pool,
        Err(e) => {
            log::warn!("quick play: no car pool: {e:#}");
            CarPool::default()
        }
    };
    log::info!("quick play: {} car(s)", pool.cars.len());
    CARS.get_or_init(|| Arc::new(pool)).clone()
}

/// The car pool, once it has been read.
pub fn cars() -> Option<Arc<CarPool>> {
    CARS.get().cloned()
}

impl CarPool {
    /// Reads the catalogue of the install and keeps the cars Quick Play can drive: a car for sale with a price, of one of the
    /// three car classes, whose folder is under `CARS/`.
    pub fn load(dir: &GameDir) -> anyhow::Result<CarPool> {
        let catalogue = career_cars::load(dir)?;
        let folders = nfsmw_data::car::list(dir);
        let physics = PhysicsData::load(dir).ok();
        let cars = cars_of(&catalogue, &folders, |folder| {
            let physics = physics.as_ref()?;
            crate::career::car_stats(physics, &folder.to_ascii_lowercase())
        });
        let cars = drivable(cars, physics.is_some());
        let population = cars.iter().filter_map(|car| car.stats).collect();
        let rides = nfsmw_data::car::CarTables::load(dir).map(|tables| tables.presets.clone()).unwrap_or_default();
        let presets = presets_of(&catalogue, &folders, &rides, |folder| {
            let physics = physics.as_ref()?;
            crate::career::car_stats(physics, &folder.to_ascii_lowercase())
        });
        Ok(CarPool { cars, population, presets })
    }
}

/// The cars that can be driven. The install's add-on cars (`levin`, `trueno`, `corollae88`) have no `pvehicle` or collision
/// bounds, so no physics can be made for them and the world refuses to start with one. Only a readable physics database can
/// tell, so without one (`physics_read` false) every car stays.
pub fn drivable(mut cars: Vec<PoolCar>, physics_read: bool) -> Vec<PoolCar> {
    if physics_read {
        cars.retain(|car| car.stats.is_some());
    }
    cars
}

/// The pool cars of a catalogue: the offered cars with a folder, in catalogue order. `stats_of` measures a car by its folder.
pub fn cars_of(
    catalogue: &[CatalogueCar],
    folders: &[String],
    stats_of: impl Fn(&str) -> Option<CarStats>,
) -> Vec<PoolCar> {
    catalogue
        .iter()
        .filter_map(|car| {
            let name = car.name.clone()?;
            let rank = car.unlock_rank?;
            let drivable = car.for_sale && car.price > 0;
            let class = matches!(car.class?, CarClass::NorthAmerica | CarClass::Europe | CarClass::Japan);
            if !drivable || !class {
                return None;
            }
            let folder = folder_for(&name, folders)?;
            Some(PoolCar {
                handle: vlt_hash(&name),
                manufacturer: car.manufacturer,
                stats: stats_of(&folder),
                name,
                folder,
                rank,
            })
        })
        .collect()
}

/// The presets whose car has a folder, with the catalogue car their front-end key names (`PresetRide::fe_key`). The key of a
/// preset is the one of the collection it was made from: `BL10` points at the Cayman's.
pub fn presets_of(
    catalogue: &[CatalogueCar],
    folders: &[String],
    rides: &[nfsmw_data::car::PresetRide],
    stats_of: impl Fn(&str) -> Option<CarStats>,
) -> Vec<PoolPreset> {
    rides
        .iter()
        .filter_map(|ride| {
            let folder = folders.iter().find(|f| f.eq_ignore_ascii_case(&ride.car_type_name))?.clone();
            let made_from = catalogue.iter().find(|car| u64::from(car.key) == ride.fe_key);
            let car = PoolCar {
                name: made_from.and_then(|car| car.name.clone()).unwrap_or_else(|| ride.preset_name.clone()),
                handle: vlt_hash(&ride.preset_name),
                manufacturer: made_from.and_then(|car| car.manufacturer),
                rank: made_from.and_then(|car| car.unlock_rank).unwrap_or(0),
                stats: stats_of(&folder),
                folder,
            };
            Some(PoolPreset { name: ride.preset_name.clone(), car })
        })
        .collect()
}

/// The `CARS/` folder of a front-end car. The collection is not always named like the folder (`carrera_gt` is the folder
/// `CARRERAGT`, `a4_32` is `A4`), so the name is tried as it is, without its underscores, and up to its first underscore.
/// **[ours]**: the exe takes the folder from the car's record, which the catalogue does not link.
pub fn folder_for(name: &str, folders: &[String]) -> Option<String> {
    let squeezed: String = name.chars().filter(|c| *c != '_').collect();
    let stem = name.split('_').next().unwrap_or(name);
    [name, squeezed.as_str(), stem]
        .into_iter()
        .find_map(|wanted| folders.iter().find(|folder| folder.eq_ignore_ascii_case(wanted)))
        .cloned()
}

/// Whether a car of this unlock rank is in the pool at this stage: it is available (rank at least the stage) or within the
/// two upcoming positions (rank at least stage minus two). **[ours]**
pub fn car_offered(rank: u8, stage: u8) -> bool {
    rank >= stage.saturating_sub(UPCOMING)
}

/// Whether the profile has the car: its rank is at least the stage (`LotCar::is_available`, `career-car-lot.md` section 1).
pub fn car_unlocked(rank: u8, stage: u8) -> bool {
    rank >= stage
}

/// Whether a race of this Blacklist bin is in the pool at this stage, by the same rule as the cars: bin 15 opens at stage 15,
/// so a bin is offered from two stages before. A race with no bin (the prologue, the collectors, the challenge series, the
/// online events) has no Blacklist position to measure and is never offered. **[ours]**
pub fn race_offered(bin: Option<u8>, stage: u8) -> bool {
    bin.is_some_and(|bin| bin >= stage.saturating_sub(UPCOMING))
}

/// The random numbers Quick Play uses: xorshift64* seeded by the caller, so a test can replay a sequence.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator from a seed. Any seed works, zero included.
    pub fn new(seed: u64) -> Self {
        // splitmix64 spreads the seed over the state, which must not be zero.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Self((z ^ (z >> 31)) | 1)
    }

    /// A generator seeded from the clock.
    pub fn from_clock() -> Self {
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        Self::new(nanos)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number below `len` (0 for an empty range).
    pub fn below(&mut self, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        ((self.next() >> 11) % len as u64) as usize
    }
}

/// The car and the race Quick Play picked, as positions in the lists it picked from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    pub car: usize,
    pub race: usize,
}

/// Picks a car and a race at random from the ones the stage offers. With `last` given, the pick differs from it when the pool
/// allows (Randomize does not give the same pair again). `None` when no car or no race is offered.
pub fn pick(cars: &[PoolCar], races: &[Track], stage: u8, rng: &mut Rng, last: Option<Pick>) -> Option<Pick> {
    let car_ids: Vec<usize> = (0..cars.len()).filter(|i| car_offered(cars[*i].rank, stage)).collect();
    let race_ids: Vec<usize> = (0..races.len()).filter(|i| race_offered(races[*i].bin, stage)).collect();
    if car_ids.is_empty() || race_ids.is_empty() {
        return None;
    }
    let draw =
        |rng: &mut Rng| Pick { car: car_ids[rng.below(car_ids.len())], race: race_ids[rng.below(race_ids.len())] };
    let mut next = draw(rng);
    let only_one = car_ids.len() * race_ids.len() < 2;
    while Some(next) == last && !only_one {
        next = draw(rng);
    }
    Some(next)
}
