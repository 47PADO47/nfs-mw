//! The suspension of one axle (docs/specs/ac-handling/suspension.md): a spring, a two-rate damper, stops and an anti-roll bar
//! around an unsprung hub that is integrated in substeps, with camber and a roll-centre load transfer. Pure: one call is one axle
//! of one tick, both wheels in lockstep. [`crate::bridge`] converts the host's query.
//!
//! Compression is positive towards the body, forces on the body are positive up, camber is outward positive.

use crate::keys::SuspensionKeys;

const G: f64 = 9.81;
/// The hub substep the spec chooses, s (`w * h < 0.2`).
const SUBSTEP: f64 = 0.0025;
/// The most substeps of a tick.
const MAX_SUBSTEPS: u32 = 16;
/// The longest tick the model accepts: the host's 0.1 s with the `f32` slack.
const MAX_DT: f64 = 0.1 * (1.0 + 1.0e-6);

/// The tuning of one axle: its keys and the track.
#[derive(Debug, Clone, PartialEq)]
pub struct AxleCfg {
    /// The keys of the axle.
    pub keys: SuspensionKeys,
    /// The distance between the tyre centres, m.
    pub track: f64,
}

/// What the host tells the model about one wheel (suspension.md section 2).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WheelIn {
    /// The wheel presses on the ground.
    pub loaded: bool,
    /// `compression_m`.
    pub c: f64,
    /// `compression_speed_mps`.
    pub c_dot: f64,
    /// `steer_angle_rad`.
    pub steer: f64,
    /// `body_accel_up_mps2`.
    pub a_up: f64,
    /// `tyre_lateral_force_n`.
    pub fy: f64,
    /// `static_load_n`.
    pub load_est: f64,
}

/// What the host tells the model about one axle for one tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AxleIn {
    /// The tick, s.
    pub dt: f64,
    /// `rest_travel_m`: the sag at rest.
    pub c_rest: f64,
    /// `max_travel_m`.
    pub c_max: f64,
    /// `corner_mass_kg`.
    pub m_corner: f64,
    /// The left wheel, then the right wheel.
    pub wheels: [WheelIn; 2],
}

/// What the model answers for one wheel.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WheelOut {
    /// Force on the body, N, positive up.
    pub body_force: f64,
    /// Vertical force between road and tyre, N.
    pub tyre_load: f64,
    /// Camber, rad, outward positive.
    pub camber: f64,
}

/// The hub of one wheel (section 3).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Hub {
    /// Hub travel from rest, m, positive towards the body.
    pub x: f64,
    /// Its rate, m/s.
    pub v: f64,
    /// The static sprung corner load (the spring preload), N.
    pub w_s: f64,
    /// The static tyre deflection, m.
    pub d0: f64,
    /// The wheel was loaded at the last call.
    pub was_loaded: bool,
}

/// The state of one axle: two hubs, and whether they must be seeded at the next call.
#[derive(Debug, Clone, PartialEq)]
pub struct AxleState {
    /// The left and the right hub.
    pub hubs: [Hub; 2],
    /// Seed both hubs at the next call (first call, a car change, a switch, a failure).
    pub reseed: bool,
}

impl AxleState {
    /// A state that seeds itself at the first call.
    pub fn new() -> AxleState {
        AxleState { hubs: [Hub::default(); 2], reseed: true }
    }
}

impl Default for AxleState {
    fn default() -> Self {
        AxleState::new()
    }
}

/// The number of substeps of a tick of `dt` seconds (V14).
pub fn substeps(dt: f64) -> u32 {
    ((dt / SUBSTEP - 1.0e-9).ceil() as u32).clamp(1, MAX_SUBSTEPS)
}

/// The spring force on the body at hub travel `x`, N.
pub fn spring(k: &SuspensionKeys, w_s: f64, x: f64) -> f64 {
    w_s + k.k_w * x + k.k2 * x * x.abs()
}

/// The damper force on the body at hub rate `v`, N, positive up (bump) or down (rebound).
pub fn damper(k: &SuspensionKeys, v: f64) -> f64 {
    if v >= 0.0 {
        return k.c_bs * v.min(k.v_kb) + k.c_bf * (v - k.v_kb).max(0.0);
    }
    let u = -v;
    -(k.c_rs * u.min(k.v_kr) + k.c_rf * (u - k.v_kr).max(0.0))
}

/// The bump and droop stops at hub travel `x`, N.
pub fn stops(k: &SuspensionKeys, x: f64) -> f64 {
    if x > k.x_up {
        return k.k_bs * (x - k.x_up);
    }
    if x < -k.x_dn {
        return k.k_bs * (x + k.x_dn);
    }
    0.0
}

/// The anti-roll bar's force on the body at the left and the right wheel for the hub travels `x` (the hubs get the opposite).
pub fn anti_roll(k: &SuspensionKeys, x: [f64; 2]) -> [f64; 2] {
    let force = k.k_arb * (x[0] - x[1]);
    [force, -force]
}

/// The roll-centre load transfer on the body at the left and the right wheel for the axle's lateral force `f_axle` (N, last tick).
pub fn roll_centre(k: &SuspensionKeys, track: f64, f_axle: f64) -> [f64; 2] {
    [k.h_rc * f_axle / track, -k.h_rc * f_axle / track]
}

/// The camber of the wheels of an axle from the end-of-tick hub travels and the steering angles (the camber equation).
pub fn camber(cfg: &AxleCfg, x: [f64; 2], steer: [f64; 2]) -> [f64; 2] {
    let k = &cfg.keys;
    let phi = (x[0] - x[1]) / cfg.track;
    let mut out = [0.0; 2];
    for i in 0..2 {
        let sigma = if i == 0 { -1.0 } else { 1.0 };
        let gamma = k.gamma0 - k.g_t * x[i] - sigma * phi + sigma * k.kappa * steer[i];
        out[i] = gamma.clamp(-0.30, 0.30);
    }
    out
}

/// Whether the model may answer for this axle (the not-handled rules of section 2, the role and the parameters aside).
pub fn valid(cfg: &AxleCfg, inp: &AxleIn) -> bool {
    let mut numbers =
        [inp.dt, inp.c_rest, inp.c_max, inp.m_corner, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    for (slot, wheel) in numbers[4..].chunks_mut(6).zip(&inp.wheels) {
        slot.copy_from_slice(&[wheel.c, wheel.c_dot, wheel.steer, wheel.a_up, wheel.fy, wheel.load_est]);
    }
    if numbers.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let m_u = unsprung(cfg, inp);
    inp.dt > 0.0 && inp.dt <= MAX_DT && inp.wheels.iter().all(|wheel| wheel.load_est > m_u * G)
}

/// The unsprung mass in use: the key, at most `0.3` of the corner mass.
fn unsprung(cfg: &AxleCfg, inp: &AxleIn) -> f64 {
    if inp.m_corner > 0.0 { cfg.keys.m_u.min(0.3 * inp.m_corner) } else { cfg.keys.m_u }
}

/// One axle of one tick: `None` is "not handled" (the host's own spring runs for both wheels). A failure reseeds both hubs at the
/// next call.
pub fn axle_call(cfg: &AxleCfg, st: &mut AxleState, inp: &AxleIn) -> Option<[WheelOut; 2]> {
    if !valid(cfg, inp) {
        st.reseed = true;
        return None;
    }
    let k = &cfg.keys;
    let m_u = unsprung(cfg, inp);
    let k_t = k.k_t;
    let seeded = st.reseed;
    if seeded {
        for (hub, wheel) in st.hubs.iter_mut().zip(&inp.wheels) {
            *hub = Hub { x: 0.0, v: 0.0, w_s: wheel.load_est - m_u * G, d0: wheel.load_est / k_t, was_loaded: false };
        }
        st.reseed = false;
    }
    let n = substeps(inp.dt);
    let h = inp.dt / f64::from(n);
    let f_axle = inp.wheels[0].fy + inp.wheels[1].fy;
    let transfer = roll_centre(k, cfg.track, f_axle);
    let mut u = [0.0; 2];
    let mut u_dot = [0.0; 2];
    for i in 0..2 {
        let wheel = &inp.wheels[i];
        u[i] = wheel.c - inp.c_rest;
        u_dot[i] = if seeded || !st.hubs[i].was_loaded { 0.0 } else { wheel.c_dot };
    }
    let (mut sum_link, mut sum_t) = ([0.0; 2], [0.0; 2]);
    for step in 1..=n {
        let (x, v) = ([st.hubs[0].x, st.hubs[1].x], [st.hubs[0].v, st.hubs[1].v]);
        let arb = anti_roll(k, x);
        let mut f_link = [0.0; 2];
        let mut f_tyre = [0.0; 2];
        for i in 0..2 {
            let hub = &st.hubs[i];
            let u_k = u[i] - u_dot[i] * inp.dt * f64::from(n - step) / f64::from(n);
            f_tyre[i] = match inp.wheels[i].loaded {
                true => (k_t * (hub.d0 + u_k - x[i]) + k.c_t * (u_dot[i] - v[i])).max(0.0),
                false => 0.0,
            };
            f_link[i] = spring(k, hub.w_s, x[i]) + damper(k, v[i]) + stops(k, x[i]) + arb[i] + transfer[i];
        }
        for i in 0..2 {
            let accel = (f_tyre[i] - f_link[i] - m_u * G) / m_u - inp.wheels[i].a_up;
            let hub = &mut st.hubs[i];
            hub.v += accel * h;
            hub.x += hub.v * h;
            sum_link[i] += f_link[i];
            sum_t[i] += f_tyre[i];
        }
    }
    let count = f64::from(n);
    let cam = camber(cfg, [st.hubs[0].x, st.hubs[1].x], [inp.wheels[0].steer, inp.wheels[1].steer]);
    let mut out = [WheelOut::default(); 2];
    for i in 0..2 {
        out[i] = WheelOut { body_force: sum_link[i] / count, tyre_load: sum_t[i] / count, camber: cam[i] };
        st.hubs[i].was_loaded = inp.wheels[i].loaded;
    }
    let sound = out.iter().all(|wheel| {
        let finite = wheel.body_force.is_finite() && wheel.tyre_load.is_finite() && wheel.camber.is_finite();
        finite
            && wheel.body_force.abs() <= 1.0e6
            && (0.0..=1.0e6).contains(&wheel.tyre_load)
            && wheel.camber.abs() <= 0.6
    });
    let wild =
        st.hubs.iter().any(|hub| !hub.x.is_finite() || !hub.v.is_finite() || hub.x.abs() > 0.5 || hub.v.abs() > 20.0);
    if !sound || wild {
        st.reseed = true;
        return None;
    }
    Some(out)
}
