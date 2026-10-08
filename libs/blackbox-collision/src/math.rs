//! Small `[f32; 3]` vector helpers.

pub type Vec3 = [f32; 3];

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub fn length(a: Vec3) -> f32 {
    dot(a, a).sqrt()
}

/// `a` scaled to length 1, or `None` when it has no length.
pub fn normalize(a: Vec3) -> Option<Vec3> {
    let l = length(a);
    (l > 0.0).then(|| scale(a, 1.0 / l))
}

pub fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    add(a, scale(sub(b, a), t))
}

/// Squared distance from `p` to the segment `a`-`b`.
pub fn segment_point_dist_sq(a: Vec3, b: Vec3, p: Vec3) -> f32 {
    let d = sub(b, a);
    let len_sq = dot(d, d);
    let t = if len_sq > 0.0 { (dot(sub(p, a), d) / len_sq).clamp(0.0, 1.0) } else { 0.0 };
    let q = add(a, scale(d, t));
    let r = sub(p, q);
    dot(r, r)
}

/// Same as [`segment_point_dist_sq`] but only in the xz plane.
pub fn segment_point_dist_sq_xz(a: Vec3, b: Vec3, p: Vec3) -> f32 {
    segment_point_dist_sq([a[0], 0.0, a[2]], [b[0], 0.0, b[2]], [p[0], 0.0, p[2]])
}

/// Fraction `t` along `a`-`b` where it crosses the triangle, if it does (both faces count).
/// Edges are inclusive within a small tolerance, so neighbouring triangles leave no crack.
pub fn segment_triangle(a: Vec3, b: Vec3, tri: &[Vec3; 3]) -> Option<f32> {
    const EPS: f32 = 1e-6;
    let dir = sub(b, a);
    let e1 = sub(tri[1], tri[0]);
    let e2 = sub(tri[2], tri[0]);
    let p = cross(dir, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = sub(a, tri[0]);
    let u = dot(s, p) * inv;
    if !(-EPS..=1.0 + EPS).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(dir, q) * inv;
    if v < -EPS || u + v > 1.0 + EPS {
        return None;
    }
    let t = dot(e2, q) * inv;
    (-EPS..=1.0 + EPS).contains(&t).then_some(t.clamp(0.0, 1.0))
}

/// Where the xz projections of segments `a0`-`a1` and `b0`-`b1` cross: the fraction along the
/// first segment. `None` when parallel or apart.
pub fn segment_segment_xz(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> Option<f32> {
    const EPS: f32 = 1e-4;
    let (dx1, dz1) = (a1[0] - a0[0], a1[2] - a0[2]);
    let (dx2, dz2) = (b1[0] - b0[0], b1[2] - b0[2]);
    let den = dz2 * dx1 - dx2 * dz1;
    if den == 0.0 {
        return None;
    }
    let (ex, ez) = (a0[0] - b0[0], a0[2] - b0[2]);
    let t = (dx2 * ez - dz2 * ex) / den;
    let u = (dx1 * ez - dz1 * ex) / den;
    ((-EPS..=1.0 + EPS).contains(&t) && (-EPS..=1.0 + EPS).contains(&u)).then_some(t.clamp(0.0, 1.0))
}
