//! A pose: one local transform per bone, and the ways to combine poses.

use glam::{Quat, Vec3};

use crate::error::Result;
use crate::skeleton::Skeleton;
use crate::transform::Transform;

/// Local transforms, one per bone of the skeleton the pose belongs to.
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub local: Vec<Transform>,
}

impl Pose {
    /// `count` identity transforms: the neutral element of an additive pose.
    #[must_use]
    pub fn identity(count: usize) -> Self {
        Self { local: vec![Transform::IDENTITY; count] }
    }

    /// `self` at weight 0, `other` at weight 1.
    #[must_use]
    pub fn blend(&self, other: &Self, weight: f32) -> Self {
        Self { local: self.local.iter().zip(&other.local).map(|(a, b)| a.lerp(*b, weight)).collect() }
    }

    /// Like [`Self::blend`] with a weight per bone, each multiplied by `weight`.
    #[must_use]
    pub fn blend_masked(&self, other: &Self, mask: &[f32], weight: f32) -> Self {
        let local = self
            .local
            .iter()
            .zip(&other.local)
            .enumerate()
            .map(|(i, (a, b))| a.lerp(*b, mask.get(i).copied().unwrap_or(0.0) * weight))
            .collect();
        Self { local }
    }

    /// Adds an additive pose on top: rotations multiply (`delta * base`), translations add, scales multiply. All
    /// three scale with `weight`.
    pub fn add_additive(&mut self, delta: &Self, weight: f32) {
        for (base, d) in self.local.iter_mut().zip(&delta.local) {
            let rotation = Quat::IDENTITY.slerp(d.rotation, weight) * base.rotation;
            *base = Transform {
                rotation: rotation.normalize(),
                translation: base.translation + d.translation * weight,
                scale: base.scale * Vec3::ONE.lerp(d.scale, weight),
            };
        }
    }

    /// Transforms relative to the skeleton's origin.
    pub fn component(&self, skeleton: &Skeleton) -> Result<Vec<Transform>> {
        let mut out = Vec::with_capacity(self.local.len());
        skeleton.component_transforms(&self.local, &mut out)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(angle: f32, x: f32) -> Pose {
        Pose { local: vec![Transform::from_rt(Quat::from_rotation_z(angle), Vec3::new(x, 0.0, 0.0))] }
    }

    #[test]
    fn blend_masked_respects_the_mask() {
        let a = one(0.0, 0.0);
        let b = one(1.0, 2.0);
        let none = a.blend_masked(&b, &[0.0], 1.0);
        assert_eq!(none.local[0].translation, Vec3::ZERO);
        let half = a.blend_masked(&b, &[0.5], 1.0);
        assert!((half.local[0].translation.x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn additive_at_full_and_zero_weight() {
        let base = one(0.5, 1.0);
        let delta =
            Pose { local: vec![Transform::new(Quat::from_rotation_z(0.25), Vec3::new(0.0, 1.0, 0.0), Vec3::ONE)] };
        let mut zero = base.clone();
        zero.add_additive(&delta, 0.0);
        assert!(zero.local[0].rotation.angle_between(base.local[0].rotation) < 1e-5);
        let mut full = base.clone();
        full.add_additive(&delta, 1.0);
        assert!(full.local[0].rotation.angle_between(Quat::from_rotation_z(0.75)) < 1e-5);
        assert!((full.local[0].translation.y - 1.0).abs() < 1e-6);
    }
}
