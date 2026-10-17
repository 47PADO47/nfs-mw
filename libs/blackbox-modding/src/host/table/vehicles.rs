//! `bb.vehicles` on the host side: the backend the game implements, and the trampolines behind `BbVehicles`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, slot_of, write_prefixed};
use crate::abi::{
    BB_ERR_ARG, BbIface, BbStatus, BbVec3, BbVehicleState, BbVehicles, IFACE_VEHICLES, IFACE_VEHICLES_VERSION,
};

/// The player's car as the game reports it. Units as in [`BbVehicleState`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VehicleState {
    /// -1 reverse, 0 neutral, 1 and up the forward gears.
    pub gear: i32,
    /// Position, metres, render space.
    pub position: [f32; 3],
    /// Linear velocity, metres per second, render space.
    pub velocity: [f32; 3],
    /// Heading, radians.
    pub heading: f32,
    /// Speed along the car, metres per second (negative in reverse).
    pub speed: f32,
    /// Engine speed, rpm.
    pub rpm: f32,
    /// Nitrous left, 0 to 1.
    pub nos: f32,
    /// Damage, 0 to 1.
    pub damage: f32,
}

/// What the game implements for `bb.vehicles`. `owner` is the id of the mod that made the call.
pub trait VehiclesBackend {
    /// The player's car.
    fn state(&self, owner: &str) -> BackendResult<VehicleState>;
    /// Move the player's car to the road nearest `position`, facing `heading`. Both already checked to be finite.
    fn teleport(&self, owner: &str, position: [f32; 3], heading: f32) -> BackendResult<()>;
    /// Clear the damage and the punctures of the player's car.
    fn repair(&self, owner: &str) -> BackendResult<()>;
    /// Spawn a traffic car near `position`. Returns its handle. The game answers `BB_ERR_UNSUPPORTED` until it can.
    fn spawn_traffic(&self, owner: &str, position: [f32; 3], heading: f32) -> BackendResult<u64>;
}

/// Build the `bb.vehicles` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn VehiclesBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbVehicles {
        head: BbIface {
            size: size_of::<BbVehicles>() as u32,
            major: IFACE_VEHICLES_VERSION.0,
            minor: IFACE_VEHICLES_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        state: Some(state),
        teleport: Some(teleport),
        repair: Some(repair),
        spawn_traffic: Some(spawn_traffic),
    });
    // Nothing the mod registers stays in the game, so unloading removes nothing; the release keeps the slot alive.
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_VEHICLES, IFACE_VEHICLES_VERSION, table, release)
}

/// A point the mod sends: three finite numbers.
fn point(value: BbVec3) -> Result<[f32; 3], BbStatus> {
    let point = [value.x, value.y, value.z];
    if !point.iter().all(|coord| coord.is_finite()) {
        return Err(BB_ERR_ARG);
    }
    Ok(point)
}

/// A value the mod sends that must be finite.
fn finite(value: f32) -> Result<f32, BbStatus> {
    if !value.is_finite() {
        return Err(BB_ERR_ARG);
    }
    Ok(value)
}

/// `state`: write the player's car to `out`, as far as the mod's size word allows.
unsafe extern "C" fn state(ctx: *mut c_void, out: *mut BbVehicleState) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        let car = slot.backend.state(&slot.owner)?;
        let wire = BbVehicleState {
            size: size_of::<BbVehicleState>() as u32,
            gear: car.gear,
            position: BbVec3 { x: car.position[0], y: car.position[1], z: car.position[2] },
            velocity: BbVec3 { x: car.velocity[0], y: car.velocity[1], z: car.velocity[2] },
            heading: car.heading,
            speed: car.speed,
            rpm: car.rpm,
            nos: car.nos,
            damage: car.damage,
        };
        // SAFETY: `out` is the mod's struct; `write_prefixed` checks its size word before it writes.
        unsafe { write_prefixed(out, &wire) }
    })
}

/// `teleport`: queue the player's car to move.
unsafe extern "C" fn teleport(ctx: *mut c_void, position: BbVec3, heading: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        let position = point(position)?;
        slot.backend.teleport(&slot.owner, position, finite(heading)?)
    })
}

/// `repair`: clear the player's car's damage.
unsafe extern "C" fn repair(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        slot.backend.repair(&slot.owner)
    })
}

/// `spawn_traffic`: a new traffic car; its handle goes to `out_id`.
unsafe extern "C" fn spawn_traffic(ctx: *mut c_void, position: BbVec3, heading: f32, out_id: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn VehiclesBackend>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        let handle = slot.backend.spawn_traffic(&slot.owner, point(position)?, finite(heading)?)?;
        // SAFETY: `out_id` is non-null (checked above) and valid for one write (the mod's contract).
        unsafe { out_id.write(handle) };
        Ok(())
    })
}
