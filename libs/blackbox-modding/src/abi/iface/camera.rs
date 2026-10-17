//! `bb.camera` 1.0: read the camera and choose a gameplay view (catalogue: docs/modding/api-world.md#camera).
//!
//! The mode numbers are the game's gameplay views. A game without a view answers `BB_ERR_ARG` for it. The free camera is
//! not a gameplay view and is not offered yet.

use core::ffi::c_void;

use super::BbIface;
use super::vehicles::BbVec3;
use crate::abi::BbStatus;

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

/// `get`: write the camera to `out`.
pub type BbCameraGetFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut BbCameraState) -> BbStatus;
/// `set_mode`: switch to the view `mode` names.
pub type BbCameraSetModeFn = unsafe extern "C" fn(ctx: *mut c_void, mode: u32) -> BbStatus;

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
}
