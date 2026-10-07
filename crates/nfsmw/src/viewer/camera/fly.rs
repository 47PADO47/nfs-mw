//! Free-fly camera: WASD to move, Space/C (or E/Q) up and down, Shift faster,
//! mouse to look around (captured cursor, or hold the right button), scroll to change speed.

use glam::{Mat4, Vec3};

use super::{direction, view_proj};
use crate::input::{Action, ActionState};

pub struct FlyCamera {
    pub position: Vec3,
    /// Heading in radians from +X.
    pub yaw: f32,
    pub pitch: f32,
    /// Metres per second.
    pub speed: f32,
}

impl FlyCamera {
    pub const FOV_Y_DEGREES: f32 = 70.0;

    pub fn forward(&self) -> Vec3 {
        direction(self.yaw, self.pitch)
    }

    pub fn update(&mut self, input: &ActionState, dt: f32) {
        self.yaw -= input.value(Action::LookX) * 0.003;
        self.pitch = (self.pitch - input.value(Action::LookY) * 0.003).clamp(-1.55, 1.55);
        self.speed = (self.speed * 1.2f32.powf(input.value(Action::Zoom))).clamp(1.0, 2000.0);

        let forward = self.forward();
        let right = forward.cross(Vec3::Z).normalize_or_zero();
        let mut motion = forward * input.value(Action::MoveForward) + right * input.value(Action::MoveRight);
        motion.z += input.value(Action::MoveUp);
        let boost = if input.pressed(Action::Boost) { 5.0 } else { 1.0 };
        // A fully pushed stick or a held key is full speed; a gentle stick push is slower.
        self.position += motion.clamp_length_max(1.0) * self.speed * boost * dt;
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        view_proj(self.position, self.position + self.forward(), Self::FOV_Y_DEGREES, aspect, 0.5)
    }
}
