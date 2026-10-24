//! A finite geometric route for the test's physical player. No recorded motion drives the body.

use glam::{Vec2, Vec3};

pub(super) struct Route {
    points: Vec<Vec3>,
    lengths: Vec<f32>,
}

impl Route {
    pub fn new(points: Vec<Vec3>) -> Self {
        assert!(points.len() >= 3);
        let mut lengths = vec![0.0];
        for pair in points.windows(2) {
            let distance = pair[0].distance(pair[1]);
            assert!(distance.is_finite() && distance > 0.0, "route geometry must advance");
            lengths.push(lengths.last().unwrap() + distance);
        }
        Self { points, lengths }
    }

    pub fn length(&self) -> f32 {
        *self.lengths.last().unwrap()
    }

    pub fn point(&self, distance: f32) -> Vec3 {
        let distance = distance.clamp(0.0, self.length());
        let end = self.lengths.partition_point(|&s| s < distance).clamp(1, self.points.len() - 1);
        let t = (distance - self.lengths[end - 1]) / (self.lengths[end] - self.lengths[end - 1]);
        self.points[end - 1].lerp(self.points[end], t)
    }

    /// Search only the nearby forward route so a crossing cannot teleport progress to a later road.
    pub fn project(&self, position: Vec3, previous: f32) -> (f32, f32) {
        let mut best = (f32::INFINITY, previous);
        for (i, pair) in self.points.windows(2).enumerate() {
            if self.lengths[i + 1] < previous - 20.0 || self.lengths[i] > previous + 150.0 {
                continue;
            }
            let delta = pair[1] - pair[0];
            let t = ((position - pair[0]).dot(delta) / delta.length_squared()).clamp(0.0, 1.0);
            let distance = position.distance(pair[0] + delta * t);
            let along = self.lengths[i] + (self.lengths[i + 1] - self.lengths[i]) * t;
            if distance < best.0 {
                best = (distance, along);
            }
        }
        (previous.max(best.1), best.0)
    }

    pub fn curvature(&self, from: f32, look: f32) -> f32 {
        let mut maximum = 0.0_f32;
        let mut at = from;
        while at <= from + look {
            let planar = |p: Vec3| Vec2::new(p.x, p.z);
            let a = planar(self.point((at - 10.0).max(0.0)));
            let b = planar(self.point(at));
            let c = planar(self.point((at + 10.0).min(self.length())));
            let ab = b - a;
            let bc = c - b;
            let denominator = ab.length() * bc.length() * (c - a).length();
            if denominator > 1e-4 {
                maximum = maximum.max(2.0 * ab.perp_dot(bc).abs() / denominator);
            }
            at += 10.0;
        }
        maximum
    }

    fn chord_deviation(&self, from: f32, look: f32) -> f32 {
        let planar = |p: Vec3| Vec2::new(p.x, p.z);
        let start = planar(self.point(from));
        let chord = planar(self.point(from + look)) - start;
        let squared = chord.length_squared();
        if squared <= 1e-4 {
            return f32::INFINITY;
        }
        self.points
            .iter()
            .zip(&self.lengths)
            .filter(|(_, s)| **s > from && **s < from + look)
            .map(|(point, _)| {
                let offset = planar(*point) - start;
                let projection = (offset.dot(chord) / squared).clamp(0.0, 1.0);
                (offset - chord * projection).length()
            })
            .fold(0.0, f32::max)
    }

    /// Bound steering geometry separately from the unchanged speed-planning horizon.
    pub fn steering_preview(&self, from: f32, maximum: f32, width: f32) -> (f32, f32) {
        let minimum = 30.0_f32.min(maximum);
        let mut chosen = (minimum, self.chord_deviation(from, minimum));
        let mut candidate = minimum;
        loop {
            let deviation = self.chord_deviation(from, candidate);
            if deviation <= width {
                chosen = (candidate, deviation);
            }
            if candidate >= maximum {
                return chosen;
            }
            candidate = (candidate + 1.0).min(maximum);
        }
    }

    pub fn coverage(&self, progress: f32) -> (f32, f32) {
        let mut turn = 0.0;
        let mut previous = None;
        let mut height = (self.points[0].y, self.points[0].y);
        let mut at = 10.0;
        while at <= progress {
            let point = self.point(at);
            height = (height.0.min(point.y), height.1.max(point.y));
            let direction = point - self.point(at - 10.0);
            let direction = Vec2::new(direction.x, direction.z).normalize_or_zero();
            if let Some(previous) = previous {
                turn += direction.dot(previous).clamp(-1.0, 1.0).acos();
            }
            previous = Some(direction);
            at += 10.0;
        }
        (turn, height.1 - height.0)
    }
}
