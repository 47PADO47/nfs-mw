//! `bb.camera` 1.1: read the camera, choose a gameplay view, and add views of the mod's own (catalogue:
//! docs/modding/api-world.md#camera).
//!
//! The mode numbers are the game's gameplay views. A game without a view answers `BB_ERR_ARG` for it. The free camera is
//! not a gameplay view and is not offered yet.
//!
//! Version 1.1 lets a mod register **extra views**: cameras the mod computes itself. The player picks one with the
//! camera hotkey or in the options like any other view; while it is the view, the host calls the mod's `update` once per
//! rendered frame, after the car has moved, with the car's state, and draws from the pose it answers. All vectors are
//! render space (x forward, y left, z up; metres, seconds).

use core::ffi::c_void;

use super::BbIface;
use super::vehicles::BbVec3;
use crate::abi::{BbStatus, BbStr};

/// The chase camera behind the car.
pub const BB_CAMERA_CHASE: u32 = 0;
/// The chase camera with the view widening as the car speeds up.
pub const BB_CAMERA_DYNAMIC_FOV: u32 = 1;
/// The bumper view.
pub const BB_CAMERA_BUMPER: u32 = 2;
/// The bonnet view.
pub const BB_CAMERA_HOOD: u32 = 3;
/// The close view.
pub const BB_CAMERA_CLOSE: u32 = 4;
/// The far view.
pub const BB_CAMERA_FAR: u32 = 5;
/// The furthest view.
pub const BB_CAMERA_SUPER_FAR: u32 = 6;
/// The drift view.
pub const BB_CAMERA_DRIFT: u32 = 7;
/// The pursuit view.
pub const BB_CAMERA_PURSUIT: u32 = 8;
/// One of the extra views mods registered (`BbCameraState::mode` while the player looks through one).
pub const BB_CAMERA_EXTRA: u32 = 100;

/// `BbCameraInput::flags`: the view was just selected, the car was moved, or the scene restarted: the mod places the
/// camera at its rest pose and drops any smoothing history.
pub const BB_CAMERA_INPUT_RESET: u32 = 1;
/// `BbCameraPose::flags`: the player's car is not drawn (a camera inside or against the car).
pub const BB_CAMERA_POSE_HIDE_CAR: u32 = 1;

/// The camera as the host reads it. Size-prefixed, as [`super::BbVehicleState`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbCameraState {
    /// Bytes the caller allows the host to write.
    pub size: u32,
    /// One of the `BB_CAMERA_*` modes.
    pub mode: u32,
    /// Where the camera is, metres.
    pub position: BbVec3,
    /// The direction it looks in, unit length.
    pub forward: BbVec3,
    /// The vertical field of view, degrees.
    pub fov: f32,
}

/// The car and the frame, as the host gives them to an extra view. Written by the host; a mod built against a newer
/// minor than the host reads `size` first and uses only the fields it covers. Strings are valid for the call.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbCameraInput {
    /// Bytes the host wrote.
    pub size: u32,
    /// `BB_CAMERA_INPUT_*` bits.
    pub flags: u32,
    /// Seconds since the last call, at least 0.
    pub dt: f32,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    /// Steering, -1 to 1, positive to the left.
    pub steer: f32,
    /// Throttle, 0 to 1.
    pub throttle: f32,
    /// Brake, 0 to 1.
    pub brake: f32,
    /// Strength of the collision this frame, 0 to 1 (0 when none).
    pub impact: f32,
    /// 1 when the car's wheels touch the ground.
    pub grounded: u32,
    /// Padding, zero.
    pub pad: u32,
    /// The origin of the car model (on the ground between the axles), metres.
    pub position: BbVec3,
    /// The car's forward axis, unit length.
    pub forward: BbVec3,
    /// The car's up axis, unit length.
    pub up: BbVec3,
    /// Linear velocity, metres per second.
    pub velocity: BbVec3,
    /// Angular velocity, radians per second.
    pub angular_velocity: BbVec3,
    /// The id of the car the player drives, as the game names it (for example `CARRERAGT`).
    pub car: BbStr,
}

/// The camera an extra view answers. Written by the mod.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BbCameraPose {
    /// Bytes the mod wrote.
    pub size: u32,
    /// `BB_CAMERA_POSE_*` bits.
    pub flags: u32,
    /// Where the camera is, metres.
    pub position: BbVec3,
    /// The direction it looks in, unit length.
    pub forward: BbVec3,
    /// Its up axis, unit length; the picture rolls with it.
    pub up: BbVec3,
    /// The vertical field of view, degrees.
    pub fov: f32,
    /// Padding, zero.
    pub pad: u32,
}

/// `available`: whether the view can be used with the car `car` (non-zero for yes). Asked when the player cycles the
/// camera and when the car changes; a view that answers no is skipped and, if it is the view, the host returns to the
/// chase camera.
pub type BbCameraAvailableFn = unsafe extern "C" fn(user: *mut c_void, car: BbStr) -> u32;
/// `update`: write the pose for this frame to `out` and answer `BB_OK`; any other status keeps the host's chase camera
/// for the frame. Called only while the view is the player's view.
pub type BbCameraUpdateFn =
    unsafe extern "C" fn(user: *mut c_void, input: *const BbCameraInput, out: *mut BbCameraPose) -> BbStatus;

/// One extra view, read by `register_view`. Its strings are valid for that call.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbCameraView {
    /// Bytes the mod filled in.
    pub size: u32,
    /// Reserved, zero.
    pub flags: u32,
    /// The view's id: plain (`[a-z0-9_.-]`), unique among the mod's views. It is saved with the camera setting as
    /// `<mod id>/<view id>`.
    pub id: BbStr,
    /// The name the camera setting shows, or a message id of the mod's catalogue (`bb.i18n`).
    pub name: BbStr,
    /// The mod's pointer, first argument of `available` and `update`; valid until the view is unregistered.
    pub user: *mut c_void,
    /// May be `None`: the view is available for every car.
    pub available: Option<BbCameraAvailableFn>,
    /// Required.
    pub update: Option<BbCameraUpdateFn>,
}

/// `get`: write the camera to `out`.
pub type BbCameraGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut BbCameraState) -> BbStatus;
/// `set_mode`: switch to the view `mode` names.
pub type BbCameraSetModeFn = unsafe extern "C" fn(ctx: *mut c_void, mode: u32) -> BbStatus;

/// `register_view`: add an extra view. Its handle goes to `out_id`. A view id the mod already uses answers
/// `BB_ERR_EXISTS`.
pub type BbCameraRegisterViewFn =
    unsafe extern "C" fn(ctx: *mut c_void, view: *const BbCameraView, out_id: *mut u64) -> BbStatus;
/// `unregister_view`: take back one view. If it is the player's view, the host returns to the chase camera.
pub type BbCameraUnregisterViewFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;
/// `select_view`: make a registered view the player's view. `BB_ERR_ARG` for a handle the mod does not hold. While the view does not fit the car the player drives, the game draws its chase camera.
pub type BbCameraSelectViewFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;

/// The `bb.camera` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbCamera {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Read the camera.
    pub get: Option<BbCameraGetFn>,
    /// Choose a view.
    pub set_mode: Option<BbCameraSetModeFn>,
    /// Added in 1.1: add an extra view.
    pub register_view: Option<BbCameraRegisterViewFn>,
    /// Added in 1.1: take back an extra view.
    pub unregister_view: Option<BbCameraUnregisterViewFn>,
    /// Added in 1.1: look through an extra view.
    pub select_view: Option<BbCameraSelectViewFn>,
}
