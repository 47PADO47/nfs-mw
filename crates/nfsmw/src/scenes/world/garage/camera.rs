//! Inspection camera for a normal world shop; driving camera settings remain untouched.
use super::WorldScene;
use crate::input::{Action, ActionState};
use crate::viewer::camera::{Camera, Lens, OrbitCamera};

impl WorldScene {
    pub(in crate::scenes::world) fn update_garage_camera(&mut self, input: &ActionState, dt: f32) {
        let Some(session) = self.career.garage.as_mut() else { return };
        let turn = f32::from(u8::from(input.pressed(Action::MenuButton3)))
            - f32::from(u8::from(input.pressed(Action::MenuButton2)));
        session.yaw += turn * dt * 1.6;
    }

    pub(in crate::scenes::world) fn garage_lens(&self, aspect: f32) -> Option<Lens> {
        let session = self.career.garage.as_ref()?;
        let drive = self.players.lead()?;
        let camera = OrbitCamera {
            target: drive.position() + glam::Vec3::Z * 0.65,
            distance: 6.5,
            yaw: drive.heading() + session.yaw,
            pitch: 0.22,
        };
        let mut lens = camera.lens(aspect);
        let half_width = camera.distance * (lens.fov_y_degrees.to_radians() * 0.5).tan() * aspect;
        lens.view = glam::Mat4::from_translation(glam::Vec3::X * 0.35 * half_width) * lens.view;
        Some(lens)
    }
}
