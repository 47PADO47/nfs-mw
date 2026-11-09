//! A road made of a height profile along z, for tests and audits: ramps, steps, crests, a landing slope, a washboard, and a surface for each
//! span. The ground is the same across x (a ridden line); the rays a tyre casts are vertical, and so are the ones this ground answers.

use glam::Vec3;

use super::{Ground, GroundHit, SurfaceGrip};

/// A polyline of `(z, height)` vertices (z rising), flat before the first and after the last, with the grip of each span between two
/// vertices.
#[derive(Clone, Debug)]
pub struct ProfileGround {
    points: Vec<(f32, f32)>,
    spans: Vec<SurfaceGrip>,
    outside: SurfaceGrip,
}

impl ProfileGround {
    /// The profile through `points`; every span, and the flat ground beyond the ends, has `surface`.
    pub fn new(points: Vec<(f32, f32)>, surface: SurfaceGrip) -> Self {
        let spans = vec![surface; points.len().saturating_sub(1)];
        Self { points, spans, outside: surface }
    }

    /// A flat road at height 0 until `z = 0`, then `drop` metres lower (a step down) from `z = at` on.
    pub fn step_down(at: f32, drop: f32, surface: SurfaceGrip) -> Self {
        Self::new(vec![(at - 0.01, 0.0), (at, -drop)], surface)
    }

    /// Replace the surface of span `index` (between vertex `index` and `index + 1`); an index past the last span changes nothing.
    pub fn with_span(mut self, index: usize, surface: SurfaceGrip) -> Self {
        if let Some(span) = self.spans.get_mut(index) {
            *span = surface;
        }
        self
    }

    /// A washboard: `bumps` bumps of height `height` (m) and length `length` (m) from `start` on, each a half sine, on a road at height 0.
    pub fn washboard(start: f32, length: f32, bumps: usize, height: f32, surface: SurfaceGrip) -> Self {
        let mut points = vec![(start - 0.01, 0.0)];
        const PER_BUMP: usize = 6;
        for bump in 0..bumps {
            for part in 0..=PER_BUMP {
                let along = part as f32 / PER_BUMP as f32;
                let z = start + (bump as f32 + along) * length;
                points.push((z, height * (along * std::f32::consts::PI).sin()));
            }
        }
        Self::new(points, surface)
    }

    /// The height, the unit normal and the surface at `z`.
    pub fn at(&self, z: f32) -> (f32, Vec3, SurfaceGrip) {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else {
            return (0.0, Vec3::Y, self.outside);
        };
        if z <= first.0 {
            return (first.1, Vec3::Y, self.outside);
        }
        if z >= last.0 {
            return (last.1, Vec3::Y, self.outside);
        }
        let index = self.points.partition_point(|point| point.0 <= z).saturating_sub(1);
        let (a, b) = (self.points[index], self.points[index + 1]);
        let along = ((z - a.0) / (b.0 - a.0).max(1e-6)).clamp(0.0, 1.0);
        let slope = (b.1 - a.1) / (b.0 - a.0).max(1e-6);
        (a.1 + (b.1 - a.1) * along, Vec3::new(0.0, 1.0, -slope).normalize(), self.spans[index])
    }
}

impl Ground for ProfileGround {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        if dir.y >= -1e-6 {
            return None;
        }
        let (height, normal, surface) = self.at(origin.z);
        let distance = (height - origin.y) / dir.y;
        (distance >= 0.0 && distance <= max_distance).then_some(GroundHit { distance, normal, surface })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRT: SurfaceGrip = SurfaceGrip { lateral: 0.6, drive: 0.6, rolling: 1.0, tag: 5 };

    #[test]
    fn a_ramp_rises_with_the_slope_and_the_ground_is_flat_beyond_its_ends() {
        let ground = ProfileGround::new(vec![(0.0, 0.0), (10.0, 2.0)], SurfaceGrip::DEFAULT);
        let (height, normal, _) = ground.at(5.0);
        assert!((height - 1.0).abs() < 1e-5);
        assert!(normal.z < 0.0 && (normal.z + 0.2 * normal.y).abs() < 1e-5, "{normal:?}");
        assert_eq!(ground.at(-3.0).0, 0.0);
        assert_eq!(ground.at(30.0).0, 2.0);
        assert_eq!(ground.at(30.0).1, Vec3::Y);
    }

    #[test]
    fn a_vertical_ray_hits_the_profile_below_it_and_only_within_reach() {
        let ground = ProfileGround::new(vec![(0.0, 0.0), (10.0, 2.0)], SurfaceGrip::DEFAULT);
        let hit = ground.hit(Vec3::new(0.0, 4.0, 5.0), Vec3::NEG_Y, 5.0).expect("the ramp is 3 m below");
        assert!((hit.distance - 3.0).abs() < 1e-5);
        assert!(ground.hit(Vec3::new(0.0, 4.0, 5.0), Vec3::NEG_Y, 2.0).is_none());
        assert!(ground.hit(Vec3::new(0.0, 0.5, 5.0), Vec3::NEG_Y, 5.0).is_none(), "the origin is under the ground");
        assert!(ground.hit(Vec3::new(0.0, 4.0, 5.0), Vec3::Y, 5.0).is_none());
    }

    #[test]
    fn a_span_has_its_own_surface() {
        let ground =
            ProfileGround::new(vec![(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)], SurfaceGrip::DEFAULT).with_span(1, DIRT);
        assert_eq!(ground.at(2.0).2.lateral, 1.0);
        assert_eq!(ground.at(7.0).2.lateral, 0.6);
        assert_eq!(ground.at(7.0).2.tag, 5);
    }

    #[test]
    fn a_step_down_and_a_washboard_have_the_heights_they_say() {
        let step = ProfileGround::step_down(10.0, 3.0, SurfaceGrip::DEFAULT);
        assert_eq!(step.at(9.0).0, 0.0);
        assert_eq!(step.at(11.0).0, -3.0);
        let board = ProfileGround::washboard(5.0, 0.5, 4, 0.06, DIRT);
        let top = (0..400).map(|i| board.at(5.0 + i as f32 * 0.005).0).fold(0.0_f32, f32::max);
        assert!((top - 0.06).abs() < 0.005, "{top}");
        assert_eq!(board.at(1.0).0, 0.0);
        assert!(board.at(8.0).0.abs() < 1e-6, "flat after the last bump");
    }
}
