//! `nfsmw.career` on the mod side: read the table, and call its functions. Each call returns the record it fills, as plain
//! ABI data; the safe wrappers in `sdk::career` turn that into owned Rust values.

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to sdk::ffi.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use blackbox_modding::abi::{BbIface, BbStr};
use blackbox_modding::sdk::{Error, Interface, ModResult};

use super::{check, read_field, sized_out};
use crate::abi::{
    BbCareerBountyAddFn, BbCareerBountyGetFn, BbCareerBountySetFn, BbCareerCashAddFn, BbCareerCashGetFn,
    BbCareerCashSetFn, BbCareerCurrentCarGetFn, BbCareerCurrentCarSetFn, BbCareerGarageAtFn, BbCareerGarageCountFn,
    BbCareerGiveCarFn, BbCareerMilestoneAtFn, BbCareerMilestoneCompleteFn, BbCareerMilestoneCountFn,
    BbCareerMilestoneProgressFn, BbCareerMilestoneRegisterFn, BbCareerRapSheetAddStatFn, BbCareerRapSheetGetFn,
    BbCareerRapSheetSetStatFn, BbCareerRemoveCarFn, BbCareerRivalAtFn, BbCareerRivalCountFn, BbCareerRivalDefeatFn,
    BbCareerRivalSetLockedFn, BbCareerSafehouseAtFn, BbCareerSafehouseCountFn, BbCareerSafehouseRegisterFn,
    BbCareerSafehouseSetCoolsHeatFn, BbCareerSafehouseSetUnlockedFn, BbCareerSaveFn, BbCareerShopAddItemFn,
    BbCareerShopAtFn, BbCareerShopCountFn, BbCareerShopRemoveItemFn, BbCareerShopSetPriceFn, BbCareerShopStockAtFn,
    BbCareerShopStockCountFn, BbCareerSlotGetFn, BbCareerStageGetFn, BbMilestone, BbMilestoneDef, BbNfsmwCareer,
    BbOwnedCar, BbRapSheet, BbRival, BbSafehouse, BbShop, BbShopItem, BbShopItemDef,
};

/// The context and function pointers of one `nfsmw.career` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct CareerFns {
    ctx: *mut c_void,
    cash_get: Option<BbCareerCashGetFn>,
    cash_add: Option<BbCareerCashAddFn>,
    cash_set: Option<BbCareerCashSetFn>,
    bounty_get: Option<BbCareerBountyGetFn>,
    bounty_add: Option<BbCareerBountyAddFn>,
    bounty_set: Option<BbCareerBountySetFn>,
    rap_sheet_get: Option<BbCareerRapSheetGetFn>,
    rap_sheet_set_stat: Option<BbCareerRapSheetSetStatFn>,
    rap_sheet_add_stat: Option<BbCareerRapSheetAddStatFn>,
    rival_count: Option<BbCareerRivalCountFn>,
    rival_at: Option<BbCareerRivalAtFn>,
    rival_defeat: Option<BbCareerRivalDefeatFn>,
    rival_set_locked: Option<BbCareerRivalSetLockedFn>,
    milestone_count: Option<BbCareerMilestoneCountFn>,
    milestone_at: Option<BbCareerMilestoneAtFn>,
    milestone_complete: Option<BbCareerMilestoneCompleteFn>,
    milestone_register: Option<BbCareerMilestoneRegisterFn>,
    milestone_progress: Option<BbCareerMilestoneProgressFn>,
    stage_get: Option<BbCareerStageGetFn>,
    garage_count: Option<BbCareerGarageCountFn>,
    garage_at: Option<BbCareerGarageAtFn>,
    give_car: Option<BbCareerGiveCarFn>,
    remove_car: Option<BbCareerRemoveCarFn>,
    current_car_get: Option<BbCareerCurrentCarGetFn>,
    current_car_set: Option<BbCareerCurrentCarSetFn>,
    save: Option<BbCareerSaveFn>,
    slot_get: Option<BbCareerSlotGetFn>,
    safehouse_count: Option<BbCareerSafehouseCountFn>,
    safehouse_at: Option<BbCareerSafehouseAtFn>,
    safehouse_set_unlocked: Option<BbCareerSafehouseSetUnlockedFn>,
    safehouse_register: Option<BbCareerSafehouseRegisterFn>,
    safehouse_set_cools_heat: Option<BbCareerSafehouseSetCoolsHeatFn>,
    shop_count: Option<BbCareerShopCountFn>,
    shop_at: Option<BbCareerShopAtFn>,
    shop_stock_count: Option<BbCareerShopStockCountFn>,
    shop_stock_at: Option<BbCareerShopStockAtFn>,
    shop_add_item: Option<BbCareerShopAddItemFn>,
    shop_remove_item: Option<BbCareerShopRemoveItemFn>,
    shop_set_price: Option<BbCareerShopSetPriceFn>,
}

/// Read the `nfsmw.career` table the host offered.
pub(crate) fn read(iface: &Interface) -> CareerFns {
    // SAFETY: the host keeps an offered table valid while the mod is loaded, and `size` is the table's own size word.
    unsafe { read_raw(iface.as_ptr(), iface.size()) }
}

/// Read the `nfsmw.career` table at `base`.
///
/// # Safety
/// `base` must point to an `nfsmw.career` table that stays valid while the mod is loaded, and `size` its size word.
pub(crate) unsafe fn read_raw(base: *const BbIface, size: u32) -> CareerFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_field`).
    unsafe {
        CareerFns {
            ctx: read_field(base, size, offset_of!(BbNfsmwCareer, ctx), null_mut()),
            cash_get: read_field(base, size, offset_of!(BbNfsmwCareer, cash_get), None),
            cash_add: read_field(base, size, offset_of!(BbNfsmwCareer, cash_add), None),
            cash_set: read_field(base, size, offset_of!(BbNfsmwCareer, cash_set), None),
            bounty_get: read_field(base, size, offset_of!(BbNfsmwCareer, bounty_get), None),
            bounty_add: read_field(base, size, offset_of!(BbNfsmwCareer, bounty_add), None),
            bounty_set: read_field(base, size, offset_of!(BbNfsmwCareer, bounty_set), None),
            rap_sheet_get: read_field(base, size, offset_of!(BbNfsmwCareer, rap_sheet_get), None),
            rap_sheet_set_stat: read_field(base, size, offset_of!(BbNfsmwCareer, rap_sheet_set_stat), None),
            rap_sheet_add_stat: read_field(base, size, offset_of!(BbNfsmwCareer, rap_sheet_add_stat), None),
            rival_count: read_field(base, size, offset_of!(BbNfsmwCareer, rival_count), None),
            rival_at: read_field(base, size, offset_of!(BbNfsmwCareer, rival_at), None),
            rival_defeat: read_field(base, size, offset_of!(BbNfsmwCareer, rival_defeat), None),
            rival_set_locked: read_field(base, size, offset_of!(BbNfsmwCareer, rival_set_locked), None),
            milestone_count: read_field(base, size, offset_of!(BbNfsmwCareer, milestone_count), None),
            milestone_at: read_field(base, size, offset_of!(BbNfsmwCareer, milestone_at), None),
            milestone_complete: read_field(base, size, offset_of!(BbNfsmwCareer, milestone_complete), None),
            milestone_register: read_field(base, size, offset_of!(BbNfsmwCareer, milestone_register), None),
            milestone_progress: read_field(base, size, offset_of!(BbNfsmwCareer, milestone_progress), None),
            stage_get: read_field(base, size, offset_of!(BbNfsmwCareer, stage_get), None),
            garage_count: read_field(base, size, offset_of!(BbNfsmwCareer, garage_count), None),
            garage_at: read_field(base, size, offset_of!(BbNfsmwCareer, garage_at), None),
            give_car: read_field(base, size, offset_of!(BbNfsmwCareer, give_car), None),
            remove_car: read_field(base, size, offset_of!(BbNfsmwCareer, remove_car), None),
            current_car_get: read_field(base, size, offset_of!(BbNfsmwCareer, current_car_get), None),
            current_car_set: read_field(base, size, offset_of!(BbNfsmwCareer, current_car_set), None),
            save: read_field(base, size, offset_of!(BbNfsmwCareer, save), None),
            slot_get: read_field(base, size, offset_of!(BbNfsmwCareer, slot_get), None),
            safehouse_count: read_field(base, size, offset_of!(BbNfsmwCareer, safehouse_count), None),
            safehouse_at: read_field(base, size, offset_of!(BbNfsmwCareer, safehouse_at), None),
            safehouse_set_unlocked: read_field(base, size, offset_of!(BbNfsmwCareer, safehouse_set_unlocked), None),
            safehouse_register: read_field(base, size, offset_of!(BbNfsmwCareer, safehouse_register), None),
            safehouse_set_cools_heat: read_field(base, size, offset_of!(BbNfsmwCareer, safehouse_set_cools_heat), None),
            shop_count: read_field(base, size, offset_of!(BbNfsmwCareer, shop_count), None),
            shop_at: read_field(base, size, offset_of!(BbNfsmwCareer, shop_at), None),
            shop_stock_count: read_field(base, size, offset_of!(BbNfsmwCareer, shop_stock_count), None),
            shop_stock_at: read_field(base, size, offset_of!(BbNfsmwCareer, shop_stock_at), None),
            shop_add_item: read_field(base, size, offset_of!(BbNfsmwCareer, shop_add_item), None),
            shop_remove_item: read_field(base, size, offset_of!(BbNfsmwCareer, shop_remove_item), None),
            shop_set_price: read_field(base, size, offset_of!(BbNfsmwCareer, shop_set_price), None),
        }
    }
}

/// Call a function that writes one `u32`-or-larger scalar to `out`, typed `T`.
fn get_scalar<T: Copy + Default>(
    call: Option<unsafe extern "C" fn(*mut c_void, *mut T) -> i32>,
    ctx: *mut c_void,
) -> ModResult<T> {
    let call = call.ok_or(Error::Unsupported)?;
    let mut out = T::default();
    // SAFETY: `out` is valid for one write; the host keeps the table valid while the mod is loaded.
    check(unsafe { call(ctx, &mut out) })?;
    Ok(out)
}

/// Call a function that writes one size-prefixed record to `out`.
fn get_record<R: Copy>(
    call: Option<unsafe extern "C" fn(*mut c_void, *mut R) -> i32>,
    ctx: *mut c_void,
) -> ModResult<R> {
    let call = call.ok_or(Error::Unsupported)?;
    // SAFETY: the record is sized by its own size word (see `sized_out`); `R` is plain data.
    let mut out: R = unsafe { sized_out() };
    // SAFETY: `out` is a record the mod owns, sized as above; the host fills at most that many bytes.
    check(unsafe { call(ctx, &mut out) })?;
    Ok(out)
}

/// Call a function that writes one size-prefixed record to `out`, by index.
fn get_indexed<R: Copy>(
    call: Option<unsafe extern "C" fn(*mut c_void, u32, *mut R) -> i32>,
    ctx: *mut c_void,
    index: u32,
) -> ModResult<R> {
    let call = call.ok_or(Error::Unsupported)?;
    // SAFETY: the record is sized by its own size word (see `sized_out`); `R` is plain data.
    let mut out: R = unsafe { sized_out() };
    // SAFETY: as in `get_record`.
    check(unsafe { call(ctx, index, &mut out) })?;
    Ok(out)
}

/// The cash.
pub(crate) fn cash_get(fns: &CareerFns) -> ModResult<i64> {
    get_scalar(fns.cash_get, fns.ctx)
}

/// Add cash with a reason label.
pub(crate) fn cash_add(fns: &CareerFns, delta: i64, reason: &str) -> ModResult<()> {
    let call = fns.cash_add.ok_or(Error::Unsupported)?;
    // SAFETY: the reason is valid for the call; scalars need no other check.
    check(unsafe { call(fns.ctx, delta, BbStr::new(reason)) })
}

/// Set the cash.
pub(crate) fn cash_set(fns: &CareerFns, value: i64) -> ModResult<()> {
    let call = fns.cash_set.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, value) })
}

/// The bounty.
pub(crate) fn bounty_get(fns: &CareerFns) -> ModResult<u64> {
    get_scalar(fns.bounty_get, fns.ctx)
}

/// Add to the bounty.
pub(crate) fn bounty_add(fns: &CareerFns, delta: u64) -> ModResult<()> {
    let call = fns.bounty_add.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, delta) })
}

/// Set the bounty.
pub(crate) fn bounty_set(fns: &CareerFns, value: u64) -> ModResult<()> {
    let call = fns.bounty_set.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, value) })
}

/// The rap sheet.
pub(crate) fn rap_sheet_get(fns: &CareerFns) -> ModResult<BbRapSheet> {
    get_record(fns.rap_sheet_get, fns.ctx)
}

/// Set one rap sheet stat.
pub(crate) fn rap_sheet_set_stat(fns: &CareerFns, stat: u32, value: u64) -> ModResult<()> {
    let call = fns.rap_sheet_set_stat.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, stat, value) })
}

/// Add to one rap sheet stat.
pub(crate) fn rap_sheet_add_stat(fns: &CareerFns, stat: u32, delta: u64) -> ModResult<()> {
    let call = fns.rap_sheet_add_stat.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, stat, delta) })
}

/// The number of rivals.
pub(crate) fn rival_count(fns: &CareerFns) -> ModResult<u32> {
    get_scalar(fns.rival_count, fns.ctx)
}

/// The rival at `index`.
pub(crate) fn rival_at(fns: &CareerFns, index: u32) -> ModResult<BbRival> {
    get_indexed(fns.rival_at, fns.ctx, index)
}

/// Defeat the rival of `rank`.
pub(crate) fn rival_defeat(fns: &CareerFns, rank: u32) -> ModResult<()> {
    let call = fns.rival_defeat.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, rank) })
}

/// Lock or unlock the rival of `rank`.
pub(crate) fn rival_set_locked(fns: &CareerFns, rank: u32, locked: bool) -> ModResult<()> {
    let call = fns.rival_set_locked.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, rank, u8::from(locked)) })
}

/// The number of milestones.
pub(crate) fn milestone_count(fns: &CareerFns) -> ModResult<u32> {
    get_scalar(fns.milestone_count, fns.ctx)
}

/// The milestone at `index`.
pub(crate) fn milestone_at(fns: &CareerFns, index: u32) -> ModResult<BbMilestone> {
    get_indexed(fns.milestone_at, fns.ctx, index)
}

/// Complete milestone `id`.
pub(crate) fn milestone_complete(fns: &CareerFns, id: u32) -> ModResult<()> {
    let call = fns.milestone_complete.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, id) })
}

/// Register a milestone; returns its id.
pub(crate) fn milestone_register(fns: &CareerFns, def: &BbMilestoneDef) -> ModResult<u32> {
    let call = fns.milestone_register.ok_or(Error::Unsupported)?;
    let mut id = 0_u32;
    // SAFETY: `def` and `id` are valid for the call; the host reads `def` by its size word.
    check(unsafe { call(fns.ctx, def, &mut id) })?;
    Ok(id)
}

/// Set the progress of milestone `id`.
pub(crate) fn milestone_progress(fns: &CareerFns, id: u32, progress: f32) -> ModResult<()> {
    let call = fns.milestone_progress.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, id, progress) })
}

/// The stage.
pub(crate) fn stage_get(fns: &CareerFns) -> ModResult<u32> {
    get_scalar(fns.stage_get, fns.ctx)
}

/// The number of owned cars.
pub(crate) fn garage_count(fns: &CareerFns) -> ModResult<u32> {
    get_scalar(fns.garage_count, fns.ctx)
}

/// The owned car at `index`.
pub(crate) fn garage_at(fns: &CareerFns, index: u32) -> ModResult<BbOwnedCar> {
    get_indexed(fns.garage_at, fns.ctx, index)
}

/// Give a car.
pub(crate) fn give_car(fns: &CareerFns, car: u64) -> ModResult<()> {
    let call = fns.give_car.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, car) })
}

/// Remove a car.
pub(crate) fn remove_car(fns: &CareerFns, car: u64) -> ModResult<()> {
    let call = fns.remove_car.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, car) })
}

/// The current car; 0 for none.
pub(crate) fn current_car_get(fns: &CareerFns) -> ModResult<u64> {
    get_scalar(fns.current_car_get, fns.ctx)
}

/// Set the current car.
pub(crate) fn current_car_set(fns: &CareerFns, car: u64) -> ModResult<()> {
    let call = fns.current_car_set.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, car) })
}

/// Save the career.
pub(crate) fn save(fns: &CareerFns) -> ModResult<()> {
    let call = fns.save.ok_or(Error::Unsupported)?;
    // SAFETY: no arguments besides the context.
    check(unsafe { call(fns.ctx) })
}

/// The save slot; 0 for none.
pub(crate) fn slot_get(fns: &CareerFns) -> ModResult<u32> {
    get_scalar(fns.slot_get, fns.ctx)
}

/// The number of safe houses.
pub(crate) fn safehouse_count(fns: &CareerFns) -> ModResult<u32> {
    get_scalar(fns.safehouse_count, fns.ctx)
}

/// The safe house at `index`.
pub(crate) fn safehouse_at(fns: &CareerFns, index: u32) -> ModResult<BbSafehouse> {
    get_indexed(fns.safehouse_at, fns.ctx, index)
}

/// Unlock or lock safe house `id`.
pub(crate) fn safehouse_set_unlocked(fns: &CareerFns, id: u32, unlocked: bool) -> ModResult<()> {
    let call = fns.safehouse_set_unlocked.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, id, u8::from(unlocked)) })
}

/// Make the interior `interior` a safe house; returns its id.
pub(crate) fn safehouse_register(fns: &CareerFns, interior: u64) -> ModResult<u32> {
    let call = fns.safehouse_register.ok_or(Error::Unsupported)?;
    let mut id = 0_u32;
    // SAFETY: scalar argument and a valid `id` slot.
    check(unsafe { call(fns.ctx, interior, &mut id) })?;
    Ok(id)
}

/// Whether a safe house cools the heat.
pub(crate) fn safehouse_set_cools_heat(fns: &CareerFns, on: bool) -> ModResult<()> {
    let call = fns.safehouse_set_cools_heat.ok_or(Error::Unsupported)?;
    // SAFETY: a scalar argument.
    check(unsafe { call(fns.ctx, u8::from(on)) })
}

/// The number of shops.
pub(crate) fn shop_count(fns: &CareerFns) -> ModResult<u32> {
    get_scalar(fns.shop_count, fns.ctx)
}

/// The shop at `index`.
pub(crate) fn shop_at(fns: &CareerFns, index: u32) -> ModResult<BbShop> {
    get_indexed(fns.shop_at, fns.ctx, index)
}

/// The number of items shop `shop` sells.
pub(crate) fn shop_stock_count(fns: &CareerFns, shop: u32) -> ModResult<u32> {
    let call = fns.shop_stock_count.ok_or(Error::Unsupported)?;
    let mut out = 0_u32;
    // SAFETY: scalar argument and a valid `out`.
    check(unsafe { call(fns.ctx, shop, &mut out) })?;
    Ok(out)
}

/// Item `index` of shop `shop`.
pub(crate) fn shop_stock_at(fns: &CareerFns, shop: u32, index: u32) -> ModResult<BbShopItem> {
    let call = fns.shop_stock_at.ok_or(Error::Unsupported)?;
    // SAFETY: the record is sized by its own size word (see `sized_out`).
    let mut out: BbShopItem = unsafe { sized_out() };
    // SAFETY: as in `get_record`.
    check(unsafe { call(fns.ctx, shop, index, &mut out) })?;
    Ok(out)
}

/// Add an item to shop `shop`; returns its id.
pub(crate) fn shop_add_item(fns: &CareerFns, shop: u32, def: &BbShopItemDef) -> ModResult<u32> {
    let call = fns.shop_add_item.ok_or(Error::Unsupported)?;
    let mut id = 0_u32;
    // SAFETY: `def` and `id` are valid for the call; the host reads `def` by its size word.
    check(unsafe { call(fns.ctx, shop, def, &mut id) })?;
    Ok(id)
}

/// Remove item `item` from shop `shop`.
pub(crate) fn shop_remove_item(fns: &CareerFns, shop: u32, item: u32) -> ModResult<()> {
    let call = fns.shop_remove_item.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, shop, item) })
}

/// Set the price of item `item` in shop `shop`.
pub(crate) fn shop_set_price(fns: &CareerFns, shop: u32, item: u32, cash: u32) -> ModResult<()> {
    let call = fns.shop_set_price.ok_or(Error::Unsupported)?;
    // SAFETY: scalar arguments.
    check(unsafe { call(fns.ctx, shop, item, cash) })
}
