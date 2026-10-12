//! Which camera a world frame is drawn from, as the camera setting picks it.

use glam::Vec3;

use crate::settings::CameraMode;
use crate::viewer::camera::original::{self, OriginalView};
use crate::viewer::camera::{Camera, ChaseView, DynamicFov, Lens};

use super::drive::{CarPose, Drive};
use super::{View, WorldScene};

impl WorldScene {
    /// The lens for this frame: an original view on the car, the chase camera (or its dynamic-FOV variant), or the
    /// free camera when there is no car to follow.
    pub(super) fn gameplay_lens(&self, aspect: f32) -> Lens {
        let Some(drive) = self.drive.as_ref().filter(|_| self.view == View::Chase) else {
            return self.camera.lens(aspect);
        };
        self.car_lens(drive, drive.pose(), drive.camera(), aspect)
    }

    /// The selected view at the current physics tick, independent of render interpolation and shake.
    pub(super) fn simulation_lens(&self, aspect: f32) -> Lens {
        let Some(drive) = self.drive.as_ref().filter(|_| self.view == View::Chase) else {
            return self.camera.lens(aspect);
        };
        self.car_lens(drive, drive.physics_pose(), drive.physics_camera(), aspect)
    }

    fn car_lens(&self, drive: &Drive, pose: CarPose, chase: ChaseView, aspect: f32) -> Lens {
        if let Some(lens) = self.original_lens(pose, drive.speed(), drive.half_extents().1, aspect) {
            return lens;
        }
        // The high-speed effects widen the view on top of the chase camera's own field of view (nitrous, for now).
        let kick = self.traffic.as_ref().map_or(0.0, |traffic| traffic.fov_kick());
        if self.camera_mode == CameraMode::DynamicFov {
            return DynamicFov { chase: &chase, speed: drive.speed().abs(), kick }.lens(aspect);
        }
        chase.lens_with_fov(aspect, chase.fov_degrees() + kick)
    }

    /// The original view the setting picks, if it picks one and the install's views are loaded.
    fn original_lens(&self, pose: CarPose, speed: f32, half_length: f32, aspect: f32) -> Option<Lens> {
        let index = original::index_of(self.camera_mode)?;
        let views = self.camera_views.as_ref()?;
        let forward = pose.rotation * Vec3::X;
        let heading = forward.y.atan2(forward.x);
        let view = OriginalView::new(&views.views[index], pose.position, heading, speed, half_length);
        debug_assert!(view.selectable(), "the camera setting picked a camera it may not choose");
        Some(view.lens(aspect))
    }
}
