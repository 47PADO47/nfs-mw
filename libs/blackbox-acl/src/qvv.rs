//! The value a track decodes to, and the small quaternion helpers the decoder needs.
//!
//! Rotations are `[x, y, z, w]`. The functions keep the order of the floating point operations of ACL's own so that
//! the results match its decoder to the last bits.

/// One transform: rotation, translation, scale (what ACL calls a `qvvf`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Qvv {
    /// Unit quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
    pub translation: [f32; 3],
    pub scale: [f32; 3],
}

impl Qvv {
    /// No rotation, no translation, unit scale.
    pub const IDENTITY: Self = Self { rotation: [0.0, 0.0, 0.0, 1.0], translation: [0.0; 3], scale: [1.0; 3] };
}

/// The quaternion whose `x, y, z` are given, with the positive `w` that makes it unit length.
pub(crate) fn from_positive_w(v: [f32; 3]) -> [f32; 4] {
    let w_squared = ((1.0 - v[0] * v[0]) - v[1] * v[1]) - v[2] * v[2];
    [v[0], v[1], v[2], w_squared.abs().sqrt()]
}

fn dot(a: [f32; 4], b: [f32; 4]) -> f32 {
    let mut sum = a[0] * b[0];
    sum += a[1] * b[1];
    sum += a[2] * b[2];
    sum + a[3] * b[3]
}

/// `q / |q|`.
pub(crate) fn normalize(q: [f32; 4]) -> [f32; 4] {
    let inv = 1.0 / dot(q, q).sqrt();
    [q[0] * inv, q[1] * inv, q[2] * inv, q[3] * inv]
}

/// Normalised linear interpolation along the shorter arc.
pub(crate) fn lerp(a: [f32; 4], b: [f32; 4], alpha: f32) -> [f32; 4] {
    let bias = if dot(a, b) >= 0.0 { 1.0 } else { -1.0 };
    let mut out = [0.0; 4];
    for ((o, &x), &y) in out.iter_mut().zip(&a).zip(&b) {
        *o = (y * bias) * alpha + (x - x * alpha);
    }
    normalize(out)
}

/// Component-wise linear interpolation.
pub(crate) fn lerp3(a: [f32; 3], b: [f32; 3], alpha: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * alpha, a[1] + (b[1] - a[1]) * alpha, a[2] + (b[2] - a[2]) * alpha]
}
