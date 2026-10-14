//! Which camera a world frame is drawn from, as the camera setting picks it.

use glam::{Mat4, Vec3};

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
        let zoom = drive.speedbreaker_zoom();
        if let Some(lens) = self.original_lens(pose, drive.speed(), drive.half_extents().1, zoom, aspect) {
            return lens;
        }
        // The high-speed effects widen the view on top of the chase camera's own field of view (nitrous, for now).
        let kick = self.traffic.as_ref().map_or(0.0, |traffic| traffic.fov_kick());
        if self.camera_mode == CameraMode::DynamicFov {
            return breaker_lens(
                DynamicFov { chase: &chase, speed: drive.speed().abs(), kick }.lens(aspect),
                pose.position,
                zoom,
                aspect,
            );
        }
        breaker_lens(chase.lens_with_fov(aspect, chase.fov_degrees() + kick), pose.position, zoom, aspect)
    }

    /// The original view the setting picks, if it picks one and the install's views are loaded.
    fn original_lens(&self, pose: CarPose, speed: f32, half_length: f32, zoom: f32, aspect: f32) -> Option<Lens> {
        let index = original::index_of(self.camera_mode)?;
        let views = self.camera_views.as_ref()?;
        let forward = pose.rotation * Vec3::X;
        let heading = forward.y.atan2(forward.x);
        let view = OriginalView::with_zoom(&views.views[index], pose.position, heading, speed, half_length, zoom);
        debug_assert!(view.selectable(), "the camera setting picked a camera it may not choose");
        Some(view.lens(aspect))
    }
}

fn breaker_lens(mut lens: Lens, car: Vec3, zoom: f32, aspect: f32) -> Lens {
    // The native POV scales spatial lag, not height. This adapter applies the
    // same target change to the custom camera's existing horizontal offset.
    let offset = (lens.position - car) * Vec3::new(1.0, 1.0, 0.0);
    let displacement = offset * (zoom.recip() - 1.0);
    lens.position += displacement;
    lens.view *= Mat4::from_translation(-displacement);
    lens.fov_y_degrees = (lens.fov_y_degrees * zoom).clamp(1.0, 179.0);
    lens.projection = crate::viewer::camera::projection(lens.fov_y_degrees, aspect, 0.3);
    lens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaker_zoom_pairs_fov_with_spatial_lag_and_preserves_height_and_direction() {
        for fov in [45.0, 60.0, 70.0, 90.0] {
            let lens = Lens::look_at(Vec3::new(-5.0, 0.0, 2.0), Vec3::ZERO, fov, 0.3, 16.0 / 9.0);
            let held = breaker_lens(lens, Vec3::ZERO, 0.6, 16.0 / 9.0);
            assert_eq!(held.fov_y_degrees, fov * 0.6);
            assert!((held.position.x - lens.position.x / 0.6).abs() < 1e-5);
            assert_eq!(held.position.z, lens.position.z);
            assert!((held.view.inverse().w_axis.truncate() - held.position).length() < 1e-5);
            assert_eq!(held.forward, lens.forward);
            assert!(held.view_proj().is_finite());
            assert_ne!(held.projection.matrix(), lens.projection.matrix());
        }
    }
}
