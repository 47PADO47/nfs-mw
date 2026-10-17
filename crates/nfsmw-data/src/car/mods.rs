//! Cars that mods add (ADR 0005 section 5; docs/modding/api-content.md "Cars").
//!
//! A mod car is a clone of a stock car type: it keeps the stock record's name, so its parts, physics and sound are the
//! clone's, and it ships its own `CARS/<FOLDER>/` geometry and textures. [`register`] is the pure part: it takes the
//! stock car table and the mods' requests, and returns the cars it admits and the reasons it refuses the others. The
//! game seals the admitted cars once, at start-up ([`seal`]); after that every car-table load sees them
//! ([`car_types`]), and the physics, cop and garage lookups read them through the functions below.

use std::sync::OnceLock;

use blackbox_carparts::CarTypeInfo;

use crate::pursuit::{CopRecord, HEAT_LEVELS, HeatRow};

/// How many cars of a mod's kind a cop wave with the car has.
pub const COP_COUNT: u32 = 2;
/// The spawn weight of a mod's cop car in its waves, in percent.
pub const COP_CHANCE: u32 = 10;

/// A mod's request for one car, with the facts the caller has checked against the install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarSpec {
    /// The car's folder under `CARS/`, in upper case.
    pub folder: String,
    /// The mod that asks for it.
    pub owner: String,
    /// The stock car it copies: a car folder or a car type name.
    pub clone: String,
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
}

/// A car the game admitted, with the car type it adds to the table.
#[derive(Debug, Clone, PartialEq)]
pub struct ModCar {
    pub folder: String,
    pub owner: String,
    pub name: Option<String>,
    pub player: bool,
    pub cop_heat: Option<Vec<u32>>,
    /// A copy of the clone's record, with the mod's folder as its geometry and the brand the mod gave.
    pub car_type: CarTypeInfo,
}

/// Why a mod's car is not added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// `clone` names no stock car (or a car another mod added).
    UnknownClone(String),
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
            Self::UnknownClone(clone) => write!(f, "clone {clone:?} is not a car of the game"),
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
/// requests. A car admitted here can be the clone of a later request. A refused car is not in either list.
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
    let Some(source) = find_type(types, &spec.clone) else {
        return Err(Reason::UnknownClone(spec.clone.clone()));
    };
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
    let mut car_type = source.clone();
    car_type.geometry_filename = format!("CARS\\{}\\GEOMETRY.BIN", spec.folder);
    if let Some(manufacturer) = &spec.manufacturer {
        car_type.manufacturer = manufacturer.clone();
    }
    Ok(ModCar {
        folder: spec.folder.clone(),
        owner: spec.owner.clone(),
        name: spec.name.clone(),
        player: spec.player,
        cop_heat: spec.cop_heat.clone(),
        car_type,
    })
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

/// The cars of this run, set once at start-up. Later calls return the cars they were given.
static SEALED: OnceLock<Vec<ModCar>> = OnceLock::new();

/// Make `cars` the mod cars of this run. The first call wins; a second returns its cars back, unchanged.
pub fn seal(cars: Vec<ModCar>) -> Result<(), Vec<ModCar>> {
    SEALED.set(cars)
}

/// The mod cars of this run: empty before [`seal`], or when no mod adds a car.
pub fn sealed() -> &'static [ModCar] {
    SEALED.get().map_or(&[], Vec::as_slice)
}

/// The car types the mod cars add to the table, after the stock ones.
pub fn car_types() -> Vec<CarTypeInfo> {
    sealed().iter().map(|car| car.car_type.clone()).collect()
}

/// The name a physics or AI lookup should use for `name`: a mod car's folder names its clone's type, which owns the
/// physics record. Any other name is returned as it is.
pub fn physics_name(name: &str) -> &str {
    physics_name_in(sealed(), name)
}

fn physics_name_in<'a>(cars: &'a [ModCar], name: &'a str) -> &'a str {
    cars.iter().find(|car| car.folder.eq_ignore_ascii_case(name)).map_or(name, |car| car.car_type.type_name.as_str())
}

/// Whether the car in `folder` is listed among the player's cars: every stock car is, and a mod car is when it asks.
pub fn is_player_car(folder: &str) -> bool {
    player_in(sealed(), folder)
}

fn player_in(cars: &[ModCar], folder: &str) -> bool {
    cars.iter().find(|car| car.folder.eq_ignore_ascii_case(folder)).is_none_or(|car| car.player)
}

/// The display name a mod gave the car in `folder`, if any. The garage's own name table is used otherwise.
pub fn display_name(folder: &str) -> Option<&'static str> {
    display_name_in(sealed(), folder)
}

fn display_name_in<'a>(cars: &'a [ModCar], folder: &str) -> Option<&'a str> {
    cars.iter().find(|car| car.folder.eq_ignore_ascii_case(folder)).and_then(|car| car.name.as_deref())
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
