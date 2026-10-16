//! `Career`: cash, bounty, the rap sheet, the blacklist, milestones, the garage, safe houses and shops (catalogue:
//! docs/modding/api-nfsmw.md, "Career").
//!
//! The host has no career state yet, so every call returns `Error::Unsupported` for now. The signatures are the ones the
//! career will keep; the field lists are [unconfirmed]. Calls marked write in the catalogue change state.

use blackbox_modding::sdk::ModResult;

use super::career_types::{Milestone, MilestoneDef, OwnedCar, RapSheet, Rival, Safehouse, Shop, ShopItem, ShopItemDef};
use super::ffi::career::{self as host, CareerFns};

/// The `nfsmw.career` interface of the host, as a mod uses it.
#[derive(Clone, Copy)]
pub struct Career {
    fns: CareerFns,
}

impl Career {
    pub(crate) fn new(fns: CareerFns) -> Self {
        Career { fns }
    }

    /// The cash.
    pub fn cash(&self) -> ModResult<i64> {
        host::cash_get(&self.fns)
    }

    /// Add cash. `reason` is a short label the host may log. (write)
    pub fn add_cash(&self, delta: i64, reason: &str) -> ModResult<()> {
        host::cash_add(&self.fns, delta, reason)
    }

    /// Set the cash. (write)
    pub fn set_cash(&self, value: i64) -> ModResult<()> {
        host::cash_set(&self.fns, value)
    }

    /// The bounty.
    pub fn bounty(&self) -> ModResult<u64> {
        host::bounty_get(&self.fns)
    }

    /// Add to the bounty. (write)
    pub fn add_bounty(&self, delta: u64) -> ModResult<()> {
        host::bounty_add(&self.fns, delta)
    }

    /// Set the bounty. (write)
    pub fn set_bounty(&self, value: u64) -> ModResult<()> {
        host::bounty_set(&self.fns, value)
    }

    /// The rap sheet.
    pub fn rap_sheet(&self) -> ModResult<RapSheet> {
        host::rap_sheet_get(&self.fns).map(|raw| RapSheet::from_abi(&raw))
    }

    /// Set one rap sheet stat by its number. (write)
    pub fn set_stat(&self, stat: u32, value: u64) -> ModResult<()> {
        host::rap_sheet_set_stat(&self.fns, stat, value)
    }

    /// Add to one rap sheet stat by its number. (write)
    pub fn add_stat(&self, stat: u32, delta: u64) -> ModResult<()> {
        host::rap_sheet_add_stat(&self.fns, stat, delta)
    }

    /// The blacklist, in rank order.
    pub fn blacklist(&self) -> ModResult<Vec<Rival>> {
        let count = host::rival_count(&self.fns)?;
        (0..count).map(|index| Rival::from_abi(&host::rival_at(&self.fns, index)?)).collect()
    }

    /// Defeat the rival of `rank`. (write)
    pub fn defeat_rival(&self, rank: u32) -> ModResult<()> {
        host::rival_defeat(&self.fns, rank)
    }

    /// Lock or unlock the rival of `rank`. (write)
    pub fn set_rival_locked(&self, rank: u32, locked: bool) -> ModResult<()> {
        host::rival_set_locked(&self.fns, rank, locked)
    }

    /// The milestones.
    pub fn milestones(&self) -> ModResult<Vec<Milestone>> {
        let count = host::milestone_count(&self.fns)?;
        (0..count).map(|index| Milestone::from_abi(&host::milestone_at(&self.fns, index)?)).collect()
    }

    /// Complete milestone `id`. (write)
    pub fn complete_milestone(&self, id: u32) -> ModResult<()> {
        host::milestone_complete(&self.fns, id)
    }

    /// Register a mod milestone; returns its id. Its progress is saved in the mod's slot data. (write)
    pub fn register_milestone(&self, def: &MilestoneDef) -> ModResult<u32> {
        host::milestone_register(&self.fns, &def.to_abi()?)
    }

    /// Set the progress of milestone `id`. (write)
    pub fn set_milestone_progress(&self, id: u32, progress: f32) -> ModResult<()> {
        host::milestone_progress(&self.fns, id, progress)
    }

    /// The blacklist and story stage.
    pub fn stage(&self) -> ModResult<u32> {
        host::stage_get(&self.fns)
    }

    /// The owned cars.
    pub fn garage(&self) -> ModResult<Vec<OwnedCar>> {
        let count = host::garage_count(&self.fns)?;
        (0..count).map(|index| OwnedCar::from_abi(&host::garage_at(&self.fns, index)?)).collect()
    }

    /// Give the player a car. (write)
    pub fn give_car(&self, car: u64) -> ModResult<()> {
        host::give_car(&self.fns, car)
    }

    /// Take a car from the player. (write)
    pub fn remove_car(&self, car: u64) -> ModResult<()> {
        host::remove_car(&self.fns, car)
    }

    /// The car the player drives; `None` for none.
    pub fn current_car(&self) -> ModResult<Option<u64>> {
        host::current_car_get(&self.fns).map(|car| (car != 0).then_some(car))
    }

    /// Set the car the player drives. (write)
    pub fn set_current_car(&self, car: u64) -> ModResult<()> {
        host::current_car_set(&self.fns, car)
    }

    /// Save the career to its slot.
    pub fn save(&self) -> ModResult<()> {
        host::save(&self.fns)
    }

    /// The save slot in use; `None` when no slot is.
    pub fn slot(&self) -> ModResult<Option<u32>> {
        host::slot_get(&self.fns).map(|slot| (slot != 0).then_some(slot))
    }

    /// The safe houses.
    pub fn safehouses(&self) -> ModResult<Vec<Safehouse>> {
        let count = host::safehouse_count(&self.fns)?;
        (0..count).map(|index| Safehouse::from_abi(&host::safehouse_at(&self.fns, index)?)).collect()
    }

    /// Unlock or lock safe house `id`. (write)
    pub fn set_safehouse_unlocked(&self, id: u32, unlocked: bool) -> ModResult<()> {
        host::safehouse_set_unlocked(&self.fns, id, unlocked)
    }

    /// Make the interior `interior` (a handle) a safe house; returns its id. (write)
    pub fn register_safehouse(&self, interior: u64) -> ModResult<u32> {
        host::safehouse_register(&self.fns, interior)
    }

    /// Whether entering a safe house cools the heat. (write)
    pub fn set_safehouse_cools_heat(&self, on: bool) -> ModResult<()> {
        host::safehouse_set_cools_heat(&self.fns, on)
    }

    /// The shops.
    pub fn shops(&self) -> ModResult<Vec<Shop>> {
        let count = host::shop_count(&self.fns)?;
        (0..count).map(|index| Shop::from_abi(&host::shop_at(&self.fns, index)?)).collect()
    }

    /// The items of shop `shop`.
    pub fn shop_stock(&self, shop: u32) -> ModResult<Vec<ShopItem>> {
        let count = host::shop_stock_count(&self.fns, shop)?;
        (0..count).map(|index| ShopItem::from_abi(&host::shop_stock_at(&self.fns, shop, index)?)).collect()
    }

    /// Add an item to shop `shop`; returns its id. (write)
    pub fn shop_add_item(&self, shop: u32, def: &ShopItemDef) -> ModResult<u32> {
        host::shop_add_item(&self.fns, shop, &def.to_abi()?)
    }

    /// Remove item `item` from shop `shop`. (write)
    pub fn shop_remove_item(&self, shop: u32, item: u32) -> ModResult<()> {
        host::shop_remove_item(&self.fns, shop, item)
    }

    /// Set the price of item `item` in shop `shop`. (write)
    pub fn set_price(&self, shop: u32, item: u32, cash: u32) -> ModResult<()> {
        host::shop_set_price(&self.fns, shop, item, cash)
    }
}
