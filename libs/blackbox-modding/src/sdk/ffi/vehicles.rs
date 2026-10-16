//! `bb.vehicles` on the mod side: read the table, read the player's car, move it, repair it.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{
    BbVec3, BbVehicleRepairFn, BbVehicleSpawnTrafficFn, BbVehicleState, BbVehicleStateFn, BbVehicleTeleportFn,
    BbVehicles,
};
use crate::sdk::{Error, Interface, ModResult, VehicleState};

/// The function pointers of one `bb.vehicles` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct VehiclesFns {
    ctx: *mut c_void,
    state: Option<BbVehicleStateFn>,
    teleport: Option<BbVehicleTeleportFn>,
    repair: Option<BbVehicleRepairFn>,
    spawn_traffic: Option<BbVehicleSpawnTrafficFn>,
}

/// Read the `bb.vehicles` table the host offered.
pub(crate) fn read(iface: Interface) -> VehiclesFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        VehiclesFns {
            ctx: read_or(&iface, offset_of!(BbVehicles, ctx), null_mut()),
            state: read_or(&iface, offset_of!(BbVehicles, state), None),
            teleport: read_or(&iface, offset_of!(BbVehicles, teleport), None),
            repair: read_or(&iface, offset_of!(BbVehicles, repair), None),
            spawn_traffic: read_or(&iface, offset_of!(BbVehicles, spawn_traffic), None),
        }
    }
}

/// The player's car. The mod sets the size word to its own layout; the host writes what it has of it.
pub(crate) fn state(fns: &VehiclesFns) -> ModResult<VehicleState> {
    let state = fns.state.ok_or(Error::Unsupported)?;
    let mut raw = BbVehicleState { size: size_of::<BbVehicleState>() as u32, ..BbVehicleState::default() };
    // SAFETY: `raw` is valid for one write of its whole layout, and its size word says so.
    status_result(unsafe { state(fns.ctx, &mut raw) })?;
    Ok(VehicleState {
        gear: raw.gear,
        position: raw.position.to_array(),
        velocity: raw.velocity.to_array(),
        heading: raw.heading,
        speed: raw.speed,
        rpm: raw.rpm,
        nos: raw.nos,
        damage: raw.damage,
    })
}

/// Queue a move of the player's car.
pub(crate) fn teleport(fns: &VehiclesFns, position: [f32; 3], heading: f32) -> ModResult<()> {
    let teleport = fns.teleport.ok_or(Error::Unsupported)?;
    let at = BbVec3 { x: position[0], y: position[1], z: position[2] };
    // SAFETY: the table is valid while the mod is loaded; the arguments are plain values.
    status_result(unsafe { teleport(fns.ctx, at, heading) })
}

/// Repair the player's car.
pub(crate) fn repair(fns: &VehiclesFns) -> ModResult<()> {
    let repair = fns.repair.ok_or(Error::Unsupported)?;
    // SAFETY: as for `teleport`.
    status_result(unsafe { repair(fns.ctx) })
}

/// Spawn a traffic car near `position`; returns the raw handle.
pub(crate) fn spawn_traffic(fns: &VehiclesFns, position: [f32; 3], heading: f32) -> ModResult<u64> {
    let spawn = fns.spawn_traffic.ok_or(Error::Unsupported)?;
    let at = BbVec3 { x: position[0], y: position[1], z: position[2] };
    let mut id = 0_u64;
    // SAFETY: `id` is valid for one write; the other arguments are plain values.
    status_result(unsafe { spawn(fns.ctx, at, heading, &mut id) })?;
    Ok(id)
}
