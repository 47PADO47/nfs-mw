//! What a renderer is given to draw one frame.
//!
//! Not final: the interface plan adds `view`, `projection`, `fog`, `camera_cut` to [`FrameParams`] and
//! a stable key to [`Instance`] (needed by temporal methods and by retained-mode backends). They
//! arrive with the caller migration; today's shapes are kept so callers compile unchanged.

use glam::{Mat4, Vec3};

use crate::MeshHandle;

/// What happened to a frame handed to `render`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStatus {
    /// The frame was drawn and presented.
    Presented,
    /// Nothing was drawn: the window is minimised or hidden, or the surface had to be reconfigured.
    Skipped,
}

/// One placed copy of a mesh.
#[derive(Debug, Clone, Copy)]
pub struct Instance {
    pub mesh: MeshHandle,
    /// Object-to-world transform.
    pub transform: Mat4,
}

/// Per-frame scene parameters.
#[derive(Debug, Clone, Copy)]
pub struct FrameParams {
    /// World to clip space with **reverse Z** (depth 1 at the near plane, 0 at the
    /// far plane or infinity), e.g. `glam::camera::rh::proj::directx::perspective_infinite_reverse`.
    pub view_proj: Mat4,
    pub camera_position: Vec3,
    /// Direction the light travels (world space).
    pub light_dir: Vec3,
    pub clear_color: [f32; 3],
    /// Linear fog between these distances from the camera, towards `clear_color`.
    /// Use `f32::MAX` for no fog.
    pub fog_start: f32,
    pub fog_end: f32,
}
