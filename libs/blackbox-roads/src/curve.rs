//! A cubic Bézier curve: the centre line of a curved segment and the lane lines displaced from it.

use glam::Vec3;

/// Smallest chord used in a division.
pub const MIN_CHORD: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bezier {
    pub points: [Vec3; 4],
}

impl Bezier {
    /// A curve from `start` to `end` with handles `start_handle` and `end_handle` (both pointing from
    /// their end point into the curve).
    pub fn with_handles(start: Vec3, start_handle: Vec3, end_handle: Vec3, end: Vec3) -> Self {
        Self { points: [start, start + start_handle, end + end_handle, end] }
    }

    /// A straight line, as a curve whose handles are a third of the way along.
    pub fn line(start: Vec3, end: Vec3) -> Self {
        let third = (end - start) / 3.0;
        Self::with_handles(start, third, -third, end)
    }

    pub fn start(&self) -> Vec3 {
        self.points[0]
    }

    pub fn end(&self) -> Vec3 {
        self.points[3]
    }

    pub fn position(&self, t: f32) -> Vec3 {
        let [a, b, c, d] = self.points;
        let u = 1.0 - t;
        a * (u * u * u) + b * (3.0 * u * u * t) + c * (3.0 * u * t * t) + d * (t * t * t)
    }

    /// The first derivative (not normalised).
    pub fn tangent(&self, t: f32) -> Vec3 {
        let [a, b, c, d] = self.points;
        let u = 1.0 - t;
        (b - a) * (3.0 * u * u) + (c - b) * (6.0 * u * t) + (d - c) * (3.0 * t * t)
    }

    fn second_derivative(&self, t: f32) -> Vec3 {
        let [a, b, c, d] = self.points;
        let u = 1.0 - t;
        (c - b * 2.0 + a) * (6.0 * u) + (d - c * 2.0 + b) * (6.0 * t)
    }

    /// Signed curvature in the xz plane (1 / radius in metres): positive when the curve turns right
    /// (towards +x for a car driving along +z).
    pub fn curvature_xz(&self, t: f32) -> f32 {
        let (d1, d2) = (self.tangent(t), self.second_derivative(t));
        let speed2 = d1.x * d1.x + d1.z * d1.z;
        if speed2 < 1e-9 {
            return 0.0;
        }
        (d1.z * d2.x - d1.x * d2.z) / (speed2 * speed2.sqrt())
    }

    /// The parameter of the point closest to `p`, found by projecting onto the chord and then walking
    /// along the curve in steps of 1 m and 0.25 m towards the better side.
    pub fn closest_t(&self, p: Vec3, length: f32) -> f32 {
        let chord = self.end() - self.start();
        let chord_len2 = chord.length_squared().max(MIN_CHORD * MIN_CHORD);
        let mut t = ((p - self.start()).dot(chord) / chord_len2).clamp(0.0, 1.0);
        let length = length.max(MIN_CHORD);
        for step_m in [1.0, 0.25] {
            let step = (step_m / length).min(1.0);
            let mut best = self.position(t).distance_squared(p);
            for sign in [-1.0, 1.0] {
                loop {
                    let next = (t + sign * step).clamp(0.0, 1.0);
                    let d = self.position(next).distance_squared(p);
                    if d >= best || next == t {
                        break;
                    }
                    best = d;
                    t = next;
                }
            }
        }
        t
    }

    /// The point-to-point length of the curve sampled in `pieces` straight pieces.
    pub fn length(&self, pieces: usize) -> f32 {
        let mut prev = self.start();
        (1..=pieces)
            .map(|i| {
                let next = self.position(i as f32 / pieces as f32);
                let d = prev.distance(next);
                prev = next;
                d
            })
            .sum()
    }
}
