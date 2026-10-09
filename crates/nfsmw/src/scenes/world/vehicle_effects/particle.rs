//! Numeric recipe from docs/specs/xenon-particle-motion.md; no reference code is ported.
use glam::{Quat, Vec3};
use nfsmw_data::vehicle_effects::EmitterStyle;

#[cfg(test)]
#[path = "particle_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Particle {
    pub origin: Vec3,
    pub velocity: Vec3,
    pub gravity: f32,
    pub previous: Vec3,
    pub age: f32,
    pub life: f32,
    pub color: [f32; 4],
    pub duration: f32,
    pub width: f32,
    pub elasticity: f32,
    pub bounces: u8,
}

impl Particle {
    pub fn spawn(style: EmitterStyle, origin: Vec3, rotation: Quat, velocity: Vec3, seed: u64, intensity: f32) -> Self {
        Self::sample(style, origin, rotation, velocity, |slot| uniform(seed, slot), intensity)
    }

    pub fn sample(
        style: EmitterStyle,
        origin: Vec3,
        rotation: Quat,
        velocity: Vec3,
        random: impl Fn(u64) -> f32,
        intensity: f32,
    ) -> Self {
        let vector = |offset| Vec3::new(random(offset), random(offset + 1), random(offset + 2));
        let point =
            origin + rotation * (Vec3::from(style.center) + (vector(0) - Vec3::splat(0.5)) * Vec3::from(style.extent));
        let velocity = (rotation * Vec3::from(style.velocity) + velocity * Vec3::from(style.inherit))
            * (Vec3::ONE + (vector(3) * 2.0 - Vec3::ONE) * Vec3::from(style.variation));
        let mut color = style.color;
        if intensity != 1.0 {
            color[3] = (intensity * 42.0).clamp(0.0, 42.0).trunc() / 255.0;
        }
        Self {
            origin: point,
            previous: point,
            velocity,
            gravity: style.gravity + style.gravity_delta * (random(6) * 2.0 - 1.0),
            age: 0.0,
            life: style.life * (1.0 - style.life_variance),
            color,
            duration: (style.length + random(7) * style.length_delta).clamp(0.0, 255.0).trunc() / 2048.0,
            width: style.height.clamp(0.0, 255.0).trunc() / 2048.0,
            elasticity: 0.0,
            bounces: 0,
        }
    }

    pub fn position(&self, age: f32) -> Vec3 {
        self.origin + self.velocity * age + Vec3::Z * (self.gravity * age * age)
    }

    pub fn speed(&self, age: f32) -> Vec3 {
        self.velocity + Vec3::Z * (2.0 * self.gravity * age)
    }

    pub fn age(&mut self, dt: f32) {
        self.previous = self.position(self.age);
        self.age += dt;
    }

    pub fn alive(&self) -> bool {
        self.age < self.life && self.position(self.age).is_finite()
    }

    /// Reflect at the swept hit, then finish this step from the new trajectory.
    pub fn bounce(&mut self, fraction: f32, point: Vec3, normal: Vec3, dt: f32) {
        let remainder = dt * (1.0 - fraction.clamp(0.0, 1.0));
        let hit_age = (self.age - remainder).max(0.0);
        let incoming = self.speed(hit_age);
        if incoming.dot(normal) >= 0.0 {
            return;
        }
        self.velocity = incoming.reflect(normal) * self.elasticity;
        self.origin = point + normal * 0.005;
        self.life -= hit_age;
        self.age = remainder;
        self.previous = self.origin;
        self.bounces += 1;
    }
}

pub(super) fn count(style: EmitterStyle, intensity: f32) -> usize {
    (style.count * intensity.max(1.0) * (1.0 - style.count_variance * 100.0)).clamp(0.0, 256.0).ceil() as usize
}

/// Independent uniform samples avoid the correlated spatial bands of sinusoidal noise.
pub(super) fn uniform(seed: u64, slot: u64) -> f32 {
    let mut word = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(slot.wrapping_mul(0xbf58_476d_1ce4_e5b9));
    word = (word ^ (word >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    word = (word ^ (word >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((word ^ (word >> 31)) >> 40) as f32 / 16_777_216.0
}
