//! Small numeric helpers shared by the controllers. All of them tolerate NaN and infinities: a non-finite
//! input is read as zero, so no controller can poison its state.

/// `x` if finite, else 0.
pub(crate) fn finite(x: f32) -> f32 {
    if x.is_finite() { x } else { 0.0 }
}

/// `a + t * (b - a)`.
pub(crate) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Moves `current` toward `target` by at most `step` (the original's `smooth(curr, target, delta)`).
pub(crate) fn slew(current: f32, target: f32, step: f32) -> f32 {
    slew_asym(current, target, step, step)
}

/// Moves `current` toward `target` by at most `up` going up and `down` going down.
pub(crate) fn slew_asym(current: f32, target: f32, up: f32, down: f32) -> f32 {
    if target > current + up {
        return current + up;
    }
    if target < current - down {
        return current - down;
    }
    target
}

/// A linear ramp: `min` at `start` and below, `max` at `finish` and above (the original's `Slope`).
/// A zero-width ramp is a step at `start`.
pub(crate) fn ramp(x: f32, start: f32, finish: f32, min: f32, max: f32) -> f32 {
    let width = finish - start;
    if width.abs() < 1e-6 {
        return if x >= start { max } else { min };
    }
    let t = ((x - start) / width).clamp(0.0, 1.0);
    t * (max - min) + min
}

/// The `y` of a cubic Bezier with control-point heights `y` at parameter `t` (clamped to 0..1).
pub(crate) fn bezier_y(y: [f32; 4], t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let u = 1.0 - t;
    u * u * u * y[0] + 3.0 * t * u * u * y[1] + 3.0 * t * t * u * y[2] + t * t * t * y[3]
}

/// A point of a cubic Bezier with control points `(x, y)` at parameter `t`.
pub(crate) fn bezier_point(points: &[[f32; 2]; 4], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * t * u * u, 3.0 * t * t * u, t * t * t];
    let mut out = [0.0; 2];
    for (weight, p) in w.iter().zip(points) {
        out[0] += weight * p[0];
        out[1] += weight * p[1];
    }
    out
}

/// The equal-power curve squared, `sin^2(pi/2 * x)`, for `x` in 0..1 (the original's `EQ_PWR_SQ`).
pub(crate) fn equal_power_sq(x: f32) -> f32 {
    let s = (x.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2).sin();
    s * s
}

/// `value / 32767` clamped to 0..1: a Q15 volume as a linear gain.
pub(crate) fn q15_gain(value: f32) -> f32 {
    (finite(value) / 32767.0).clamp(0.0, 1.0)
}
