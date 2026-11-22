//! `bb.handling` 1.1: the suspension hook. A mod replaces the vertical force of an axle: the spring, the damper, the stops and
//! the anti-roll bar, and it tells the host the camber and the vertical load of its wheels (catalogue:
//! docs/modding/api-handling-suspension.md).
//!
//! The host calls the hook once per axle per physics tick, the front axle first, because the two wheels of an axle are
//! coupled and a model integrates them together. The struct rules of `handling.rs` apply: size first, fixed-width fields, reserved
//! words at the end, and a field is appended by taking a reserved word.

use core::ffi::c_void;

use crate::abi::BbStatus;

/// The callback answered for the axle: its result is valid.
pub const BB_SUSPENSION_HANDLED: BbStatus = 1;
/// The callback declines the axle: the host runs its own spring for both wheels. Not a fault.
pub const BB_SUSPENSION_NOT_HANDLED: BbStatus = 0;

/// What the host tells the callback about one wheel of the axle. A fixed element of [`BbSuspensionQuery::wheels`]; it grows
/// only by taking a reserved word.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbSuspensionWheel {
    /// 1 when the wheel presses on the ground, 0 when it is airborne.
    pub loaded: u32,
    /// How far the wheel is pushed up from full droop (m), from 0 to `max_travel_m`; 0 when the wheel is not touching.
    pub compression_m: f32,
    /// The rate of change of the compression over the last tick (m/s, positive when compressing).
    pub compression_speed_mps: f32,
    /// The steering angle of this wheel (rad, positive to the right; 0 on an axle that does not steer).
    pub steer_angle_rad: f32,
    /// The acceleration of the body point above the wheel along the body's up axis (m/s^2), kinematic: 0 at rest, about -9.81 in
    /// free fall. Taken over the last tick; 0 on the first tick.
    pub body_accel_up_mps2: f32,
    /// The lateral force the tyre of this wheel put on the body in the last tick (N, positive to the right); 0 when airborne.
    pub tyre_lateral_force_n: f32,
    /// The vertical load the host estimates for this wheel at rest (N).
    pub static_load_n: f32,
    /// Zero today.
    pub reserved: [u32; 3],
}

/// What the host tells the callback about one axle for one tick. Size-prefixed. SI units. Compression is positive when the
/// wheel moves up towards the body; a force on the body is positive when it pushes the body up. Reserved words are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbSuspensionQuery {
    /// Bytes the host filled in.
    pub size: u32,
    /// 0 front axle, 1 rear axle.
    pub axle_index: u32,
    /// The kind of vehicle: `BB_ROLE_*`.
    pub role: u32,
    /// Changes whenever the vehicle the hook is applied to changes: the revision `bb.vehicles` `spec` reports.
    pub vehicle_revision: u32,
    /// The tick length (s).
    pub dt: f32,
    /// The compression of a wheel of this axle standing at rest on flat ground (m): the sag at which the vehicle's own spring carries
    /// the wheel's static load. `compression_m - rest_travel_m` is 0 for a car at rest. It depends on the car, not on the ride-height tuning.
    pub rest_travel_m: f32,
    /// The largest compression (m).
    pub max_travel_m: f32,
    /// The sprung mass the host attributes to one wheel (kg): a quarter of the vehicle mass.
    pub corner_mass_kg: f32,
    /// The left wheel at index 0, the right wheel at index 1.
    pub wheels: [BbSuspensionWheel; 2],
    /// Zero today.
    pub reserved: [u32; 8],
}

/// What the callback answers for one wheel. A fixed element of [`BbSuspensionResult::wheels`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbSuspensionWheelResult {
    /// The force on the body at the wheel along the body's up axis (N, positive up); it may be negative.
    pub body_force_n: f32,
    /// The vertical force between the road and the tyre (N), at or above 0; it becomes the tyre's vertical load.
    pub tyre_load_n: f32,
    /// The camber of the wheel (rad, positive when the top leans away from the centre line of the car, on either side); the
    /// host hands it to the tyre hook of the same wheel in the same tick.
    pub camber_rad: f32,
    /// Zero today.
    pub reserved: [u32; 2],
}

/// What the callback answers for one axle. Size-prefixed: the host sets `size` to its layout and zeroes the rest before the
/// call, and reads only fields the callback's own layout covers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbSuspensionResult {
    /// Bytes of the layout the host reads.
    pub size: u32,
    /// The left wheel at index 0, the right wheel at index 1.
    pub wheels: [BbSuspensionWheelResult; 2],
    /// Zero today.
    pub reserved: [u32; 8],
}

/// The suspension callback. Called on the main thread, once per axle per tick, inside the physics step. It returns
/// `BB_SUSPENSION_HANDLED` after writing `result`, `BB_SUSPENSION_NOT_HANDLED` to decline, or a negative `BB_ERR_*` code, which
/// counts as a fault. `query` and `result` are valid for the call only.
pub type BbSuspensionHookFn = unsafe extern "C" fn(
    user: *mut c_void,
    query: *const BbSuspensionQuery,
    result: *mut BbSuspensionResult,
) -> BbStatus;

/// A suspension hook registration. Size-prefixed; reserved words are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbSuspensionHook {
    /// Bytes the mod filled in.
    pub size: u32,
    /// Reserved flags; zero today.
    pub flags: u32,
    /// The mod's pointer, passed to the callback. It must stay valid until the hook is unregistered or the mod unloads.
    pub user: *mut c_void,
    /// The callback.
    pub callback: Option<BbSuspensionHookFn>,
    /// Zero today.
    pub reserved: [u64; 4],
}

impl Default for BbSuspensionHook {
    fn default() -> Self {
        BbSuspensionHook { size: 0, flags: 0, user: core::ptr::null_mut(), callback: None, reserved: [0; 4] }
    }
}

/// `register_suspension`: register the suspension hook. One hook per mod; a second call answers `BB_ERR_EXISTS`. The hook's
/// handle goes to `out_id`. `BB_ERR_DENIED` in an online session, `BB_ERR_ARG` for a missing callback or a size below the first
/// fields, `BB_ERR_UNSUPPORTED` when the game does not implement the hook.
pub type BbHandlingRegisterSuspensionFn =
    unsafe extern "C" fn(ctx: *mut c_void, hook: *const BbSuspensionHook, out_id: *mut u64) -> BbStatus;
/// `unregister_suspension`: remove the hook by handle. The built-in springs run from the next tick.
pub type BbHandlingUnregisterSuspensionFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;
