//! The integer conversions of the original, on `f32`. They are the reference for every rounding in the
//! synthesiser, so the grain choices land on the same samples.

/// Nearest integer, ties away from zero (`x > 0 ? trunc(x + 0.5) : trunc(x - 0.5)`).
pub(crate) fn round(x: f32) -> i32 {
    if x > 0.0 {
        return (x + 0.5) as i32;
    }
    (x - 0.5) as i32
}

/// Largest integer not above `x`.
pub(crate) fn floor(x: f32) -> i32 {
    x.floor() as i32
}

/// Smallest integer not below `x`.
pub(crate) fn ceil(x: f32) -> i32 {
    x.ceil() as i32
}
