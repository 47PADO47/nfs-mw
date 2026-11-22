//! One call of the tyre hook for one wheel (docs/specs/ac-handling/wheel-dynamics.md): validity, the substep count, the
//! substeps, the trapezoid mean of the forces, the momentum limit and the airborne wheel. Pure; the host's types are
//! converted at the edge by [`crate::bridge`].

use crate::constants::{MAX_DT, MAX_SUBSTEPS, SUBSTEPS_PER_SECOND};
use crate::params::TyreParams;
use crate::substep::Frame;
use crate::tyre_force::ForceOut;

/// What the host tells the model about one wheel for one tick (SI, `f64`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelInput {
    /// Rolling radius, m.
    pub radius: f64,
    /// The host's wheel inertia, kg m^2 (before the model's `inertia_scale`).
    pub inertia: f64,
    /// Vertical load, N.
    pub fz: f64,
    /// Patch velocity along the wheel's heading, m/s.
    pub vx: f64,
    /// Patch velocity to the right, m/s.
    pub vy: f64,
    /// Wheel spin at the start of the tick, rad/s.
    pub spin: f64,
    /// Drive torque at the hub, N m, signed.
    pub drive_torque: f64,
    /// Total brake torque at the hub, N m, at or above 0.
    pub brake_torque: f64,
    /// The tick, s.
    pub dt: f64,
    /// The sprung mass of this wheel, kg.
    pub corner_mass: f64,
    /// Surface multiplier of lateral force.
    pub surf_lat: f64,
    /// Surface multiplier of drive force.
    pub surf_drive: f64,
    /// Surface multiplier of rolling resistance.
    pub surf_rolling: f64,
    /// The total grip multiplier: grip gain times tyre condition times the thermal factor.
    pub grip: f64,
    /// Camber in the wheel frame's own sign (the top towards +x is positive), held for the tick; 0 without a suspension hook.
    pub gamma: f64,
    /// Whether the wheel presses on the ground.
    pub loaded: bool,
}

impl Default for WheelInput {
    fn default() -> Self {
        WheelInput {
            radius: 0.33,
            inertia: 10.0,
            fz: 4000.0,
            vx: 0.0,
            vy: 0.0,
            spin: 0.0,
            drive_torque: 0.0,
            brake_torque: 0.0,
            dt: 1.0 / 60.0,
            corner_mass: 4000.0 / 9.81,
            surf_lat: 1.0,
            surf_drive: 1.0,
            surf_rolling: 1.0,
            grip: 1.0,
            gamma: 0.0,
            loaded: true,
        }
    }
}

/// What a tick answers (SI, `f64`; `slip_angle` in turns).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WheelOutput {
    /// Mean longitudinal force over the tick, N.
    pub fx: f64,
    /// Mean lateral force over the tick, N.
    pub fy: f64,
    /// Mean self-aligning torque over the tick, N m.
    pub mz: f64,
    /// Wheel spin at the end of the tick, rad/s.
    pub spin: f64,
    /// 1 while gripping, `1 / n` beyond; 0 airborne.
    pub traction: f64,
    /// Wheel speed minus ground speed along the heading at the end of the tick, m/s.
    pub slip: f64,
    /// The relaxed lateral slip as an angle, in turns.
    pub slip_angle: f64,
    /// Mean over the substeps of `|fx * vsx| + |fy * vsy|`, W: the sliding power the thermal model heats the tyre with.
    pub p_slide: f64,
    /// Mean over the substeps of the speed of the rubber over the road, m/s.
    pub v_slide: f64,
}

/// The per-wheel state kept between ticks (section 3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelState {
    /// Relaxed longitudinal slip.
    pub sx_r: f64,
    /// Relaxed lateral slip.
    pub sy_r: f64,
    /// Normalised combined slip of the last substep.
    pub n_prev: f64,
    /// Re-seed from the instantaneous slips at the next loaded call.
    pub fresh: bool,
}

impl WheelState {
    /// A wheel with no history: it seeds itself at the first loaded call.
    pub const fn fresh() -> Self {
        WheelState { sx_r: 0.0, sy_r: 0.0, n_prev: 0.0, fresh: true }
    }
}

impl Default for WheelState {
    fn default() -> Self {
        WheelState::fresh()
    }
}

/// Whether the model may answer for this query (the not-handled rules of section 2).
pub fn valid(i: &WheelInput) -> bool {
    let numbers = [
        i.radius,
        i.inertia,
        i.fz,
        i.vx,
        i.vy,
        i.spin,
        i.drive_torque,
        i.brake_torque,
        i.dt,
        i.corner_mass,
        i.surf_lat,
        i.surf_drive,
        i.surf_rolling,
        i.grip,
        i.gamma,
    ];
    if numbers.iter().any(|value| !value.is_finite()) {
        return false;
    }
    // A tick that comes from `f32` is a little above 0.1 when the host says 0.1 s.
    let longest = MAX_DT * (1.0 + 1.0e-6);
    i.dt > 0.0
        && i.dt <= longest
        && i.radius >= 0.05
        && i.inertia > 0.0
        && i.corner_mass > 0.0
        && i.fz >= 0.0
        && i.gamma.abs() <= 1.0
}

/// The number of substeps of a tick of `dt` seconds; `setting` 0 is automatic, 1 to 24 overrides it (section 4.9).
pub fn substep_count(dt: f64, setting: u32) -> u32 {
    if setting > 0 {
        return setting.min(MAX_SUBSTEPS);
    }
    let wanted = (dt * SUBSTEPS_PER_SECOND - 1.0e-4).ceil();
    (wanted as u32).clamp(1, MAX_SUBSTEPS)
}

/// One call for one wheel with the automatic or the set number of substeps. `None` is "not handled": the state is marked
/// fresh and the host runs its own tyre.
pub fn wheel_tick(p: &TyreParams, state: &mut WheelState, i: &WheelInput, setting: u32) -> Option<WheelOutput> {
    tick_n(p, state, i, substep_count(i.dt, setting))
}

/// As [`wheel_tick`] with exactly `n` substeps (at least 1, no upper bound: the convergence tests use 96).
pub fn tick_n(p: &TyreParams, state: &mut WheelState, i: &WheelInput, n: u32) -> Option<WheelOutput> {
    if !valid(i) {
        *state = WheelState::fresh();
        return None;
    }
    let n = n.max(1);
    let h = i.dt / f64::from(n);
    let inertia = i.inertia * p.inertia_scale;
    if !i.loaded || i.fz <= 0.0 {
        return Some(airborne(p, state, i, n, h, inertia));
    }
    let frame = Frame { p, i, inertia, speed: i.vx.hypot(i.vy), h };
    if state.fresh {
        let slips = frame.slips(i.spin);
        *state = WheelState { sx_r: slips.sx, sy_r: slips.sy, n_prev: 0.0, fresh: false };
    }
    let mut spin = i.spin;
    let first = frame.force_at(state.sx_r, state.sy_r, spin);
    let mut sum = ForceOut { fx: first.fx * 0.5, fy: first.fy * 0.5, mz: first.mz * 0.5, n: 0.0 };
    let (mut p_slide, mut v_slide) = (0.0, 0.0);
    for k in 1..=n {
        let (new_spin, end) = frame.step(state, spin);
        spin = new_spin;
        let slide_x = i.radius * spin - i.vx;
        p_slide += (end.fx * slide_x).abs() + (end.fy * i.vy).abs();
        v_slide += slide_x.hypot(i.vy);
        let weight = if k == n { 0.5 } else { 1.0 };
        sum.fx += weight * end.fx;
        sum.fy += weight * end.fy;
        sum.mz += weight * end.mz;
    }
    let count = f64::from(n);
    let (fx, fy) = momentum_limit(p, i, sum.fx / count, sum.fy / count);
    let traction = if state.n_prev <= 1.0 { 1.0 } else { 1.0 / state.n_prev };
    Some(WheelOutput {
        fx,
        fy,
        mz: sum.mz / count,
        spin,
        traction,
        slip: i.radius * spin - i.vx,
        slip_angle: state.sy_r.atan() / std::f64::consts::TAU,
        p_slide: p_slide / count,
        v_slide: v_slide / count,
    })
}

/// The low-speed limit of section 4.6: a stiff tyre must not reverse a creeping car within one tick.
fn momentum_limit(p: &TyreParams, i: &WheelInput, fx: f64, fy: f64) -> (f64, f64) {
    let share = p.stick_cap * i.corner_mass / i.dt;
    let cap_y = share * i.vy.abs();
    let cap_x = share * i.vx.abs();
    let fy = if fy.abs() > cap_y { fy.signum() * cap_y } else { fy };
    let opposes = fx * i.vx < 0.0;
    let fx = if opposes && fx.abs() > cap_x { fx.signum() * cap_x } else { fx };
    (fx, fy)
}

/// The airborne wheel (section 4.7): the spin decays by the bearing drag and the brake, nothing else acts.
fn airborne(p: &TyreParams, state: &mut WheelState, i: &WheelInput, n: u32, h: f64, inertia: f64) -> WheelOutput {
    let mut spin = i.spin;
    let removable = h * i.brake_torque / inertia;
    for _ in 0..n {
        let free = (spin + h * i.drive_torque / inertia) / (1.0 + h * p.air_drag_nms / inertia);
        spin = if free.abs() <= removable { 0.0 } else { free - free.signum() * removable };
    }
    *state = WheelState::fresh();
    WheelOutput { spin, ..WheelOutput::default() }
}
