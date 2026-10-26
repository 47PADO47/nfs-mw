//! Cars that mods add (ADR 0005 section 5; docs/modding/api-content.md "Cars").
//!
//! A mod car is a car of its own: it is one more entry in the car table, named by its folder, and it ships its own
//! `CARS/<FOLDER>/` geometry and textures. Nothing of it is taken from a car of the game. [`register`] is the pure part:
//! it takes the stock car table and the mods' requests, and returns the cars it admits and the reasons it refuses the
//! others. The
//! game publishes the admitted cars at start-up and again when a mod adds more later ([`publish`]); every car-table load after that sees them
//! ([`car_types`]), and the physics, cop and garage lookups read them through the functions below.

use std::sync::{PoisonError, RwLock};

use blackbox_carparts::CarTypeInfo;

use crate::pursuit::{CopRecord, HEAT_LEVELS, HeatRow};

pub mod custom;
mod engine;
pub(super) mod parts;
pub use blackbox_vehicle::stats::{CarStats, Drive};
pub use engine::{CarEffects, EngineLayer, EngineSound};

/// How many cars of a mod's kind a cop wave with the car has.
pub const COP_COUNT: u32 = 2;
/// The spawn weight of a mod's cop car in its waves, in percent.
pub const COP_CHANCE: u32 = 10;

/// A mod's request for one car, with the facts the caller has checked against the install.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSpec {
    /// The car's folder under `CARS/`, in upper case.
    pub folder: String,
    /// The mod that asks for it.
    pub owner: String,
    /// The display name, if the mod gives one.
    pub name: Option<String>,
    /// The brand, if the mod gives one.
    pub manufacturer: Option<String>,
    /// Whether the car is listed among the player's cars.
    pub player: bool,
    /// The cop heat levels whose waves get the car, or `None` for a car that is not a cop. Levels are not checked yet.
    pub cop_heat: Option<Vec<u32>>,
    /// Whether the mod's own `files/` folder holds `CARS/<folder>/GEOMETRY.BIN`.
    pub geometry_in_mod: bool,
    /// What the car is: its mass, engine, gearbox, tyres and brakes, from which its physics are built.
    pub stats: CarStats,
    /// The car's own engine loops, if the mod gives them.
    pub engine_sound: Option<EngineSound>,
    /// The car's own recordings of its shifts, turbo, backfire and pops, if the mod gives them.
    pub car_effects: Option<CarEffects>,
}

/// A car the game admitted, with the car type it adds to the table.
#[derive(Debug, Clone, PartialEq)]
pub struct ModCar {
    pub folder: String,
    pub owner: String,
    pub name: Option<String>,
    /// The brand the mod gave. `None` shows no brand.
    pub manufacturer: Option<String>,
    pub player: bool,
    pub cop_heat: Option<Vec<u32>>,
    /// What the car is: its physics are built from these, not from any stock car's.
    pub stats: CarStats,
    /// The car's own engine loops, if the mod gives them.
    pub engine_sound: Option<EngineSound>,
    /// The car's own recordings of its shifts, turbo, backfire and pops, if the mod gives them.
    pub car_effects: Option<CarEffects>,
    /// The car's entry in the car table: its folder as the type name, its geometry and the brand the mod gave.
    pub car_type: CarTypeInfo,
}

/// Why a mod's car is not added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The mod's `files/` folder has no `CARS/<folder>/GEOMETRY.BIN`.
    NoGeometry,
    /// The folder is a car the game already has, or another mod's car.
    NameTaken,
    /// A cop heat level the game does not have.
    NoSuchHeat(u32),
}

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoGeometry => write!(f, "the mod's files have no CARS/<folder>/GEOMETRY.BIN for it"),
            Self::NameTaken => write!(f, "the folder is already a car"),
            Self::NoSuchHeat(level) => write!(f, "heat level {level} is not one of the game's {HEAT_LEVELS}"),
        }
    }
}

/// A request that was not admitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    pub folder: String,
    pub owner: String,
    pub reason: Reason,
}

/// Check the requests in order against `stock` and against each other, and return the admitted cars and the refused
/// requests. A refused car is not in either list.
pub fn register(stock: &[CarTypeInfo], specs: &[CarSpec]) -> (Vec<ModCar>, Vec<Rejected>) {
    let mut types = stock.to_vec();
    let mut cars: Vec<ModCar> = Vec::new();
    let mut rejected = Vec::new();
    for spec in specs {
        match admit(&types, &cars, spec) {
            Ok(car) => {
                types.push(car.car_type.clone());
                cars.push(car);
            }
            Err(reason) => {
                rejected.push(Rejected { folder: spec.folder.clone(), owner: spec.owner.clone(), reason });
            }
        }
    }
    (cars, rejected)
}

fn admit(types: &[CarTypeInfo], cars: &[ModCar], spec: &CarSpec) -> Result<ModCar, Reason> {
    if !spec.geometry_in_mod {
        return Err(Reason::NoGeometry);
    }
    let taken = find_type(types, &spec.folder).is_some() || cars.iter().any(|car| car.folder == spec.folder);
    if taken {
        return Err(Reason::NameTaken);
    }
    if let Some(&level) = spec.cop_heat.iter().flatten().find(|&&level| level == 0 || level > HEAT_LEVELS) {
        return Err(Reason::NoSuchHeat(level));
    }
    Ok(ModCar {
        folder: spec.folder.clone(),
        owner: spec.owner.clone(),
        name: spec.name.clone(),
        manufacturer: spec.manufacturer.clone(),
        player: spec.player,
        cop_heat: spec.cop_heat.clone(),
        stats: spec.stats.clone(),
        engine_sound: spec.engine_sound.clone(),
        car_effects: spec.car_effects.clone(),
        car_type: car_type_of(spec, types.len()),
    })
}

/// The car table entry of a mod car, made from the car's own request: its folder is its type name and the prefix of
/// its solids, and its type hash is the hash of that name. A car that only chases (a cop role without the player role)
/// is a cop type; any other is a racer.
fn car_type_of(spec: &CarSpec, index: usize) -> CarTypeInfo {
    let cop_only = spec.cop_heat.is_some() && !spec.player;
    CarTypeInfo {
        type_name: spec.folder.clone(),
        base_model_name: spec.folder.clone(),
        geometry_filename: format!("CARS\\{}\\GEOMETRY.BIN", spec.folder),
        manufacturer: spec.manufacturer.clone().unwrap_or_default(),
        type_name_hash: blackbox_hash::bstring_hash(&spec.folder),
        index: i32::try_from(index).unwrap_or(i32::MAX),
        usage_type: i32::from(cop_only),
        default_skin_number: 1,
        skinnable: true,
        default_base_paint: 0,
    }
}

/// The folder of a car type: the `CARS/<folder>/` part of its geometry path, in either separator style.
fn folder_of(info: &CarTypeInfo) -> Option<&str> {
    info.geometry_filename.split(['/', '\\']).nth(1)
}

/// The car type named `id`: the one whose folder is `id`, else the one whose type name is `id` (case-insensitive).
fn find_type<'a>(types: &'a [CarTypeInfo], id: &str) -> Option<&'a CarTypeInfo> {
    types
        .iter()
        .find(|info| folder_of(info).is_some_and(|folder| folder.eq_ignore_ascii_case(id)))
        .or_else(|| types.iter().find(|info| info.type_name.eq_ignore_ascii_case(id)))
}

/// The cars of this run: the ones the mods declared at start-up, then the ones that mods added while the game ran. Each
/// publication leaks the new list, so [`published`] can hand out a slice that outlives the lock; a run makes a handful.
static PUBLISHED: RwLock<&'static [ModCar]> = RwLock::new(&[]);

/// Add `cars` to the mod cars of this run. They are in every car table loaded from now on. A car whose folder is
/// already published is not added twice.
pub fn publish(cars: Vec<ModCar>) {
    let mut published = PUBLISHED.write().unwrap_or_else(PoisonError::into_inner);
    let fresh = cars.into_iter().filter(|car| !published.iter().any(|known| known.folder == car.folder));
    let all: Vec<ModCar> = published.iter().cloned().chain(fresh).collect();
    if all.len() == published.len() {
        return;
    }
    *published = Box::leak(all.into_boxed_slice());
}

/// The mod cars of this run so far: empty before [`publish`], or when no mod adds a car.
pub fn published() -> &'static [ModCar] {
    *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner)
}

/// The car types the mod cars add to the table, after the stock ones.
pub fn car_types() -> Vec<CarTypeInfo> {
    published().iter().map(|car| car.car_type.clone()).collect()
}

/// Whether the car in `folder` is listed among the player's cars: every stock car is, and a mod car is when it asks.
pub fn is_player_car(folder: &str) -> bool {
    player_in(published(), folder)
}

fn player_in(cars: &[ModCar], folder: &str) -> bool {
    cars.iter().find(|car| car.folder.eq_ignore_ascii_case(folder)).is_none_or(|car| car.player)
}

/// The display name a mod gave the car in `folder`, if any. The garage's own name table is used otherwise.
pub fn display_name(folder: &str) -> Option<&'static str> {
    display_name_in(published(), folder)
}

fn display_name_in<'a>(cars: &'a [ModCar], folder: &str) -> Option<&'a str> {
    cars.iter().find(|car| car.folder.eq_ignore_ascii_case(folder)).and_then(|car| car.name.as_deref())
}

/// The brand a mod gave the car in `folder`, if any.
pub fn manufacturer(folder: &str) -> Option<&'static str> {
    find_in(published(), folder).and_then(|car| car.manufacturer.as_deref())
}

/// The stats of the mod car in `folder`. `None` for a car that is not a mod car.
pub fn stats(folder: &str) -> Option<&'static CarStats> {
    find_in(published(), folder).map(|car| &car.stats)
}

/// The engine loops a mod gave the car in `folder`. `None` for a car that is not a mod car or has no loops of its own.
pub fn engine_sound(folder: &str) -> Option<&'static EngineSound> {
    find_in(published(), folder).and_then(|car| car.engine_sound.as_ref())
}

/// The recordings of the shifts, turbo, backfire and pops a mod gave the car in `folder`. `None` for a car that is not a
/// mod car or has none.
pub fn car_effects(folder: &str) -> Option<&'static CarEffects> {
    find_in(published(), folder).and_then(|car| car.car_effects.as_ref())
}

/// Whether `folder` is a car a mod added.
pub fn is_mod_car(folder: &str) -> bool {
    find_in(published(), folder).is_some()
}

fn find_in<'a>(cars: &'a [ModCar], folder: &str) -> Option<&'a ModCar> {
    cars.iter().find(|car| car.folder.eq_ignore_ascii_case(folder))
}

/// Add a cop row for each mod car whose heat list has `heat` to `row`, the wave of that heat level. The row names the
/// car by its folder in lower case, which is how the cop tables and the AI name cars.
pub fn add_cop_rows(cars: &[ModCar], heat: u32, row: &mut HeatRow) {
    for car in cars {
        let in_wave = car.cop_heat.as_deref().is_some_and(|levels| levels.contains(&heat));
        if !in_wave {
            continue;
        }
        row.cops.push(CopRecord { name: car.folder.to_ascii_lowercase(), count: COP_COUNT, chance: COP_CHANCE });
    }
}

#[cfg(test)]
mod tests;
