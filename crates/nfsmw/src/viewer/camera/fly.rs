//! Free-fly camera: WASD to move, Space/C (or E/Q) up and down, Shift faster,
//! hold the right mouse button to look around, scroll to change speed.

use glam::{Mat4, Vec3};
use winit::keyboard::KeyCode;

use super::{direction, view_proj};
use crate::viewer::Input;

pub struct FlyCamera {
    pub position: Vec3,
    /// Heading in radians from +X.
    pub yaw: f32,
    pub pitch: f32,
    /// Metres per second.
    pub speed: f32,
    pub far: f32,
}

impl FlyCamera {
    pub fn forward(&self) -> Vec3 {
        direction(self.yaw, self.pitch)
    }

    pub fn update(&mut self, input: &Input, dt: f32) {
        if input.right_button() {
            let (dx, dy) = input.mouse_delta();
            self.yaw -= dx * 0.003;
            self.pitch = (self.pitch - dy * 0.003).clamp(-1.55, 1.55);
        }
        self.speed = (self.speed * 1.2f32.powf(input.scroll())).clamp(1.0, 2000.0);

        let forward = self.forward();
        let right = forward.cross(Vec3::Z).normalize_or_zero();
        let axis =
            |pos: KeyCode, neg: KeyCode| f32::from(u8::from(input.key(pos))) - f32::from(u8::from(input.key(neg)));
        let mut motion = forward * axis(KeyCode::KeyW, KeyCode::KeyS) + right * axis(KeyCode::KeyD, KeyCode::KeyA);
        motion.z += axis(KeyCode::Space, KeyCode::KeyC) + axis(KeyCode::KeyE, KeyCode::KeyQ);
        let boost = if input.key(KeyCode::ShiftLeft) || input.key(KeyCode::ShiftRight) { 5.0 } else { 1.0 };
        self.position += motion.normalize_or_zero() * self.speed * boost * dt;
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        view_proj(self.position, self.position + self.forward(), 70.0, aspect, 0.5, self.far)
    }
}
