//! A road segment's centre line as a short polyline: a straight segment is its two ends; a curved one is cut in halves
//! until each piece's chord is within the tolerance of the curve at its middle, then thinned with Douglas-Peucker at
//! the same tolerance, so a gentle bend keeps few points and a tight one keeps what it needs.
//!
//! The error is measured in render space (x, y), the plane the map is drawn in; the height is dropped. The sampling and
//! the thinning each allow the tolerance, so the polyline stays within twice it of the curve.

use blackbox_roads::Bezier;
use glam::Vec3;

/// Every curve is cut at least this many times in halves (into quarters), so an S bend whose middle sits on its chord
/// is not taken for a straight line.
const MIN_LEVEL: u32 = 2;
/// No piece is cut further than this (1/256 of the curve).
const MAX_LEVEL: u32 = 8;

/// The points of `curve` in render (x, y), carried there by `to_render`: its ends for a straight segment, else sampled
/// and thinned to `tolerance` metres.
pub(super) fn polyline(
    curve: &Bezier,
    curved: bool,
    tolerance: f32,
    to_render: &impl Fn(Vec3) -> [f32; 2],
) -> Vec<[f32; 2]> {
    let start = to_render(curve.start());
    let end = to_render(curve.end());
    if !curved {
        return vec![start, end];
    }
    let mut points = vec![start];
    let pieces = Pieces { curve, tolerance, to_render };
    pieces.cut((0.0, start), (1.0, end), 0, &mut points);
    douglas_peucker(&points, tolerance)
}

/// The sampling of one curve.
struct Pieces<'a, F> {
    curve: &'a Bezier,
    tolerance: f32,
    to_render: &'a F,
}

impl<F: Fn(Vec3) -> [f32; 2]> Pieces<'_, F> {
    /// Pushes the points after `a` up to and including `b`, cutting the piece between them while its middle is farther
    /// than the tolerance from its chord.
    fn cut(&self, a: (f32, [f32; 2]), b: (f32, [f32; 2]), level: u32, out: &mut Vec<[f32; 2]>) {
        let t = (a.0 + b.0) / 2.0;
        let middle = (self.to_render)(self.curve.position(t));
        let flat = level >= MIN_LEVEL && distance_to_segment(middle, a.1, b.1) <= self.tolerance;
        if flat || level >= MAX_LEVEL {
            out.push(b.1);
            return;
        }
        self.cut(a, (t, middle), level + 1, out);
        self.cut((t, middle), b, level + 1, out);
    }
}

/// `points` thinned: the ends are kept, and a point between two kept ones only when it is farther than `tolerance`
/// from the line between them (Douglas-Peucker, iterative).
pub(super) fn douglas_peucker(points: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let last = points.len() - 1;
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[last] = true;
    let mut spans = vec![(0, last)];
    while let Some((a, b)) = spans.pop() {
        let farthest = (a + 1..b).map(|i| (i, distance_to_segment(points[i], points[a], points[b]))).fold(
            (a, 0.0),
            |best, next| match next.1 > best.1 {
                true => next,
                false => best,
            },
        );
        if farthest.1 <= tolerance {
            continue;
        }
        keep[farthest.0] = true;
        spans.push((a, farthest.0));
        spans.push((farthest.0, b));
    }
    points.iter().zip(keep).filter(|(_, kept)| *kept).map(|(point, _)| *point).collect()
}

/// The distance from `p` to the segment from `a` to `b` (to `a` for a segment of no length).
pub(super) fn distance_to_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let span = d[0] * d[0] + d[1] * d[1];
    let t = match span > 0.0 {
        true => (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / span).clamp(0.0, 1.0),
        false => 0.0,
    };
    (p[0] - a[0] - d[0] * t).hypot(p[1] - a[1] - d[1] * t)
}
