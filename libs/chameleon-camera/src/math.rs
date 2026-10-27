//! Small vector helpers on `[f32; 3]` in host space (x forward, y left, z up). Every function returns finite numbers
//! for finite input, and the normalising ones take a fallback for a zero vector.

use crate::params::Vec3;

/// World up in host space.
pub const WORLD_UP: Vec3 = [0.0, 0.0, 1.0];

/// Miles per hour in one metre per second.
pub const MPS_TO_MPH: f32 = 2.236_936;

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: Vec3, k: f32) -> Vec3 {
    [a[0] * k, a[1] * k, a[2] * k]
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

/// `a` scaled to unit length, or `fallback` when `a` is (nearly) zero or not finite.
pub fn normalize_or(a: Vec3, fallback: Vec3) -> Vec3 {
    let len = length(a);
    if !len.is_finite() || len < 1e-6 {
        return fallback;
    }
    scale(a, 1.0 / len)
}

/// `a` with every non-finite component replaced by the one of `fallback`.
pub fn finite_or(a: Vec3, fallback: Vec3) -> Vec3 {
    std::array::from_fn(|n| if a[n].is_finite() { a[n] } else { fallback[n] })
}

pub fn finite_f32(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// `a` rotated about the unit vector `axis` by `radians` (right-handed), Rodrigues' formula.
pub fn rotate_about(a: Vec3, axis: Vec3, radians: f32) -> Vec3 {
    let (sin, cos) = radians.sin_cos();
    let parallel = scale(axis, dot(axis, a) * (1.0 - cos));
    add(add(scale(a, cos), scale(cross(axis, a), sin)), parallel)
}

/// `a` turned about +z (the heading) by `radians`.
pub fn rotate_z(a: Vec3, radians: f32) -> Vec3 {
    let (sin, cos) = radians.sin_cos();
    [a[0] * cos - a[1] * sin, a[0] * sin + a[1] * cos, a[2]]
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

/// The share of the way from `low` to `high` that `value` has made, in 0..=1 (0 when the range is empty).
pub fn ramp(value: f32, low: f32, high: f32) -> f32 {
    let width = high - low;
    if width.abs() < 1e-6 {
        return 0.0;
    }
    clamp01((value - low) / width)
}

/// The weight an exponential filter with time constant `tau` gives the new sample over `dt` (1 when `tau` is 0).
pub fn filter_weight(dt: f32, tau: f32) -> f32 {
    if tau <= 1e-6 {
        return 1.0;
    }
    1.0 - (-dt / tau).exp()
}

/// An angle wrapped to -pi..pi.
pub fn wrap_angle(radians: f32) -> f32 {
    let turn = std::f32::consts::TAU;
    let wrapped = (radians + std::f32::consts::PI).rem_euclid(turn) - std::f32::consts::PI;
    finite_f32(wrapped, 0.0)
}

/// One step of a critically damped spring on an angle: `(angle, velocity)` after `dt`. Stable for any `dt`; the
/// angle approaches `target` along the shorter way and does not overshoot it when it starts at rest.
pub fn spring_angle(angle: f32, velocity: f32, target: f32, omega: f32, dt: f32) -> (f32, f32) {
    let change = wrap_angle(angle - target);
    let decay = (-omega * dt).exp();
    let temp = (velocity + omega * change) * dt;
    let new_velocity = (velocity - omega * temp) * decay;
    let new_change = (change + temp) * decay;
    (wrap_angle(target + new_change), new_velocity)
}

/// The vertical field of view that goes with a horizontal one at `aspect` (width over height), in degrees.
pub fn vertical_fov_deg(horizontal_deg: f32, aspect: f32) -> f32 {
    let aspect = if aspect.is_finite() && aspect > 0.1 { aspect } else { 16.0 / 9.0 };
    let half = (horizontal_deg.clamp(1.0, 175.0).to_radians() * 0.5).tan();
    2.0 * (half / aspect).atan().to_degrees()
}
