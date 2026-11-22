//! Tyre temperature, pressure, wear and the grip multiplier (docs/specs/ac-handling/tyre-thermal.md). Pure: the state of one
//! tyre and its update; [`crate::grip::ThermalGrip`] keeps four of them behind the `GripScale` seam.
//!
//! Temperatures in degrees C, pressures in kPa gauge, forces in N, speeds in m/s, wear in km of virtual sliding distance.

/// Atmospheric pressure, kPa.
pub const P_ATM: f64 = 101.325;
/// The longest step of the explicit temperature update, s (a longer tick is split).
pub const MAX_STEP: f64 = 0.5;

/// The constants of section 5, the road-compound defaults. The spec gives no per-compound table, so every compound uses these.
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalParams {
    /// Stripe heat capacity, J/K.
    pub c_s: f64,
    /// Core heat capacity, J/K.
    pub c_c: f64,
    /// Stripe to stripe, W/K.
    pub g_ss: f64,
    /// Stripe to core, W/K.
    pub g_sc: f64,
    /// Stripe to air: constant and per m/s.
    pub g_a: [f64; 2],
    /// Stripe to road at the reference load, W/K.
    pub g_rd0: f64,
    /// Reference load of `g_rd0`, N.
    pub fz_ref: f64,
    /// Core to air: constant and per m/s.
    pub g_ca: [f64; 2],
    /// Share of the sliding power that heats the tyre.
    pub eta_f: f64,
    /// Rolling resistance coefficient of the heat.
    pub c_rr: f64,
    /// Pressure effect on the rolling resistance.
    pub k_rrp: f64,
    /// Share of the rolling heat that goes into the tread.
    pub rho_s: f64,
    /// Camber effect on the heat distribution, 1/rad.
    pub k_cam: f64,
    /// Pressure effect on the heat distribution.
    pub k_pr: f64,
    /// How far from the core to the patch the grip temperature sits.
    pub w_cp: f64,
    /// Optimum temperature, C.
    pub t_opt: f64,
    /// Half width of the plateau, C.
    pub h: f64,
    /// Width of the fall below the plateau, C.
    pub s_c: f64,
    /// Width of the fall above the plateau, C.
    pub s_h: f64,
    /// Floor of the temperature factor.
    pub g_min: f64,
    /// Cold inflation pressure, kPa.
    pub p_set: f64,
    /// The pressure the tyre likes, kPa.
    pub p_ideal: f64,
    /// Grip lost per kPa from the ideal.
    pub k_gp: f64,
    /// Share of the rolled distance that wears like sliding.
    pub eps_roll: f64,
    /// Temperature above which the rubber wears faster, C.
    pub t_hot: f64,
    /// Wear rate gain per K above `t_hot`.
    pub k_wt: f64,
    /// Wear knots: `(km, factor)`.
    pub wear_knots: [(f64, f64); 7],
    /// Temperature of a warm start, C.
    pub t_warm: f64,
}

impl Default for ThermalParams {
    fn default() -> Self {
        ThermalParams {
            c_s: 1800.0,
            c_c: 10000.0,
            g_ss: 20.0,
            g_sc: 30.0,
            g_a: [3.0, 0.6],
            g_rd0: 5.0,
            fz_ref: 3500.0,
            g_ca: [3.0, 0.15],
            eta_f: 0.5,
            c_rr: 0.012,
            k_rrp: 0.5,
            rho_s: 0.4,
            k_cam: 3.0,
            k_pr: 1.5,
            w_cp: 0.25,
            t_opt: 75.0,
            h: 12.0,
            s_c: 45.0,
            s_h: 45.0,
            g_min: 0.55,
            p_set: 190.0,
            p_ideal: 230.0,
            k_gp: 0.0012,
            eps_roll: 0.01,
            t_hot: 100.0,
            k_wt: 0.02,
            wear_knots: [(0.0, 1.0), (0.5, 0.985), (2.0, 0.95), (4.0, 0.88), (6.0, 0.80), (8.0, 0.75), (10.0, 0.72)],
            t_warm: 70.0,
        }
    }
}

impl ThermalParams {
    /// The fraction of the heat each stripe takes (4.1); sums to 1. `gamma` is outward positive.
    pub fn shares(&self, gamma: f64, pressure: f64) -> [f64; 3] {
        let third = 1.0 / 3.0;
        let a = (-self.k_cam * gamma).clamp(-third, third);
        let eta = (self.k_pr * (pressure / self.p_ideal - 1.0)).clamp(-0.3, 0.3);
        let w0 = (third + a) * (1.0 - eta);
        let w2 = (third - a) * (1.0 - eta);
        [w0, 1.0 - w0 - w2, w2]
    }

    /// The gauge pressure of a tyre inflated to `p_set` at `t_set` when its core is at `t_c` (4.4).
    pub fn pressure(&self, p_set: f64, t_set: f64, t_c: f64) -> f64 {
        (p_set + P_ATM) * (t_c + 273.15) / (t_set + 273.15) - P_ATM
    }

    /// The factor on the rolling resistance coefficient (4.4).
    pub fn pressure_rr_scale(&self, pressure: f64) -> f64 {
        1.0 + self.k_rrp * (self.p_ideal / pressure.max(50.0) - 1.0)
    }

    /// The temperature factor of the grip (4.6).
    pub fn g_t(&self, t: f64) -> f64 {
        let d = t - self.t_opt;
        let s = if d < 0.0 { self.s_c } else { self.s_h };
        let e = ((d.abs() - self.h).max(0.0)) / s;
        self.g_min + (1.0 - self.g_min) * (-0.5 * e * e).exp()
    }

    /// The pressure factor of the grip (4.6).
    pub fn g_p(&self, pressure: f64) -> f64 {
        1.0 / (1.0 + self.k_gp * (pressure - self.p_ideal).abs())
    }

    /// The wear factor of the grip (4.6): linear between the knots, flat outside the first and the last.
    pub fn g_w(&self, wear: f64) -> f64 {
        let knots = &self.wear_knots;
        if wear <= knots[0].0 {
            return knots[0].1;
        }
        for pair in knots.windows(2) {
            let ((w0, f0), (w1, f1)) = (pair[0], pair[1]);
            if wear <= w1 {
                return f0 + (f1 - f0) * (wear - w0) / (w1 - w0);
            }
        }
        knots[knots.len() - 1].1
    }

    /// The interface of the spec: `grip_scale(temp, pressure, wear)`, in `(0, 1]`.
    pub fn grip_scale(&self, t_eff: f64, pressure: f64, wear: f64) -> f64 {
        self.g_t(t_eff) * self.g_p(pressure) * self.g_w(wear)
    }

    /// Remaining tyre life in per cent (4.7).
    pub fn life_percent(&self, wear: f64) -> f64 {
        let end = self.wear_knots[self.wear_knots.len() - 1].1;
        100.0 * (self.g_w(wear) - end) / (1.0 - end)
    }
}

/// What one tick tells the tyre about itself (2.2), means over the tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalTick {
    /// The tick, s.
    pub dt: f64,
    /// Mean of `|Fx vsx| + |Fy vsy|` over the substeps, W.
    pub p_slide: f64,
    /// Mean speed of the rubber over the road, m/s.
    pub v_slide: f64,
    /// Vertical load, N.
    pub fz: f64,
    /// Body speed, m/s.
    pub v: f64,
    /// Camber, outward positive, rad.
    pub gamma: f64,
    /// Ambient air, C.
    pub t_amb: f64,
    /// Road surface, C.
    pub t_road: f64,
    /// The tyre is on the ground.
    pub loaded: bool,
}

/// The four numbers of the HUD (2.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudValues {
    /// Contact-patch mean, C.
    pub surface: f64,
    /// Core, C.
    pub core: f64,
    /// Pressure, kPa.
    pub pressure: f64,
    /// Remaining life, per cent.
    pub life: f64,
}

/// The state of one tyre (section 3).
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalState {
    /// Surface temperatures: inner, centre, outer stripe.
    pub t_s: [f64; 3],
    /// Core and gas temperature.
    pub t_c: f64,
    /// Wear, km.
    pub wear: f64,
    /// Cold pressure the tyre was inflated to, kPa.
    pub p_set: f64,
    /// Temperature it was inflated at, C.
    pub t_set: f64,
}

impl ThermalState {
    /// A fresh tyre: every temperature `t_start`, no wear, inflated to the preset's cold pressure at `t_amb`.
    pub fn seed(params: &ThermalParams, t_start: f64, t_amb: f64) -> ThermalState {
        ThermalState { t_s: [t_start; 3], t_c: t_start, wear: 0.0, p_set: params.p_set, t_set: t_amb }
    }

    /// The gauge pressure now.
    pub fn pressure(&self, params: &ThermalParams) -> f64 {
        params.pressure(self.p_set, self.t_set, self.t_c)
    }

    /// The contact-patch mean temperature, weighted like the heat (4.5).
    pub fn t_cp(&self, params: &ThermalParams, gamma: f64) -> f64 {
        let shares = params.shares(gamma, self.pressure(params));
        shares.iter().zip(self.t_s).map(|(share, t)| share * t).sum()
    }

    /// The temperature the grip sees: the core and a quarter of the way to the patch (4.5).
    pub fn t_eff(&self, params: &ThermalParams, gamma: f64) -> f64 {
        self.t_c + params.w_cp * (self.t_cp(params, gamma) - self.t_c)
    }

    /// The grip factor at the start of a tick (4.8).
    pub fn grip(&self, params: &ThermalParams, gamma: f64) -> f64 {
        params.grip_scale(self.t_eff(params, gamma), self.pressure(params), self.wear)
    }

    /// The HUD values for a wheel at camber `gamma`.
    pub fn hud(&self, params: &ThermalParams, gamma: f64) -> HudValues {
        HudValues {
            surface: self.t_cp(params, gamma),
            core: self.t_c,
            pressure: self.pressure(params),
            life: params.life_percent(self.wear),
        }
    }

    /// The end of a tick (4.8): wear with the patch temperature from before, then the temperatures. A tick over 0.5 s is split.
    pub fn step(&mut self, params: &ThermalParams, tick: &ThermalTick) {
        if !tick.dt.is_finite() || tick.dt <= 0.0 {
            return;
        }
        let pieces = (tick.dt / MAX_STEP).ceil().max(1.0);
        let dt = tick.dt / pieces;
        for _ in 0..(pieces as u32) {
            self.step_once(params, tick, dt);
        }
    }

    fn step_once(&mut self, p: &ThermalParams, tick: &ThermalTick, dt: f64) {
        let pressure = self.pressure(p);
        if tick.loaded {
            let t_cp = self.t_cp(p, tick.gamma);
            let m_hot = 1.0 + p.k_wt * (t_cp - p.t_hot).max(0.0);
            self.wear += (tick.v_slide + p.eps_roll * tick.v) * dt * 1.0e-3 * m_hot;
        }
        let (q_f, q_r) = match tick.loaded {
            true => (p.eta_f * tick.p_slide, (p.c_rr * p.pressure_rr_scale(pressure) * tick.fz * tick.v).max(0.0)),
            false => (0.0, 0.0),
        };
        let shares = p.shares(tick.gamma, pressure);
        let q_c = (1.0 - p.rho_s) * q_r;
        let g_air = p.g_a[0] + p.g_a[1] * tick.v;
        let g_rd = if tick.loaded { p.g_rd0 * (tick.fz / p.fz_ref).sqrt() } else { 0.0 };
        let g_ca = p.g_ca[0] + p.g_ca[1] * tick.v;
        let ts = self.t_s;
        let q =
            [p.g_ss * (ts[1] - ts[0]), p.g_ss * (ts[0] - ts[1]) + p.g_ss * (ts[2] - ts[1]), p.g_ss * (ts[1] - ts[2])];
        let mut d_ts = [0.0; 3];
        for i in 0..3 {
            let heat = shares[i] * (q_f + p.rho_s * q_r);
            d_ts[i] = (heat
                + q[i]
                + p.g_sc * (self.t_c - ts[i])
                + g_air * (tick.t_amb - ts[i])
                + g_rd * (tick.t_road - ts[i]))
                / p.c_s;
        }
        let d_tc =
            (q_c + p.g_sc * ts.iter().map(|t| t - self.t_c).sum::<f64>() + g_ca * (tick.t_amb - self.t_c)) / p.c_c;
        for i in 0..3 {
            self.t_s[i] += d_ts[i] * dt;
        }
        self.t_c += d_tc * dt;
    }
}
