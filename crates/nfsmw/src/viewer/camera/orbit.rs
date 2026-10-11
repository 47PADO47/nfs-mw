//! Orbit camera for model viewing: left-drag rotates, scroll zooms.

use glam::Vec3;

use super::{Camera, Lens, direction};
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
}

impl Camera for OrbitCamera {
    fn lens(&self, aspect: f32) -> Lens {
        Lens::look_at(self.eye(), self.target, Self::FOV_Y_DEGREES, self.near(), aspect)
    }
}
