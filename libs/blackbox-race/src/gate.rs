//! A gate: an oriented box across the road that a racer crosses (a checkpoint, a finish line, a speed trap).

use glam::Vec3;

/// An oriented box. `forward` and `side` span the ground plane, `up` is the height axis; `half` holds the half sizes
/// along `forward`, `side` and `up`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gate {
    pub centre: Vec3,
    pub forward: Vec3,
    pub side: Vec3,
    pub up: Vec3,
    pub half: Vec3,
}

impl Gate {
    /// A gate at `centre` facing `forward`, with `up` as the height axis and `sizes` the full extents along
    /// `forward`, `side` and `up`. `forward` is flattened against `up` and normalised; `side = up x forward`.
    pub fn new(centre: Vec3, forward: Vec3, up: Vec3, sizes: [f32; 3]) -> Self {
        let up = up.try_normalize().unwrap_or(Vec3::Y);
        let flat = forward - up * forward.dot(up);
        let forward = flat.try_normalize().unwrap_or_else(|| up.any_orthonormal_vector());
        let side = up.cross(forward);
        Self { centre, forward, side, up, half: Vec3::from_array(sizes) * 0.5 }
    }

    /// The offset of `point` from the centre along `forward`, `side` and `up`.
    pub fn local(&self, point: Vec3) -> Vec3 {
        let offset = point - self.centre;
        Vec3::new(offset.dot(self.forward), offset.dot(self.side), offset.dot(self.up))
    }

    /// Whether `point` is inside the box (the surface counts as inside).
    pub fn contains(&self, point: Vec3) -> bool {
        self.local(point).abs().cmple(self.half).all()
    }

    /// Where the trigger's flares stand: `spacing` metres to each side of the centre along the gate's width axis, or
    /// the centre alone when `spacing` is not above zero: one marker effect at each of centre minus and centre plus
    /// `side * spacing`, where `side` is the up axis crossed with the heading (`docs/specs/race-finish-effects.md`
    /// section 1).
    pub fn flare_points(&self, spacing: f32) -> Vec<Vec3> {
        match spacing > 0.0 {
            true => vec![self.centre - self.side * spacing, self.centre + self.side * spacing],
            false => vec![self.centre],
        }
    }

    /// Whether the segment from `from` to `to` touches the box. This catches a fast racer that jumps over a thin gate
    /// between two ticks, which a point test would miss.
    pub fn touches(&self, from: Vec3, to: Vec3) -> bool {
        let (a, b) = (self.local(from), self.local(to));
        let delta = b - a;
        let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
        for axis in 0..3 {
            let (start, step, limit) = (a[axis], delta[axis], self.half[axis]);
            if step.abs() < 1e-9 {
                if start.abs() > limit {
                    return false;
                }
                continue;
            }
            let (t0, t1) = ((-limit - start) / step, (limit - start) / step);
            enter = enter.max(t0.min(t1));
            leave = leave.min(t0.max(t1));
            if enter > leave {
                return false;
            }
        }
        true
    }
}

/// A sphere across the road (a speed trap): a point and a segment test.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub centre: Vec3,
    pub radius: f32,
}

impl Sphere {
    /// Whether `point` is inside (the surface counts as inside).
    pub fn contains(&self, point: Vec3) -> bool {
        point.distance_squared(self.centre) <= self.radius * self.radius
    }

    /// Whether the segment from `from` to `to` touches the sphere.
    pub fn touches(&self, from: Vec3, to: Vec3) -> bool {
        let segment = to - from;
        let length_squared = segment.length_squared();
        let t = match length_squared > 1e-12 {
            true => ((self.centre - from).dot(segment) / length_squared).clamp(0.0, 1.0),
            false => 0.0,
        };
        self.contains(from + segment * t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> Gate {
        // Thin along forward (+z), 40 m wide, 10 m high; y is up.
        Gate::new(Vec3::new(10.0, 0.0, 100.0), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
    }

    #[test]
    fn a_point_inside_the_box_is_contained() {
        let g = gate();
        assert!(g.contains(Vec3::new(10.0, 0.0, 100.9)));
        assert!(!g.contains(Vec3::new(10.0, 0.0, 101.1)), "past the 1 m half thickness");
        assert!(g.contains(Vec3::new(29.9, 4.9, 100.0)));
        assert!(!g.contains(Vec3::new(30.1, 0.0, 100.0)));
        assert!(!g.contains(Vec3::new(10.0, 5.1, 100.0)));
    }

    #[test]
    fn flares_stand_a_spacing_to_each_side_along_the_width() {
        let g = gate();
        let points = g.flare_points(5.0);
        assert_eq!(points.len(), 2);
        // The gate heads +z with y up, so its width axis is x: the flares are 5 m either side of the centre.
        let xs = [points[0].x.min(points[1].x), points[0].x.max(points[1].x)];
        assert!((xs[0] - 5.0).abs() < 1e-4 && (xs[1] - 15.0).abs() < 1e-4);
        assert!((points[0].distance(points[1]) - 10.0).abs() < 1e-4, "twice the spacing apart");
        assert!(((points[0] + points[1]) * 0.5 - g.centre).length() < 1e-4, "centred on the gate");
        for p in points {
            assert!(g.local(p).x.abs() < 1e-4, "on the gate's own line, not ahead or behind it");
        }
    }

    #[test]
    fn a_gate_without_spacing_has_one_flare_at_its_centre() {
        let g = gate();
        assert_eq!(g.flare_points(0.0), vec![g.centre]);
        assert_eq!(g.flare_points(-3.0), vec![g.centre]);
    }

    #[test]
    fn a_fast_racer_that_jumps_the_gate_still_touches_it() {
        let g = gate();
        let (from, to) = (Vec3::new(10.0, 0.0, 95.0), Vec3::new(10.0, 0.0, 105.0));
        assert!(!g.contains(from) && !g.contains(to));
        assert!(g.touches(from, to));
    }

    #[test]
    fn a_segment_beside_or_short_of_the_gate_misses() {
        let g = gate();
        assert!(!g.touches(Vec3::new(40.0, 0.0, 95.0), Vec3::new(40.0, 0.0, 105.0)), "beside it");
        assert!(!g.touches(Vec3::new(10.0, 0.0, 90.0), Vec3::new(10.0, 0.0, 98.0)), "stops short");
        assert!(!g.touches(Vec3::new(10.0, 8.0, 95.0), Vec3::new(10.0, 8.0, 105.0)), "over it");
    }

    #[test]
    fn a_segment_that_starts_inside_touches() {
        let g = gate();
        assert!(g.touches(Vec3::new(10.0, 0.0, 100.0), Vec3::new(10.0, 0.0, 99.0)));
    }

    #[test]
    fn a_sphere_catches_a_segment_that_passes_through_it() {
        let sphere = Sphere { centre: Vec3::new(0.0, 0.0, 100.0), radius: 20.0 };
        assert!(sphere.contains(Vec3::new(0.0, 0.0, 119.0)));
        assert!(!sphere.contains(Vec3::new(0.0, 0.0, 121.0)));
        assert!(sphere.touches(Vec3::new(0.0, 0.0, 50.0), Vec3::new(0.0, 0.0, 150.0)), "jumps clean over the centre");
        assert!(sphere.touches(Vec3::new(15.0, 0.0, 50.0), Vec3::new(15.0, 0.0, 150.0)), "inside the radius");
        assert!(!sphere.touches(Vec3::new(25.0, 0.0, 50.0), Vec3::new(25.0, 0.0, 150.0)), "beside it");
        assert!(!sphere.touches(Vec3::new(0.0, 0.0, 50.0), Vec3::new(0.0, 0.0, 70.0)), "stops short");
        assert!(sphere.touches(Vec3::new(0.0, 0.0, 100.0), Vec3::new(0.0, 0.0, 100.0)), "a point inside");
    }

    #[test]
    fn the_gate_turns_with_its_forward() {
        let g = Gate::new(Vec3::ZERO, Vec3::X, Vec3::Y, [2.0, 40.0, 10.0]);
        assert!(g.contains(Vec3::new(0.5, 0.0, 19.0)), "wide along z when facing x");
        assert!(!g.contains(Vec3::new(19.0, 0.0, 0.5)));
    }
}
