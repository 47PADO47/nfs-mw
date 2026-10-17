//! `nfsmw.career` on the host side: the trampolines behind [`BbNfsmwCareer`] and the builder that makes the table.

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to host table, io and slot modules.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use blackbox_modding::abi::{BbIface, BbStatus, BbStr, MAX_MESSAGE_BYTES};

use super::CareerBackend;
use crate::abi::{
    BbMilestone, BbMilestoneDef, BbNfsmwCareer, BbOwnedCar, BbRapSheet, BbRival, BbSafehouse, BbShop, BbShopItem,
    BbShopItemDef, IFACE_CAREER_VERSION,
};
use crate::host::io::{put, put_sized, read_sized};
use crate::host::slot::{Slot, guard, slot_of, text_arg};

/// A built `nfsmw.career` table for one mod. It keeps the table and the backend alive.
pub struct CareerTable {
    table: Box<BbNfsmwCareer>,
    _slot: Rc<Slot<dyn CareerBackend>>,
}

impl CareerTable {
    /// The header of the table: the pointer `query_interface` answers with.
    pub fn header(&self) -> *const BbIface {
        core::ptr::from_ref(&*self.table).cast::<BbIface>()
    }
}

/// Build the `nfsmw.career` table for mod `owner`, backed by `backend`.
pub fn career_table(backend: Rc<dyn CareerBackend>, owner: &str) -> CareerTable {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbNfsmwCareer {
        head: BbIface {
            size: size_of::<BbNfsmwCareer>() as u32,
            major: IFACE_CAREER_VERSION.0,
            minor: IFACE_CAREER_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        cash_get: Some(cash_get),
        cash_add: Some(cash_add),
        cash_set: Some(cash_set),
        bounty_get: Some(bounty_get),
        bounty_add: Some(bounty_add),
        bounty_set: Some(bounty_set),
        rap_sheet_get: Some(rap_sheet_get),
        rap_sheet_set_stat: Some(rap_sheet_set_stat),
        rap_sheet_add_stat: Some(rap_sheet_add_stat),
        rival_count: Some(rival_count),
        rival_at: Some(rival_at),
        rival_defeat: Some(rival_defeat),
        rival_set_locked: Some(rival_set_locked),
        milestone_count: Some(milestone_count),
        milestone_at: Some(milestone_at),
        milestone_complete: Some(milestone_complete),
        milestone_register: Some(milestone_register),
        milestone_progress: Some(milestone_progress),
        stage_get: Some(stage_get),
        garage_count: Some(garage_count),
        garage_at: Some(garage_at),
        give_car: Some(give_car),
        remove_car: Some(remove_car),
        current_car_get: Some(current_car_get),
        current_car_set: Some(current_car_set),
        save: Some(save),
        slot_get: Some(slot_get),
        safehouse_count: Some(safehouse_count),
        safehouse_at: Some(safehouse_at),
        safehouse_set_unlocked: Some(safehouse_set_unlocked),
        safehouse_register: Some(safehouse_register),
        safehouse_set_cools_heat: Some(safehouse_set_cools_heat),
        shop_count: Some(shop_count),
        shop_at: Some(shop_at),
        shop_stock_count: Some(shop_stock_count),
        shop_stock_at: Some(shop_stock_at),
        shop_add_item: Some(shop_add_item),
        shop_remove_item: Some(shop_remove_item),
        shop_set_price: Some(shop_set_price),
    });
    CareerTable { table, _slot: slot }
}

/// Read a flag the mod sends as a byte: any non-zero value is true.
fn flag(raw: u8) -> bool {
    raw != 0
}

/// `cash_get`.
unsafe extern "C" fn cash_get(ctx: *mut c_void, out: *mut i64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let value = slot.backend.cash_get(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, value) }
    })
}

/// `cash_add`: the reason is copied for the call only.
unsafe extern "C" fn cash_add(ctx: *mut c_void, delta: i64, reason: BbStr) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        // SAFETY: the reason is valid for this call (ADR 0005 section 4).
        let reason = unsafe { text_arg(reason, MAX_MESSAGE_BYTES)? };
        slot.backend.cash_add(&slot.owner, delta, reason)
    })
}

/// `cash_set`.
unsafe extern "C" fn cash_set(ctx: *mut c_void, value: i64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.cash_set(&slot.owner, value)
    })
}

/// `bounty_get`.
unsafe extern "C" fn bounty_get(ctx: *mut c_void, out: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let value = slot.backend.bounty_get(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, value) }
    })
}

/// `bounty_add`.
unsafe extern "C" fn bounty_add(ctx: *mut c_void, delta: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.bounty_add(&slot.owner, delta)
    })
}

/// `bounty_set`.
unsafe extern "C" fn bounty_set(ctx: *mut c_void, value: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.bounty_set(&slot.owner, value)
    })
}

/// `rap_sheet_get`: a size-prefixed record.
unsafe extern "C" fn rap_sheet_get(ctx: *mut c_void, out: *mut BbRapSheet) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let sheet = slot.backend.rap_sheet_get(&slot.owner)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &sheet) }
    })
}

/// `rap_sheet_set_stat`.
unsafe extern "C" fn rap_sheet_set_stat(ctx: *mut c_void, stat: u32, value: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.rap_sheet_set_stat(&slot.owner, stat, value)
    })
}

/// `rap_sheet_add_stat`.
unsafe extern "C" fn rap_sheet_add_stat(ctx: *mut c_void, stat: u32, delta: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.rap_sheet_add_stat(&slot.owner, stat, delta)
    })
}

/// `rival_count`.
unsafe extern "C" fn rival_count(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let count = slot.backend.rival_count(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, count) }
    })
}

/// `rival_at`.
unsafe extern "C" fn rival_at(ctx: *mut c_void, index: u32, out: *mut BbRival) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let rival = slot.backend.rival_at(&slot.owner, index)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &rival) }
    })
}

/// `rival_defeat`.
unsafe extern "C" fn rival_defeat(ctx: *mut c_void, rank: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.rival_defeat(&slot.owner, rank)
    })
}

/// `rival_set_locked`.
unsafe extern "C" fn rival_set_locked(ctx: *mut c_void, rank: u32, locked: u8) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.rival_set_locked(&slot.owner, rank, flag(locked))
    })
}

/// `milestone_count`.
unsafe extern "C" fn milestone_count(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let count = slot.backend.milestone_count(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, count) }
    })
}

/// `milestone_at`.
unsafe extern "C" fn milestone_at(ctx: *mut c_void, index: u32, out: *mut BbMilestone) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let milestone = slot.backend.milestone_at(&slot.owner, index)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &milestone) }
    })
}

/// `milestone_complete`.
unsafe extern "C" fn milestone_complete(ctx: *mut c_void, id: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.milestone_complete(&slot.owner, id)
    })
}

/// `milestone_register`: the definition is read by its size word; the id goes to `out_id`.
unsafe extern "C" fn milestone_register(ctx: *mut c_void, def: *const BbMilestoneDef, out_id: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        // SAFETY: `def` is the mod's record, read by its own size word (see `read_sized`).
        let def = unsafe { read_sized(def)? };
        let id = slot.backend.milestone_register(&slot.owner, &def)?;
        // SAFETY: `out_id` is checked for null by `put`.
        unsafe { put(out_id, id) }
    })
}

/// `milestone_progress`.
unsafe extern "C" fn milestone_progress(ctx: *mut c_void, id: u32, progress: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.milestone_progress(&slot.owner, id, progress)
    })
}

/// `stage_get`.
unsafe extern "C" fn stage_get(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let stage = slot.backend.stage_get(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, stage) }
    })
}

/// `garage_count`.
unsafe extern "C" fn garage_count(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let count = slot.backend.garage_count(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, count) }
    })
}

/// `garage_at`.
unsafe extern "C" fn garage_at(ctx: *mut c_void, index: u32, out: *mut BbOwnedCar) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let car = slot.backend.garage_at(&slot.owner, index)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &car) }
    })
}

/// `give_car`.
unsafe extern "C" fn give_car(ctx: *mut c_void, car: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.give_car(&slot.owner, car)
    })
}

/// `remove_car`.
unsafe extern "C" fn remove_car(ctx: *mut c_void, car: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.remove_car(&slot.owner, car)
    })
}

/// `current_car_get`.
unsafe extern "C" fn current_car_get(ctx: *mut c_void, out: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let car = slot.backend.current_car_get(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, car) }
    })
}

/// `current_car_set`.
unsafe extern "C" fn current_car_set(ctx: *mut c_void, car: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.current_car_set(&slot.owner, car)
    })
}

/// `save`.
unsafe extern "C" fn save(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.save(&slot.owner)
    })
}

/// `slot_get`.
unsafe extern "C" fn slot_get(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let number = slot.backend.slot_get(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, number) }
    })
}

/// `safehouse_count`.
unsafe extern "C" fn safehouse_count(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let count = slot.backend.safehouse_count(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, count) }
    })
}

/// `safehouse_at`.
unsafe extern "C" fn safehouse_at(ctx: *mut c_void, index: u32, out: *mut BbSafehouse) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let house = slot.backend.safehouse_at(&slot.owner, index)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &house) }
    })
}

/// `safehouse_set_unlocked`.
unsafe extern "C" fn safehouse_set_unlocked(ctx: *mut c_void, id: u32, unlocked: u8) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.safehouse_set_unlocked(&slot.owner, id, flag(unlocked))
    })
}

/// `safehouse_register`.
unsafe extern "C" fn safehouse_register(ctx: *mut c_void, interior: u64, out_id: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let id = slot.backend.safehouse_register(&slot.owner, interior)?;
        // SAFETY: `out_id` is checked for null by `put`.
        unsafe { put(out_id, id) }
    })
}

/// `safehouse_set_cools_heat`.
unsafe extern "C" fn safehouse_set_cools_heat(ctx: *mut c_void, on: u8) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.safehouse_set_cools_heat(&slot.owner, flag(on))
    })
}

/// `shop_count`.
unsafe extern "C" fn shop_count(ctx: *mut c_void, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let count = slot.backend.shop_count(&slot.owner)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, count) }
    })
}

/// `shop_at`.
unsafe extern "C" fn shop_at(ctx: *mut c_void, index: u32, out: *mut BbShop) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let shop = slot.backend.shop_at(&slot.owner, index)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &shop) }
    })
}

/// `shop_stock_count`.
unsafe extern "C" fn shop_stock_count(ctx: *mut c_void, shop: u32, out: *mut u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let count = slot.backend.shop_stock_count(&slot.owner, shop)?;
        // SAFETY: `out` is checked for null by `put`.
        unsafe { put(out, count) }
    })
}

/// `shop_stock_at`.
unsafe extern "C" fn shop_stock_at(ctx: *mut c_void, shop: u32, index: u32, out: *mut BbShopItem) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        let item = slot.backend.shop_stock_at(&slot.owner, shop, index)?;
        // SAFETY: `out` is a record the mod sized (see `put_sized`).
        unsafe { put_sized(out, &item) }
    })
}

/// `shop_add_item`: the definition is read by its size word; the id goes to `out_id`.
unsafe extern "C" fn shop_add_item(
    ctx: *mut c_void,
    shop: u32,
    def: *const BbShopItemDef,
    out_id: *mut u32,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        // SAFETY: `def` is the mod's record, read by its own size word (see `read_sized`).
        let def = unsafe { read_sized(def)? };
        let id = slot.backend.shop_add_item(&slot.owner, shop, &def)?;
        // SAFETY: `out_id` is checked for null by `put`.
        unsafe { put(out_id, id) }
    })
}

/// `shop_remove_item`.
unsafe extern "C" fn shop_remove_item(ctx: *mut c_void, shop: u32, item: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.shop_remove_item(&slot.owner, shop, item)
    })
}

/// `shop_set_price`.
unsafe extern "C" fn shop_set_price(ctx: *mut c_void, shop: u32, item: u32, cash: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CareerBackend>(ctx)? };
        slot.backend.shop_set_price(&slot.owner, shop, item, cash)
    })
}
