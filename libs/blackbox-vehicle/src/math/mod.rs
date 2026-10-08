//! Small numeric helpers shared by every module: ramps, lerps, evenly spaced tables, piecewise graphs,
//! and unit conversions. Vectors, quaternions and matrices come from `glam`.

mod units;

pub use units::*;

/// Linear interpolation from `a` to `b` by `t` (not clamped).
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// `clamp((x - lo) / (hi - lo), 0, 1)`; 0 when the range is empty or reversed.
pub fn ramp(x: f32, lo: f32, hi: f32) -> f32 {
    if hi - lo <= 0.0 { 0.0 } else { ((x - lo) / (hi - lo)).clamp(0.0, 1.0) }
}

/// Lookup in `values` taken at evenly spaced points from `lo` to `hi`, linearly interpolated and
/// clamped to the first and last value outside the range. An empty slice gives 0, one value is constant.
pub fn table(values: &[f32], lo: f32, hi: f32, x: f32) -> f32 {
    match values {
        [] => 0.0,
        [only] => *only,
        _ => {
            let last = values.len() - 1;
            if hi <= lo {
                return values[0];
            }
            let pos = (last as f32 * (x - lo) / (hi - lo)).clamp(0.0, last as f32);
            let i = (pos.floor() as usize).min(last);
            let t = pos - i as f32;
            lerp(values[i], values[(i + 1).min(last)], t)
        }
    }
}

/// Piecewise-linear graph through `(x, y)` points sorted by `x`, clamped to the end values.
pub fn graph(points: &[(f32, f32)], x: f32) -> f32 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else { return 0.0 };
    if x <= first.0 {
        return first.1;
    }
    if x >= last.0 {
        return last.1;
    }
    for w in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if x <= x1 {
            return if x1 > x0 { lerp(y0, y1, (x - x0) / (x1 - x0)) } else { y1 };
        }
    }
    last.1
}

/// Replaces non-finite values (NaN, infinities) by `fallback`. Used on outputs that feed back into state.
pub fn finite_or(x: f32, fallback: f32) -> f32 {
    if x.is_finite() { x } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_clamps() {
        assert_eq!(ramp(-1.0, 0.0, 2.0), 0.0);
        assert_eq!(ramp(1.0, 0.0, 2.0), 0.5);
        assert_eq!(ramp(5.0, 0.0, 2.0), 1.0);
        assert_eq!(ramp(1.0, 2.0, 2.0), 0.0);
    }

    #[test]
    fn table_interpolates_and_clamps() {
        let t = [0.0, 10.0, 30.0];
        assert_eq!(table(&t, 0.0, 2.0, -5.0), 0.0);
        assert_eq!(table(&t, 0.0, 2.0, 0.5), 5.0);
        assert_eq!(table(&t, 0.0, 2.0, 1.5), 20.0);
        assert_eq!(table(&t, 0.0, 2.0, 9.0), 30.0);
        assert_eq!(table(&[], 0.0, 1.0, 0.3), 0.0);
        assert_eq!(table(&[4.0], 0.0, 1.0, 0.3), 4.0);
    }

    #[test]
    fn graph_interpolates() {
        let g = [(0.0, 0.0), (1.0, 2.0), (3.0, 2.0)];
        assert_eq!(graph(&g, -1.0), 0.0);
        assert_eq!(graph(&g, 0.5), 1.0);
        assert_eq!(graph(&g, 2.0), 2.0);
        assert_eq!(graph(&g, 9.0), 2.0);
    }

    #[test]
    fn rpm_round_trip() {
        let rpm = 6500.0;
        assert!((rad_to_rpm(rpm_to_rad(rpm)) - rpm).abs() < 0.5);
    }
}
