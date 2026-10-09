//! Spark-only interior exclusion. This uses the car's collision box, not its mesh silhouette.
use glam::{Quat, Vec3};
use nfsmw_data::car::physics::CarBounds;

use super::super::{drive::CarPose, space};

#[derive(Clone, Copy)]
pub(super) struct BodyClip {
    origin: Vec3,
    inverse: Quat,
    pivot: Vec3,
    half: Vec3,
}

impl BodyClip {
    pub fn new(pose: CarPose, bounds: CarBounds) -> Option<Self> {
        let half = bounds.half_dimensions;
        if !pose.position.is_finite()
            || !pose.rotation.is_normalized()
            || !bounds.pivot.is_finite()
            || !half.is_finite()
            || half.min_element() <= 0.0
        {
            return None;
        }
        Some(Self {
            origin: pose.position,
            inverse: pose.rotation.inverse(),
            pivot: space::to_render(bounds.pivot.to_array()),
            half: Vec3::new(half.z, half.x, half.y),
        })
    }

    fn local(&self, point: Vec3) -> Vec3 {
        self.inverse * (point - self.origin) - self.pivot
    }

    pub fn contains(&self, point: Vec3) -> bool {
        self.local(point).abs().cmple(self.half).all()
    }

    /// Visible fractions of head -> tail, retaining the original mask coordinates.
    pub fn outside(&self, head: Vec3, tail: Vec3) -> [[f32; 2]; 2] {
        let start = self.local(head);
        let delta = self.local(tail) - start;
        let (mut enter, mut exit) = (0.0_f32, 1.0_f32);
        for axis in 0..3 {
            if delta[axis].abs() < 1e-7 {
                if start[axis].abs() > self.half[axis] {
                    return [[0.0, 1.0], [1.0, 1.0]];
                }
                continue;
            }
            let a = (-self.half[axis] - start[axis]) / delta[axis];
            let b = (self.half[axis] - start[axis]) / delta[axis];
            enter = enter.max(a.min(b));
            exit = exit.min(a.max(b));
            if enter >= exit {
                return [[0.0, 1.0], [1.0, 1.0]];
            }
        }
        [[0.0, enter], [exit, 1.0]]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_interior_crossings_at_the_rotated_render_pose() {
        let pose = CarPose {
            position: Vec3::new(10.0, 20.0, 30.0),
            rotation: Quat::from_rotation_z(1.2),
            wheels: [Default::default(); 4],
        };
        let bounds = CarBounds { half_dimensions: Vec3::new(1.0, 0.5, 2.0), pivot: Vec3::Y * 0.8 };
        let body = BodyClip::new(pose, bounds).unwrap();
        let world = |local| pose.position + pose.rotation * (local + Vec3::Z * 0.8);
        let ranges = body.outside(world(Vec3::X * -4.0), world(Vec3::X * 4.0));
        assert!((ranges[0][1] - 0.25).abs() < 1e-5 && (ranges[1][0] - 0.75).abs() < 1e-5);
        assert!(body.contains(world(Vec3::ZERO)));
        assert!(!body.contains(world(Vec3::Z)));
        assert_eq!(body.outside(world(Vec3::ZERO), world(Vec3::X)), [[0.0, 0.0], [1.0, 1.0]]);
        assert_eq!(body.outside(world(Vec3::Z), world(Vec3::Z + Vec3::X)), [[0.0, 1.0], [1.0, 1.0]]);

        let mut vertices = Vec::new();
        for range in ranges {
            super::super::geometry::streak_range(
                &mut vertices,
                world(Vec3::X * -4.0),
                world(Vec3::X * 4.0),
                0.01,
                [255; 4],
                world(Vec3::Z * 10.0),
                Vec3::NEG_Z,
                range,
            );
        }
        assert_eq!(vertices.len(), 12, "a crossing has two exterior pieces");
        assert!(vertices.iter().all(|v| body.local(Vec3::from(v.position)).x.abs() >= 2.0 - 1e-5));
        assert!(vertices[..6].iter().all(|v| v.uv[1] <= 0.25 + 1e-5));
        assert!(vertices[6..].iter().all(|v| v.uv[1] >= 0.75 - 1e-5));
    }
}
