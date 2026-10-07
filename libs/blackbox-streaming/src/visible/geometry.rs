//! Point tests against a boundary polygon (`docs/specs/visible-sections.md`).

use super::Boundary;

impl Boundary {
    /// Whether the point is inside the bounding rectangle and the polygon (even-odd rule, with
    /// each edge covering the half-open y range between its ends).
    pub fn contains(&self, p: [f32; 2]) -> bool {
        let in_box = (0..2).all(|i| self.bbox_min[i] <= p[i] && p[i] <= self.bbox_max[i]);
        if !in_box || self.points.is_empty() {
            return false;
        }
        let [x, y] = p;
        let mut inside = false;
        let mut j = self.points.len() - 1;
        for (i, a) in self.points.iter().enumerate() {
            let b = self.points[j];
            if ((a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1]))
                && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0]
            {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    /// 0 inside, otherwise the distance to the nearest edge.
    pub fn distance_outside(&self, p: [f32; 2]) -> f32 {
        if self.contains(p) {
            return 0.0;
        }
        let n = self.points.len();
        (0..n).map(|i| segment_distance(p, self.points[i], self.points[(i + 1) % n])).fold(f32::INFINITY, f32::min)
    }
}

fn segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (ab, ap) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if len2 > 0.0 { ((ap[0] * ab[0] + ap[1] * ab[1]) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let d = [ap[0] - ab[0] * t, ap[1] - ab[1] * t];
    (d[0] * d[0] + d[1] * d[1]).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> Boundary {
        Boundary {
            section: 101,
            panorama: false,
            bbox_min: [0.0, 0.0],
            bbox_max: [10.0, 10.0],
            centre: [5.0, 5.0],
            points: vec![[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]],
        }
    }

    #[test]
    fn inside_and_distance() {
        let t = triangle();
        assert!(t.contains([2.0, 2.0]));
        assert!(!t.contains([8.0, 8.0])); // in the box, outside the polygon
        assert!(!t.contains([-1.0, 2.0]));
        assert_eq!(t.distance_outside([2.0, 2.0]), 0.0);
        assert!((t.distance_outside([-3.0, 5.0]) - 3.0).abs() < 1e-5);
        assert!((t.distance_outside([6.0, 6.0]) - 2.0f32.sqrt()).abs() < 1e-5);
        assert!((t.distance_outside([13.0, -4.0]) - 5.0).abs() < 1e-5); // nearest is a corner
    }
}
