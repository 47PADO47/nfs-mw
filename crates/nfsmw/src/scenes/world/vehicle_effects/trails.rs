use std::collections::VecDeque;

use blackbox_gfx::EffectVertex;
use glam::Vec3;
use nfsmw_data::vehicle_effects::EmitterStyle;

use super::super::drive::CarPose;
use super::geometry;
use super::particle::{Particle, count};
use crate::settings::XenonEffects;

pub(super) struct Trails {
    particles: VecDeque<Particle>,
    carry: f32,
    pub emitted: u64,
}

impl Default for Trails {
    fn default() -> Self {
        Self { particles: VecDeque::new(), carry: 0.0, emitted: 0 }
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
    /// Drops the oldest particles until at most `cap` are left.
    pub fn truncate(&mut self, cap: usize) {
        while self.particles.len() > cap {
            self.particles.pop_front();
        }
    }

    pub fn age(&mut self, dt: f32) {
        self.particles.iter_mut().for_each(|p| p.age(dt));
        self.particles.retain(Particle::alive);
    }

    pub fn emit(
        &mut self,
        style: Option<EmitterStyle>,
        pose: CarPose,
        velocity: Vec3,
        nitrous: bool,
        tuning: &XenonEffects,
        dt: f32,
    ) {
        let Some(style) = style else {
            self.carry = 0.0;
            return;
        };
        let Some(intensity) = tuning.trail_strength(velocity.length(), nitrous) else {
            self.carry = 0.0;
            return;
        };
        self.carry = (self.carry + tuning.contrail_rate * dt).min(4.0);
        let dispatches = self.carry.floor() as usize;
        self.carry -= dispatches as f32;
        let count = count(style, intensity) * dispatches;
        for i in 0..count {
            let birth_age = dt.min(4.0 / tuning.contrail_rate) * i as f32 / count as f32;
            let mut p = Particle::spawn(
                style,
                pose.position - velocity * birth_age,
                pose.rotation,
                velocity,
                self.emitted,
                intensity,
            );
            p.age(birth_age);
            while self.particles.len() >= tuning.max_trails {
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
