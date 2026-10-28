//! The Magic Formula 94 tyre model (Pacejka, "Tyre and Vehicle Dynamics", appendix 3, the Bakker–Pacejka form).
//!
//! The coefficients keep their published units: vertical load in kN, slip angle and camber in degrees, longitudinal
//! slip in percent, forces in N and the self-aligning moment in N m. The three sets are the lateral force
//! (`a0..a10`, `a111`, `a112`, `a12`, `a13`), the longitudinal force (`b0..b10`) and the aligning moment (`c0..c17`).
//!
//! Signs follow the formula: a positive slip angle gives a positive lateral force. The caller decides which way that
//! points on the vehicle.

/// The coefficients of one tyre on one surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mf94 {
    /// `a0..a10`, then `a111`, `a112`, `a12`, `a13`.
    pub a: [f32; 15],
    /// `b0..b10`.
    pub b: [f32; 11],
    /// `c0..c17`.
    pub c: [f32; 18],
}

const A_111: usize = 11;
const A_112: usize = 12;
const A_12: usize = 13;
const A_13: usize = 14;

/// `D sin(C atan(B x - E (B x - atan(B x))))`.
fn magic(b: f32, c: f32, d: f32, e: f32, x: f32) -> f32 {
    let bx = b * x;
    d * (c * (bx - e * (bx - bx.atan())).atan()).sin()
}

/// The stiffness factor `B = BCD / (C D)`, 0 when the peak vanishes.
fn stiffness(bcd: f32, c: f32, d: f32) -> f32 {
    let denominator = c * d;
    if denominator.abs() < 1e-6 {
        return 0.0;
    }
    bcd / denominator
}

impl Mf94 {
    /// The peak lateral force `D` (N) at `fz_kn`.
    pub fn lateral_peak(&self, fz_kn: f32) -> f32 {
        (self.a[1] * fz_kn + self.a[2]) * fz_kn
    }

    /// The lateral force (N) at vertical load `fz_kn`, slip angle `alpha_deg` and camber `gamma_deg`.
    pub fn lateral(&self, fz_kn: f32, alpha_deg: f32, gamma_deg: f32) -> f32 {
        let a = &self.a;
        let c = a[0];
        let d = self.lateral_peak(fz_kn);
        let bcd = a[3] * (2.0 * (fz_kn / a[4].max(1e-6)).atan()).sin() * (1.0 - a[5] * gamma_deg.abs());
        let e = (a[6] * fz_kn + a[7]).min(1.0);
        let sh = a[8] * gamma_deg + a[9] * fz_kn + a[10];
        let sv = a[A_12] * fz_kn + a[A_13] + (a[A_111] * fz_kn + a[A_112]) * gamma_deg * fz_kn;
        magic(stiffness(bcd, c, d), c, d, e, alpha_deg + sh) + sv
    }

    /// The peak longitudinal force `D` (N) at `fz_kn`.
    pub fn longitudinal_peak(&self, fz_kn: f32) -> f32 {
        (self.b[1] * fz_kn + self.b[2]) * fz_kn
    }

    /// The longitudinal force (N) at vertical load `fz_kn` and slip ratio `slip_percent`.
    pub fn longitudinal(&self, fz_kn: f32, slip_percent: f32) -> f32 {
        let b = &self.b;
        let c = b[0];
        let d = self.longitudinal_peak(fz_kn);
        let bcd = (b[3] * fz_kn * fz_kn + b[4] * fz_kn) * (-b[5] * fz_kn).exp();
        let e = (b[6] * fz_kn * fz_kn + b[7] * fz_kn + b[8]).min(1.0);
        let sh = b[9] * fz_kn + b[10];
        magic(stiffness(bcd, c, d), c, d, e, slip_percent + sh)
    }

    /// The self-aligning moment (N m) at vertical load `fz_kn`, slip angle `alpha_deg` and camber `gamma_deg`.
    pub fn aligning(&self, fz_kn: f32, alpha_deg: f32, gamma_deg: f32) -> f32 {
        let k = &self.c;
        let c = k[0];
        let d = (k[1] * fz_kn + k[2]) * fz_kn;
        let bcd = (k[3] * fz_kn * fz_kn + k[4] * fz_kn) * (1.0 - k[6] * gamma_deg.abs()) * (-k[5] * fz_kn).exp();
        let e = ((k[7] * fz_kn * fz_kn + k[8] * fz_kn + k[9]) * (1.0 - k[10] * gamma_deg.abs())).min(1.0);
        let sh = k[11] * gamma_deg + k[12] * fz_kn + k[13];
        let sv = (k[14] * fz_kn * fz_kn + k[15] * fz_kn) * gamma_deg + k[16] * fz_kn + k[17];
        magic(stiffness(bcd, c, d), c, d, e, alpha_deg + sh) + sv
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A road tyre in the range of the published fits: a peak friction near 1.3 that falls with load.
    pub(crate) fn road_tyre() -> Mf94 {
        let mut a = [0.0; 15];
        a[0] = 1.3;
        a[1] = -10.0;
        a[2] = 1300.0;
        a[3] = 1200.0;
        a[4] = 6.0;
        a[5] = 0.002;
        let mut b = [0.0; 11];
        b[0] = 1.5;
        b[1] = -10.0;
        b[2] = 1250.0;
        b[4] = 200.0;
        Mf94 { a, b, c: [0.0; 18] }
    }

    #[test]
    fn the_lateral_force_is_odd_in_the_slip_angle() {
        let tyre = road_tyre();
        let right = tyre.lateral(2.0, 3.0, 0.0);
        let left = tyre.lateral(2.0, -3.0, 0.0);
        assert!(right > 0.0);
        assert!((right + left).abs() < 1e-3, "{right} {left}");
    }

    #[test]
    fn the_lateral_force_peaks_near_d_and_levels_off() {
        let tyre = road_tyre();
        let peak = tyre.lateral_peak(2.0);
        let near = (0..400).map(|n| tyre.lateral(2.0, n as f32 * 0.05, 0.0)).fold(0.0, f32::max);
        assert!(near <= peak * 1.001 && near > peak * 0.95, "peak {peak}, found {near}");
        assert!(tyre.lateral(2.0, 40.0, 0.0) > 0.0);
    }

    #[test]
    fn the_cornering_stiffness_matches_bcd() {
        let tyre = road_tyre();
        let fz: f32 = 2.0;
        let bcd = 1200.0 * (2.0 * (fz / 6.0).atan()).sin();
        let slope = tyre.lateral(fz, 0.01, 0.0) / 0.01;
        assert!((slope - bcd).abs() / bcd < 0.01, "{slope} {bcd}");
    }

    #[test]
    fn a_vertical_shift_gives_camber_thrust() {
        let mut tyre = road_tyre();
        tyre.a[A_111] = 2.0;
        assert!(tyre.lateral(2.0, 0.0, 30.0) > 0.0);
        assert_eq!(road_tyre().lateral(2.0, 0.0, 30.0), 0.0);
    }

    #[test]
    fn the_longitudinal_force_follows_the_slip_sign_and_the_load() {
        let tyre = road_tyre();
        assert!(tyre.longitudinal(2.0, 8.0) > 0.0);
        assert!(tyre.longitudinal(2.0, -8.0) < 0.0);
        assert!(tyre.longitudinal(3.0, 8.0) > tyre.longitudinal(1.0, 8.0));
    }

    #[test]
    fn a_tyre_with_no_coefficients_gives_no_force() {
        let tyre = Mf94 { a: [0.0; 15], b: [0.0; 11], c: [0.0; 18] };
        assert_eq!(tyre.lateral(2.0, 5.0, 10.0), 0.0);
        assert_eq!(tyre.longitudinal(2.0, 5.0), 0.0);
        assert_eq!(tyre.aligning(2.0, 5.0, 10.0), 0.0);
    }
}
