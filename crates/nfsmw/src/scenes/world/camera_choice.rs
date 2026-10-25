//! Which camera a world frame is drawn from, as the camera setting picks it.

use crate::settings::CameraMode;
use crate::viewer::camera::{Camera, ChaseView, DynamicFov, Lens};

use super::drive::Drive;
use super::{View, ViewSource, WorldScene};

/// The aspect ratio of a player's view in a session: fixed, so no window size reaches the rules.
const SESSION_ASPECT: f32 = 16.0 / 9.0;

/// A player's view in a session: its car's chase camera on the physics body, at the fixed aspect and the camera's own
/// field of view, with no camera-mode setting, cut scene or field-of-view kick of this machine.
fn player_lens(drive: &Drive) -> Lens {
    let chase = drive.physics_camera();
    chase.lens_with_fov(SESSION_ASPECT, chase.fov_degrees())
}

impl WorldScene {
    /// The lens for this frame: an original view on the car, the chase camera (or its dynamic-FOV variant), or the
    /// free camera when there is no car to follow.
    pub(super) fn gameplay_lens(&self, aspect: f32) -> Lens {
        let _trace = blackbox_trace::scope("World: gameplay lens");
        if let Some(lens) = self.extra_lens(aspect) {
            return lens;
        }
        if let Some(lens) = self.garage_lens(aspect) {
            return lens;
        }
        if let Some(lens) = self.moment.lens(aspect) {
            return lens;
        }
        if let Some(lens) = self.nis.lens(aspect) {
            return lens;
        }
        if let Some(lens) = self.photo_lens(aspect) {
            return lens;
        }
        let Some(drive) = self.players.lead().filter(|_| self.view == View::Chase) else {
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
        let _trace = blackbox_trace::scope("World: simulation lens");
        if let Some(lens) = self.moment.lens(aspect) {
            return lens;
        }
        if let Some(lens) = self.nis.lens(aspect) {
            return lens;
        }
        if let Some(lens) = self.photo_lens(aspect) {
            return lens;
        }
        let Some(drive) = self.players.lead().filter(|_| self.view == View::Chase) else {
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

    /// The lead's view as the gameplay rules read it (`session_rules.rs`): this machine's simulation lens in single
    /// player, the lead car's own chase camera in a session.
    pub(super) fn rules_lens(&self, aspect: f32) -> Lens {
        if self.rules.views == ViewSource::LocalCamera {
            return self.simulation_lens(aspect);
        }
        let Some(drive) = self.players.lead().filter(|_| self.view == View::Chase) else {
            return self.simulation_lens(aspect);
        };
        player_lens(drive)
    }

    /// A co-op guest's view as the gameplay rules read it: the guest's camera as this machine shows it in single player,
    /// its car's own chase camera in a session.
    pub(super) fn guest_rules_lens(&self, drive: &Drive) -> Lens {
        match self.rules.views {
            ViewSource::LocalCamera => self.car_lens(drive, drive.camera(), self.physics_aspect),
            ViewSource::PlayerCars => player_lens(drive),
        }
    }

    /// The view-projections of every other player's rules view (all but the car the police follow, as the traffic sees
    /// them); empty outside co-op and sessions.
    pub(super) fn guest_view_projs(&self) -> Vec<glam::Mat4> {
        if self.view != View::Chase {
            return Vec::new();
        }
        let primary = self.chase_primary();
        let guests = self.players.iter().filter(|guest| guest.id != primary && guest.drive.has_sim());
        guests.map(|guest| self.guest_rules_lens(&guest.drive).view_proj()).collect()
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
        if self.camera_mode == CameraMode::DynamicFov && self.extra.camera.speed_zoom {
            let offset = self.extra.camera.fov - 60.0;
            let mut lens = DynamicFov { chase: &chase, speed: drive.speed().abs(), kick: kick + offset }.lens(aspect);
            lens.fov_y_degrees = lens.fov_y_degrees.clamp(20.0, 150.0);
            lens.projection = crate::viewer::camera::projection(lens.fov_y_degrees, aspect, 0.3);
            return lens;
        }
        chase.lens_with_fov(aspect, chase.fov_degrees() + kick)
    }
}
