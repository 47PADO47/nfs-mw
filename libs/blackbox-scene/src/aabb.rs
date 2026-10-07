//! Axis-aligned bounding boxes.

use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb { min: Vec3::splat(f32::MAX), max: Vec3::splat(f32::MIN) };

    pub fn new(min: impl Into<Vec3>, max: impl Into<Vec3>) -> Self {
        Self { min: min.into(), max: max.into() }
    }

    pub fn is_empty(&self) -> bool {
        self.min.cmpgt(self.max).any()
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn extend(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb { min: self.min.min(other.min), max: self.max.max(other.max) }
    }

    /// The box around this box after `m`.
    pub fn transformed(&self, m: &Mat4) -> Aabb {
        let mut out = Aabb::EMPTY;
        for i in 0..8 {
            let corner = Vec3::new(
                [self.min.x, self.max.x][i & 1],
                [self.min.y, self.max.y][(i >> 1) & 1],
                [self.min.z, self.max.z][i >> 2],
            );
            out.extend(m.transform_point3(corner));
        }
        out
    }

    /// Distance from `p` to the box (0 inside).
    pub fn distance_to(&self, p: Vec3) -> f32 {
        (self.min - p).max(p - self.max).max(Vec3::ZERO).length()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_and_distance() {
        let b = Aabb::new([0.0, 0.0, 0.0], [1.0, 2.0, 3.0]);
        let moved = b.transformed(&Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)));
        assert_eq!(moved, Aabb::new([10.0, 0.0, 0.0], [11.0, 2.0, 3.0]));
        assert_eq!(b.distance_to(Vec3::new(0.5, 1.0, 1.0)), 0.0);
        assert_eq!(b.distance_to(Vec3::new(4.0, 1.0, 1.0)), 3.0);
        assert!(Aabb::EMPTY.is_empty());
        assert_eq!(b.center(), Vec3::new(0.5, 1.0, 1.5));
    }
}
