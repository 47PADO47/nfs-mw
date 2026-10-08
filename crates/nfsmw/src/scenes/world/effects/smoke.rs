//! Bounded deterministic smoke, including time-based stationary burnout emission.

use std::collections::VecDeque;

use crate::settings::SmokeQuality;
use blackbox_render::{EffectLayer, EffectVertex};
use glam::Vec3;

use super::{Contact, MAX_HIGH_PARTICLES, MAX_PARTICLES};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Particle {
    position: Vec3,
    velocity: Vec3,
    size: f32,
    age: f32,
    life: f32,
    opacity: f32,
    seed: f32,
}

pub(super) struct Smoke {
    pub quality: SmokeQuality,
    particles: VecDeque<Particle>,
    emission: [f32; 4],
    pub emitted: u64,
    order: Vec<usize>,
}

impl Default for Smoke {
    fn default() -> Self {
        Self {
            quality: SmokeQuality::Standard,
            particles: VecDeque::with_capacity(MAX_PARTICLES),
            emission: [0.0; 4],
            emitted: 0,
            order: Vec::with_capacity(MAX_PARTICLES),
        }
    }
}

impl Smoke {
    pub fn set_quality(&mut self, quality: SmokeQuality) {
        if self.quality == quality {
            return;
        }
        self.quality = quality;
        self.clear();
        let limit = self.limit();
        self.particles.reserve(limit);
        self.order.reserve(limit);
    }

    pub fn limit(&self) -> usize {
        match self.quality {
            SmokeQuality::Standard => MAX_PARTICLES,
            SmokeQuality::High => MAX_HIGH_PARTICLES,
        }
    }
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn oldest(&self) -> f32 {
        self.particles.iter().map(|p| p.age).fold(0.0, f32::max)
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.disconnect();
    }

    pub fn disconnect(&mut self) {
        self.emission = [0.0; 4];
    }

    pub fn age(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.age += dt;
            p.position += p.velocity * dt;
        }
        self.particles.retain(|p| p.age < p.life && p.position.is_finite());
    }

    pub fn emit(&mut self, wheel: usize, contact: Option<Contact>, velocity: Vec3, dt: f32) {
        let Some(c) = contact.filter(|c| c.smoke > 0.0) else {
            self.emission[wheel] = 0.0;
            return;
        };
        let detailed = self.quality == SmokeQuality::High;
        let rate = if detailed { 70.0 } else { 35.0 };
        // A long frame cannot owe more puffs than the buffer holds.
        self.emission[wheel] = (self.emission[wheel] + c.smoke * rate * dt).min(self.limit() as f32);
        while self.emission[wheel] >= 1.0 {
            self.emission[wheel] -= 1.0;
            let n = self.emitted as f32;
            let vary = (n * 2.399_963).sin();
            let side = c.forward.cross(c.normal).normalize_or_zero();
            let velocity = velocity * 0.08 + side * (vary * 0.45) + Vec3::Z * (0.65 + vary.abs() * 0.35);
            if self.particles.len() == self.limit() {
                self.particles.pop_front();
            }
            self.particles.push_back(Particle {
                position: c.point + c.normal * 0.12,
                velocity,
                size: if detailed { 0.22 + vary.abs() * 0.12 } else { 0.30 + vary.abs() * 0.14 },
                age: 0.0,
                life: 1.4 + vary.abs() * 0.6,
                opacity: if detailed { 0.55 * c.smoke } else { 0.36 * c.smoke },
                seed: n * 2.399_963,
            });
            self.emitted += 1;
        }
    }

    pub fn geometry(&mut self, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        self.order.clear();
        self.order.extend(0..self.particles.len());
        self.order.sort_unstable_by(|&a, &b| {
            let distance = |i: usize| (self.particles[i].position - camera).dot(forward);
            distance(b).total_cmp(&distance(a))
        });
        let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
        let up = right.cross(forward).normalize_or(Vec3::Z);
        for &i in &self.order {
            let p = self.particles[i];
            let scale = p.size + p.age * 0.52;
            let fade_in = (p.age / 0.10).min(1.0);
            let alpha = p.opacity * fade_in * (1.0 - p.age / p.life).powi(2);
            let angle = if self.quality == SmokeQuality::High { p.seed + p.age * 0.3 } else { 0.0 };
            let (sin, cos) = angle.sin_cos();
            let (r, u) = ((right * cos + up * sin) * scale, (up * cos - right * sin) * scale);
            let corners = [p.position - r - u, p.position + r - u, p.position + r + u, p.position - r + u];
            EffectLayer::particle_quad(out, corners, [172, 174, 177, (alpha * 255.0) as u8], [p.age, p.seed]);
        }
    }
}
