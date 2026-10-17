//! `bb.world` on the host side: the backend the game implements, and the trampolines behind `BbWorld`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, emit_text, guard, slot_of};
use crate::abi::{BB_ERR_ARG, BbIface, BbSinkFn, BbStatus, BbWorld, IFACE_WORLD, IFACE_WORLD_VERSION};

/// The most cop cars a traffic density may ask for, as a percentage of the cars.
pub const MAX_COP_SHARE: u32 = 100;

/// What the game implements for `bb.world`. `owner` is the id of the mod that made the call.
pub trait WorldBackend {
    /// The name of the track. The game answers `BB_ERR_NOT_READY` while no world is loaded.
    fn track(&self, owner: &str) -> BackendResult<String>;
    /// The name of the district at the player's position.
    fn section(&self, owner: &str) -> BackendResult<String>;
    /// Keep it raining (`always`), or leave it to the weather (not `always`), at `amount` (0 to 1, already checked).
    fn set_rain(&self, owner: &str, always: bool, amount: f32) -> BackendResult<()>;
    /// Keep `cars` computer-driven cars on the road, `cop_share` percent of them patrol cops (already checked to be at most
    /// [`MAX_COP_SHARE`]).
    fn set_traffic(&self, owner: &str, cars: u32, cop_share: u32) -> BackendResult<()>;
}

/// Build the `bb.world` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn WorldBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbWorld {
        head: BbIface { size: size_of::<BbWorld>() as u32, major: IFACE_WORLD_VERSION.0, minor: IFACE_WORLD_VERSION.1 },
        ctx: Slot::ctx(&slot),
        track: Some(track),
        section: Some(section),
        set_rain: Some(set_rain),
        set_traffic: Some(set_traffic),
    });
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_WORLD, IFACE_WORLD_VERSION, table, release)
}

/// `track`: the track's name goes to the sink.
unsafe extern "C" fn track(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn WorldBackend>(ctx)? };
        let name = slot.backend.track(&slot.owner)?;
        // SAFETY: the sink and its user pointer are the mod's, valid for this call.
        unsafe { emit_text(sink, sink_user, &name) }
    })
}

/// `section`: the district's name goes to the sink.
unsafe extern "C" fn section(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn WorldBackend>(ctx)? };
        let name = slot.backend.section(&slot.owner)?;
        // SAFETY: the sink and its user pointer are the mod's, valid for this call.
        unsafe { emit_text(sink, sink_user, &name) }
    })
}

/// `set_rain`: `always` is any non-zero value; the amount is a finite fraction from 0 to 1.
unsafe extern "C" fn set_rain(ctx: *mut c_void, always: u8, amount: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn WorldBackend>(ctx)? };
        if !(0.0..=1.0).contains(&amount) {
            return Err(BB_ERR_ARG);
        }
        slot.backend.set_rain(&slot.owner, always != 0, amount)
    })
}

/// `set_traffic`: a cop share above [`MAX_COP_SHARE`] is refused.
unsafe extern "C" fn set_traffic(ctx: *mut c_void, cars: u32, cop_share: u32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn WorldBackend>(ctx)? };
        if cop_share > MAX_COP_SHARE {
            return Err(BB_ERR_ARG);
        }
        slot.backend.set_traffic(&slot.owner, cars, cop_share)
    })
}
