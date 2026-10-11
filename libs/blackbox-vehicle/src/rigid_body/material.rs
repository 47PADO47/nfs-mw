//! Material and effective-mass parameters for contacts with other bodies.

use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectMaterial {
    /// Static and kinetic friction coefficients.
    pub friction: [f32; 2],
    /// Restitution along each local body axis.
    pub elasticity: Vec3,
    /// Multipliers on the principal moments of inertia during a contact.
    pub inertia_scale: Vec3,
    /// Multiplier on contact mass only; does not change inertia or integration mass.
    pub mass_scale: f32,
}

impl ObjectMaterial {
    /// Geometric mean of the two materials, with kinetic friction limited by static.
    pub fn friction_with(&self, other: &Self) -> [f32; 2] {
        let static_friction = (self.friction[0] * other.friction[0]).max(0.0).sqrt();
        let kinetic = (self.friction[1] * other.friction[1]).max(0.0).sqrt();
        [static_friction, kinetic.min(static_friction)]
    }

    /// Restitution projected onto a unit normal expressed in the body's local frame.
    pub fn restitution(&self, local_normal: Vec3) -> f32 {
        (local_normal * self.elasticity).length()
    }
}

impl Default for ObjectMaterial {
    fn default() -> Self {
        Self { friction: [0.5, 0.3], elasticity: Vec3::splat(0.2), inertia_scale: Vec3::ONE, mass_scale: 1.0 }
    }
}
