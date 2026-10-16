//! `bb.vehicles` 1.0: the player's car, read and moved (catalogue: docs/modding/api-world.md#vehicles-and-the-player).
//!
//! Only the player's car is offered in 1.0. Other vehicles (traffic, cops, racers) need handles of their own and come in a
//! later minor. `spawn_traffic` is reserved: the game places traffic ahead of the player and has no call to spawn one at a
//! point yet, so it answers `BB_ERR_UNSUPPORTED`.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::BbStatus;

/// A point or a direction in render space: x forward, y left, z up, metres.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbVec3 {
    /// Forward.
    pub x: f32,
    /// Left.
    pub y: f32,
    /// Up.
    pub z: f32,
}

impl BbVec3 {
    /// The three coordinates, x, y, z.
    pub const fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

/// The player's car as the host reads it. Size-prefixed: the host writes the first `size` bytes of its layout, so a mod
/// built against an older minor gets the fields it knows.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbVehicleState {
    /// Bytes the caller allows the host to write.
    pub size: u32,
    /// -1 reverse, 0 neutral, 1 and up the forward gears.
    pub gear: i32,
    /// Position, metres.
    pub position: BbVec3,
    /// Linear velocity, metres per second.
    pub velocity: BbVec3,
    /// Heading in radians, from +x towards +y.
    pub heading: f32,
    /// Speed along the car, metres per second (negative in reverse).
    pub speed: f32,
    /// Engine speed, revolutions per minute.
    pub rpm: f32,
    /// Nitrous left, 0 to 1 (0 for a car without the part).
    pub nos: f32,
    /// Damage, 0 undamaged to 1 with every panel, paint and glass level at its top.
    pub damage: f32,
}

/// `state`: write the player's car to `out`.
pub type BbVehicleStateFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut BbVehicleState) -> BbStatus;
/// `teleport`: put the player's car on the road nearest `position`, facing `heading` (radians). Queued, like `reset`.
pub type BbVehicleTeleportFn = unsafe extern "C" fn(ctx: *mut c_void, position: BbVec3, heading: f32) -> BbStatus;
/// `repair`: clear the visible damage and the punctures of the player's car.
pub type BbVehicleRepairFn = unsafe extern "C" fn(ctx: *mut c_void) -> BbStatus;
/// `spawn_traffic`: spawn a traffic car near `position`. Its handle goes to `out_id`. Reserved, unsupported in 1.0.
pub type BbVehicleSpawnTrafficFn =
    unsafe extern "C" fn(ctx: *mut c_void, position: BbVec3, heading: f32, out_id: *mut u64) -> BbStatus;

/// The `bb.vehicles` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbVehicles {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the player's car.
    pub state: Option<BbVehicleStateFn>,
    /// Move the player's car.
    pub teleport: Option<BbVehicleTeleportFn>,
    /// Repair the player's car.
    pub repair: Option<BbVehicleRepairFn>,
    /// Spawn a traffic car (reserved).
    pub spawn_traffic: Option<BbVehicleSpawnTrafficFn>,
}
