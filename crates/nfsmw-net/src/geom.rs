//! Small vector and quaternion helpers for the client's drawn poses. Quaternions are `[x, y, z, w]` and need not be
//! normalised on input; every function that returns one returns it normalised. A degenerate quaternion reads as the
//! identity, so a bad sample can never poison a pose with NaN.

pub type Vec3 = [f32; 3];
pub type Quat = [f32; 4];

/// No rotation.
pub const IDENTITY: Quat = [0.0, 0.0, 0.0, 1.0];

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn length(a: Vec3) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

pub fn distance(a: Vec3, b: Vec3) -> f32 {
    length(sub(a, b))
}

/// `a` at `u = 0` to `b` at `u = 1`.
pub fn lerp(a: Vec3, b: Vec3, u: f32) -> Vec3 {
    add(a, scale(sub(b, a), u))
}

/// Cubic Hermite between `p0` and `p1` with tangents `m0` and `m1` (velocity times the span).
pub fn hermite(p0: Vec3, m0: Vec3, p1: Vec3, m1: Vec3, u: f32) -> Vec3 {
    let u2 = u * u;
    let u3 = u2 * u;
    let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
    let h10 = u3 - 2.0 * u2 + u;
    let h01 = -2.0 * u3 + 3.0 * u2;
    let h11 = u3 - u2;
    let mut out = [0.0; 3];
    for (axis, slot) in out.iter_mut().enumerate() {
        *slot = h00 * p0[axis] + h10 * m0[axis] + h01 * p1[axis] + h11 * m1[axis];
    }
    out
}

fn quat_dot(a: Quat, b: Quat) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

/// `q` at unit length, or the identity when it has no length or is not finite.
pub fn normalize(q: Quat) -> Quat {
    let n = quat_dot(q, q).sqrt();
    if !n.is_finite() || n < 1.0e-12 {
        return IDENTITY;
    }
    [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
}

/// The rotation `a` after `b` (`a * b`).
pub fn mul(a: Quat, b: Quat) -> Quat {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    normalize([
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ])
}

/// The inverse rotation of a unit quaternion.
pub fn conjugate(q: Quat) -> Quat {
    let q = normalize(q);
    [-q[0], -q[1], -q[2], q[3]]
}

/// The angle in radians between two orientations, `0..=π`. `q` and `-q` are the same orientation.
pub fn angle_between(a: Quat, b: Quat) -> f32 {
    let d = quat_dot(normalize(a), normalize(b)).abs().min(1.0);
    2.0 * d.acos()
}

/// Spherical interpolation along the shorter arc.
pub fn slerp(a: Quat, b: Quat, u: f32) -> Quat {
    let a = normalize(a);
    let mut b = normalize(b);
    let mut d = quat_dot(a, b);
    if d < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        d = -d;
    }
    if d > 0.9995 {
        return normalize([
            a[0] + (b[0] - a[0]) * u,
            a[1] + (b[1] - a[1]) * u,
            a[2] + (b[2] - a[2]) * u,
            a[3] + (b[3] - a[3]) * u,
        ]);
    }
    let theta = d.acos();
    let s = theta.sin();
    let wa = ((1.0 - u) * theta).sin() / s;
    let wb = (u * theta).sin() / s;
    normalize([a[0] * wa + b[0] * wb, a[1] * wa + b[1] * wb, a[2] * wa + b[2] * wb, a[3] * wa + b[3] * wb])
}

/// The rotation by the rotation vector `v`: axis times angle in radians.
pub fn from_rotation_vector(v: Vec3) -> Quat {
    let angle = length(v);
    if !angle.is_finite() {
        return IDENTITY;
    }
    if angle < 1.0e-9 {
        return normalize([v[0] * 0.5, v[1] * 0.5, v[2] * 0.5, 1.0]);
    }
    let half = angle * 0.5;
    let s = half.sin() / angle;
    [v[0] * s, v[1] * s, v[2] * s, half.cos()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaw(angle: f32) -> Quat {
        from_rotation_vector([0.0, angle, 0.0])
    }

    #[test]
    fn slerp_takes_the_shorter_arc_and_hits_the_ends() {
        let a = yaw(0.1);
        let b = yaw(0.5);
        assert!(angle_between(slerp(a, b, 0.0), a) < 1.0e-3);
        assert!(angle_between(slerp(a, b, 1.0), b) < 1.0e-3);
        assert!(angle_between(slerp(a, b, 0.5), yaw(0.3)) < 1.0e-3);
        let negated = [-b[0], -b[1], -b[2], -b[3]];
        assert!(angle_between(slerp(a, negated, 0.5), yaw(0.3)) < 1.0e-3);
    }

    #[test]
    fn the_angle_ignores_the_sign_of_the_quaternion() {
        let q = yaw(1.0);
        assert!(angle_between(q, [-q[0], -q[1], -q[2], -q[3]]) < 1.0e-3);
        assert!((angle_between(yaw(0.0), yaw(1.0)) - 1.0).abs() < 1.0e-4);
    }

    #[test]
    fn a_degenerate_quaternion_is_the_identity() {
        assert_eq!(normalize([0.0; 4]), IDENTITY);
        assert_eq!(normalize([f32::NAN, 0.0, 0.0, 1.0]), IDENTITY);
        assert_eq!(from_rotation_vector([f32::INFINITY, 0.0, 0.0]), IDENTITY);
    }

    #[test]
    fn multiplying_by_the_conjugate_gives_the_identity() {
        let q = mul(yaw(0.7), from_rotation_vector([0.3, 0.0, 0.1]));
        assert!(angle_between(mul(q, conjugate(q)), IDENTITY) < 1.0e-3);
    }

    #[test]
    fn hermite_passes_through_its_ends() {
        let p0 = [1.0, 2.0, 3.0];
        let p1 = [4.0, 0.0, -1.0];
        assert_eq!(hermite(p0, [1.0; 3], p1, [2.0; 3], 0.0), p0);
        assert_eq!(hermite(p0, [1.0; 3], p1, [2.0; 3], 1.0), p1);
    }
}
