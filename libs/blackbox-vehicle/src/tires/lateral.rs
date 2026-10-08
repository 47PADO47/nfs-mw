use crate::math::table;

/// Lateral force families, one per two degrees of slip angle (0, 2, ..., 12), each sampled at the corrected
/// load `x = 0, 2, 4, 6, 8, 10`.
const FAMILIES: [[f32; 6]; 7] = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.2, 2.3, 3.0, 3.0, 2.8],
    [0.0, 1.7, 3.2, 4.3, 5.1, 5.2],
    [0.0, 1.8, 3.5, 4.9, 5.8, 6.1],
    [0.0, 1.83, 3.6, 5.0, 5.96, 6.4],
    [0.0, 1.86, 3.7, 5.1, 6.13, 6.7],
    [0.0, 1.9, 3.8, 5.2, 6.3, 7.1],
];

/// Lateral force magnitude (N) of a tire at `load` (N) and absolute `slip_angle` (turns, 1.0 = 360 deg).
///
/// Roughly linear in load at small loads and saturating (doubling the load gives less than double the
/// force), rising with slip angle toward a plateau near 12 degrees. `grip_scale` is the tire's
/// `GRIP_SCALE` and `grip_boost` the per-step lateral scale.
pub fn lateral_force(load: f32, slip_angle: f32, grip_scale: f32, grip_boost: f32) -> f32 {
    let angle_deg = slip_angle.abs() * 360.0;
    let seg = angle_deg * 0.5;
    let k = seg.floor();
    let frac = seg - k;
    let x = load.max(0.0) * 0.001 * 0.8;
    let family = |i: usize| table(&FAMILIES[i], 0.0, 10.0, x);
    let value = if k >= 6.0 {
        family(6)
    } else {
        let k = k as usize;
        family(k) + frac * (family(k + 1) - family(k))
    };
    grip_scale * 1000.0 * grip_boost * 2.5 * value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_slip_no_force() {
        assert_eq!(lateral_force(4000.0, 0.0, 1.0, 1.0), 0.0);
    }

    #[test]
    fn matches_the_table_at_a_node() {
        // 4 degrees = 1/90 turn, load giving x = 4 (5000 N): family row 2, value 3.2.
        let f = lateral_force(5000.0, 4.0 / 360.0, 1.0, 1.0);
        assert!((f - 3.2 * 2500.0).abs() < 1.0, "{f}");
    }

    #[test]
    fn saturates_with_load_and_angle() {
        let a = lateral_force(4000.0, 10.0 / 360.0, 1.0, 1.0);
        let b = lateral_force(8000.0, 10.0 / 360.0, 1.0, 1.0);
        assert!(b > a && b < 2.0 * a, "{a} {b}");
        let plateau = lateral_force(4000.0, 30.0 / 360.0, 1.0, 1.0);
        assert_eq!(plateau, lateral_force(4000.0, 90.0 / 360.0, 1.0, 1.0));
    }

    #[test]
    fn scales_linearly_with_grip() {
        let base = lateral_force(4000.0, 6.0 / 360.0, 1.0, 1.0);
        assert!((lateral_force(4000.0, 6.0 / 360.0, 1.2, 1.5) - base * 1.8).abs() < 1e-2);
    }
}
