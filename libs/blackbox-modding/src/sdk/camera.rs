//! `Camera`: read the camera and choose a gameplay view (catalogue: docs/modding/api-world.md#camera).

use crate::abi::{
    BB_CAMERA_BUMPER, BB_CAMERA_CHASE, BB_CAMERA_CLOSE, BB_CAMERA_DRIFT, BB_CAMERA_DYNAMIC_FOV, BB_CAMERA_FAR,
    BB_CAMERA_HOOD, BB_CAMERA_PURSUIT, BB_CAMERA_SUPER_FAR,
};

use super::ffi::camera::{self as host_camera, CameraFns};
use super::{Error, ModResult};

/// A gameplay view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraKind {
    Chase,
    DynamicFov,
    Bumper,
    Hood,
    Close,
    Far,
    SuperFar,
    Drift,
    Pursuit,
}

impl CameraKind {
    fn from_raw(raw: u32) -> Option<Self> {
        Some(match raw {
            BB_CAMERA_CHASE => CameraKind::Chase,
            BB_CAMERA_DYNAMIC_FOV => CameraKind::DynamicFov,
            BB_CAMERA_BUMPER => CameraKind::Bumper,
            BB_CAMERA_HOOD => CameraKind::Hood,
            BB_CAMERA_CLOSE => CameraKind::Close,
            BB_CAMERA_FAR => CameraKind::Far,
            BB_CAMERA_SUPER_FAR => CameraKind::SuperFar,
            BB_CAMERA_DRIFT => CameraKind::Drift,
            BB_CAMERA_PURSUIT => CameraKind::Pursuit,
            _ => return None,
        })
    }

    fn raw(self) -> u32 {
        match self {
            CameraKind::Chase => BB_CAMERA_CHASE,
            CameraKind::DynamicFov => BB_CAMERA_DYNAMIC_FOV,
            CameraKind::Bumper => BB_CAMERA_BUMPER,
            CameraKind::Hood => BB_CAMERA_HOOD,
            CameraKind::Close => BB_CAMERA_CLOSE,
            CameraKind::Far => BB_CAMERA_FAR,
            CameraKind::SuperFar => BB_CAMERA_SUPER_FAR,
            CameraKind::Drift => BB_CAMERA_DRIFT,
            CameraKind::Pursuit => BB_CAMERA_PURSUIT,
        }
    }
}

/// The camera at one moment, in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    /// The view it shows.
    pub kind: CameraKind,
    /// Where the camera is, metres.
    pub position: [f32; 3],
    /// The direction it looks in, unit length.
    pub forward: [f32; 3],
    /// Vertical field of view, degrees.
    pub fov: f32,
}

/// The `bb.camera` interface of the host, as a mod uses it. Obtained from `Ctx::camera`.
#[derive(Clone, Copy)]
pub struct Camera {
    fns: CameraFns,
}

impl Camera {
    pub(crate) fn new(fns: CameraFns) -> Self {
        Camera { fns }
    }

    /// The camera now.
    pub fn get(&self) -> ModResult<CameraState> {
        let raw = host_camera::get(&self.fns)?;
        let kind = CameraKind::from_raw(raw.mode).ok_or(Error::Host)?;
        Ok(CameraState { kind, position: raw.position.to_array(), forward: raw.forward.to_array(), fov: raw.fov })
    }

    /// Switch to the view `kind`. The change is made after the mods run.
    pub fn set_mode(&self, kind: CameraKind) -> ModResult<()> {
        host_camera::set_mode(&self.fns, kind.raw())
    }
}
