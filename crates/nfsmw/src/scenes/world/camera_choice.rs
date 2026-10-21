//! Which camera a world frame is drawn from, as the camera setting picks it.

use crate::settings::CameraMode;
use crate::viewer::camera::{Camera, ChaseView, DynamicFov, Lens};

use super::drive::Drive;
use super::{View, WorldScene};

impl WorldScene {
    /// The lens for this frame: an original view on the car, the chase camera (or its dynamic-FOV variant), or the
    /// free camera when there is no car to follow.
    pub(super) fn gameplay_lens(&self, aspect: f32) -> Lens {
        if let Some(lens) = self.moment.lens(aspect) {
            return lens;
        }
        let Some(drive) = self.drive.as_ref().filter(|_| self.view == View::Chase) else {
            return self.camera.lens(aspect);
        };
        let lens = drive
            .original_body(true)
            .and_then(|body| self.original_camera.lens(body, aspect, true))
            .unwrap_or_else(|| self.car_lens(drive, drive.camera(), aspect));
        match crate::viewer::camera::original::index_of(self.camera_mode) {
            None => self.constrain_custom_camera(lens, aspect, true),
            Some(_) => lens,
        }
    }

    /// The selected retained shot on the physical body, independent of render interpolation.
    pub(super) fn simulation_lens(&self, aspect: f32) -> Lens {
        if let Some(lens) = self.moment.lens(aspect) {
            return lens;
        }
        let Some(drive) = self.drive.as_ref().filter(|_| self.view == View::Chase) else {
            return self.camera.lens(aspect);
        };
        let lens = drive
            .original_body(false)
            .and_then(|body| self.original_camera.lens(body, aspect, false))
            .unwrap_or_else(|| self.car_lens(drive, drive.physics_camera(), aspect));
        match crate::viewer::camera::original::index_of(self.camera_mode) {
            None => self.constrain_custom_camera(lens, aspect, false),
            Some(_) => lens,
        }
    }

    pub(super) fn change_camera_mode(&mut self, mode: CameraMode) {
        if self.camera_mode != mode {
            let old = crate::viewer::camera::original::index_of(self.camera_mode);
            let new = crate::viewer::camera::original::index_of(mode);
            if old.is_none() || new.is_none() {
                self.original_camera.reset();
                self.original_player_step = None;
            }
            if !matches!((old, new), (Some(2..=6), Some(2..=6))) {
                self.cut.arm();
            }
        }
        self.camera_mode = mode;
    }

    /// Raw custom intent. Speedbreaker optics belong to the real-time camera timeline.
    pub(super) fn car_lens(&self, drive: &Drive, chase: ChaseView, aspect: f32) -> Lens {
        debug_assert!(chase.selectable());
        let kick = self.traffic.as_ref().map_or(0.0, |traffic| traffic.fov_kick());
        if self.camera_mode == CameraMode::DynamicFov {
            return DynamicFov { chase: &chase, speed: drive.speed().abs(), kick }.lens(aspect);
        }
        chase.lens_with_fov(aspect, chase.fov_degrees() + kick)
    }
}
