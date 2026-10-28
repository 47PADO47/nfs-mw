//! Piecewise-linear curves of `(x, y)` points, the form the controllers' tuning tables take.

/// The value of a `(x, y)` curve at `x`, held flat outside its ends; 0 for an empty curve.
pub fn curve(points: &[(f32, f32)], x: f32) -> f32 {
    let Some(&(first_x, first_y)) = points.first() else { return 0.0 };
    if x <= first_x {
        return first_y;
    }
    for pair in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if x <= x1 {
            return y0 + (y1 - y0) * ((x - x0) / (x1 - x0).max(1e-6));
        }
    }
    points.last().map_or(0.0, |p| p.1)
}

/// Like [`curve`], but an empty curve is the factor 1 (a tuning table that is not there changes nothing).
pub fn factor(points: &[(f32, f32)], x: f32) -> f32 {
    match points.is_empty() {
        true => 1.0,
        false => curve(points, x),
    }
}

/// `x` mapped from `[from.0, from.1]` to `[0, 1]`, held at the ends. A range of zero width is a step at its start.
pub fn ramp(x: f32, from: [f32; 2]) -> f32 {
    let width = from[1] - from[0];
    if width.abs() < 1e-9 {
        return f32::from(x >= from[0]);
    }
    ((x - from[0]) / width).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_interpolates_and_holds_its_ends() {
        let points = [(0.0, 0.0), (10.0, 100.0)];
        assert_eq!(curve(&points, -5.0), 0.0);
        assert_eq!(curve(&points, 5.0), 50.0);
        assert_eq!(curve(&points, 50.0), 100.0);
        assert_eq!(curve(&[], 1.0), 0.0);
        assert_eq!(factor(&[], 1.0), 1.0);
    }

    #[test]
    fn a_ramp_runs_either_way_and_a_flat_one_is_a_step() {
        assert_eq!(ramp(0.5, [0.0, 1.0]), 0.5);
        assert_eq!(ramp(-0.3, [-0.3, -0.5]), 0.0);
        assert!((ramp(-0.4, [-0.3, -0.5]) - 0.5).abs() < 1e-6);
        assert_eq!(ramp(3.0, [2.0, 2.0]), 1.0);
        assert_eq!(ramp(1.0, [2.0, 2.0]), 0.0);
    }
}
