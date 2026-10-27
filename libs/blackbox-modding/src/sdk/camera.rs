//! `Camera`: read the camera and choose a gameplay view (catalogue: docs/modding/api-world.md#camera).

use crate::abi::{
    BB_CAMERA_BUMPER, BB_CAMERA_CHASE, BB_CAMERA_CLOSE, BB_CAMERA_DRIFT, BB_CAMERA_DYNAMIC_FOV, BB_CAMERA_EXTRA,
    BB_CAMERA_FAR, BB_CAMERA_HOOD, BB_CAMERA_PURSUIT, BB_CAMERA_SUPER_FAR,
};

use super::ctx::Keep;
use super::ffi::HostTable;
use super::ffi::camera::{self as host_camera, CameraFns};
use super::{CameraViewId, Error, ModResult};

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
    /// One of the extra views mods registered ([`Camera::register_view`]). Reported by [`Camera::get`]; to switch to one,
    /// use [`Camera::select_view`].
    Extra,
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
            BB_CAMERA_EXTRA => CameraKind::Extra,
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
            CameraKind::Extra => BB_CAMERA_EXTRA,
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

/// The car and the frame an extra view is stepped with, in render space (x forward, y left, z up; metres, seconds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraInput<'a> {
    /// The view was just selected or the car was moved: place the camera at its rest pose and drop smoothing history.
    pub reset: bool,
    /// Seconds since the last call.
    pub dt: f32,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    /// -1 to 1, positive to the left.
    pub steer: f32,
    /// 0 to 1.
    pub throttle: f32,
    /// 0 to 1.
    pub brake: f32,
    /// Strength of the collision this frame, 0 to 1.
    pub impact: f32,
    /// Whether the wheels touch the ground.
    pub grounded: bool,
    /// The car's body origin, metres.
    pub position: [f32; 3],
    /// The car's forward axis, unit length.
    pub forward: [f32; 3],
    /// The car's up axis, unit length.
    pub up: [f32; 3],
    /// Linear velocity, m/s.
    pub velocity: [f32; 3],
    /// Angular velocity, rad/s.
    pub angular_velocity: [f32; 3],
    /// The id of the car the player drives, as the game names it.
    pub car: &'a str,
}

/// The camera an extra view answers, in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPose {
    /// Where the camera is, metres.
    pub position: [f32; 3],
    /// The direction it looks in, unit length.
    pub forward: [f32; 3],
    /// Its up axis, unit length; the picture rolls with it.
    pub up: [f32; 3],
    /// Vertical field of view, degrees.
    pub fov: f32,
    /// Do not draw the player's car (a camera inside or against it).
    pub hide_car: bool,
}

/// An extra camera view a mod computes itself. The host calls it from the render loop, on the main thread, never from inside
/// another callback of the same mod.
pub trait CameraView {
    /// Whether the view fits the car `car` (its id). Asked when the player cycles the camera and when the car changes; a
    /// view that answers no is skipped. The default is yes.
    fn available(&mut self, _car: &str) -> bool {
        true
    }

    /// The pose for this frame, or `None` to keep the host's chase camera for the frame. Called only while the view is the
    /// player's view, once per rendered frame, after the car has moved.
    fn update(&mut self, input: &CameraInput<'_>) -> Option<CameraPose>;
}

/// The `bb.camera` interface of the host, as a mod uses it. Obtained from `Ctx::camera`.
#[derive(Clone)]
pub struct Camera {
    fns: CameraFns,
    host: HostTable,
    keep: Keep,
}

impl Camera {
    pub(crate) fn new(fns: CameraFns, host: HostTable, keep: Keep) -> Self {
        Camera { fns, host, keep }
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

    /// Add an extra view the player can choose with the camera hotkey and in the options (needs a host with `bb.camera`
    /// 1.1; older hosts answer `Error::Unsupported`). `id` is plain (`a-z`, `0-9`, `_`, `.`, `-`) and unique among this
    /// mod's views; the game saves the choice as `<mod id>/<id>`. `name` is the text the camera setting shows, or a message
    /// id of the mod's catalogue.
    pub fn register_view<V>(&self, id: &str, name: &str, view: V) -> ModResult<CameraViewId>
    where
        V: CameraView + 'static,
    {
        host_camera::register_view(&self.fns, self.host, &self.keep, id, name, Box::new(view))
    }

    /// Take back one extra view. If it is the player's view the game returns to the chase camera.
    pub fn unregister_view(&self, id: CameraViewId) -> ModResult<()> {
        host_camera::unregister_view(&self.fns, id)
    }

    /// Look through an extra view now. While it does not fit the player's car, the game draws its chase camera.
    pub fn select_view(&self, id: CameraViewId) -> ModResult<()> {
        host_camera::select_view(&self.fns, id)
    }
}
