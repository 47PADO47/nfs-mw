//! The sale rules of the career car lot (`docs/specs/career-car-lot.md` section 4).
//!
//! A sale runs only from the Safe House Car Select at stage 15 or lower. The car needs a career record, must be
//! free to drive, and at least one other free car must remain. The price is half the car's price field, with the
//! integer shift of [`sale_proceeds`]. The sold numbers move into the history first ([`SoldHistory::absorb`]).
//!
//! The spec's refusals are checked in its order; the checks that come before it (the place and the stage) and the
//! price lookup are added here.

use super::catalogue::{LotCar, by_frontend_key};
use crate::economy::sale_proceeds;
use crate::settings::CareerSettings;
use crate::stable::{CarRecord, CareerRecord, FREE_HANDLE, Stable};

/// The last stage at which a car can be sold (spec 4: "stage is 15 or lower").
pub const MAX_SALE_STAGE: u8 = 15;

/// Why a sale was refused. The sale changes nothing when it returns one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaleError {
    /// Not from the Safe House Car Select, or the stage is past 15 (spec 4).
    NotAllowed,
    /// The car has no career record: nothing happens (spec 4.1).
    NoCareerRecord,
    /// The car is impounded: "This car is impounded, you cannot sell it." (spec 4.2).
    Impounded,
    /// One or fewer free cars would be left: "You cannot sell the only car left in your safe house." (spec 4.3).
    /// The count is of live career records that are free to drive, the car being sold included.
    LastCar,
    /// No catalogue entry has the car's front-end key, so its price field is unknown. Not in the spec.
    NoPrice,
}

/// What a sale did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sale {
    /// The handle of the sold car.
    pub handle: u32,
    /// The cash added: half the price field.
    pub proceeds: u32,
    /// The new current car, when the sold car was the current one. `None` when the current car did not change.
    /// **[unconfirmed]** see [`replacement_current`].
    pub new_current: Option<u32>,
}

/// Sells the car with `handle` (spec section 4).
///
/// `safe_house` is true when the sale comes from the Safe House Car Select (spec 4). On success the car record and
/// its career record are cleared, the career numbers go to the sold history, the proceeds go to the cash, and the
/// current car moves to [`replacement_current`] when the sold car was the current one.
///
/// **[unconfirmed]** clearing the career record. The spec says the car leaves the stable but
/// not whether its career record is freed in the same step; the record is cleared here so no record is left
/// pointing at nothing.
pub fn sell(
    settings: &mut CareerSettings,
    stable: &mut Stable,
    catalogue: &[LotCar],
    handle: u32,
    safe_house: bool,
) -> Result<Sale, SaleError> {
    if !safe_house || settings.stage > MAX_SALE_STAGE {
        return Err(SaleError::NotAllowed);
    }
    let index = stable.career_index_for_handle(handle).ok_or(SaleError::NoCareerRecord)?;
    let career: CareerRecord = *stable.careers.get(index).ok_or(SaleError::NoCareerRecord)?;
    if !career.is_free_to_drive() {
        return Err(SaleError::Impounded);
    }
    if free_car_count(stable) <= 1 {
        return Err(SaleError::LastCar);
    }
    let car: CarRecord = *stable.car_by_handle(handle).ok_or(SaleError::NoCareerRecord)?;
    let price = by_frontend_key(catalogue, car.frontend_key).ok_or(SaleError::NoPrice)?.price;

    let proceeds = sale_proceeds(price);
    let was_current = settings.car_handle == handle;
    let next = replacement_current(stable, handle);

    stable.sold.absorb(&career);
    if let Some(slot) = stable.cars.iter_mut().find(|slot| slot.handle == handle) {
        *slot = CarRecord::free();
    }
    if let Some(record) = stable.careers.get_mut(index) {
        *record = CareerRecord::unused();
    }
    settings.add_cash(proceeds);

    let new_current = if was_current { Some(next.unwrap_or(FREE_HANDLE)) } else { None };
    if let Some(current) = new_current {
        settings.car_handle = current;
    }
    Ok(Sale { handle, proceeds, new_current })
}

/// The car that becomes current when the current car is sold: the first car in slot order that is not the sold one,
/// is live (see [`crate::stable::Stable::live_career_indices`]) and is free to drive. `None` when there is none.
///
/// **[unconfirmed]** The spec says only that the screen re-reads the current car after the sale (section 4). The
/// slot-order choice is this crate's guess; the original may pick another car or leave the handle as it was.
pub fn replacement_current(stable: &Stable, sold: u32) -> Option<u32> {
    stable
        .cars
        .iter()
        .filter(|car| car.handle != sold && car.counts())
        .find(|car| stable.career_for_handle(car.handle).is_some_and(CareerRecord::is_free_to_drive))
        .map(|car| car.handle)
}

/// The live career records that are free to drive (spec 4.3).
fn free_car_count(stable: &Stable) -> usize {
    stable.live_records().filter(|record| record.is_free_to_drive()).count()
}
