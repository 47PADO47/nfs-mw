//! The tyre force function (docs/specs/ac-handling/tyre-force.md): slip and load in, longitudinal force, lateral force and
//! self-aligning torque out. Pure, allocation-free, no state. Everything is in `f64`; the glue narrows at the boundary.

use crate::constants::{FZ0, GRIP_MAX, PEAK_FOLLOW, TRAIL_FADE_SPEED, TRAIL_N_END, TRAIL_UNDER};
use crate::params::TyreParams;

/// The arguments of [`tyre_force`]. Physics axes: x right, z forward; angles in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForceIn {
    /// Wheel radius, m (only for the contact patch length of the aligning torque).
    pub radius: f64,
    /// Vertical load, N; `<= 0` gives all zero.
    pub fz: f64,
    /// Longitudinal theoretical slip, positive when the patch is driven forward.
    pub sx: f64,
    /// Lateral theoretical slip, positive when the patch slides to the right.
    pub sy: f64,
    /// Camber in the wheel frame's own sign (top towards +x is positive); 0 until the suspension hook exists.
    pub gamma: f64,
    /// Speed of the rubber over the road, m/s.
    pub slide_speed: f64,
    /// Patch speed, m/s (fades the aligning torque in at walking pace).
    pub speed: f64,
    /// The host's lateral surface multiplier.
    pub surf_lat: f64,
    /// The host's drive surface multiplier.
    pub surf_drive: f64,
    /// The total grip multiplier: the player's gain times the tyre condition times the thermal factor.
    pub grip: f64,
}

impl Default for ForceIn {
    fn default() -> Self {
        ForceIn {
            radius: 0.33,
            fz: 0.0,
            sx: 0.0,
            sy: 0.0,
            gamma: 0.0,
            slide_speed: 0.0,
            speed: 20.0,
            surf_lat: 1.0,
            surf_drive: 1.0,
            grip: 1.0,
        }
    }
}

/// What [`tyre_force`] answers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ForceOut {
    /// Force along the wheel's forward axis, N.
    pub fx: f64,
    /// Force along the wheel's right axis, N; opposes the slide.
    pub fy: f64,
    /// Self-aligning torque, N m; positive turns the heading to the right.
    pub mz: f64,
    /// Normalised combined slip: 0 at rest, 1 at the peak of the grip curve, above 1 sliding.
    pub n: f64,
}

/// The normalised curve `f(n)`: a cubic rise to 1 at `n = 1`, then a hyperbolic fall to the sliding level `a` at speed `q`.
pub fn curve(n: f64, a: f64, q: f64) -> f64 {
    if n <= 1.0 {
        let rest = 1.0 - n;
        return 1.0 - rest * rest * rest;
    }
    a + (1.0 - a) / (1.0 + q * (n - 1.0))
}

/// A smooth sign: `x / sqrt(x^2 + 1e-6)`.
fn smooth_sign(x: f64) -> f64 {
    x / (x * x + 1.0e-6).sqrt()
}

/// The pneumatic trail and with it the self-aligning torque (section 4.7).
fn aligning(p: &TyreParams, i: &ForceIn, fy: f64, n: f64) -> f64 {
    let deflection = (i.fz / p.k_vert).min(0.3 * i.radius);
    let patch = 2.0 * (2.0 * i.radius * deflection - deflection * deflection).max(0.0).sqrt();
    let t0 = p.trail_ratio * patch;
    let x = (n / TRAIL_N_END).clamp(0.0, 1.0);
    let smooth = 3.0 * x * x - 2.0 * x * x * x;
    let trail = t0 * (1.0 - (1.0 + TRAIL_UNDER) * smooth) * i.speed.clamp(0.0, TRAIL_FADE_SPEED);
    -trail * fy
}

/// The three outputs of one tyre for one slip state (sections 4.1 to 4.7).
pub fn tyre_force(p: &TyreParams, i: &ForceIn) -> ForceOut {
    if i.fz.is_nan() || i.fz <= 0.0 {
        return ForceOut::default();
    }
    // 4.1 load and grip scaling
    let fr = i.fz / FZ0;
    let lf_y = fr.powf(p.exp_y - 1.0).min(p.load_factor_max);
    let lf_x = fr.powf(p.exp_x - 1.0).min(p.load_factor_max);
    let u_y = (i.grip * i.surf_lat).clamp(0.0, GRIP_MAX);
    let u_x = (i.grip * i.surf_drive).clamp(0.0, GRIP_MAX);
    let sv = 1.0 / (1.0 + p.speed_sens * i.slide_speed);
    // 4.2 camber
    let sy_eff = i.sy - p.camber_gain * i.gamma.sin();
    let gc = -i.gamma * smooth_sign(sy_eff);
    let cam = (1.0 + p.camber_d1 * gc - p.camber_d2 * gc * gc).clamp(0.5, 1.5);
    // 4.3 peak friction and peak slip
    let mu_y = p.mu_y0 * u_y * lf_y * cam * sv;
    let brake = if i.sx < 0.0 { p.brake_gain } else { 1.0 };
    let mu_x = p.mu_x0() * u_x * lf_x * sv * brake;
    let s0 = p.peak_angle_deg.to_radians().tan();
    let flexf = (1.0 + p.flex * (fr - 1.0)).clamp(0.3, 3.0);
    let s_py = s0 * flexf * (1.0 + PEAK_FOLLOW * (u_y - 1.0)).clamp(0.3, 2.0);
    let s_px = s0 / p.cx_ratio * flexf * (1.0 + PEAK_FOLLOW * (u_x - 1.0)).clamp(0.3, 2.0);
    // 4.4 normalised combined slip
    let hx = i.sx / s_px;
    let hy = sy_eff / s_py;
    let n = (hx.abs().powf(p.comb_exp) + hy.abs().powf(p.comb_exp)).powf(1.0 / p.comb_exp);
    if n.is_nan() || n < 1.0e-9 {
        return ForceOut::default();
    }
    let share = hx * hx + hy * hy;
    if share <= 0.0 {
        return ForceOut::default();
    }
    let w = hx * hx / share;
    // 4.5 the curve, blended by the share of the longitudinal slip
    let a = p.fall_y + w * (p.fall_x - p.fall_y);
    let q = p.q_y + w * (p.q_x - p.q_y);
    let f = curve(n, a, q);
    // 4.6 forces, 4.7 aligning torque
    let fx = mu_x * i.fz * f * hx / n;
    let fy = -mu_y * i.fz * f * hy / n;
    ForceOut { fx, fy, mz: aligning(p, i, fy, n), n }
}
