//! Local offset pulses; see the original-camera-transients behavioral spec.

use glam::Vec3;

#[derive(Debug, Clone)]
pub(super) struct Transients {
    collision_peak: f32,
    collision_remaining: f32,
    previous_gear: Option<i32>,
    gear_elapsed: f32,
}

impl Default for Transients {
    fn default() -> Self {
        Self { collision_peak: 0.0, collision_remaining: 0.0, previous_gear: None, gear_elapsed: 1.5 }
    }
}

impl Transients {
    pub fn apply(&mut self, outside: bool, mut eye: Vec3, mut target: Vec3, input: Input) -> (Vec3, Vec3) {
        self.gear_elapsed += input.dt;
        let changed = self.previous_gear.is_some_and(|old| old != input.gear);
        self.previous_gear = Some(input.gear);
        if input.mapped_acceleration.x.abs().max(input.mapped_acceleration.y.abs()) >= 24.0
            && input.collision_damping > self.collision_peak
        {
            self.collision_peak = input.collision_damping;
            self.collision_remaining = 1.0;
        }
        if self.collision_peak > 0.0 && self.collision_remaining > 0.0 {
            let amount =
                linear(&[0.0, 0.02, 0.25, 0.99, 0.0], self.collision_remaining * 4.0) * self.collision_peak * 0.25;
            eye.x *= 1.0 - amount;
            target.x *= 1.0 + amount;
        }
        if self.collision_remaining > 0.0 {
            self.collision_remaining -= input.dt;
        }
        if self.collision_remaining <= 0.0 {
            self.collision_peak = 0.0;
        }
        if !outside {
            return (eye, target);
        }
        if self.gear_elapsed >= 1.5 {
            if changed && input.speed > 10.0 {
                self.gear_elapsed = 0.0;
            }
            return (eye, target);
        }
        let response =
            linear(&[0.0, 0.2, 0.13, -0.08, -0.2, -0.13, -0.07, -0.03, 0.0], self.gear_elapsed * (8.0 / 1.5));
        eye.x += (1.0 - (input.speed * 0.01).clamp(0.0, 1.0)) * eye.x.abs() * 0.2 * response;
        (eye, target)
    }
}

pub(super) struct Input {
    pub mapped_acceleration: Vec3,
    pub collision_damping: f32,
    pub speed: f32,
    pub gear: i32,
    pub dt: f32,
}

fn linear(values: &[f32], position: f32) -> f32 {
    let position = position.clamp(0.0, (values.len() - 1) as f32);
    let lower = (position as usize).min(values.len() - 2);
    values[lower] + (values[lower + 1] - values[lower]) * (position - lower as f32)
}
