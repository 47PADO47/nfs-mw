//! The prices of a vehicle that has components (`garage/options`): a component's own price when its data gives one, else what the
//! shop already charged for that place (the first tier of the model category's table, the stage of the level's table).
//!
//! Which choices are *unlocked* stays the career's: a model category and a level keep their entity and tier, so a bike's rims are
//! open when the career opens the rims. A component slot only the components know (`Row::Slot`) has no table of its own and is
//! priced and gated as a first-tier body part (`SESSION_ENTITY`). The rider's gear has no tier at all (`rider_gear::price`).

use anyhow::Result;
use nfsmw_career::economy::shop::Item;
use nfsmw_data::car::customization::Category;

use super::{Car, Pricing, Row, Selection};
use crate::garage::menu_slots::slot_of;
use crate::garage::options::{Slot, Store, Value, VehicleOptions};
use crate::garage::rider_gear;

/// The shop category a component slot without a model category is priced and unlocked as: the body kits'.
fn session_entity() -> u32 {
    nfsmw_data::car::shop::visual_entity(Category::Body)
}

/// The slot `row` is the shop's view of, in the options of `selection`.
fn slot_in(options: &Option<VehicleOptions>, row: Row) -> Option<&Slot> {
    slot_of(options.as_ref()?, row)
}

impl Pricing {
    /// `item`, with the price the vehicle's components give ([`adjust`]).
    pub(super) fn by_components(
        &self,
        row: Row,
        item: Item,
        selected: &Selection,
        installed: &Selection,
        car: &Car,
        backroom: bool,
    ) -> Item {
        adjust(row, item, selected, installed, car, backroom)
    }

    /// The item of a component slot only the components know ([`session`]); a component with no price of its own costs a
    /// first-tier body part.
    pub(super) fn slot(&self, row: Row, selected: &Selection, installed: &Selection, car: &Car) -> Result<Item> {
        session(row, selected, installed, car, || self.prices.cost(session_entity(), 1))
    }

    /// The item of one part of the rider's gear: the part's default price unless the profile owns the item.
    pub(super) fn gear(part: rider_gear::Part, selected: &Selection) -> Item {
        gear(part, selected, rider_gear::owns)
    }
}

/// `item`, with the price the vehicle's components give: the installed component's for the cost, a quarter of the replaced
/// component's for the trade-in of a model part. A vehicle without components, a choice with no price in the data, and the
/// backroom (which pays with markers) keep the shop's own numbers.
pub(crate) fn adjust(
    row: Row,
    mut item: Item,
    selected: &Selection,
    installed: &Selection,
    car: &Car,
    backroom: bool,
) -> Item {
    if backroom {
        return item;
    }
    let (now, before) = (VehicleOptions::of_selection(car, selected), VehicleOptions::of_selection(car, installed));
    let Some(slot) = slot_in(&now, row) else { return item };
    if let Some(price) = slot.installed().price.filter(|_| slot.changed()) {
        item.cost = price;
    }
    let models = matches!(row, Row::Part(category) if Category::MODELS.contains(&category));
    if let (true, Some(old)) = (models, slot_in(&before, row))
        && let Some(price) = old.installed().price.filter(|_| old.changed())
    {
        item.trade_in = price / 4;
    }
    item
}

/// The item of a component slot only the components know: the price of the choice in the cart (`fallback` when its data gives
/// none), gated as a body part. Going back to stock, or emptying the slot, costs nothing.
pub(crate) fn session(
    row: Row,
    selected: &Selection,
    installed: &Selection,
    car: &Car,
    fallback: impl FnOnce() -> Result<u32>,
) -> Result<Item> {
    let now = VehicleOptions::of_selection(car, selected);
    let before = VehicleOptions::of_selection(car, installed);
    let free = Item { entity: session_entity(), level: 0, cost: 0, trade_in: 0, marker: None };
    let (Some(slot), old) = (slot_in(&now, row), slot_in(&before, row)) else { return Ok(free) };
    if !matches!(slot.installed().value, Value::Component(_))
        || matches!(slot.store, Store::Category(_) | Store::Level(_))
    {
        return Ok(free);
    }
    let cost = match slot.installed().price {
        Some(price) => price,
        None => fallback()?,
    };
    let trade_in = old.filter(|old| old.changed()).and_then(|old| old.installed().price).unwrap_or(0) / 4;
    Ok(Item { entity: session_entity(), level: 1, cost, trade_in, marker: None })
}

/// The item of one part of the rider's gear: the part's default price unless `owned` says the profile has the item. No tier:
/// gear is open from the first visit.
pub(crate) fn gear(
    part: rider_gear::Part,
    selected: &Selection,
    owned: impl Fn(rider_gear::Part, &str) -> bool,
) -> Item {
    let cost = rider_gear::price::cost(part, selected.gear.get(part), owned);
    Item { entity: 0, level: 0, cost, trade_in: 0, marker: None }
}
