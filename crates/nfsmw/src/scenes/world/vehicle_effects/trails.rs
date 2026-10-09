use std::collections::VecDeque;

use blackbox_render::EffectVertex;
use glam::Vec3;
use nfsmw_data::vehicle_effects::EmitterStyle;

use super::super::drive::CarPose;
use super::{MAX_TRAILS, geometry};

pub(super) const ACTIVATION_SPEED: f32 = 44.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Particle {
    point: Vec3,
    velocity: Vec3,
    axis: Vec3,
    length: f32,
    age: f32,
    life: f32,
    color: [f32; 4],
}

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
        for p in &mut self.particles {
            p.age += dt;
            p.point += p.velocity * dt;
        }
        self.particles.retain(|p| p.age < p.life && p.point.is_finite());
    }

    pub fn emit(
        &mut self,
        style: Option<EmitterStyle>,
        pose: CarPose,
        velocity: Vec3,
        rear: Vec3,
        half: Vec3,
        dt: f32,
    ) {
        let forward = pose.rotation * Vec3::X;
        let speed = velocity.dot(forward);
        let Some(style) = style.filter(|_| speed >= ACTIVATION_SPEED) else {
            self.carry = 0.0;
            return;
        };
        let intensity = (0.1 + (speed / ACTIVATION_SPEED - 1.0) * 0.65).clamp(0.1, 0.75);
        self.carry = (self.carry + intensity * 240.0 * dt).min(24.0);
        let count = self.carry.floor() as usize;
        self.carry -= count as f32;
        for _ in 0..count {
            let n = self.emitted;
            let local = rear
                + Vec3::new(
                    geometry::noise(n, 0.7) * 0.25,
                    geometry::noise(n, 1.7) * half.y * 1.4,
                    geometry::noise(n, 2.7).abs() * half.z * 1.8,
                );
            if self.particles.len() == MAX_TRAILS {
                self.particles.pop_front();
            }
            self.particles.push_back(Particle {
                point: pose.position + pose.rotation * local,
                velocity: velocity * 0.25,
                axis: forward,
                length: (speed * 0.045).clamp(1.5, 4.0),
                age: 0.0,
                life: (style.life + geometry::noise(n, 3.3) * style.life_variance).clamp(0.08, 0.5),
                color: style.color,
            });
            self.emitted += 1;
        }
    }

    pub fn geometry(&self, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        for p in &self.particles {
            let fade = (1.0 - p.age / p.life).powi(2);
            geometry::streak(
                out,
                p.point,
                p.point - p.axis * p.length,
                0.024,
                geometry::color(p.color, fade),
                camera,
                forward,
            );
        }
    }
}
