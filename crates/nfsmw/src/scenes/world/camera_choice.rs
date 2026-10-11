//! Which camera a world frame is drawn from, as the camera setting picks it.

use glam::Vec3;

use crate::settings::CameraMode;
use crate::viewer::camera::original::{self, OriginalView};
use crate::viewer::camera::{Camera, DynamicFov, Lens};

use super::View;
use super::WorldScene;

impl WorldScene {
    /// The lens for this frame: an original view on the car, the chase camera (or its dynamic-FOV variant), or the
    /// free camera when there is no car to follow.
    pub(super) fn gameplay_lens(&self, aspect: f32) -> Lens {
        let Some(drive) = self.drive.as_ref().filter(|_| self.view == View::Chase) else {
            return self.camera.lens(aspect);
        };
        if let Some(lens) = self.original_lens(drive.pose(), drive.speed(), drive.half_extents().1, aspect) {
            return lens;
        }
        let chase = drive.camera();
        if self.camera_mode == CameraMode::DynamicFov {
            return DynamicFov { chase: &chase, speed: drive.speed().abs() }.lens(aspect);
        }
        chase.lens(aspect)
    }

    /// The original view the setting picks, if it picks one and the install's views are loaded.
    fn original_lens(
        &self,
        pose: crate::scenes::world::drive::CarPose,
        speed: f32,
        half_length: f32,
        aspect: f32,
    ) -> Option<Lens> {
        let index = original::index_of(self.camera_mode)?;
        let views = self.camera_views.as_ref()?;
        let forward = pose.rotation * Vec3::X;
        let heading = forward.y.atan2(forward.x);
        let view = OriginalView::new(&views.views[index], pose.position, heading, speed, half_length);
        debug_assert!(view.selectable(), "the camera setting picked a camera it may not choose");
        Some(view.lens(aspect))
    }
}
