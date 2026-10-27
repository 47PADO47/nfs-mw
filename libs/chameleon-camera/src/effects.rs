//! Pieces shared by the camera rigs: the acceleration estimate and the impact shake.

use crate::math::{filter_weight, finite_f32, ramp};
use crate::params::ImpactShake;

/// Longitudinal acceleration estimated from the speed the host passes each frame (m/s per second, filtered).
#[derive(Debug, Clone, Default)]
pub struct AccelFilter {
    previous: f32,
    value: f32,
    ready: bool,
}

impl AccelFilter {
    /// Forget the history and start from `speed` with no acceleration.
    pub fn reset(&mut self, speed: f32) {
        self.previous = speed;
        self.value = 0.0;
        self.ready = true;
    }

    /// Fold in this frame's speed; returns the filtered acceleration (positive = speeding up).
    pub fn update(&mut self, speed: f32, dt: f32, tau: f32) -> f32 {
        if !self.ready {
            self.reset(speed);
        }
        if dt <= 0.0 {
            return self.value;
        }
        let raw = finite_f32((speed - self.previous) / dt, 0.0);
        self.previous = speed;
        self.value += (raw - self.value) * filter_weight(dt, tau);
        self.value
    }

    pub fn value(&self) -> f32 {
        self.value
    }
}

/// The shake that follows a collision: a damped oscillation per rotation axis (`[pitch, yaw, roll]`).
#[derive(Debug, Clone, Default)]
pub struct ImpactState {
    time: f32,
    strength: f32,
}

impl ImpactState {
    /// Start a shake of `strength` (0..1), unless a stronger one is still running.
    pub fn trigger(&mut self, strength: f32, params: &ImpactShake) {
        let strength = finite_f32(strength, 0.0).clamp(0.0, 1.0);
        if strength <= 0.0 || strength < self.remaining(params) {
            return;
        }
        self.time = 0.0;
        self.strength = strength;
    }

    /// The strength left now: the starting strength, faded towards the end of the duration.
    fn remaining(&self, params: &ImpactShake) -> f32 {
        self.strength * (1.0 - ramp(self.time, 0.0, params.scalars[0].max(1e-3)))
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn advance(&mut self, dt: f32, params: &ImpactShake) {
        if self.strength <= 0.0 {
            return;
        }
        self.time += dt;
        if self.time >= params.scalars[0].max(1e-3) {
            self.clear();
        }
    }

    /// Rotation offsets in degrees `[pitch, yaw, roll]` for the shake as it is now.
    pub fn rotation_deg(&self, params: &ImpactShake) -> [f32; 3] {
        let fade = 1.0 - ramp(self.time, 0.0, params.scalars[0].max(1e-3));
        std::array::from_fn(|axis| {
            let envelope = (-self.time / params.decay_s[axis].max(1e-3)).exp();
            params.amplitude_deg[axis] * self.strength * envelope * fade * (params.frequency[axis] * self.time).sin()
        })
    }

    pub fn active(&self) -> bool {
        self.strength > 0.0
    }
}
