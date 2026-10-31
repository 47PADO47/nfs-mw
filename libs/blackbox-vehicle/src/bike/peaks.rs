//! Where the Magic Formula 94 force of a tyre peaks, at a load and camber: the slips of the negative and positive peak of the
//! longitudinal and of the lateral force (`docs/specs/ride6-tyre-mf94.md`, section 3). The normalised-slip combination of the
//! two forces is done in these slips (`ride6-tyre.md`, section 3).
//!
//! The game interpolates a table of them (and solves exactly outside it, the load is not clamped); the table is a cache of the
//! formula, so this solves the root equation `E atan(B x) + (1 - E) B x = tan(pi / (2 C))` directly (`Shape::peak_offset`).

use super::magic::Mf94;

/// The slips at the peaks, in the units the wheel feeds the formula: the slip ratio (1 = 100 percent) of the longitudinal
/// force and the slip angle in radians of the lateral force (the wheel's `Slip.y`, the sine of the slip angle, is used as
/// if it were that angle), negative and positive side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Peaks {
    pub x_negative: f32,
    pub x_positive: f32,
    pub y_negative: f32,
    pub y_positive: f32,
}

/// Percent to ratio, and degrees to radians, as the binary writes them (`0.01`, `0.0174533`).
const PERCENT: f32 = 0.01;
const DEGREE: f32 = 0.017_453_3;

/// The peak slips of `tyre` at vertical load `fz_kn` and camber `gamma_deg` (the camber the lateral force takes).
pub fn peaks(tyre: &Mf94, fz_kn: f32, gamma_deg: f32) -> Peaks {
    let longitudinal = tyre.longitudinal_shape(fz_kn);
    let lateral = tyre.lateral_shape(fz_kn, gamma_deg);
    let (x_s, y_s) = (longitudinal.peak_offset(), lateral.peak_offset());
    Peaks {
        x_negative: (-x_s - longitudinal.sh) * PERCENT,
        x_positive: (x_s - longitudinal.sh) * PERCENT,
        y_negative: (-y_s - lateral.sh) * DEGREE,
        y_positive: (y_s - lateral.sh) * DEGREE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::magic::tests::road_tyre;

    #[test]
    fn the_peaks_sit_on_either_side_of_zero_slip() {
        let p = peaks(&road_tyre(), 2.0, 0.0);
        assert!(p.x_negative < -0.02 && p.x_positive > 0.02, "{p:?}");
        assert!(p.y_negative < -0.02 && p.y_positive > 0.02, "{p:?}");
        assert!((p.x_positive + p.x_negative).abs() < 0.02);
    }

    #[test]
    fn the_force_at_a_peak_is_the_greatest_the_curve_gives() {
        let tyre = road_tyre();
        let p = peaks(&tyre, 2.0, 0.0);
        let at_peak = tyre.longitudinal(2.0, p.x_positive / PERCENT);
        let highest = (0..2000).map(|n| tyre.longitudinal(2.0, n as f32 * 0.05)).fold(0.0, f32::max);
        assert!(at_peak >= highest * 0.999, "{at_peak} {highest}");
        let lateral = tyre.lateral(2.0, p.y_positive / DEGREE, 0.0);
        let best = (0..900).map(|n| tyre.lateral(2.0, n as f32 * 0.05, 0.0)).fold(0.0, f32::max);
        assert!(lateral >= best * 0.999, "{lateral} {best}");
    }

    /// The test values of the spec (the KTM front tyre, row `Tarmac`, computed from the equations).
    #[test]
    fn the_peaks_match_the_values_the_spec_computes() {
        let mut a = [0.0; 15];
        a[..11].copy_from_slice(&[1.2, -10.0, 1350.0, 1200.0, 6.0, 0.002, 0.0, 0.0, -0.01, 0.0, 0.0]);
        a[11] = 2.0;
        let mut b = [0.0; 11];
        b.copy_from_slice(&[1.5, -10.0, 1250.0, 0.0, 200.0, 0.0, -0.1, 0.0, 0.1, 0.0, 0.0]);
        let tyre = Mf94 { a, b, c: [0.0; 18] };
        let p = peaks(&tyre, 1.0, 0.0);
        assert!((p.x_positive - 0.161_081).abs() < 5e-4, "{}", p.x_positive);
        assert!((p.y_positive - 0.269_13).abs() < 5e-4, "{}", p.y_positive);
    }

    #[test]
    fn a_camber_thrust_moves_the_lateral_peaks() {
        let mut tyre = road_tyre();
        tyre.a[8] = -0.05;
        let flat = peaks(&tyre, 2.0, 0.0);
        let leaned = peaks(&tyre, 2.0, 30.0);
        assert!((flat.y_positive - leaned.y_positive).abs() > 1e-4);
    }

    #[test]
    fn a_tyre_with_no_coefficients_still_gives_finite_peaks() {
        let tyre = Mf94 { a: [0.0; 15], b: [0.0; 11], c: [0.0; 18] };
        let p = peaks(&tyre, 2.0, 0.0);
        assert!(p.x_negative.is_finite() && p.y_positive.is_finite());
    }
}
