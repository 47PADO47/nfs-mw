//! `bb.vehicles` 1.2: the player's car, read and moved (catalogue: docs/modding/api-world.md#vehicles-and-the-player).
//!
//! 1.1 appends to the state the slip angle, the steering, the strongest hit and the ground contact of the car, and appends
//! `add_nos` to the table. 1.2 appends `set_drift_assist`. A mod built against 1.0 reads the 1.0 fields of the same struct and
//! never sees the table grow.
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
    /// 1.1. Body slip angle in radians: the angle between where the car points and where it moves, positive when it moves
    /// to the left of its heading; 0 below 1 m/s forward speed.
    pub slip_angle: f32,
    /// 1.1. Steering, -1 to 1, positive to the left.
    pub steering: f32,
    /// 1.1. The strongest hit of the latest physics step against the world or an object, 0 to 1 (a closing speed of 20 m/s
    /// or more is 1). The ground does not count. A hit may be reported for several frames if the physics did not step.
    pub impact: f32,
    /// 1.1. 1 while at least one wheel touches the ground, else 0.
    pub on_ground: u32,
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

/// `add_nos` (1.1): add `amount` of a full tank (0 to 1, finite) to the player's nitrous, up to a full tank. Queued, like
/// `repair`. A car without a nitrous system ignores it.
pub type BbVehicleAddNosFn = unsafe extern "C" fn(ctx: *mut c_void, amount: f32) -> BbStatus;

/// `set_drift_assist` (1.2): turn the drift assist of the player's car on (non-zero) or off (zero). The assist is a physics
/// layer that makes a slide easy to start with the handbrake and to hold with the wheel; it is off until a mod asks. Queued,
/// like `repair`, and it stays on across cars and until the mod turns it off.
pub type BbVehicleSetDriftAssistFn = unsafe extern "C" fn(ctx: *mut c_void, enabled: u32) -> BbStatus;

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
    /// Add nitrous to the player's car (1.1).
    pub add_nos: Option<BbVehicleAddNosFn>,
    /// Turn the drift assist on or off (1.2).
    pub set_drift_assist: Option<BbVehicleSetDriftAssistFn>,
}
