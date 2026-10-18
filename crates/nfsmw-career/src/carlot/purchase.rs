//! The purchase rules of the career car lot (`docs/specs/career-car-lot.md` section 3).
//!
//! The checks run in the spec's order: the car must be buyable at the stage (section 1), the price must not be above
//! the cash (3.1), the owned cars must be under the limit (3.2). Two checks the spec does not list are added after
//! those, both for a full table: a free career record (the stable has 25) and a free car slot (200).

use super::catalogue::{LotCar, LotState};
use super::{Limits, owned_count};
use crate::settings::{CareerSettings, PROLOGUE_STAGE};
use crate::stable::{CarRecord, CareerRecord, Stable};

/// The dialog before a purchase (spec 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchasePrompt {
    /// Stage 16, the prologue: a plain "Are you sure you wish to buy this car?".
    Confirm,
    /// Stage 15 or lower: Drive Now, Deliver or Cancel.
    DriveOrDeliver,
}

/// The prompt shown at `stage` (spec 3.3).
pub const fn prompt_for(stage: u8) -> PurchasePrompt {
    if stage >= PROLOGUE_STAGE {
        return PurchasePrompt::Confirm;
    }
    PurchasePrompt::DriveOrDeliver
}

/// How a purchase arrives (spec 3.4 and 3.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Drive Now: the new car becomes the current car (spec 3.5).
    DriveNow,
    /// Deliver: the car goes to the Safe House and the current car does not change (spec 3.4).
    ///
    /// The prologue's plain confirm (stage 16) names no delivery. **[unconfirmed]** the caller passes `Deliver`
    /// there, the only choice that leaves the current car alone.
    Deliver,
}

/// Why a purchase was refused. The purchase changes nothing when it returns one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseError {
    /// Not offered in the lot (class 3, or the for-sale flag false; spec 1). **[unconfirmed]** the spec does not
    /// state this as a refusal; the lot shows the car as not for sale.
    NotOffered,
    /// Locked at this stage (spec 1). **[unconfirmed]** the spec shows the car greyed but does not name the refusal.
    Locked,
    /// The price is above the cash (spec 3.1): "Not enough cash to buy this car!".
    NotEnoughCash,
    /// The owned career cars are at the limit (spec 3.2): "There's no more room in your Safe House."
    GarageFull,
    /// No free career record, or the limit on career records is reached (spec 3.2, the stable refuses at 25).
    NoCareerSlot,
    /// No free car slot in the stable (200 slots). Not in the spec.
    NoCarSlot,
}

/// What a purchase created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Purchase {
    /// The handle of the new car.
    pub handle: u32,
    /// The car slot the record went to.
    pub car_slot: usize,
    /// The career record index the car points at.
    pub career_index: u8,
    /// The price taken from the cash.
    pub price: u32,
}

/// Buys `car` into the stable and charges the cash (spec section 3).
///
/// The new record copies the entry's keys and class bit, gets the owned flag, a fresh career record
/// ([`CareerRecord::new`]: strike limit 3, everything else 0, spec 3 and career-save 4.3) and the given customization
/// index. With [`Delivery::DriveNow`] the car becomes the settings' current car.
///
/// **[unconfirmed]** the flag word: the record gets the class mask and the owned flag as the stable writes owned
/// cars (`CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED`), not only the entry's class bit the spec names.
/// **[unconfirmed]** the customization index: the spec says a free customization record is taken; the customization
/// slots are raw bytes in this crate, so the caller chooses the index. The preset default parts are not built here.
pub fn buy(
    settings: &mut CareerSettings,
    stable: &mut Stable,
    car: &LotCar,
    delivery: Delivery,
    customization_index: u8,
    limits: &Limits,
) -> Result<Purchase, PurchaseError> {
    check_offer(car, settings.stage)?;
    if car.price > settings.cash {
        return Err(PurchaseError::NotEnoughCash);
    }
    if owned_count(stable) >= limits.owned_cars {
        return Err(PurchaseError::GarageFull);
    }
    if used_career_records(stable) >= limits.career_records {
        return Err(PurchaseError::NoCareerSlot);
    }
    let career_index = stable.careers.iter().position(CareerRecord::is_unused).ok_or(PurchaseError::NoCareerSlot)?;
    let career_index = u8::try_from(career_index).map_err(|_| PurchaseError::NoCareerSlot)?;
    let car_slot = stable.cars.iter().position(CarRecord::is_free).ok_or(PurchaseError::NoCarSlot)?;

    let record = CarRecord {
        handle: car.handle,
        frontend_key: car.frontend_key,
        vehicle_key: car.vehicle_key,
        flags: CarRecord::CLASS_MASK | CarRecord::FLAG_OWNED,
        customization_index,
        career_index,
        tail: [0; 2],
    };
    let slot = stable.cars.get_mut(car_slot).ok_or(PurchaseError::NoCarSlot)?;
    *slot = record;
    let career = stable.careers.get_mut(usize::from(career_index)).ok_or(PurchaseError::NoCareerSlot)?;
    *career = CareerRecord::new(career_index);

    // The cash check above covers the price, so the spend cannot clamp.
    let paid = settings.spend_cash(car.price);
    debug_assert!(paid, "the cash check covers the price");
    if delivery == Delivery::DriveNow {
        settings.car_handle = car.handle;
    }
    Ok(Purchase { handle: car.handle, car_slot, career_index, price: car.price })
}

/// The lot state of the car must be buyable at this stage (spec 1).
fn check_offer(car: &LotCar, stage: u8) -> Result<(), PurchaseError> {
    match car.state_at(stage) {
        LotState::Buyable => Ok(()),
        LotState::Locked => Err(PurchaseError::Locked),
        LotState::NotOffered => Err(PurchaseError::NotOffered),
    }
}

/// The career records in use.
fn used_career_records(stable: &Stable) -> usize {
    stable.careers.iter().filter(|record| !record.is_unused()).count()
}
