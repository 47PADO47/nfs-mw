//! Ordinary textured particles; implementation of `pc-collision-particles.md`.

use glam::{EulerRot, Quat, Vec3};
use nfsmw_data::vehicle_effects::PcEmitter;

use super::particle::uniform;

pub(super) struct PcParticle {
    pub profile: PcEmitter,
    pub point: Vec3,
    velocity: Vec3,
    acceleration: Vec3,
    remaining: u16,
    initial_angle: u8,
    initial_full_angle: f32,
    direction: f32,
    rotation_offset: f32,
    pub phase: u16,
    pub fresh: bool,
}

impl PcParticle {
    pub fn spawn(p: PcEmitter, point: Vec3, rotation: Quat, owner: Vec3, seed: u64) -> Self {
        let u = |slot| uniform(seed, slot);
        let sample = |value: f32, variance: f32, slot| value * (1.0 - variance + variance * u(slot));
        let vector = |slot| Vec3::new(u(slot), u(slot + 1), u(slot + 2));
        let mut velocity = Vec3::from(p.velocity) + vector(0) * Vec3::from(p.velocity_delta);
        let mut acceleration = Vec3::from(p.acceleration) + vector(3) * Vec3::from(p.acceleration_delta);
        if !p.one_sided {
            velocity = rotation * (Vec3::from(p.velocity) + (vector(0) * 2.0 - 1.0) * Vec3::from(p.velocity_delta));
            acceleration = Vec3::from(p.acceleration) + (vector(3) * 2.0 - 1.0) * Vec3::from(p.acceleration_delta);
        }
        let speed = sample(p.speed, p.speed_variance, 6);
        if p.speed * (1.0 - p.speed_variance) != 0.0 {
            let spread = p.spread.to_radians();
            let jitter = (vector(7) - 0.5) * spread;
            let mut direction = Quat::from_euler(EulerRot::XYZ, jitter.x, jitter.y, jitter.z) * (rotation * Vec3::Z);
            if p.disc {
                let azimuth = u(10) * std::f32::consts::TAU;
                direction = rotation * (Quat::from_rotation_y(jitter.y) * Vec3::new(azimuth.cos(), azimuth.sin(), 0.0));
            }
            velocity = direction * speed;
            acceleration = Vec3::ZERO;
        }
        velocity += owner * sample(p.inherit, p.inherit_variance, 11).clamp(0.0, 1.0);
        let point = point + rotation * (Vec3::from(p.center) + (vector(12) - 0.5) * Vec3::from(p.extent));
        let full_angle = ((u(15) - 0.5) * p.angle_range / 360.0 * 65536.0) as i32 & 0xfffe;
        let phase = match p.grid {
            0 => 0,
            n if p.random_frame => (u(17) * (n * n - 1) as f32).floor() as u16 * (65535 / (n * n - 1)) as u16,
            _ => 0,
        };
        Self {
            profile: p,
            point,
            velocity,
            acceleration,
            remaining: (sample(p.life, p.life_variance, 16).min(60.0) * 1024.0) as u16,
            initial_angle: (full_angle >> 8) as u8,
            initial_full_angle: full_angle as f32 / 65536.0 * std::f32::consts::TAU,
            direction: match p.random_direction && u(18) > 0.5 {
                true => -1.0,
                false => 1.0,
            },
            rotation_offset: (u(19) * p.rotation_variance * 255.0).trunc() / 255.0,
            phase,
            fresh: true,
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.fresh = false;
        self.remaining = self.remaining.saturating_sub((dt * 1024.0) as u16);
        self.velocity *= (1.0 - dt * self.profile.drag * self.velocity.length()).max(0.0);
        if self.profile.gravity != 0.0 {
            self.velocity.z -= self.profile.gravity * dt;
        }
        if self.profile.gravity == 0.0 {
            self.velocity += self.acceleration * dt;
        }
        self.point += self.velocity * dt;
        if self.profile.grid != 0 {
            let n = self.profile.grid * self.profile.grid;
            let delta = (dt * f32::from(self.profile.fps) / n as f32 * 65535.0) as u32;
            let sum = u32::from(self.phase) + delta;
            self.phase = match sum + delta > 65535 {
                true => sum.wrapping_sub(65535) as u16,
                false => sum as u16,
            };
        }
    }

    pub fn progress(&self) -> f32 {
        if self.fresh {
            return 0.0;
        }
        1.0 - f32::from(self.remaining) / (self.profile.life * 1024.0)
    }

    pub fn color(&self) -> [u8; 4] {
        let weights = weights(self.profile.keys, self.progress());
        std::array::from_fn(|channel| {
            (weights.iter().enumerate().map(|(i, &w)| w * self.profile.color[i][channel]).sum::<f32>() * 255.0)
                .clamp(0.0, 255.0) as u8
        })
    }

    pub fn size_angle(&self) -> (f32, f32) {
        let w = weights(self.profile.keys, self.progress());
        let size = w.iter().zip(self.profile.size).map(|(w, v)| w * v).sum::<f32>() * 0.5;
        let relative = w.iter().zip(self.profile.angles).map(|(w, v)| w * v).sum::<f32>();
        if self.fresh {
            return (size, self.initial_full_angle + self.direction * relative.to_radians());
        }
        (
            size,
            f32::from(self.initial_angle) * 257.0 / 65536.0 * std::f32::consts::TAU
                + self.direction * relative.to_radians() * (1.0 + self.rotation_offset),
        )
    }

    pub fn alive(&self) -> bool {
        self.remaining > 0
            && self.point.is_finite()
            && (self.fresh || self.profile.no_alpha_kill || self.color()[3] > self.profile.alpha_kill)
    }

    pub fn uv(&self) -> [[f32; 2]; 4] {
        if self.profile.grid == 0 {
            return [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        }
        let n = self.profile.grid;
        let index = (f32::from(self.phase) / 65535.0 * (n * n - 1) as f32) as u32;
        let x = (index % n) as f32 / n as f32;
        let y = (index / n) as f32 / n as f32;
        let d = 1.0 / n as f32;
        [[x, y], [x + d, y], [x + d, y + d], [x, y + d]]
    }
}

pub(super) fn weights(keys: [f32; 4], age: f32) -> [f32; 4] {
    std::array::from_fn(|i| (0..4).filter(|&j| j != i).map(|j| (age - keys[j]) / (keys[i] - keys[j])).product())
}
