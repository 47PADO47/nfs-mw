//! The extra camera views mods register (`bb.camera` 1.1): stepping the one the player picked and drawing from its pose.
//!
//! The view is stepped once per rendered frame, after the car has moved and before the frame's lens is read, with the
//! presented body of the lead car (so a camera mounted on the car does not lag it). A view that does not fit the car, is
//! unknown (its mod is not loaded) or declines the frame leaves the chase camera in charge.

use glam::Vec3;

use super::{View, WorldScene};
use crate::modding::camera_views;
use crate::settings::{CameraMode, ViewKey};
use crate::viewer::camera::Lens;

/// Distance, metres, the car may jump between two frames before the view is told to start over (a respawn or a teleport).
const TELEPORT_DISTANCE: f32 = 50.0;
/// The near plane of an extra view, metres: a bonnet or bumper camera sits against the car.
const NEAR: f32 = 0.1;

/// The state of the extra view between frames.
#[derive(Default)]
pub(super) struct ExtraViewState {
    /// This frame's pose, if the view answered.
    pose: Option<blackbox_modding::host::ViewPose>,
    /// The view and the car the last call was for.
    last: Option<(ViewKey, String)>,
    last_position: Option<Vec3>,
    last_forward: Option<Vec3>,
    last_step: Option<u32>,
}

impl WorldScene {
    /// The car the lead player drives, by the id the mods know.
    fn extra_view_car(&self) -> Option<String> {
        self.players.lead().map(|drive| drive.car_name.clone())
    }

    /// The extra views that fit the lead car, in registration order.
    pub(super) fn extra_view_keys(&self) -> Vec<ViewKey> {
        let Some(car) = self.extra_view_car() else { return Vec::new() };
        camera_views::available_keys(&car)
    }

    /// The view the camera setting picked, while it is registered and fits the lead car in the chase view.
    pub(super) fn active_extra_view(&self) -> Option<ViewKey> {
        let CameraMode::Extra(key) = self.camera_mode else { return None };
        if self.view != View::Chase {
            return None;
        }
        let car = self.extra_view_car()?;
        camera_views::available(key, &car).then_some(key)
    }

    /// Step the player's extra view for this frame. Call after the car has moved.
    pub(super) fn update_extra_view(&mut self, dt: f32) {
        let Some(key) = self.active_extra_view() else {
            self.extra_view = ExtraViewState::default();
            return;
        };
        let Some(car) = self.extra_view_car() else { return };
        let Some(drive) = self.players.lead() else { return };
        let Some(shown) = drive.extra_view_body(self.extra_view.last_step) else {
            self.extra_view.pose = None;
            return;
        };
        let body = shown.body;
        let previous = &self.extra_view;
        let jumped = previous.last_position.is_some_and(|last| last.distance(body.position) > TELEPORT_DISTANCE);
        let reset = previous.last.as_ref() != Some(&(key, car.clone())) || jumped;
        let dt = dt.max(0.0);
        // The yaw, pitch and roll rates from the change of the axes since the last frame.
        let angular_velocity = match (previous.last_forward, dt > 1e-4 && !reset) {
            (Some(last), true) => last.cross(body.forward.normalize_or_zero()) / dt,
            _ => Vec3::ZERO,
        };
        let input = blackbox_modding::host::ViewInput {
            reset,
            dt,
            speed: body.speed,
            steer: shown.steer,
            throttle: shown.throttle,
            brake: shown.brake,
            impact: shown.impact,
            grounded: body.grounded,
            position: body.position.to_array(),
            forward: body.forward.to_array(),
            up: body.up.to_array(),
            velocity: body.velocity.to_array(),
            angular_velocity: angular_velocity.to_array(),
            car: &car,
        };
        let pose = camera_views::update(key, &input);
        self.extra_view = ExtraViewState {
            pose,
            last: Some((key, car)),
            last_position: Some(body.position),
            last_forward: Some(body.forward.normalize_or_zero()),
            last_step: Some(shown.step),
        };
    }

    /// Whether the extra view in use asks for the player's car not to be drawn.
    pub(super) fn extra_view_hides_car(&self) -> bool {
        self.active_extra_view().is_some() && self.extra_view.pose.is_some_and(|pose| pose.hide_car)
    }

    /// The lens of the extra view, when one is active and answered this frame.
    pub(super) fn extra_view_lens(&self, aspect: f32) -> Option<Lens> {
        self.active_extra_view()?;
        let pose = self.extra_view.pose?;
        let (eye, forward, up) = (Vec3::from_array(pose.position), Vec3::from_array(pose.forward), Vec3::from_array(pose.up));
        let fov = pose.fov.clamp(20.0, 150.0);
        let mut lens = Lens::look_at(eye, eye + forward, fov, NEAR, aspect);
        lens.view = glam::camera::rh::view::look_at_mat4(eye, eye + forward, up);
        Some(lens)
    }
}
