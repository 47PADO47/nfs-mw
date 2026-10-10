//! What a renderer is given to draw one frame.

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

/// A stable name for one placed object, the same from frame to frame while the object exists.
///
/// A renderer that keeps objects between frames (a retained-mode engine, or a temporal method that
/// needs last frame's transform for motion vectors) matches instances to its own entities with it.
/// The immediate-mode native renderer ignores it. Callers pick any scheme that is unique among the
/// instances of one frame, for example [`InstanceKey::new`] with a tile and an index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceKey(pub u64);

impl InstanceKey {
    /// For an instance that lives one frame or has no identity worth keeping. Several instances
    /// may share it; a renderer treats each as new every frame.
    pub const TRANSIENT: Self = Self(u64::MAX);

    /// A key from a group (a tile, a car) and an index inside it (an instance, a part).
    /// `(u32::MAX, u32::MAX)` is reserved for [`TRANSIENT`](Self::TRANSIENT).
    pub const fn new(group: u32, index: u32) -> Self {
        Self(((group as u64) << 32) | index as u64)
    }

    /// The group and the index [`new`](Self::new) was given.
    pub const fn parts(self) -> (u32, u32) {
        ((self.0 >> 32) as u32, self.0 as u32)
    }

    pub const fn is_transient(self) -> bool {
        self.0 == Self::TRANSIENT.0
    }
}

/// One placed copy of a mesh.
#[derive(Debug, Clone, Copy)]
pub struct Instance {
    pub mesh: MeshHandle,
    /// Object-to-world transform.
    pub transform: Mat4,
    /// Which object this is, from frame to frame.
    pub key: InstanceKey,
}

impl Instance {
    /// An instance with no identity ([`InstanceKey::TRANSIENT`]).
    pub const fn new(mesh: MeshHandle, transform: Mat4) -> Self {
        Self { mesh, transform, key: InstanceKey::TRANSIENT }
    }

    /// An instance that keeps `key` from frame to frame.
    pub const fn keyed(mesh: MeshHandle, transform: Mat4, key: InstanceKey) -> Self {
        Self { mesh, transform, key }
    }
}

/// How view space maps to clip space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// A perspective with no far plane and **reverse Z** (depth 1 at the near plane, 0 at infinity),
    /// for wgpu's clip space (Y up).
    PerspectiveInfiniteReverse {
        /// Vertical field of view in radians.
        fov_y: f32,
        /// Width over height.
        aspect: f32,
        /// Distance of the near plane.
        near: f32,
    },
    /// No projection: positions are already in clip space (2D frames, with the view the identity).
    Identity,
}

impl Projection {
    /// View space to clip space.
    pub fn matrix(&self) -> Mat4 {
        match *self {
            Self::PerspectiveInfiniteReverse { fov_y, aspect, near } => {
                glam::camera::rh::proj::directx::perspective_infinite_reverse(fov_y, aspect, near)
            }
            Self::Identity => Mat4::IDENTITY,
        }
    }
}

/// Linear fog between two distances from the camera, towards the frame's clear colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fog {
    /// Distance where the fog begins.
    pub start: f32,
    /// Distance where the fog is complete.
    pub end: f32,
}

/// Per-frame scene parameters.
///
/// The matrices are the camera's own, never jittered: a renderer that jitters for temporal methods
/// does it itself, and scene culling keeps using these.
#[derive(Debug, Clone, Copy)]
pub struct FrameParams {
    /// World to view space.
    pub view: Mat4,
    pub projection: Projection,
    pub camera_position: Vec3,
    /// Direction the light travels (world space).
    pub light_dir: Vec3,
    pub clear_color: [f32; 3],
    /// Fog towards `clear_color`, or `None` for no fog.
    pub fog: Option<Fog>,
    /// The camera jumped (a teleport, a scene or camera switch): a renderer that reuses the last
    /// frame (temporal anti-aliasing, temporal upscalers) drops its history.
    pub camera_cut: bool,
}

impl FrameParams {
    /// World to clip space.
    pub fn view_proj(&self) -> Mat4 {
        self.projection.matrix() * self.view
    }

    /// The fog distances in the form shaders read them: no fog is `f32::MAX` for both.
    pub fn fog_range(&self) -> [f32; 2] {
        match self.fog {
            Some(Fog { start, end }) => [start, end],
            None => [f32::MAX, f32::MAX],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_their_parts_and_never_collide_with_transient() {
        let key = InstanceKey::new(7, 99);
        assert_eq!(key.parts(), (7, 99));
        assert!(!key.is_transient());
        assert!(InstanceKey::TRANSIENT.is_transient());
        assert_ne!(InstanceKey::new(0, 0), InstanceKey::TRANSIENT);
        assert_ne!(InstanceKey::new(1, 0), InstanceKey::new(0, 1));
    }

    #[test]
    fn instances_without_a_key_are_transient() {
        let mesh = MeshHandle::from_raw(1);
        assert!(Instance::new(mesh, Mat4::IDENTITY).key.is_transient());
        assert_eq!(Instance::keyed(mesh, Mat4::IDENTITY, InstanceKey::new(1, 2)).key.parts(), (1, 2));
    }

    #[test]
    fn view_proj_is_the_projection_times_the_view() {
        let view = glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, -5.0, 1.0), Vec3::new(0.0, 5.0, 1.0), Vec3::Z);
        let projection = Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.5, near: 0.1 };
        let frame = FrameParams {
            view,
            projection,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0; 3],
            fog: None,
            camera_cut: false,
        };
        assert_eq!(frame.view_proj(), projection.matrix() * view);
        let ndc = frame.view_proj() * Vec3::new(0.0, -4.0, 1.0).extend(1.0);
        assert!((ndc.x / ndc.w).abs() < 1e-5 && ndc.z > 0.0, "a point ahead is on the centre line in front");
    }

    #[test]
    fn the_identity_projection_leaves_the_view_alone() {
        let frame = FrameParams {
            view: Mat4::IDENTITY,
            projection: Projection::Identity,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0; 3],
            fog: None,
            camera_cut: false,
        };
        assert_eq!(frame.view_proj(), Mat4::IDENTITY);
    }

    #[test]
    fn no_fog_is_the_far_away_range() {
        let mut frame = FrameParams {
            view: Mat4::IDENTITY,
            projection: Projection::Identity,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0; 3],
            fog: None,
            camera_cut: false,
        };
        assert_eq!(frame.fog_range(), [f32::MAX; 2]);
        frame.fog = Some(Fog { start: 5.0, end: 9.0 });
        assert_eq!(frame.fog_range(), [5.0, 9.0]);
    }
}
