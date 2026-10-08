/// Splits torque between two outputs `a` and `b` of a differential.
///
/// `bias` is the share given to `a` when open and `factor` the lock factor (0 open, 1 sends everything to
/// the slower side). Without traction on either side, with `locked`, or with `factor <= 0` the split is the
/// plain `(bias, 1 - bias)`.
pub fn calc_split(bias: f32, factor: f32, locked: bool, traction: [bool; 2], omega: [f32; 2]) -> [f32; 2] {
    let open = [bias, 1.0 - bias];
    if !(traction[0] && traction[1]) || locked || factor <= 0.0 {
        return open;
    }
    let av_a = omega[0] * (1.0 - bias);
    let av_b = omega[1] * bias;
    let c = (av_a + av_b).abs();
    if c <= f32::EPSILON {
        return open;
    }
    let split_a = (1.0 - factor) * bias + factor * av_b.abs() / c;
    let split_b = (1.0 - factor) * (1.0 - bias) + factor * av_a.abs() / c;
    [split_a.clamp(0.0, 1.0), split_b.clamp(0.0, 1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_diff_splits_by_bias() {
        assert_eq!(calc_split(0.5, 0.0, false, [true, true], [10.0, 5.0]), [0.5, 0.5]);
        assert_eq!(calc_split(0.3, 1.0, true, [true, true], [10.0, 5.0]), [0.3, 0.7]);
        assert_eq!(calc_split(0.5, 1.0, false, [true, false], [10.0, 5.0]), [0.5, 0.5]);
    }

    #[test]
    fn locked_factor_sends_torque_to_the_slower_wheel() {
        let s = calc_split(0.5, 1.0, false, [true, true], [30.0, 10.0]);
        assert!(s[1] > s[0], "{s:?}");
        assert!((s[0] - 0.25).abs() < 1e-6 && (s[1] - 0.75).abs() < 1e-6);
        let half = calc_split(0.5, 0.5, false, [true, true], [30.0, 10.0]);
        assert!(half[1] > half[0] && half[1] < s[1]);
    }

    #[test]
    fn equal_speeds_split_evenly() {
        let s = calc_split(0.5, 1.0, false, [true, true], [10.0, 10.0]);
        assert!((s[0] - 0.5).abs() < 1e-6);
    }
}
