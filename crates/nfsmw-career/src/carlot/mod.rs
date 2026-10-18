//! The career car lot (`docs/specs/career-car-lot.md`): the catalogue entries and their unlock rules
//! ([`catalogue`]), the purchase checks ([`purchase`]) and the sale checks ([`sale`]).
//!
//! Pure logic over [`CareerSettings`] and [`Stable`]. Nothing reads `FE_ATTRIB.bin` here: the caller fills
//! [`LotCar`] entries from the front-end collections, so this crate does not depend on `nfsmw-data`.
//!
//! The garage limits are [`Limits`] values. The spec gives them as literals in the screen code (section 8 says
//! they can be configuration values); [`Limits::SPEC`] holds those literals.

pub mod catalogue;
pub mod purchase;
pub mod sale;
pub mod stats;
#[cfg(test)]
mod tests;

pub use catalogue::{
    LotCar, LotClass, LotState, bonus_flag, bonus_unlocked, by_frontend_key, listed, rival_car_available,
};
pub use purchase::{Delivery, Purchase, PurchaseError, PurchasePrompt, buy, prompt_for};
pub use sale::{Sale, SaleError, replacement_current, sell};

use crate::stable::{CAREER_SLOTS, CarRecord, Stable};

/// Low-half flag of a My Cars custom, a quick-race car (spec section 7, `0x4`).
pub const FLAG_CUSTOM: u32 = 0x4;
/// Low-half mark that leaves a car out of the owned count (spec section 3.2, `0x40`). **[unconfirmed]** what the
/// mark means; the spec only says the owned count skips it.
pub const FLAG_MARKED: u32 = 0x40;

/// The garage and record limits (spec sections 3.2 and 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Owned career cars: a purchase is refused at this count (spec 3.2, literal 10).
    pub owned_cars: usize,
    /// Career record slots: a purchase is refused when no slot is free (spec 3.2, the stable refuses at 25).
    pub career_records: usize,
    /// Quick-race customs kept in My Cars (spec 7, literal 20).
    pub customs: usize,
}

impl Limits {
    /// The values the original uses.
    pub const SPEC: Limits = Limits { owned_cars: 10, career_records: CAREER_SLOTS, customs: 20 };
}

/// The number of owned career cars: records with the owned flag, a class bit, not free and not marked `0x40`
/// (spec section 3.2). Used by the purchase check.
pub fn owned_count(stable: &Stable) -> usize {
    stable.cars.iter().filter(|car| is_owned_career_car(car)).count()
}

/// True for a car record that counts toward [`owned_count`].
fn is_owned_career_car(car: &CarRecord) -> bool {
    !car.is_free() && car.is_owned() && car.has_class_mask() && car.flags & FLAG_MARKED == 0
}

/// The number of quick-race customs: records with the custom flag `0x4` (spec section 7).
pub fn custom_count(stable: &Stable) -> usize {
    stable.cars.iter().filter(|car| !car.is_free() && car.flags & FLAG_CUSTOM != 0).count()
}
