//! `nfsmw.career` 0.2: cash, bounty, the rap sheet, the blacklist, milestones, the garage, safe houses and shops.
//!
//! Every slot is in the table, so the layout is fixed. The game answers the slots its career has
//! (`crates/nfsmw/src/modding/backends/career.rs`) and `BB_ERR_UNSUPPORTED` for the rest. Field lists and the stat
//! numbers are [unconfirmed] against the career spec. Slots that write state are marked write (W) in the catalogue.

use core::ffi::c_void;

use blackbox_modding::abi::{BbIface, BbStatus, BbStr};

use super::career_records::{
    BbMilestone, BbMilestoneDef, BbOwnedCar, BbRapSheet, BbRival, BbSafehouse, BbShop, BbShopItem, BbShopItemDef,
};

/// `cash_get`: write the cash to `out`.
pub type BbCareerCashGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut i64) -> BbStatus;
/// `cash_add`: add `delta` cash, with a short reason the host may log (W).
pub type BbCareerCashAddFn = unsafe extern "C" fn(ctx: *mut c_void, delta: i64, reason: BbStr) -> BbStatus;
/// `cash_set`: set the cash (W).
pub type BbCareerCashSetFn = unsafe extern "C" fn(ctx: *mut c_void, value: i64) -> BbStatus;
/// `bounty_get`: write the bounty to `out`.
pub type BbCareerBountyGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u64) -> BbStatus;
/// `bounty_add`: add to the bounty (W).
pub type BbCareerBountyAddFn = unsafe extern "C" fn(ctx: *mut c_void, delta: u64) -> BbStatus;
/// `bounty_set`: set the bounty (W).
pub type BbCareerBountySetFn = unsafe extern "C" fn(ctx: *mut c_void, value: u64) -> BbStatus;
/// `rap_sheet_get`: write the rap sheet to `out`.
pub type BbCareerRapSheetGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut BbRapSheet) -> BbStatus;
/// `rap_sheet_set_stat`: set one rap sheet stat (W).
pub type BbCareerRapSheetSetStatFn = unsafe extern "C" fn(ctx: *mut c_void, stat: u32, value: u64) -> BbStatus;
/// `rap_sheet_add_stat`: add to one rap sheet stat (W).
pub type BbCareerRapSheetAddStatFn = unsafe extern "C" fn(ctx: *mut c_void, stat: u32, delta: u64) -> BbStatus;
/// `rival_count`: write the number of blacklist rivals to `out`.
pub type BbCareerRivalCountFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;
/// `rival_at`: write the rival at `index` to `out`.
pub type BbCareerRivalAtFn = unsafe extern "C" fn(ctx: *mut c_void, index: u32, out: *mut BbRival) -> BbStatus;
/// `rival_defeat`: defeat the rival of `rank` (W).
pub type BbCareerRivalDefeatFn = unsafe extern "C" fn(ctx: *mut c_void, rank: u32) -> BbStatus;
/// `rival_set_locked`: lock (non-zero) or unlock (zero) the rival of `rank` (W).
pub type BbCareerRivalSetLockedFn = unsafe extern "C" fn(ctx: *mut c_void, rank: u32, locked: u8) -> BbStatus;
/// `milestone_count`: write the number of milestones to `out`.
pub type BbCareerMilestoneCountFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;
/// `milestone_at`: write the milestone at `index` to `out`.
pub type BbCareerMilestoneAtFn = unsafe extern "C" fn(ctx: *mut c_void, index: u32, out: *mut BbMilestone) -> BbStatus;
/// `milestone_complete`: complete the milestone `id` (W).
pub type BbCareerMilestoneCompleteFn = unsafe extern "C" fn(ctx: *mut c_void, id: u32) -> BbStatus;
/// `milestone_register`: register a mod milestone; its id goes to `out_id` (W).
pub type BbCareerMilestoneRegisterFn =
    unsafe extern "C" fn(ctx: *mut c_void, def: *const BbMilestoneDef, out_id: *mut u32) -> BbStatus;
/// `milestone_progress`: set the progress of milestone `id` (W).
pub type BbCareerMilestoneProgressFn = unsafe extern "C" fn(ctx: *mut c_void, id: u32, progress: f32) -> BbStatus;
/// `stage_get`: write the blacklist and story stage to `out`.
pub type BbCareerStageGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;
/// `garage_count`: write the number of owned cars to `out`.
pub type BbCareerGarageCountFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;
/// `garage_at`: write the owned car at `index` to `out`.
pub type BbCareerGarageAtFn = unsafe extern "C" fn(ctx: *mut c_void, index: u32, out: *mut BbOwnedCar) -> BbStatus;
/// `give_car`: add a car to the garage (W).
pub type BbCareerGiveCarFn = unsafe extern "C" fn(ctx: *mut c_void, car: u64) -> BbStatus;
/// `remove_car`: remove a car from the garage (W).
pub type BbCareerRemoveCarFn = unsafe extern "C" fn(ctx: *mut c_void, car: u64) -> BbStatus;
/// `current_car_get`: write the car the player drives to `out`, or 0 for none.
pub type BbCareerCurrentCarGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u64) -> BbStatus;
/// `current_car_set`: set the car the player drives (W).
pub type BbCareerCurrentCarSetFn = unsafe extern "C" fn(ctx: *mut c_void, car: u64) -> BbStatus;
/// `save`: write the career to the save slot.
pub type BbCareerSaveFn = unsafe extern "C" fn(ctx: *mut c_void) -> BbStatus;
/// `slot_get`: write the save slot number to `out`, or 0 when no slot is in use.
pub type BbCareerSlotGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;
/// `safehouse_count`: write the number of safe houses to `out`.
pub type BbCareerSafehouseCountFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;
/// `safehouse_at`: write the safe house at `index` to `out`.
pub type BbCareerSafehouseAtFn = unsafe extern "C" fn(ctx: *mut c_void, index: u32, out: *mut BbSafehouse) -> BbStatus;
/// `safehouse_set_unlocked`: unlock (non-zero) or lock (zero) safe house `id` (W).
pub type BbCareerSafehouseSetUnlockedFn = unsafe extern "C" fn(ctx: *mut c_void, id: u32, unlocked: u8) -> BbStatus;
/// `safehouse_register`: make the interior `interior` (a handle) a safe house; the id goes to `out_id` (W).
pub type BbCareerSafehouseRegisterFn =
    unsafe extern "C" fn(ctx: *mut c_void, interior: u64, out_id: *mut u32) -> BbStatus;
/// `safehouse_set_cools_heat`: whether entering a safe house cools the heat (non-zero) or not (W).
pub type BbCareerSafehouseSetCoolsHeatFn = unsafe extern "C" fn(ctx: *mut c_void, on: u8) -> BbStatus;
/// `shop_count`: write the number of shops to `out`.
pub type BbCareerShopCountFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u32) -> BbStatus;
/// `shop_at`: write the shop at `index` to `out`.
pub type BbCareerShopAtFn = unsafe extern "C" fn(ctx: *mut c_void, index: u32, out: *mut BbShop) -> BbStatus;
/// `shop_stock_count`: write the number of items shop `shop` sells to `out`.
pub type BbCareerShopStockCountFn = unsafe extern "C" fn(ctx: *mut c_void, shop: u32, out: *mut u32) -> BbStatus;
/// `shop_stock_at`: write item `index` of shop `shop` to `out`.
pub type BbCareerShopStockAtFn =
    unsafe extern "C" fn(ctx: *mut c_void, shop: u32, index: u32, out: *mut BbShopItem) -> BbStatus;
/// `shop_add_item`: add an item to shop `shop`; its id goes to `out_id` (W).
pub type BbCareerShopAddItemFn =
    unsafe extern "C" fn(ctx: *mut c_void, shop: u32, def: *const BbShopItemDef, out_id: *mut u32) -> BbStatus;
/// `shop_remove_item`: remove item `item` from shop `shop` (W).
pub type BbCareerShopRemoveItemFn = unsafe extern "C" fn(ctx: *mut c_void, shop: u32, item: u32) -> BbStatus;
/// `shop_set_price`: set the price of item `item` in shop `shop` (W).
pub type BbCareerShopSetPriceFn = unsafe extern "C" fn(ctx: *mut c_void, shop: u32, item: u32, cash: u32) -> BbStatus;

/// The `nfsmw.career` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbNfsmwCareer {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the cash.
    pub cash_get: Option<BbCareerCashGetFn>,
    /// Add cash.
    pub cash_add: Option<BbCareerCashAddFn>,
    /// Set cash.
    pub cash_set: Option<BbCareerCashSetFn>,
    /// Read the bounty.
    pub bounty_get: Option<BbCareerBountyGetFn>,
    /// Add bounty.
    pub bounty_add: Option<BbCareerBountyAddFn>,
    /// Set bounty.
    pub bounty_set: Option<BbCareerBountySetFn>,
    /// Read the rap sheet.
    pub rap_sheet_get: Option<BbCareerRapSheetGetFn>,
    /// Set a rap sheet stat.
    pub rap_sheet_set_stat: Option<BbCareerRapSheetSetStatFn>,
    /// Add to a rap sheet stat.
    pub rap_sheet_add_stat: Option<BbCareerRapSheetAddStatFn>,
    /// Count the rivals.
    pub rival_count: Option<BbCareerRivalCountFn>,
    /// Read a rival.
    pub rival_at: Option<BbCareerRivalAtFn>,
    /// Defeat a rival.
    pub rival_defeat: Option<BbCareerRivalDefeatFn>,
    /// Lock or unlock a rival.
    pub rival_set_locked: Option<BbCareerRivalSetLockedFn>,
    /// Count the milestones.
    pub milestone_count: Option<BbCareerMilestoneCountFn>,
    /// Read a milestone.
    pub milestone_at: Option<BbCareerMilestoneAtFn>,
    /// Complete a milestone.
    pub milestone_complete: Option<BbCareerMilestoneCompleteFn>,
    /// Register a mod milestone.
    pub milestone_register: Option<BbCareerMilestoneRegisterFn>,
    /// Set a milestone's progress.
    pub milestone_progress: Option<BbCareerMilestoneProgressFn>,
    /// Read the stage.
    pub stage_get: Option<BbCareerStageGetFn>,
    /// Count the owned cars.
    pub garage_count: Option<BbCareerGarageCountFn>,
    /// Read an owned car.
    pub garage_at: Option<BbCareerGarageAtFn>,
    /// Give a car.
    pub give_car: Option<BbCareerGiveCarFn>,
    /// Remove a car.
    pub remove_car: Option<BbCareerRemoveCarFn>,
    /// Read the current car.
    pub current_car_get: Option<BbCareerCurrentCarGetFn>,
    /// Set the current car.
    pub current_car_set: Option<BbCareerCurrentCarSetFn>,
    /// Save the career.
    pub save: Option<BbCareerSaveFn>,
    /// Read the save slot.
    pub slot_get: Option<BbCareerSlotGetFn>,
    /// Count the safe houses.
    pub safehouse_count: Option<BbCareerSafehouseCountFn>,
    /// Read a safe house.
    pub safehouse_at: Option<BbCareerSafehouseAtFn>,
    /// Unlock or lock a safe house.
    pub safehouse_set_unlocked: Option<BbCareerSafehouseSetUnlockedFn>,
    /// Register an interior as a safe house.
    pub safehouse_register: Option<BbCareerSafehouseRegisterFn>,
    /// Set whether a safe house cools the heat.
    pub safehouse_set_cools_heat: Option<BbCareerSafehouseSetCoolsHeatFn>,
    /// Count the shops.
    pub shop_count: Option<BbCareerShopCountFn>,
    /// Read a shop.
    pub shop_at: Option<BbCareerShopAtFn>,
    /// Count a shop's items.
    pub shop_stock_count: Option<BbCareerShopStockCountFn>,
    /// Read a shop item.
    pub shop_stock_at: Option<BbCareerShopStockAtFn>,
    /// Add a shop item.
    pub shop_add_item: Option<BbCareerShopAddItemFn>,
    /// Remove a shop item.
    pub shop_remove_item: Option<BbCareerShopRemoveItemFn>,
    /// Set a shop item's price.
    pub shop_set_price: Option<BbCareerShopSetPriceFn>,
}
