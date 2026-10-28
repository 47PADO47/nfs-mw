//! The automatic gearbox's change points, generated from the torque curve and the gear ratios as the reference game does
//! (`docs/specs/ride6-gearbox-auto.md`, section 1), and reduced to the two shares of the rev limit the manifest takes.
//!
//! For each pair of gears `n`, `n + 1` the reference sweeps the engine speed `w` from the rpm of peak torque to the rev limit
//! in steps of 1 % of `MaxRPM - MinRPM`. With `r = G(n+1) / G(n)`, `t1 = T(w)` the relative wheel force now and
//! `t2 = T(w r) r` after the change:
//!
//! - the change-up point is the first `w / MaxRPM` where `t1 * up_tolerance < t2` (1.0 if there is none);
//! - the change-down point of gear `n + 1` is the last `w r / MaxRPM` where `t2 < t1 * down_tolerance` (the first if none).
//!
//! The manifest has one change-up and one change-down share for all gears, both of the engine speed in the gear the bike is
//! in. The change-up shares are the generated points; the change-down point of gear `n + 1` is compared in the lower gear
//! (the default mode of the handler), so in the gear it leaves it is `point * G(n+1) / G(n)` of the rev limit. The importer
//! writes the **lowest** of each: no gear changes up or down sooner than the reference would (a named choice, documented in
//! `docs/modding/ride6-bikes.md`).

use super::curve::eval;
use super::defaults;

/// The reduced change points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shift {
    pub up: f64,
    pub down: Option<f64>,
}

/// The tolerances of one gear change: `ChangeUpTolerance` and `ChangeDownTolerance` of the lower gear's row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    pub up: f64,
    pub down: f64,
}

/// Reduce the generated change points of a bike.
///
/// `curve` is `(rpm, share of the peak)`; `tolerances[n]` belongs to the change from gear `n + 1` to `n + 2` (one fewer than
/// the gears); `idle_rpm` is `MinRPM`.
pub fn reduce(curve: &[[f64; 2]], gears: &[f64], tolerances: &[Tolerance], idle_rpm: f64, max_rpm: f64) -> Shift {
    let changes = gears.len().saturating_sub(1).min(tolerances.len());
    if changes == 0 {
        return Shift { up: defaults::UP_SHIFT_WITHOUT_GEARS, down: None };
    }
    // The rpm of the curve's maximum: where the sweep starts.
    let start = curve.iter().fold((f64::MIN, 0.0), |best, p| if p[1] > best.0 { (p[1], p[0]) } else { best }).1;
    let step = (max_rpm - idle_rpm) * 0.01;
    let mut up = f64::INFINITY;
    let mut down = f64::INFINITY;
    for (n, tolerance) in tolerances.iter().take(changes).enumerate() {
        let ratio = gears[n + 1] / gears[n];
        let (mut up_point, mut down_point) = (1.0, 0.0);
        let mut w = start;
        while w < max_rpm && step > 0.0 {
            let t1 = eval(curve, w);
            let t2 = eval(curve, w * ratio) * ratio;
            if up_point == 1.0 && t1 * tolerance.up < t2 {
                up_point = w / max_rpm;
            }
            if down_point == 0.0 || t2 < t1 * tolerance.down {
                down_point = w * ratio / max_rpm;
            }
            w += step;
        }
        up = up.min(up_point);
        down = down.min(down_point * ratio);
    }
    Shift { up, down: Some(down) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A torque curve that rises to its peak at 6000 rpm and falls steeply after it.
    fn curve() -> Vec<[f64; 2]> {
        vec![[2000.0, 0.6], [6000.0, 1.0], [10000.0, 0.3]]
    }

    #[test]
    fn a_one_speed_bike_never_changes() {
        let shift = reduce(&curve(), &[2.0], &[], 2000.0, 10000.0);
        assert_eq!(shift, Shift { up: 1.0, down: None });
    }

    #[test]
    fn the_points_follow_the_sweep() {
        // Two gears, ratio 0.8. The wheel force after the change beats the force now once the curve has fallen enough:
        // t2 = T(0.8 w) * 0.8 against t1 = T(w), with a tolerance of 1.
        let tolerances = [Tolerance { up: 1.0, down: 1.0 }];
        let shift = reduce(&curve(), &[2.0, 1.6], &tolerances, 2000.0, 10000.0);
        // Past the peak T(w) = 1 - 0.7 (w - 6000) / 4000; the change pays from where 0.8 T(0.8 w) > T(w), between 7000 and
        // 8000 rpm.
        assert!(shift.up > 0.7 && shift.up < 0.8, "{shift:?}");
        let down = shift.down.expect("a change point down");
        // The change down is a share of the rev limit in the upper gear, under the change up.
        assert!(down > 0.1 && down < shift.up, "{shift:?}");
        // A tolerance above 1 postpones the change up.
        let slow = reduce(&curve(), &[2.0, 1.6], &[Tolerance { up: 1.2, down: 1.0 }], 2000.0, 10000.0);
        assert!(slow.up >= shift.up);
    }
}
