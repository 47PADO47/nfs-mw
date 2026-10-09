use std::collections::VecDeque;

use blackbox_render::EffectVertex;
use glam::Vec3;
use nfsmw_data::vehicle_effects::EmitterStyle;

use super::super::drive::CarPose;
use super::particle::{Particle, count};
use super::{MAX_TRAILS, geometry};

pub(super) const ACTIVATION_SPEED: f32 = 44.0;
const DISPATCH_RATE: f32 = 60.0;

pub(super) struct Trails {
    particles: VecDeque<Particle>,
    carry: f32,
    pub emitted: u64,
}

impl Default for Trails {
    fn default() -> Self {
        Self { particles: VecDeque::with_capacity(MAX_TRAILS), carry: 0.0, emitted: 0 }
    }
}

impl Trails {
    pub fn len(&self) -> usize {
        self.particles.len()
    }
    pub fn clear(&mut self) {
        self.particles.clear();
        self.carry = 0.0;
    }

    pub fn age(&mut self, dt: f32) {
        self.particles.iter_mut().for_each(|p| p.age(dt));
        self.particles.retain(Particle::alive);
    }

    pub fn emit(&mut self, style: Option<EmitterStyle>, pose: CarPose, velocity: Vec3, dt: f32) {
        let speed = velocity.length();
        let Some(style) = style.filter(|_| speed >= ACTIVATION_SPEED) else {
            self.carry = 0.0;
            return;
        };
        let intensity = (0.1 + (speed / ACTIVATION_SPEED - 1.0) * 0.65).clamp(0.1, 0.75);
        self.carry = (self.carry + DISPATCH_RATE * dt).min(4.0);
        let dispatches = self.carry.floor() as usize;
        self.carry -= dispatches as f32;
        let count = count(style, intensity) * dispatches;
        for i in 0..count {
            let birth_age = dt.min(4.0 / DISPATCH_RATE) * i as f32 / count as f32;
            let mut p = Particle::spawn(
                style,
                pose.position - velocity * birth_age,
                pose.rotation,
                velocity,
                self.emitted,
                intensity,
            );
            p.age(birth_age);
            if self.particles.len() == MAX_TRAILS {
                self.particles.pop_front();
            }
            self.particles.push_back(p);
            self.emitted += 1;
        }
    }

    pub fn geometry(&self, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        for p in &self.particles {
            geometry::streak(
                out,
                p.position(p.age + p.duration),
                p.position(p.age),
                p.width * 0.5,
                geometry::color(p.color, 1.0),
                camera,
                forward,
            );
        }
    }
}
