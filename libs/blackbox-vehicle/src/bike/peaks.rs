//! Where the Magic Formula 94 force of a tyre peaks, at a load and camber: the slips of the negative and positive peak of the
//! longitudinal and of the lateral force. The combination of the two forces is done in these slips (`ride6-tyre.md`,
//! section 3). The game keeps a table of them, sampled by load and camber; the table is a cache of the formula, so this
//! evaluates the formula itself: a coarse scan, then a golden-section refinement around the best sample.

use super::magic::Mf94;

/// The slips at the peaks: the slip ratio (1 = 100 percent) of the longitudinal force and the sine of the slip angle of the
/// lateral force, negative and positive side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Peaks {
    pub x_negative: f32,
    pub x_positive: f32,
    pub y_negative: f32,
    pub y_positive: f32,
}

/// The longitudinal scan covers this slip (percent) either side of zero, the lateral one this angle (degrees). Numerical
/// bounds of the search, wide enough for any road tyre.
const SCAN_PERCENT: f32 = 100.0;
const SCAN_DEGREES: f32 = 45.0;
const SCAN_STEPS: usize = 100;
const REFINE_STEPS: usize = 14;
const GOLDEN: f32 = 0.618_034;

/// The argument in `[low, high]` where `f` is greatest (`sign` 1) or least (`sign` -1).
fn extremum(f: impl Fn(f32) -> f32, low: f32, high: f32, sign: f32) -> f32 {
    let step = (high - low) / SCAN_STEPS as f32;
    let mut best = low;
    let mut best_value = f32::NEG_INFINITY;
    for index in 0..=SCAN_STEPS {
        let x = low + step * index as f32;
        let value = sign * f(x);
        if value > best_value {
            best_value = value;
            best = x;
        }
    }
    let (mut a, mut b) = ((best - step).max(low), (best + step).min(high));
    for _ in 0..REFINE_STEPS {
        let (c, d) = (b - GOLDEN * (b - a), a + GOLDEN * (b - a));
        match sign * f(c) > sign * f(d) {
            true => b = d,
            false => a = c,
        }
    }
    0.5 * (a + b)
}

/// The peak slips of `tyre` at vertical load `fz_kn` and camber `gamma_deg` (the camber the lateral force takes).
pub fn peaks(tyre: &Mf94, fz_kn: f32, gamma_deg: f32) -> Peaks {
    let fz = fz_kn.max(0.01);
    let lateral = |alpha: f32| tyre.lateral(fz, alpha, gamma_deg);
    let longitudinal = |percent: f32| tyre.longitudinal(fz, percent);
    Peaks {
        x_negative: extremum(longitudinal, -SCAN_PERCENT, SCAN_PERCENT, -1.0) * 0.01,
        x_positive: extremum(longitudinal, -SCAN_PERCENT, SCAN_PERCENT, 1.0) * 0.01,
        y_negative: extremum(lateral, -SCAN_DEGREES, SCAN_DEGREES, -1.0).to_radians().sin(),
        y_positive: extremum(lateral, -SCAN_DEGREES, SCAN_DEGREES, 1.0).to_radians().sin(),
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
        let at_peak = tyre.longitudinal(2.0, p.x_positive * 100.0);
        let highest = (0..2000).map(|n| tyre.longitudinal(2.0, n as f32 * 0.05)).fold(0.0, f32::max);
        assert!(at_peak >= highest * 0.999, "{at_peak} {highest}");
        let alpha = p.y_positive.asin().to_degrees();
        let lateral = tyre.lateral(2.0, alpha, 0.0);
        let best = (0..900).map(|n| tyre.lateral(2.0, n as f32 * 0.05, 0.0)).fold(0.0, f32::max);
        assert!(lateral >= best * 0.999, "{lateral} {best}");
    }

    #[test]
    fn a_camber_thrust_moves_the_lateral_peaks() {
        let mut tyre = road_tyre();
        tyre.a[11] = 2.0;
        let flat = peaks(&tyre, 2.0, 0.0);
        let leaned = peaks(&tyre, 2.0, 30.0);
        assert!(
            (flat.y_positive - leaned.y_positive).abs() > 1e-4 || (flat.y_negative - leaned.y_negative).abs() > 1e-4
        );
    }

    #[test]
    fn a_tyre_with_no_coefficients_still_gives_finite_peaks() {
        let tyre = Mf94 { a: [0.0; 15], b: [0.0; 11], c: [0.0; 18] };
        let p = peaks(&tyre, 2.0, 0.0);
        assert!(p.x_negative.is_finite() && p.y_positive.is_finite());
    }
}
