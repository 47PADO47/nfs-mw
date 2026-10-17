//! `bb.map` on the mod side: read the table, and add, update and remove blips.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{BbBlip, BbMap, BbMapAddFn, BbMapRemoveFn, BbMapUpdateFn, BbStr};
use crate::sdk::{BlipId, Error, Interface, ModResult};

/// The function pointers of one `bb.map` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct MapFns {
    ctx: *mut c_void,
    add: Option<BbMapAddFn>,
    update: Option<BbMapUpdateFn>,
    remove: Option<BbMapRemoveFn>,
}

/// Read the `bb.map` table the host offered.
pub(crate) fn read(iface: Interface) -> MapFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        MapFns {
            ctx: read_or(&iface, offset_of!(BbMap, ctx), null_mut()),
            add: read_or(&iface, offset_of!(BbMap, add), None),
            update: read_or(&iface, offset_of!(BbMap, update), None),
            remove: read_or(&iface, offset_of!(BbMap, remove), None),
        }
    }
}

/// The blip as the host reads it: this SDK always writes the full, current layout.
fn wire_blip(style: &str, position: [f32; 3], active: bool) -> BbBlip {
    BbBlip {
        size: size_of::<BbBlip>() as u32,
        active: u8::from(active),
        _pad: [0; 3],
        style: BbStr::new(style),
        position,
    }
}

/// Add a blip; returns its handle.
pub(crate) fn add(fns: &MapFns, style: &str, position: [f32; 3], active: bool) -> ModResult<BlipId> {
    let add = fns.add.ok_or(Error::Unsupported)?;
    let blip = wire_blip(style, position, active);
    let mut id = 0_u64;
    // SAFETY: `blip` lives for the call and names a `style` valid for it; `id` is valid for one write.
    status_result(unsafe { add(fns.ctx, &blip, &mut id) })?;
    Ok(BlipId::from_raw(id))
}

/// Replace a blip's state.
pub(crate) fn update(fns: &MapFns, id: BlipId, style: &str, position: [f32; 3], active: bool) -> ModResult<()> {
    let update = fns.update.ok_or(Error::Unsupported)?;
    let blip = wire_blip(style, position, active);
    // SAFETY: as for `add`.
    status_result(unsafe { update(fns.ctx, id.raw(), &blip) })
}

/// Remove a blip.
pub(crate) fn remove(fns: &MapFns, id: BlipId) -> ModResult<()> {
    let remove = fns.remove.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { remove(fns.ctx, id.raw()) })
}
