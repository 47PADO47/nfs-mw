//! `bb.handling`: a mod replaces how a vehicle's tyres, suspension and controls produce the forces and commands of the physics
//! step (catalogue: docs/modding/api-handling.md).
//!
//! The interface is generic: it names no game and assumes no model. A mod registers up to three optional hooks, one per
//! concern, and the host calls each for the vehicles it applies to. Each callback answers, or declines and the host runs its own
//! code. Everything a callback needs about the car comes in the query, or from `bb.vehicles` `spec`.
//!
//! | Hook | Since | Called | Replaces |
//! |---|---|---|---|
//! | tyre | 1.0 | once per wheel per tick | the tyre forces of a wheel |
//! | suspension (`handling_suspension.rs`) | 1.1 | once per axle per tick | the spring force of an axle |
//! | controls (`handling_controls.rs`) | 1.2 | once per vehicle per tick | the pedals, brake split and steering target |
//!
//! A hook is two function pointers appended to the table, plus its own `#[repr(C)]` query, result and registration structs
//! that follow the rules below. Nothing already in the table moves, so a mod built for an older minor reads only the fields its
//! `head.size` covers.
//!
//! Struct rules: each query, result and registration struct starts with `size: u32` (the bytes of the writer's layout),
//! holds fixed-width fields only, and ends with reserved words that are zero today. A reader that is older than the writer
//! reads the first `size` bytes it knows, and a reader that is newer treats the bytes the `size` does not cover as zero.
//! A field is appended by taking a reserved word, never by moving one.

use core::ffi::c_void;

use super::BbIface;
use super::handling_controls::{BbHandlingRegisterControlsFn, BbHandlingUnregisterControlsFn};
use super::handling_suspension::{BbHandlingRegisterSuspensionFn, BbHandlingUnregisterSuspensionFn};
use crate::abi::BbStatus;

/// The callback answered for the wheel: its result is valid.
pub const BB_TYRE_HANDLED: BbStatus = 1;
/// The callback declines the wheel: the host runs its own tyre update for it. Not a fault.
pub const BB_TYRE_NOT_HANDLED: BbStatus = 0;

/// The registration asks for the callback to be called for every vehicle role (player, traffic, cops, racers) the host
/// applies hooks to. Without it the host calls only for the player's car. The host in v1 applies hooks to the player only
/// and ignores the flag; it is reserved so a later host can offer other vehicles without a new table.
pub const BB_TYRE_FLAG_ALL_ROLES: u32 = 1;

/// What the host tells the callback about one wheel for one tick. Size-prefixed. SI units; the wheel frame has a
/// forward axis (the wheel's heading, steering included) and a lateral axis that points to the right of it. Reserved
/// words are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbTyreQuery {
    /// Bytes the host filled in.
    pub size: u32,
    /// 0 front left, 1 front right, 2 rear left, 3 rear right.
    pub wheel_index: u32,
    /// The kind of vehicle: `BB_ROLE_*`.
    pub role: u32,
    /// 1 when the wheel presses on the ground, 0 when it is airborne.
    pub loaded: u32,
    /// Changes whenever the vehicle the hook is applied to changes (another car): the revision `bb.vehicles` `spec` reports
    /// for the same vehicle.
    pub vehicle_revision: u32,
    /// The surface id under the wheel (opaque; 0 when airborne).
    pub surface_tag: u32,
    /// The tick length (s).
    pub dt: f32,
    /// Vertical load on the tyre (N); 0 when airborne.
    pub vertical_load_n: f32,
    /// Speed of the contact patch along the lateral axis (m/s, positive to the right); 0 when airborne. Sampled at the start of
    /// this tick's wheel update, before this tick's forces move the body.
    pub patch_lateral_velocity: f32,
    /// Speed of the contact patch along the wheel's heading (m/s, positive forward); 0 when airborne. Sampled like the lateral one.
    pub patch_forward_velocity: f32,
    /// Speed of the whole body (m/s).
    pub body_speed: f32,
    /// Wheel spin (rad/s), positive when rolling forward.
    pub wheel_spin: f32,
    /// Drive torque at the hub this tick (N m, signed: positive pushes the wheel forward).
    pub drive_torque_nm: f32,
    /// Total brake torque at the hub this tick (N m), the handbrake included: always at or above 0, and it opposes the rotation.
    pub brake_torque_nm: f32,
    /// Rolling radius of the wheel (m).
    pub wheel_radius_m: f32,
    /// Moment of inertia of the wheel (kg m^2): the host's constant for its own models. A mod may use its own.
    pub wheel_inertia_kgm2: f32,
    /// Surface multiplier of lateral force (1 on dry tarmac).
    pub surface_lateral: f32,
    /// Surface multiplier of longitudinal force (1 on dry tarmac).
    pub surface_drive: f32,
    /// Surface multiplier of rolling resistance (1 on dry tarmac).
    pub surface_rolling: f32,
    /// 1 for a sound tyre, below 1 for a damaged one.
    pub tyre_condition: f32,
    /// Ambient air temperature (degrees Celsius).
    pub ambient_temperature_c: f32,
    /// Road surface temperature (degrees Celsius).
    pub road_temperature_c: f32,
    /// The sprung mass the host attributes to this wheel (kg): a quarter of the vehicle mass. Appended in 1.0 before release,
    /// taking the first reserved word.
    pub corner_mass_kg: f32,
    /// The camber of the wheel this tick (rad, positive when the top of the wheel leans away from the centre line of the car, on
    /// either side), as the suspension hook answered for it; 0 when no suspension hook answered. Appended in 1.1, taking the
    /// second reserved word; a host older than 1.1 leaves it 0.
    pub camber_rad: f32,
    /// Zero today.
    pub reserved: [u32; 6],
}

/// What the callback answers for one wheel. Size-prefixed: the host sets `size` to its layout and zeroes the rest before
/// the call, and reads only fields the callback's own layout covers (the callback writes the fields it knows).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbTyreResult {
    /// Bytes of the layout the host reads.
    pub size: u32,
    /// Mean longitudinal force over the tick (N, positive forward).
    pub longitudinal_force_n: f32,
    /// Mean lateral force over the tick (N, positive to the right).
    pub lateral_force_n: f32,
    /// Mean self-aligning torque over the tick (N m, positive turns the wheel to the right).
    pub aligning_torque_nm: f32,
    /// The wheel spin at the end of the tick (rad/s).
    pub wheel_spin: f32,
    /// 1 while the tyre grips, below 1 while it slides (0 to 1); feeds the effects.
    pub traction: f32,
    /// Wheel speed minus ground speed along the heading (m/s); feeds the effects.
    pub slip: f32,
    /// Slip angle in turns (1.0 is 360 degrees), positive when the patch moves to the right; feeds the effects.
    pub slip_angle: f32,
    /// Zero today.
    pub reserved: [u32; 8],
}

/// The tyre callback. Called on the main thread, once per wheel per tick, inside the physics step. It returns
/// `BB_TYRE_HANDLED` after writing `result`, `BB_TYRE_NOT_HANDLED` to decline, or a negative `BB_ERR_*` code, which counts
/// as a fault. `query` and `result` are valid for the call only. Host calls that are valid inside a mod callback may be made.
pub type BbTyreHookFn =
    unsafe extern "C" fn(user: *mut c_void, query: *const BbTyreQuery, result: *mut BbTyreResult) -> BbStatus;

/// A tyre hook registration. Size-prefixed; reserved words are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbTyreHook {
    /// Bytes the mod filled in.
    pub size: u32,
    /// `BB_TYRE_FLAG_*`.
    pub flags: u32,
    /// The mod's pointer, passed to the callback. It must stay valid until the hook is unregistered or the mod unloads.
    pub user: *mut c_void,
    /// The callback.
    pub callback: Option<BbTyreHookFn>,
    /// Zero today.
    pub reserved: [u64; 4],
}

impl Default for BbTyreHook {
    fn default() -> Self {
        BbTyreHook { size: 0, flags: 0, user: core::ptr::null_mut(), callback: None, reserved: [0; 4] }
    }
}

/// `register_tyre`: register the tyre hook. One hook per mod; a second call answers `BB_ERR_EXISTS`. The hook's handle goes
/// to `out_id`. `BB_ERR_DENIED` in an online session, `BB_ERR_ARG` for a missing callback or a size below the first fields.
pub type BbHandlingRegisterTyreFn =
    unsafe extern "C" fn(ctx: *mut c_void, hook: *const BbTyreHook, out_id: *mut u64) -> BbStatus;
/// `unregister_tyre`: remove the hook by handle. The built-in tyres run from the next tick.
pub type BbHandlingUnregisterTyreFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;

/// The `bb.handling` table. Field order is part of the ABI; new hooks are appended after the last field. 1.0 is the first 32
/// bytes, 1.1 adds 16 and 1.2 another 16.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbHandling {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Register the tyre hook.
    pub register_tyre: Option<BbHandlingRegisterTyreFn>,
    /// Remove the tyre hook.
    pub unregister_tyre: Option<BbHandlingUnregisterTyreFn>,
    /// Register the suspension hook. Appended in 1.1.
    pub register_suspension: Option<BbHandlingRegisterSuspensionFn>,
    /// Remove the suspension hook. Appended in 1.1.
    pub unregister_suspension: Option<BbHandlingUnregisterSuspensionFn>,
    /// Register the controls hook. Appended in 1.2.
    pub register_controls: Option<BbHandlingRegisterControlsFn>,
    /// Remove the controls hook. Appended in 1.2.
    pub unregister_controls: Option<BbHandlingUnregisterControlsFn>,
}
