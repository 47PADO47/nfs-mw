//! Translation, rotation and scale of one bone.

use glam::{Mat4, Quat, Vec3};

/// A rotation, a translation and a per-axis scale. A point is scaled, then rotated, then moved:
/// `p' = rotation * (scale * p) + translation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub rotation: Quat,
    pub translation: Vec3,
    pub scale: Vec3,
}

impl Transform {
    /// Leaves points where they are.
    pub const IDENTITY: Self = Self { rotation: Quat::IDENTITY, translation: Vec3::ZERO, scale: Vec3::ONE };

    #[must_use]
    pub const fn new(rotation: Quat, translation: Vec3, scale: Vec3) -> Self {
        Self { rotation, translation, scale }
    }

    /// A transform with unit scale.
    #[must_use]
    pub const fn from_rt(rotation: Quat, translation: Vec3) -> Self {
        Self { rotation, translation, scale: Vec3::ONE }
    }

    /// `self` applied after `child`: the child's frame expressed in the parent's frame's parent.
    /// With `self` the parent's component transform and `child` the bone's local transform, the result is the bone's
    /// component transform. Scale composes per axis and shear is dropped.
    #[must_use]
    pub fn compose(self, child: Self) -> Self {
        Self {
            rotation: (self.rotation * child.rotation).normalize(),
            translation: self.rotation * (self.scale * child.translation) + self.translation,
            scale: self.scale * child.scale,
        }
    }

    /// The transform that undoes `self`. Exact for a uniform scale; for a non-uniform scale use [`Self::to_mat4`] and
    /// invert the matrix.
    #[must_use]
    pub fn inverse(self) -> Self {
        let scale = Vec3::new(recip(self.scale.x), recip(self.scale.y), recip(self.scale.z));
        let rotation = self.rotation.conjugate();
        Self { rotation, translation: -(rotation * (scale * self.translation)), scale }
    }

    #[must_use]
    pub fn transform_point(self, point: Vec3) -> Vec3 {
        self.rotation * (self.scale * point) + self.translation
    }

    /// Rotates (and scales) a direction; the translation is ignored.
    #[must_use]
    pub fn transform_vector(self, vector: Vec3) -> Vec3 {
        self.rotation * (self.scale * vector)
    }

    #[must_use]
    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Spherical interpolation of the rotation, linear of translation and scale; `weight` 0 gives `self`, 1 `other`.
    #[must_use]
    pub fn lerp(self, other: Self, weight: f32) -> Self {
        Self {
            rotation: self.rotation.slerp(other.rotation, weight).normalize(),
            translation: self.translation.lerp(other.translation, weight),
            scale: self.scale.lerp(other.scale, weight),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

fn recip(value: f32) -> f32 {
    if value.abs() < f32::EPSILON {
        return 0.0;
    }
    1.0 / value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Transform {
        Transform::new(Quat::from_rotation_y(0.7), Vec3::new(1.0, 2.0, 3.0), Vec3::splat(2.0))
    }

    #[test]
    fn inverse_undoes_the_transform() {
        let t = sample();
        let p = Vec3::new(0.3, -4.0, 5.0);
        let back = t.inverse().transform_point(t.transform_point(p));
        assert!(back.distance(p) < 1e-4, "{back:?}");
    }

    #[test]
    fn compose_matches_matrix_product() {
        let a = sample();
        let b = Transform::new(Quat::from_rotation_x(-0.4), Vec3::new(-1.0, 0.5, 2.0), Vec3::ONE);
        let p = Vec3::new(1.0, 1.0, 1.0);
        let by_mul = a.compose(b).transform_point(p);
        let by_mat = (a.to_mat4() * b.to_mat4()).transform_point3(p);
        assert!(by_mul.distance(by_mat) < 1e-4);
    }

    #[test]
    fn lerp_ends_and_middle() {
        let a = Transform::IDENTITY;
        let b = Transform::new(Quat::from_rotation_z(1.0), Vec3::new(2.0, 0.0, 0.0), Vec3::splat(3.0));
        assert!(a.lerp(b, 0.0).translation.distance(a.translation) < 1e-6);
        assert!(a.lerp(b, 1.0).translation.distance(b.translation) < 1e-6);
        let half = a.lerp(b, 0.5);
        assert!((half.scale.x - 2.0).abs() < 1e-6);
        assert!(half.rotation.angle_between(Quat::from_rotation_z(0.5)) < 1e-4);
    }

    #[test]
    fn inverse_of_zero_scale_does_not_blow_up() {
        let t = Transform::new(Quat::IDENTITY, Vec3::ONE, Vec3::ZERO);
        assert!(t.inverse().translation.is_finite());
    }
}
