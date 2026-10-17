//! `bb.map` on the host side: the backend the game implements, and the trampolines behind `BbMap`.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::addr_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, name_of, slot_of};
use crate::abi::{BB_ERR_ARG, BbBlip, BbIface, BbMap, BbStatus, IFACE_MAP, IFACE_MAP_VERSION, has_field};

/// A blip as the game receives it, with the mod's text borrowed for the call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlipSpec<'a> {
    /// Name of a blip style the host registered.
    pub style: &'a str,
    /// Position in render space (x forward, y left, z up), metres.
    pub position: [f32; 3],
    /// Whether the blip is drawn.
    pub active: bool,
}

/// What the game implements for `bb.map`. `owner` is the id of the mod that made the call.
pub trait MapBackend {
    /// Add a blip and return its handle.
    fn add(&self, owner: &str, blip: &BlipSpec<'_>) -> BackendResult<u64>;
    /// Replace the state of a blip.
    fn update(&self, owner: &str, handle: u64, blip: &BlipSpec<'_>) -> BackendResult<()>;
    /// Remove a blip.
    fn remove(&self, owner: &str, handle: u64) -> BackendResult<()>;
}

/// Build the `bb.map` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn MapBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbMap {
        head: BbIface { size: size_of::<BbMap>() as u32, major: IFACE_MAP_VERSION.0, minor: IFACE_MAP_VERSION.1 },
        ctx: Slot::ctx(&slot),
        add: Some(add),
        update: Some(update),
        remove: Some(remove),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        kept.release_with(|backend, owner, handle| {
            let _ = backend.remove(owner, handle);
        })
    });
    Attached::new(IFACE_MAP, IFACE_MAP_VERSION, table, release)
}

/// Read a blip the mod wrote. The blip's `size` must cover `position`, which ends the fields a blip has to carry, so
/// `style`, `active` and `position` are all covered once it passes. A non-finite position is refused.
///
/// # Safety
/// `blip` must be null, or valid for reads of the bytes its own `size` field declares.
unsafe fn read_blip<'a>(blip: *const BbBlip) -> Result<BlipSpec<'a>, BbStatus> {
    if blip.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the size word is the first field of every version of the blip (see `# Safety`).
    let size = unsafe { addr_of!((*blip).size).read() };
    if !has_field(size, offset_of!(BbBlip, position) + size_of::<[f32; 3]>()) {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the blip's size covers `style`, `active` and `position` (checked above).
    let style = unsafe { name_of(addr_of!((*blip).style).read()) }?;
    // SAFETY: as above.
    let active = unsafe { addr_of!((*blip).active).read() } != 0;
    // SAFETY: as above.
    let position = unsafe { addr_of!((*blip).position).read() };
    if !position.iter().all(|value| value.is_finite()) {
        return Err(BB_ERR_ARG);
    }
    Ok(BlipSpec { style, position, active })
}

/// `add`: a new blip; its handle goes to `out_id`.
unsafe extern "C" fn add(ctx: *mut c_void, blip: *const BbBlip, out_id: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn MapBackend>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the mod's blip is valid for the call, and `read_blip` checks its size.
        let spec = unsafe { read_blip(blip) }?;
        let handle = slot.backend.add(&slot.owner, &spec)?;
        slot.remember(handle);
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// `update`: new state for one blip.
unsafe extern "C" fn update(ctx: *mut c_void, handle: u64, blip: *const BbBlip) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn MapBackend>(ctx)? };
        // SAFETY: as in `add`.
        let spec = unsafe { read_blip(blip) }?;
        slot.backend.update(&slot.owner, handle, &spec)
    })
}

/// `remove`: drop one blip.
unsafe extern "C" fn remove(ctx: *mut c_void, handle: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn MapBackend>(ctx)? };
        slot.backend.remove(&slot.owner, handle)?;
        slot.forget(handle);
        Ok(())
    })
}
