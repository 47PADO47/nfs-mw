//! Original contact feedback, sampled on the car's presentation timeline.
//! Behavior and PC evidence: `docs/specs/contact-camera-shake.md`.

use glam::{Mat4, Vec3};

const ENVELOPE: [(f32, f32); 5] = [(0.0, 1.0), (0.1, 0.7), (0.3, 0.5), (0.5, 0.4), (1.0, 0.0)];
const FREQUENCY: f32 = 11.74265;

fn envelope(phase: f32) -> f32 {
    if !(0.0..1.0).contains(&phase) {
        return 0.0;
    }
    let [(a, va), (b, vb)] = ENVELOPE.windows(2).find(|p| phase < p[1].0).unwrap() else { unreachable!() };
    va + (vb - va) * ((phase - a) / (b - a))
}

#[derive(Debug, Clone, Copy)]
struct Pulse {
    start: f64,
    duration: f32,
    translation: Vec3,
}

impl Pulse {
    fn phase(self, time: f64) -> f32 {
        ((time - self.start) / f64::from(self.duration)) as f32
    }

    fn sample(self, time: f64) -> Vec3 {
        let phase = self.phase(time);
        if ((phase * FREQUENCY) as i32 & 1) != 0 {
            return Vec3::ZERO;
        }
        self.translation * envelope(phase)
    }
}

pub(in crate::viewer::camera) struct Shake {
    time: f64,
    step: f64,
    pending: Vec3,
    current: Option<Pulse>,
    previous: Option<Pulse>,
    /// The player's shake multiplier (`[custom_hud] camera_shake`): 1 keeps the original amplitude, 0 removes the shake.
    intensity: f32,
}

impl Default for Shake {
    fn default() -> Self {
        Self { time: 0.0, step: 0.0, pending: Vec3::ZERO, current: None, previous: None, intensity: 1.0 }
    }
}

impl Shake {
    /// Scales the shake's amplitude by `intensity`, 0 to 1. A value that is not a number keeps the original amplitude.
    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = scaled_intensity(intensity);
    }

    /// Drops every pulse, as after a teleport, keeping the intensity.
    pub fn clear(&mut self) {
        *self = Self { intensity: self.intensity, ..Self::default() };
    }

    pub fn request(&mut self, acceleration: Vec3) {
        let biased = acceleration * Vec3::new(0.7, 0.7, 1.0);
        if biased.is_finite() && biased.length_squared() > self.pending.length_squared() {
            self.pending = biased;
        }
    }

    pub fn advance(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.step = f64::from(dt);
        self.time += self.step;
        self.select_pending();
    }

    /// A later native camera source selects at the same clock instant.
    pub fn select(&mut self, acceleration: Vec3) {
        self.request(acceleration);
        self.select_pending();
    }

    fn select_pending(&mut self) {
        let acceleration = std::mem::take(&mut self.pending);
        let length = acceleration.length();
        if length < 100.0 {
            return;
        }
        let ratio = (length.min(600.0) - 100.0) * 0.002;
        let magnitude = 0.1 + 0.3 * ratio;
        let amplitude = self.current.map_or(0.0, |p| envelope(p.phase(self.time)));
        if magnitude <= amplitude {
            return;
        }
        self.previous = self.current;
        self.current = Some(Pulse {
            start: self.time,
            duration: 0.3 + 0.4 * ratio,
            translation: acceleration * (-magnitude / length),
        });
    }

    pub fn sample(&self, alpha: f32) -> Vec3 {
        let time = self.time - self.step * f64::from(1.0 - alpha.clamp(0.0, 1.0));
        let pulse = self.current.filter(|p| p.start <= time).or(self.previous);
        pulse.map_or(Vec3::ZERO, |p| p.sample(time)) * self.intensity
    }
}

/// The shake multiplier the camera uses for a setting: clamped to 0 to 1, and 1 for a value that is not a number.
pub(in crate::viewer::camera) fn scaled_intensity(intensity: f32) -> f32 {
    match intensity.is_finite() {
        true => intensity.clamp(0.0, 1.0),
        false => 1.0,
    }
}

/// The original applies translation and quantised X/Y rotations to the view matrix.
pub(in crate::viewer::camera) fn apply(view: Mat4, translation: Vec3) -> Mat4 {
    let local = view.transform_vector3(translation);
    let radians = std::f32::consts::TAU / 65536.0;
    let x = (-1200.0 * (local.z + local.y)) as i32;
    let y = (2000.0 * local.x) as i32;
    Mat4::from_rotation_y(y as f32 * radians)
        * Mat4::from_rotation_x(x as f32 * radians)
        * Mat4::from_translation(local)
        * view
}

#[cfg(test)]
mod tests;
