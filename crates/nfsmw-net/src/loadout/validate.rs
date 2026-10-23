//! The server's check of a loadout against its game data and the session's restriction.

use super::{CarRestriction, Loadout};
use crate::message::macros::wire_enum;

/// What the server knows about a car type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CarInfo {
    /// The car's class, as the session's class restriction names it.
    pub class: u8,
}

/// The game data a loadout is checked against. The game implements it over its car and part tables; tests implement it
/// over a small list.
pub trait Catalog {
    /// The car type's facts, or `None` for an unknown car.
    fn car(&self, car_type: u32) -> Option<CarInfo>;
    /// Whether `part` is a known part that fits `slot` of `car_type`.
    fn part_fits(&self, car_type: u32, slot: u16, part: u32) -> bool;
    /// Whether `paint` is a known paint id.
    fn paint_known(&self, paint: u32) -> bool;
    /// Whether `wheel_set` is a known wheel set (0 is the car's own).
    fn wheel_set_known(&self, wheel_set: u32) -> bool;
    /// The performance rating of the car with the loadout's parts, or `None` when it cannot be rated.
    fn performance(&self, loadout: &Loadout) -> Option<u16>;
}

wire_enum! {
    /// Why a loadout was refused. The code is the `detail` of a `LoadoutRejected` rejection. Append-only.
    pub enum LoadoutReject {
        UnknownCar = 1,
        UnknownPart = 2,
        /// Two parts name the same slot.
        DuplicateSlot = 3,
        UnknownPaint = 4,
        UnknownWheels = 5,
        /// The car is not in the session's allowed list.
        CarNotAllowed = 6,
        /// The car is not of the session's class.
        ClassNotAllowed = 7,
        /// The loadout's performance rating is above the session's cap, or cannot be rated.
        OverPerformanceCap = 8,
    }
}

impl LoadoutReject {
    /// The `detail` code of a `JoinRejected(LoadoutRejected)`.
    pub fn detail(self) -> u16 {
        u16::from(self.code())
    }
}

/// Checks `loadout` against `catalog` and `restriction`: the car first, then the restriction on the car, the parts,
/// the paint, the wheels and last the performance cap, which needs the parts.
pub fn validate(loadout: &Loadout, catalog: &impl Catalog, restriction: &CarRestriction) -> Result<(), LoadoutReject> {
    let car = catalog.car(loadout.car_type).ok_or(LoadoutReject::UnknownCar)?;
    if let CarRestriction::Cars(allowed) = restriction
        && !allowed.contains(&loadout.car_type)
    {
        return Err(LoadoutReject::CarNotAllowed);
    }
    if let CarRestriction::Class(class) = restriction
        && car.class != *class
    {
        return Err(LoadoutReject::ClassNotAllowed);
    }
    for (index, choice) in loadout.parts.iter().enumerate() {
        if loadout.parts.iter().take(index).any(|earlier| earlier.slot == choice.slot) {
            return Err(LoadoutReject::DuplicateSlot);
        }
        if !catalog.part_fits(loadout.car_type, choice.slot, choice.part) {
            return Err(LoadoutReject::UnknownPart);
        }
    }
    if !catalog.paint_known(loadout.paint.id) {
        return Err(LoadoutReject::UnknownPaint);
    }
    if !catalog.wheel_set_known(loadout.wheel_set) {
        return Err(LoadoutReject::UnknownWheels);
    }
    if let CarRestriction::PerformanceCap(cap) = restriction {
        let rating = catalog.performance(loadout).ok_or(LoadoutReject::OverPerformanceCap)?;
        if rating > *cap {
            return Err(LoadoutReject::OverPerformanceCap);
        }
    }
    Ok(())
}
