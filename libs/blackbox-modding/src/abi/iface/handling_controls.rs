//! `bb.handling` 1.2: the controls hook. A mod filters the driver's pedals and steering before the vehicle turns them into engine,
//! brake and steering commands, and it may take over the steering target (catalogue: docs/modding/api-handling-controls.md).
//!
//! The host calls the hook once per physics tick for the vehicles it applies to. The struct rules of `handling.rs` apply: size
//! first, fixed-width fields, reserved words at the end, and a field is appended by taking a reserved word.

use core::ffi::c_void;

use crate::abi::BbStatus;

/// The callback answered: its result is valid.
pub const BB_CONTROLS_HANDLED: BbStatus = 1;
/// The callback declines: the controls stay as the host shaped them. Not a fault.
pub const BB_CONTROLS_NOT_HANDLED: BbStatus = 0;

/// `BbControlsResult::flags`: `steer_target_rad` is valid and replaces the host's steering shaping. Without it the host keeps its own
/// steering and ignores `steer_target_rad`.
pub const BB_CONTROLS_FLAG_STEER: u32 = 1;

/// What the host tells the callback about the driver and the car for one tick. Size-prefixed. SI units unless a field says
/// otherwise. Positive steering, yaw rate and lateral acceleration are to the right; positive longitudinal acceleration is
/// forward. Reserved words are zero. Wheel arrays are in the order front left, front right, rear left, rear right.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbControlsQuery {
    /// Bytes the host filled in.
    pub size: u32,
    /// The kind of vehicle: `BB_ROLE_*`.
    pub role: u32,
    /// Changes whenever the vehicle the hook is applied to changes: the revision `bb.vehicles` `spec` reports.
    pub vehicle_revision: u32,
    /// The tick length (s).
    pub dt: f32,
    /// The throttle request after the host's input shaping, 0 to 1.
    pub throttle: f32,
    /// The brake request after the host's input shaping (idle hold included), 0 to 1.
    pub brake: f32,
    /// The handbrake request, 0 to 1.
    pub handbrake: f32,
    /// The steering request, -1 to 1, positive to the right.
    pub steer: f32,
    /// The speed along the vehicle's heading (m/s), negative while moving backwards.
    pub forward_speed: f32,
    /// Rotation about the body's up axis (rad/s, positive to the right).
    pub yaw_rate: f32,
    /// Acceleration along the body's lateral axis over the last tick (m/s^2, positive to the right).
    pub lateral_accel: f32,
    /// Acceleration along the vehicle's heading over the last tick (m/s^2, positive forward).
    pub longitudinal_accel: f32,
    /// Bit `i` is set when wheel `i` pressed on the ground in the last tick.
    pub wheel_loaded_mask: u32,
    /// The slip ratio of each wheel in the last tick: (wheel surface speed - ground speed) / ground speed along the direction
    /// of travel, above 0 spinning, below 0 dragging, -1 locked; 0 for an airborne wheel.
    pub slip_ratio: [f32; 4],
    /// The slip angle of each wheel in the last tick, in turns (1.0 is 360 degrees), positive when the contact patch moves to the
    /// right; 0 for an airborne wheel.
    pub slip_angle: [f32; 4],
    /// Zero today.
    pub reserved: [u32; 7],
}

/// What the callback answers. Size-prefixed: the host sets `size` to its layout and zeroes the rest before the call, and reads
/// only fields the callback's own layout covers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbControlsResult {
    /// Bytes of the layout the host reads.
    pub size: u32,
    /// `BB_CONTROLS_FLAG_*`.
    pub flags: u32,
    /// The throttle the engine is given, 0 to 1.
    pub throttle: f32,
    /// The brake demand of each wheel, 0 to 1.5, bias folded in; 1 is the full brake torque of that wheel. It replaces the host's
    /// brake split.
    pub brake: [f32; 4],
    /// The angle of the inside front wheel (rad, positive to the right); read only with `BB_CONTROLS_FLAG_STEER`.
    pub steer_target_rad: f32,
    /// Zero today.
    pub reserved: [u32; 8],
}

/// The controls callback. Called on the main thread, once per tick, inside the physics step. It returns `BB_CONTROLS_HANDLED`
/// after writing `result`, `BB_CONTROLS_NOT_HANDLED` to decline, or a negative `BB_ERR_*` code, which counts as a fault.
pub type BbControlsHookFn =
    unsafe extern "C" fn(user: *mut c_void, query: *const BbControlsQuery, result: *mut BbControlsResult) -> BbStatus;

/// A controls hook registration. Size-prefixed; reserved words are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbControlsHook {
    /// Bytes the mod filled in.
    pub size: u32,
    /// Reserved flags; zero today.
    pub flags: u32,
    /// The mod's pointer, passed to the callback. It must stay valid until the hook is unregistered or the mod unloads.
    pub user: *mut c_void,
    /// The callback.
    pub callback: Option<BbControlsHookFn>,
    /// Zero today.
    pub reserved: [u64; 4],
}

impl Default for BbControlsHook {
    fn default() -> Self {
        BbControlsHook { size: 0, flags: 0, user: core::ptr::null_mut(), callback: None, reserved: [0; 4] }
    }
}

/// `register_controls`: register the controls hook. One hook per mod; a second call answers `BB_ERR_EXISTS`. The hook's handle
/// goes to `out_id`. `BB_ERR_DENIED` in an online session, `BB_ERR_ARG` for a missing callback or a size below the first fields,
/// `BB_ERR_UNSUPPORTED` when the game does not implement the hook.
pub type BbHandlingRegisterControlsFn =
    unsafe extern "C" fn(ctx: *mut c_void, hook: *const BbControlsHook, out_id: *mut u64) -> BbStatus;
/// `unregister_controls`: remove the hook by handle. The controls stay as the host shapes them from the next tick.
pub type BbHandlingUnregisterControlsFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;
