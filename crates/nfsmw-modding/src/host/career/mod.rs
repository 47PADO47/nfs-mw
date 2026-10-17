//! `nfsmw.career` on the host side: the backend trait and the builder. The career has no state in the port yet, so every
//! method has a default body that answers `BB_ERR_UNSUPPORTED`. A game that implements the career overrides the methods it
//! has, and the others keep answering unsupported.

mod table;

use blackbox_modding::abi::BB_ERR_UNSUPPORTED;

use crate::abi::{
    BbMilestone, BbMilestoneDef, BbOwnedCar, BbRapSheet, BbRival, BbSafehouse, BbShop, BbShopItem, BbShopItemDef,
};

use super::BackendResult;

pub use table::{CareerTable, career_table};

/// What the game implements for `nfsmw.career`. `owner` is the id of the mod that made the call.
///
/// Each method's default body is `Err(BB_ERR_UNSUPPORTED)`. Index arguments are checked by the backend: an index past the
/// end answers `BB_ERR_NOT_FOUND`.
#[allow(unused_variables)] // The defaults ignore their arguments.
pub trait CareerBackend {
    /// The cash.
    fn cash_get(&self, owner: &str) -> BackendResult<i64> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Add cash. `reason` is a short label for the log.
    fn cash_add(&self, owner: &str, delta: i64, reason: &str) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Set the cash.
    fn cash_set(&self, owner: &str, value: i64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The bounty.
    fn bounty_get(&self, owner: &str) -> BackendResult<u64> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Add to the bounty.
    fn bounty_add(&self, owner: &str, delta: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Set the bounty.
    fn bounty_set(&self, owner: &str, value: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The rap sheet.
    fn rap_sheet_get(&self, owner: &str) -> BackendResult<BbRapSheet> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Set one rap sheet stat.
    fn rap_sheet_set_stat(&self, owner: &str, stat: u32, value: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Add to one rap sheet stat.
    fn rap_sheet_add_stat(&self, owner: &str, stat: u32, delta: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The number of blacklist rivals.
    fn rival_count(&self, owner: &str) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The rival at `index`.
    fn rival_at(&self, owner: &str, index: u32) -> BackendResult<BbRival> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Defeat the rival of `rank`.
    fn rival_defeat(&self, owner: &str, rank: u32) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Lock or unlock the rival of `rank`.
    fn rival_set_locked(&self, owner: &str, rank: u32, locked: bool) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The number of milestones.
    fn milestone_count(&self, owner: &str) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The milestone at `index`.
    fn milestone_at(&self, owner: &str, index: u32) -> BackendResult<BbMilestone> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Complete milestone `id`.
    fn milestone_complete(&self, owner: &str, id: u32) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Register a mod milestone; returns its id.
    fn milestone_register(&self, owner: &str, def: &BbMilestoneDef) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Set the progress of milestone `id`.
    fn milestone_progress(&self, owner: &str, id: u32, progress: f32) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The blacklist and story stage.
    fn stage_get(&self, owner: &str) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The number of owned cars.
    fn garage_count(&self, owner: &str) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The owned car at `index`.
    fn garage_at(&self, owner: &str, index: u32) -> BackendResult<BbOwnedCar> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Give the player a car.
    fn give_car(&self, owner: &str, car: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Take a car from the player.
    fn remove_car(&self, owner: &str, car: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The car the player drives; 0 for none.
    fn current_car_get(&self, owner: &str) -> BackendResult<u64> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Set the car the player drives.
    fn current_car_set(&self, owner: &str, car: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Save the career to its slot.
    fn save(&self, owner: &str) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The save slot in use; 0 for none.
    fn slot_get(&self, owner: &str) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The number of safe houses.
    fn safehouse_count(&self, owner: &str) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The safe house at `index`.
    fn safehouse_at(&self, owner: &str, index: u32) -> BackendResult<BbSafehouse> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Unlock or lock safe house `id`.
    fn safehouse_set_unlocked(&self, owner: &str, id: u32, unlocked: bool) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Make the interior `interior` a safe house; returns its id.
    fn safehouse_register(&self, owner: &str, interior: u64) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Whether entering a safe house cools the heat.
    fn safehouse_set_cools_heat(&self, owner: &str, on: bool) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The number of shops.
    fn shop_count(&self, owner: &str) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The shop at `index`.
    fn shop_at(&self, owner: &str, index: u32) -> BackendResult<BbShop> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// The number of items shop `shop` sells.
    fn shop_stock_count(&self, owner: &str, shop: u32) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Item `index` of shop `shop`.
    fn shop_stock_at(&self, owner: &str, shop: u32, index: u32) -> BackendResult<BbShopItem> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Add an item to shop `shop`; returns its id.
    fn shop_add_item(&self, owner: &str, shop: u32, def: &BbShopItemDef) -> BackendResult<u32> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Remove item `item` from shop `shop`.
    fn shop_remove_item(&self, owner: &str, shop: u32, item: u32) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Set the price of item `item` in shop `shop`.
    fn shop_set_price(&self, owner: &str, shop: u32, item: u32, cash: u32) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
}
