//! Orbit camera for model viewing: left-drag rotates, scroll zooms.

use glam::{Mat4, Vec3};

use super::{direction, view_proj};
use crate::viewer::Input;

pub struct OrbitCamera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl OrbitCamera {
    pub fn update(&mut self, input: &Input) {
        if input.left_button() || input.right_button() {
            let (dx, dy) = input.mouse_delta();
            self.yaw -= dx * 0.008;
            self.pitch = (self.pitch + dy * 0.008).clamp(-1.5, 1.5);
        }
        self.distance = (self.distance * 0.9f32.powf(input.scroll())).max(0.1);
    }

    pub fn eye(&self) -> Vec3 {
        self.target + direction(self.yaw, self.pitch) * self.distance
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let near = (self.distance * 0.01).max(0.01);
        view_proj(self.eye(), self.target, 55.0, aspect, near, self.distance * 10.0)
    }
}
