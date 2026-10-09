//! Orbit camera for model viewing: left-drag rotates, scroll zooms.

use blackbox_gfx::Projection;
use glam::{Mat4, Vec3};

use super::{direction, projection, view};
use crate::input::{Action, ActionState};

pub struct OrbitCamera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl OrbitCamera {
    pub fn update(&mut self, input: &ActionState) {
        self.yaw -= input.value(Action::OrbitX) * 0.008;
        self.pitch = (self.pitch + input.value(Action::OrbitY) * 0.008).clamp(-1.5, 1.5);
        self.distance = (self.distance * 0.9f32.powf(input.value(Action::Zoom))).max(0.1);
    }

    pub fn eye(&self) -> Vec3 {
        self.target + direction(self.yaw, self.pitch) * self.distance
    }

    const FOV_Y_DEGREES: f32 = 55.0;

    fn near(&self) -> f32 {
        (self.distance * 0.01).max(0.01)
    }

    pub fn view(&self) -> Mat4 {
        view(self.eye(), self.target)
    }

    pub fn projection(&self, aspect: f32) -> Projection {
        projection(Self::FOV_Y_DEGREES, aspect, self.near())
    }
}
