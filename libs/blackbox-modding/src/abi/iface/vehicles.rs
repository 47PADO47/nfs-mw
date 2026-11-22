//! `bb.vehicles` 1.3: the player's car, read and moved (catalogue: docs/modding/api-world.md#vehicles-and-the-player).
//!
//! 1.1 appends to the state the slip angle, the steering, the strongest hit and the ground contact of the car, and appends
//! `add_nos` to the table. 1.2 appends `set_drift_assist`. 1.3 appends `spec`, a read-only physical summary of the car. A mod built against 1.0 reads the 1.0 fields of the same struct and
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

/// `BbVehicleSpec::drive_layout`: the front axle is driven.
pub const BB_DRIVE_FRONT: u32 = 0;
/// The rear axle is driven.
pub const BB_DRIVE_REAR: u32 = 1;
/// Both axles are driven.
pub const BB_DRIVE_ALL: u32 = 2;
/// Forward gears `BbVehicleSpec::gear_ratios` holds.
pub const BB_SPEC_GEARS: usize = 8;

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

/// The player's car in physical numbers (1.3). Size-prefixed like [`BbVehicleState`]: the host writes the first `size` bytes
/// of its layout. SI units, except where a name says otherwise. Every value is the host's best estimate from the car's data:
/// a mod may use or ignore any of them. A quantity the host has no number for is 0, and the catalogue
/// (docs/modding/api-handling.md) lists which those are for each host. Axle pairs are front then rear. The reserved words
/// are zero today; a later minor takes them for new fields.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbVehicleSpec {
    /// Bytes the caller allows the host to write.
    pub size: u32,
    /// Changes whenever the player's car changes (another car, or a change of its data). The tyre query carries the same
    /// number as `vehicle_revision`, so a hook can tell which car a call is for. Starts at 1 for the first car.
    pub revision: u32,
    /// Mass of the whole car (kg).
    pub mass_kg: f32,
    /// Distance between the axles (m).
    pub wheelbase_m: f32,
    /// Distance between the tyre centres of the front axle (m).
    pub track_front_m: f32,
    /// Distance between the tyre centres of the rear axle (m).
    pub track_rear_m: f32,
    /// Height of the centre of gravity above the road at rest (m).
    pub cg_height_m: f32,
    /// Share of the weight on the front axle at rest, 0 to 1.
    pub front_weight_fraction: f32,
    /// Rolling radius of the front tyres (m).
    pub tyre_radius_front_m: f32,
    /// Rolling radius of the rear tyres (m).
    pub tyre_radius_rear_m: f32,
    /// Section width of the front tyres (m).
    pub tyre_width_front_m: f32,
    /// Section width of the rear tyres (m).
    pub tyre_width_rear_m: f32,
    /// Friction coefficient of the front tyres at the limit of grip (dimensionless).
    pub peak_friction_front: f32,
    /// Friction coefficient of the rear tyres at the limit of grip (dimensionless).
    pub peak_friction_rear: f32,
    /// Friction coefficient of the front tyres while sliding (dimensionless).
    pub sliding_friction_front: f32,
    /// Friction coefficient of the rear tyres while sliding (dimensionless).
    pub sliding_friction_rear: f32,
    /// `BB_DRIVE_FRONT`, `BB_DRIVE_REAR` or `BB_DRIVE_ALL`.
    pub drive_layout: u32,
    /// Forward gears the car has; the first `min(gear_count, 8)` entries of `gear_ratios` are set.
    pub gear_count: u32,
    /// The highest engine power (W), before forced induction and nitrous.
    pub max_power_w: f32,
    /// The engine speed of that power (rpm).
    pub max_power_rpm: f32,
    /// The rev limiter (rpm).
    pub redline_rpm: f32,
    /// The final drive ratio.
    pub final_drive: f32,
    /// The forward gear ratios from first gear, without the final drive. Entries past `gear_count` are 0.
    pub gear_ratios: [f32; BB_SPEC_GEARS],
    /// Brake torque at one front wheel (N m).
    pub brake_torque_front_nm: f32,
    /// Brake torque at one rear wheel (N m).
    pub brake_torque_rear_nm: f32,
    /// Drag area `Cd * A` (m^2): `0.5 * 1.225 * area * speed^2` is the drag with the throttle open. 0 when unknown.
    pub drag_area_m2: f32,
    /// Lift area of the downforce (m^2): `0.5 * 1.225 * area * speed^2` is the downforce. 0 when unknown.
    pub downforce_area_m2: f32,
    /// Zero today.
    pub reserved: [u32; 8],
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

/// `spec` (1.3): write the physical summary of the player's car to `out`, as far as its size word allows.
/// `BB_ERR_NOT_READY` while no car is driven.
pub type BbVehicleSpecFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut BbVehicleSpec) -> BbStatus;

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
    /// Read the physical summary of the player's car (1.3).
    pub spec: Option<BbVehicleSpecFn>,
}
