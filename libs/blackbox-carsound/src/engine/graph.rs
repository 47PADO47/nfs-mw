//! The 7-point polyline a shift stage's Bezier curve is sampled into.

use crate::math::bezier_point;
use crate::tuning::ShiftStage;

const POINTS: usize = 7;

/// A polyline over milliseconds: `(x ms, y rpm)`.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Graph {
    points: [[f32; 2]; POINTS],
}

impl Graph {
    /// The y at `x`, held at the first and last point outside the polyline.
    pub fn value(&self, x: f32) -> f32 {
        let first = self.points[0];
        let last = self.points[POINTS - 1];
        if x.is_nan() || x <= first[0] {
            return first[1];
        }
        if x >= last[0] {
            return last[1];
        }
        for pair in self.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if x < a[0] || x >= b[0] {
                continue;
            }
            let dx = b[0] - a[0];
            if dx.abs() > 1e-6 {
                return a[1] + (x - a[0]) / dx * (b[1] - a[1]);
            }
            return a[1] + (b[1] - a[1]) * 0.5;
        }
        first[1]
    }

    /// The x of the last point: the stage's length in milliseconds.
    pub fn end_ms(&self) -> f32 {
        self.points[POINTS - 1][0]
    }
}

/// Samples the stage's curve at 7 equally spaced parameters, scaling x by `time_ms` and y by `rpm`.
pub(super) fn fill_graph(stage: &ShiftStage) -> Graph {
    let mut graph = Graph::default();
    for (k, point) in graph.points.iter_mut().enumerate() {
        let t = k as f32 / (POINTS - 1) as f32;
        let p = bezier_point(&stage.curve, t);
        *point = [p[0] * stage.time_ms as f32, p[1] * stage.rpm as f32];
    }
    graph
}
