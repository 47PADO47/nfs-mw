//! The Magic Formula 94 tyre model (Pacejka, "Tyre and Vehicle Dynamics", appendix 3, the Bakker–Pacejka form), evaluated as
//! the reference game's tyre model object does (`docs/specs/ride6-tyre-mf94.md`).
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

/// Below this load (kN) every coefficient is zero (the binary's guard).
const MIN_LOAD: f32 = 1e-8;

/// The six parameters of the shape function `u = B (x + Sh)`, `F = D sin(C atan(E atan(u) + (1 - E) u)) + Sv`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Shape {
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub sh: f32,
    pub sv: f32,
}

impl Shape {
    /// The stiffness factor `B = BCD / (C D)`, 0 when the peak vanishes (the formula would divide by zero).
    fn new(bcd: f32, c: f32, d: f32, e: f32, sh: f32, sv: f32) -> Self {
        let denominator = c * d;
        let b = match denominator.abs() < 1e-6 {
            true => 0.0,
            false => bcd / denominator,
        };
        Self { b, c, d, e, sh, sv }
    }

    /// The force at the slip `x` (percent, or degrees), shift and vertical shift included.
    pub fn force(&self, x: f32) -> f32 {
        let u = self.b * (x + self.sh);
        let y = self.e * u.atan() + (1.0 - self.e) * u;
        self.d * (self.c * y.atan()).sin() + self.sv
    }

    /// The slip, without the shift, at which the pure force peaks: the positive root of
    /// `E atan(B x) + (1 - E) B x = tan(pi / (2 C))`. 0 when there is no peak (`C <= 1`, no stiffness, no root).
    pub fn peak_offset(&self) -> f32 {
        if self.c <= 1.0 || self.b <= 0.0 {
            return 0.0;
        }
        let target = (std::f32::consts::PI / (2.0 * self.c)).tan();
        let f = |x: f32| self.e * (self.b * x).atan() + (1.0 - self.e) * self.b * x - target;
        let mut high = 1.0 / self.b;
        for _ in 0..40 {
            if f(high) > 0.0 {
                break;
            }
            high *= 2.0;
        }
        if f(high) <= 0.0 {
            return 0.0;
        }
        let mut low = 0.0;
        for _ in 0..40 {
            let middle = 0.5 * (low + high);
            match f(middle) < 0.0 {
                true => low = middle,
                false => high = middle,
            }
        }
        0.5 * (low + high)
    }
}

impl Mf94 {
    /// The shape of the longitudinal force at load `fz_kn`.
    pub fn longitudinal_shape(&self, fz_kn: f32) -> Shape {
        if fz_kn < MIN_LOAD {
            return Shape::default();
        }
        let b = &self.b;
        let d = (b[1] * fz_kn + b[2]) * fz_kn;
        let bcd = (b[3] * fz_kn * fz_kn + b[4] * fz_kn) * (-b[5] * fz_kn).exp();
        let e = b[6] * fz_kn * fz_kn + b[7] * fz_kn + b[8];
        Shape::new(bcd, b[0], d, e, b[9] * fz_kn + b[10], 0.0)
    }

    /// The shape of the lateral force at load `fz_kn` and camber `gamma_deg`.
    pub fn lateral_shape(&self, fz_kn: f32, gamma_deg: f32) -> Shape {
        if fz_kn < MIN_LOAD {
            return Shape::default();
        }
        let a = &self.a;
        let d = (a[1] * fz_kn + a[2]) * fz_kn;
        let bcd = a[3] * (2.0 * (fz_kn / a[4].max(1e-6)).atan()).sin() * (1.0 - gamma_deg.abs() * a[5]);
        let e = a[6] * fz_kn + a[7];
        let sh = a[8] * gamma_deg + a[9] * fz_kn + a[10];
        let sv = ((a[11] * fz_kn + a[12]) * gamma_deg + a[13]) * fz_kn + a[14];
        Shape::new(bcd, a[0], d, e, sh, sv)
    }

    /// The shape of the self-aligning moment at load `fz_kn` and camber `gamma_deg`.
    pub fn aligning_shape(&self, fz_kn: f32, gamma_deg: f32) -> Shape {
        if fz_kn < MIN_LOAD {
            return Shape::default();
        }
        let k = &self.c;
        let d = k[1] * fz_kn * fz_kn + k[2] * fz_kn;
        let bcd = (-k[5] * fz_kn).exp() * (k[3] * fz_kn * fz_kn + k[4] * fz_kn) * (1.0 - k[6] * gamma_deg.abs());
        let e = (k[7] * fz_kn * fz_kn + k[8] * fz_kn + k[9]) * (1.0 - k[10] * gamma_deg.abs());
        let sh = k[11] * gamma_deg + k[12] * fz_kn + k[13];
        let sv = (k[14] * fz_kn * fz_kn + k[15] * fz_kn) * gamma_deg + k[16] * fz_kn + k[17];
        Shape::new(bcd, k[0], d, e, sh, sv)
    }

    /// The peak lateral force `D` (N) at `fz_kn`.
    pub fn lateral_peak(&self, fz_kn: f32) -> f32 {
        (self.a[1] * fz_kn + self.a[2]) * fz_kn
    }

    /// The lateral force (N) at vertical load `fz_kn`, slip angle `alpha_deg` and camber `gamma_deg`.
    pub fn lateral(&self, fz_kn: f32, alpha_deg: f32, gamma_deg: f32) -> f32 {
        self.lateral_shape(fz_kn, gamma_deg).force(alpha_deg)
    }

    /// The peak longitudinal force `D` (N) at `fz_kn`.
    pub fn longitudinal_peak(&self, fz_kn: f32) -> f32 {
        (self.b[1] * fz_kn + self.b[2]) * fz_kn
    }

    /// The longitudinal force (N) at vertical load `fz_kn` and slip ratio `slip_percent`.
    pub fn longitudinal(&self, fz_kn: f32, slip_percent: f32) -> f32 {
        self.longitudinal_shape(fz_kn).force(slip_percent)
    }

    /// The self-aligning moment (N m) at vertical load `fz_kn`, slip angle `alpha_deg` and camber `gamma_deg`.
    pub fn aligning(&self, fz_kn: f32, alpha_deg: f32, gamma_deg: f32) -> f32 {
        self.aligning_shape(fz_kn, gamma_deg).force(alpha_deg)
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
        tyre.a[11] = 2.0;
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
