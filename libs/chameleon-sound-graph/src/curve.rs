//! The curves of a value modulator: points with a left and a right tangent, drawn as cubic Bezier segments.
//!
//! A point is `(x, y)` with two tangents `(tx, ty)`. This reader takes a tangent as the *fraction* of the segment's
//! width and height at which the Bezier handle sits: the right handle of point `a` is
//! `a + (tx * width, ty * height)` and the left handle of point `b` is `b - (tx * width, ty * height)`, with `width` and
//! `height` the extents of the segment. That reading keeps the curves of the Porsche's engine-load mixer inside their
//! points (monotone where the points are) and gives the default tangent `(1, 0)` a flat start; it is not proven
//! (docs/formats/chameleon-sound-graph.md).

/// One point of a curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    /// Tangent arriving at the point.
    pub left: [f32; 2],
    /// Tangent leaving the point.
    pub right: [f32; 2],
}

/// A curve over `x`, flat outside its first and last point.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Curve {
    pub points: Vec<Point>,
}

impl Curve {
    /// Whether the curve has no point (the modulator then maps its input linearly).
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// The curve's value at `x`.
    pub fn eval(&self, x: f32) -> f32 {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else { return x };
        if x <= first.x {
            return first.y;
        }
        if x >= last.x {
            return last.y;
        }
        let Some(pair) = self.points.windows(2).find(|pair| x <= pair[1].x) else { return last.y };
        segment(&pair[0], &pair[1], x)
    }

    /// `count` samples `(x, y)` spread evenly from the first to the last point, at least two.
    pub fn sample(&self, count: usize) -> Vec<(f32, f32)> {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else { return Vec::new() };
        let count = count.max(2);
        (0..count)
            .map(|step| {
                let x = first.x + (last.x - first.x) * step as f32 / (count - 1) as f32;
                (x, self.eval(x))
            })
            .collect()
    }
}

/// The Bezier segment from `a` to `b` evaluated at `x`: the parameter whose x is `x` is found by bisection.
fn segment(a: &Point, b: &Point, x: f32) -> f32 {
    let (width, height) = (b.x - a.x, b.y - a.y);
    if width <= f32::EPSILON {
        return b.y;
    }
    let c1 = (a.x + a.right[0] * width, a.y + a.right[1] * height);
    let c2 = (b.x - b.left[0] * width, b.y - b.left[1] * height);
    let bezier = |s: f32, p0: f32, p1: f32, p2: f32, p3: f32| {
        let u = 1.0 - s;
        u * u * u * p0 + 3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s * p3
    };
    let (mut low, mut high) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let mid = 0.5 * (low + high);
        if bezier(mid, a.x, c1.0, c2.0, b.x) < x {
            low = mid;
            continue;
        }
        high = mid;
    }
    bezier(0.5 * (low + high), a.y, c1.1, c2.1, b.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f32, y: f32, left: [f32; 2], right: [f32; 2]) -> Point {
        Point { x, y, left, right }
    }

    #[test]
    fn an_empty_curve_is_the_identity() {
        assert_eq!(Curve::default().eval(0.3), 0.3);
        assert!(Curve::default().sample(8).is_empty());
    }

    #[test]
    fn flat_outside_the_points_and_through_them() {
        let curve =
            Curve { points: vec![point(0.2, 0.1, [1.0, 0.0], [0.5, 0.5]), point(0.8, 0.9, [0.5, 0.5], [1.0, 0.0])] };
        assert_eq!(curve.eval(-1.0), 0.1);
        assert_eq!(curve.eval(2.0), 0.9);
        assert!((curve.eval(0.2) - 0.1).abs() < 1e-4);
        assert!((curve.eval(0.8) - 0.9).abs() < 1e-4);
    }

    #[test]
    fn straight_handles_make_a_straight_line() {
        let curve = Curve {
            points: vec![
                point(0.0, 0.0, [1.0, 0.0], [1.0 / 3.0, 1.0 / 3.0]),
                point(1.0, 1.0, [1.0 / 3.0, 1.0 / 3.0], [1.0, 0.0]),
            ],
        };
        for x in [0.1, 0.25, 0.5, 0.9] {
            assert!((curve.eval(x) - x).abs() < 1e-3, "{x}: {}", curve.eval(x));
        }
    }

    #[test]
    fn flat_handles_give_an_s_curve_that_stays_between_the_points() {
        let curve =
            Curve { points: vec![point(0.0, 0.0, [1.0, 0.0], [0.4, 0.0]), point(1.0, 1.0, [0.4, 0.0], [1.0, 0.0])] };
        let samples = curve.sample(9);
        assert_eq!(samples.len(), 9);
        assert!(samples.windows(2).all(|pair| pair[1].1 >= pair[0].1 - 1e-5), "{samples:?}");
        assert!(curve.eval(0.25) < 0.25 && curve.eval(0.75) > 0.75);
    }
}
