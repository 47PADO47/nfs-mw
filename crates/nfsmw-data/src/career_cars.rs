//! The career car lot's cars: the `frontend` collections that carry a price, from `GLOBAL/FE_ATTRIB.bin`.
//! Spec: `docs/specs/career-car-lot.md` §1, §2 and §8. Only the reader is here; the purchase and sale rules
//! (§3, §4) are not.

use std::collections::HashMap;

use anyhow::{Context, Result};
use blackbox_attrib::{CollectionRef, Database, Value};
use game_install::GameDir;

use crate::read_unwrapped;
use crate::shared;

/// The file with the `frontend` collections of the car lot. Its class definitions come from `GLOBAL/ATTRIBUTES.BIN`.
pub const FE_ATTRIB_FILE: &str = "GLOBAL/FE_ATTRIB.bin";
/// The class holding every collection read here.
const CLASS: &str = "frontend";
/// The vehicle class, whose `frontend` field names a car's front-end collection.
const VEHICLE_CLASS: &str = "pvehicle";
/// The `frontend` field of a `pvehicle` collection (a reference into the `frontend` class; the field is named after it).
const FRONTEND_FIELD: u32 = 0x8588_5722;

/// Field keys of the `frontend` class (name hashes, `career-car-lot.md` §1).
const PRICE_KEY: u32 = 0xE616_73E9;
const UNLOCK_RANK_KEY: u32 = 0x60F9_9F57;
const FOR_SALE_KEY: u32 = 0xFBCF_1A48;
const CAR_CLASS_KEY: u32 = 0x0DE0_09BE;
/// The manufacturer index (layout offset 0x40, `docs/specs/quick-play.md` section 4): 1 BMW up to 25 Corvette.
const MANUFACTURER_KEY: u32 = 0xBAF4_AB4C;

/// The car classes of the car lot (layout offset 0x38).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarClass {
    NorthAmerica,
    Europe,
    Japan,
    /// The root "vehicles" class: traffic, cops and specials. Not sold in the lot.
    Root,
}

impl CarClass {
    /// The stored code: 0 North America, 1 Europe, 2 Japan, 3 the root class.
    pub fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::NorthAmerica),
            1 => Some(Self::Europe),
            2 => Some(Self::Japan),
            3 => Some(Self::Root),
            _ => None,
        }
    }
}

/// One car of the car lot, as the attribute database stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueCar {
    /// The collection key (`vlt_hash` of the car's name).
    pub key: u32,
    /// The collection's name, when the database knows it.
    pub name: Option<String>,
    /// Price in dollars.
    pub price: i32,
    /// Whether the car is sold in the lot.
    pub for_sale: bool,
    /// The car is available while the player's stage is at most this value. `None` when the field is unset or out of range.
    pub unlock_rank: Option<u8>,
    /// `None` when the field is unset or the code is unknown.
    pub class: Option<CarClass>,
    /// The manufacturer index (`MANUFACTURER_KEY`, layout offset `0x40`): 1 BMW, 2 Ford, 3 Subaru ... 25 Corvette
    /// (`docs/specs/quick-play.md` section 4). `None` when the field is unset or past a byte.
    pub manufacturer: Option<u8>,
    /// The `pvehicle` collection whose `frontend` field names this car (`career-car-lot.md` section 1.1): the car's
    /// vehicle, whose name the `CARS/` folder and the car type carry. The names differ for five cars (`imprezzawrx` is
    /// the vehicle `imprezawrx`, `a3_20t` is `a3`). `None` when no vehicle names the car.
    pub vehicle: Option<Vehicle>,
}

/// The vehicle collection of a catalogue car.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vehicle {
    /// The `pvehicle` collection's key.
    pub key: u32,
    /// The `pvehicle` collection's name, e.g. `imprezawrx`.
    pub name: String,
}

impl CatalogueCar {
    /// The name the car's vehicle carries (its `CARS/` folder and car type, in lower case): the vehicle's name, else the
    /// front-end collection's.
    pub fn vehicle_name(&self) -> Option<&str> {
        self.vehicle.as_ref().map(|vehicle| vehicle.name.as_str()).or(self.name.as_deref())
    }
}

/// Every `frontend` collection that has a price, in database order. Cars that are not for sale are included, with
/// `for_sale` false (the spec's §2 table is the subset with `for_sale` set).
pub fn catalogue(db: &Database) -> Vec<CatalogueCar> {
    let vehicles = vehicles(db);
    db.collections_of(CLASS)
        .filter_map(car)
        .map(|mut car| {
            car.vehicle = vehicles.get(&car.key).cloned();
            car
        })
        .collect()
}

/// The vehicle of each front-end collection: the first `pvehicle` collection, in database order, whose own `frontend`
/// field points at it. A field a vehicle only inherits (the challenge copies of a car) does not count.
fn vehicles(db: &Database) -> HashMap<u32, Vehicle> {
    let mut found = HashMap::new();
    for vehicle in db.collections_of(VEHICLE_CLASS) {
        let Some(target) = vehicle.data().own(FRONTEND_FIELD).and_then(Value::as_ref_spec) else { continue };
        let Some(name) = vehicle.name() else { continue };
        found.entry(target.collection).or_insert_with(|| Vehicle { key: vehicle.key(), name: name.to_owned() });
    }
    found
}

/// The catalogue in the order of a new career's stable (`career-car-lot.md` section 1.2): the default stable gets one car
/// per vehicle as the `pvehicle` class is walked (`Database::collections_in_table_order`), and the lot lists the stable in
/// slot order before it sorts. A car is placed by its vehicle's position in `vehicle_order`; cars with no vehicle there
/// follow, in catalogue order.
pub fn in_stable_order(mut cars: Vec<CatalogueCar>, vehicle_order: &[u32]) -> Vec<CatalogueCar> {
    let position = |car: &CatalogueCar| {
        let key = car.vehicle.as_ref()?.key;
        vehicle_order.iter().position(|vehicle| *vehicle == key)
    };
    cars.sort_by_key(|car| position(car).unwrap_or(usize::MAX));
    cars
}

/// The keys of the `pvehicle` collections in the order a walk of the class gives, the order a new career's stable takes
/// its cars in.
pub fn vehicle_walk_order(db: &Database) -> Vec<u32> {
    db.collections_in_table_order(blackbox_attrib::vlt_hash(VEHICLE_CLASS))
        .iter()
        .map(|vehicle| vehicle.key())
        .collect()
}

/// Read the car lot's cars from the install: class definitions from `GLOBAL/ATTRIBUTES.BIN`, collections from
/// `GLOBAL/FE_ATTRIB.bin`. This is the installed file, which a mod tool may have changed (`career-car-lot.md` 2.2).
pub fn load(dir: &GameDir) -> Result<Vec<CatalogueCar>> {
    // The session's class definitions are shared; the collections go into this call's own copy.
    let attributes = shared::attributes(dir)?;
    let mut db = Database::clone(&attributes);
    let fe_attrib = read_unwrapped(dir, FE_ATTRIB_FILE)?;
    db.load(&fe_attrib).with_context(|| format!("parsing {FE_ATTRIB_FILE}"))?;
    Ok(catalogue(&db))
}

/// Whether a car with `unlock_rank` is available at `stage`: the rank is at least the stage (§1).
pub fn available_at(unlock_rank: u8, stage: u8) -> bool {
    unlock_rank >= stage
}

/// One car, or `None` when the collection has no price (it is not a car of the lot).
fn car(c: CollectionRef<'_>) -> Option<CatalogueCar> {
    let price = c.get_by_key(PRICE_KEY)?.as_i32()?;
    Some(CatalogueCar {
        key: c.key(),
        name: c.name().map(str::to_owned),
        price,
        for_sale: c.get_by_key(FOR_SALE_KEY).and_then(Value::as_bool).unwrap_or(false),
        unlock_rank: c.get_by_key(UNLOCK_RANK_KEY).and_then(Value::as_i64).and_then(|r| u8::try_from(r).ok()),
        class: c.get_by_key(CAR_CLASS_KEY).and_then(Value::as_u32).and_then(CarClass::from_code),
        manufacturer: c.get_by_key(MANUFACTURER_KEY).and_then(Value::as_u32).and_then(|m| u8::try_from(m).ok()),
        vehicle: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_codes_follow_the_spec() {
        assert_eq!(CarClass::from_code(0), Some(CarClass::NorthAmerica));
        assert_eq!(CarClass::from_code(1), Some(CarClass::Europe));
        assert_eq!(CarClass::from_code(2), Some(CarClass::Japan));
        assert_eq!(CarClass::from_code(3), Some(CarClass::Root));
    }

    #[test]
    fn an_unknown_class_code_is_none() {
        assert_eq!(CarClass::from_code(4), None);
        assert_eq!(CarClass::from_code(u32::MAX), None);
    }

    #[test]
    fn a_car_is_available_when_its_rank_is_at_least_the_stage() {
        // The prologue is stage 16: a rank 16 car is there from the start.
        assert!(available_at(16, 16));
        assert!(!available_at(14, 16));
        // After rival 15 is beaten the stage is 14, so a rank 14 car unlocks.
        assert!(available_at(14, 14));
        // Rank 2 is still available at stage 1.
        assert!(available_at(2, 1));
        assert!(!available_at(1, 2));
    }

    fn named(name: &str, vehicle: Option<&str>) -> CatalogueCar {
        CatalogueCar {
            key: 0,
            name: Some(name.to_owned()),
            price: 1,
            for_sale: true,
            unlock_rank: Some(16),
            class: Some(CarClass::Europe),
            manufacturer: None,
            vehicle: vehicle.map(|name| Vehicle { key: 0, name: name.to_owned() }),
        }
    }

    #[test]
    fn the_stable_order_follows_the_vehicle_walk_and_puts_unlinked_cars_last() {
        let linked = |name: &str, key: u32| {
            let mut car = named(name, Some(name));
            car.vehicle.as_mut().unwrap().key = key;
            car
        };
        let cars = vec![named("mod_car", None), linked("punto", 3), linked("imprezzawrx", 1), linked("is300", 2)];
        let ordered = in_stable_order(cars, &[1, 2, 3]);
        let names: Vec<_> = ordered.iter().map(|car| car.name.as_deref().unwrap()).collect();
        assert_eq!(names, ["imprezzawrx", "is300", "punto", "mod_car"]);
    }

    #[test]
    fn a_car_without_a_vehicle_link_uses_its_own_name() {
        assert_eq!(named("is300", None).vehicle_name(), Some("is300"));
        assert_eq!(named("a3_20t", Some("a3")).vehicle_name(), Some("a3"));
    }
}
