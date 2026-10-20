//! The career car lot's cars: the `frontend` collections that carry a price, from `GLOBAL/FE_ATTRIB.bin`.
//! Spec: `docs/specs/career-car-lot.md` §1, §2 and §8. Only the reader is here; the purchase and sale rules
//! (§3, §4) are not.

use anyhow::{Context, Result};
use blackbox_attrib::{CollectionRef, Database, Value};
use game_install::GameDir;

use crate::read_unwrapped;
use crate::shared;

/// The file with the `frontend` collections of the car lot. Its class definitions come from `GLOBAL/ATTRIBUTES.BIN`.
pub const FE_ATTRIB_FILE: &str = "GLOBAL/FE_ATTRIB.bin";
/// The class holding every collection read here.
const CLASS: &str = "frontend";

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
}

/// Every `frontend` collection that has a price, in database order. Cars that are not for sale are included, with
/// `for_sale` false (the spec's §2 table is the subset with `for_sale` set).
pub fn catalogue(db: &Database) -> Vec<CatalogueCar> {
    db.collections_of(CLASS).filter_map(car).collect()
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
}
