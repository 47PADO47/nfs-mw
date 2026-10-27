use crate::math::{Vec3, cross, dot, normalize, scale, sub};

fn side(a: Vec3, b: Vec3, p: Vec3) -> f32 {
    (a[0] - b[0]) * (p[2] - b[2]) - (a[2] - b[2]) * (p[0] - b[0])
}

pub(super) fn contains_winding(pts: [Vec3; 3], point: Vec3, right: bool) -> bool {
    let values = [side(pts[0], pts[1], point), side(pts[1], pts[2], point), side(pts[2], pts[0], point)];
    if right {
        return values.into_iter().all(|value| value <= 0.0);
    }
    values.into_iter().all(|value| value >= 0.0)
}

/// Exact horizontal edge tests, including the asymmetric boundary winding rule.
pub(super) fn contains(pts: [Vec3; 3], point: Vec3) -> bool {
    let values = [side(pts[0], pts[1], point), side(pts[1], pts[2], point), side(pts[2], pts[0], point)];
    if values[0] > 0.0 {
        return values[1] >= 0.0 && values[2] >= 0.0;
    }
    values[1] <= 0.0 && values[2] <= 0.0
}

pub(super) fn normal(pts: [Vec3; 3], maximum_y: f32) -> Vec3 {
    let mut n = normalize(cross(sub(pts[1], pts[0]), sub(pts[0], pts[2]))).unwrap_or([0.0, 1.0, 0.0]);
    if n[1] < 0.0 {
        n = scale(n, -1.0);
    }
    n[1] = n[1].min(maximum_y);
    n
}

pub(super) fn height(point: Vec3, normal: Vec3, query: Vec3) -> f32 {
    if normal[1] == 0.0 {
        return point[1];
    }
    point[1] - ((query[0] - point[0]) * normal[0] + (query[2] - point[2]) * normal[2]) / normal[1]
}

/// Signed plane distance retains the original Z/Y-then-X accumulation order.
pub(super) fn separation(point: Vec3, query: Vec3, normal: Vec3) -> f32 {
    ((point[2] - query[2]) * normal[2] + (point[1] - query[1]) * normal[1]) + (point[0] - query[0]) * normal[0]
}

/// Horizontal box and infinite-edge-line cylinder tests; no vertical cutoff.
pub(super) fn overlaps(pts: [Vec3; 3], center: Vec3, radius: f32, epsilon: f32) -> bool {
    for axis in [0, 2] {
        let lo = pts.iter().map(|p| p[axis]).fold(f32::INFINITY, f32::min);
        let hi = pts.iter().map(|p| p[axis]).fold(f32::NEG_INFINITY, f32::max);
        if hi < center[axis] - radius || lo > center[axis] + radius {
            return false;
        }
    }
    let winding = side(pts[0], pts[1], pts[2]);
    for (a, b) in [(pts[0], pts[1]), (pts[1], pts[2]), (pts[2], pts[0])] {
        if side(a, b, center) * winding > 0.0 {
            continue;
        }
        let edge = [b[0] - a[0], 0.0, b[2] - a[2]];
        let offset = [center[0] - a[0], 0.0, center[2] - a[2]];
        let length = dot(edge, edge);
        let mut projection = 0.0;
        if length > epsilon {
            projection = dot(offset, edge) / length;
        }
        let delta = sub(offset, scale(edge, projection));
        if dot(delta, delta) >= radius * radius {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn separation_matches_an_independent_original_instruction_fixture() {
        let actual = super::separation([0.1, 0.1, 0.3], [0.0; 3], [0.2, 0.96, 0.17]);
        assert_eq!(actual.to_bits(), 0x3e2b020c);
    }
}
