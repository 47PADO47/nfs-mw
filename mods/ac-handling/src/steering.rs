//! Steering: lock and ratio, speed-sensitive lock, rate limit, counter-steer allowance, and the optional force-feedback signal
//! (docs/specs/ac-handling/steering.md section 4). Pure: [`steer_step`] is the steering stage of one tick of the controls hook,
//! [`ffb_step`] the force-feedback signal after it.

use crate::keys::SteeringKeys;

/// What turns the player's input into steering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Device {
    /// A gamepad stick or the keyboard: the speed-sensitive lock and the rate limit apply.
    #[default]
    Pad,
    /// A steering wheel: the device's own angle is trusted.
    Wheel,
}

impl Device {
    /// The name used in settings and commands.
    pub fn name(self) -> &'static str {
        match self {
            Device::Pad => "pad",
            Device::Wheel => "wheel",
        }
    }

    /// The device a name says.
    pub fn parse(name: &str) -> Option<Device> {
        match name {
            "pad" => Some(Device::Pad),
            "wheel" => Some(Device::Wheel),
            _ => None,
        }
    }
}

/// The steering state (section 3).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SteerState {
    /// The current target angle, rad.
    pub delta: f64,
    /// The previous tick's angle, rad.
    pub delta_prev: f64,
    /// The filtered column torque, N m.
    pub y_mz: f64,
}

impl SteerState {
    /// A state with the road wheels at `delta` (the seed of section 3) and nothing filtered.
    pub fn seeded(delta: f64) -> SteerState {
        SteerState { delta, delta_prev: delta, y_mz: 0.0 }
    }
}

/// The speed factor of the pad's lock (4.2).
pub fn lock_factor(k: &SteeringKeys, v: f64) -> f64 {
    1.0 - k.s_sens * (1.0 - k.f_min) * v * v / (v * v + k.v0 * k.v0)
}

/// The rate at which the pad moves the wheels away from the centre, rad/s (4.2).
pub fn rate_out(k: &SteeringKeys, v: f64) -> f64 {
    k.r_lo + (k.r_hi - k.r_lo) / (1.0 + (v / k.v_r).powi(2))
}

fn sign(x: f64) -> f64 {
    if x > 0.0 {
        return 1.0;
    }
    if x < 0.0 {
        return -1.0;
    }
    0.0
}

/// The steering ratio `R`: steering wheel angle per road-wheel angle (4.1).
pub fn ratio(k: &SteeringKeys) -> f64 {
    k.theta_max_deg / k.delta_max_deg
}

/// The steering stage of one tick: the new target angle of the inside front wheel, rad (4.2). `alpha_rear` is the mean rear slip
/// angle in radians, positive when the patches slide to the right.
pub fn steer_step(
    k: &SteeringKeys,
    device: Device,
    st: &mut SteerState,
    steer_in: f64,
    v: f64,
    alpha_rear: f64,
    dt: f64,
) -> f64 {
    let delta_max = k.delta_max_deg.to_radians();
    let u0 = steer_in.clamp(-1.0, 1.0);
    let gamma = if device == Device::Pad { k.gamma } else { 1.0 };
    let u = sign(u0) * u0.abs().powf(gamma);
    let speed = v.abs();
    let f = if device == Device::Pad { lock_factor(k, speed) } else { 1.0 };
    let mut lim = f * delta_max;
    if sign(u) != 0.0 && sign(u) == sign(alpha_rear) {
        lim = lim.max(delta_max.min(k.c_cs * alpha_rear.abs()));
    }
    let target = u * lim;
    st.delta_prev = st.delta;
    if device == Device::Wheel {
        if k.tau_f > 0.0 {
            st.delta += (target - st.delta) * (1.0 - (-dt / k.tau_f).exp());
        } else {
            st.delta = target;
        }
        st.delta = st.delta.clamp(-delta_max, delta_max);
        return st.delta;
    }
    let r_out = rate_out(k, speed);
    let r = if (target - st.delta) * st.delta < 0.0 { k.rho_ret * r_out } else { r_out };
    st.delta += (target - st.delta).clamp(-r * dt, r * dt);
    st.delta = st.delta.clamp(-delta_max, delta_max);
    st.delta
}

/// The optional force-feedback value and the column torque behind it (4.3), after [`steer_step`] of the same tick. `mz` is the
/// aligning torque of the two front tyres (positive turns the wheel to the right) and `omega` their spin.
pub fn ffb_step(k: &SteeringKeys, st: &mut SteerState, mz: [f64; 2], omega: [f64; 2], v: f64, dt: f64) -> (f64, f64) {
    let r = ratio(k);
    let delta_dot = (st.delta - st.delta_prev) / dt;
    let col = (mz[0] + mz[1]) / r;
    st.y_mz += (col - st.y_mz) * (1.0 - (-dt / k.tau_ff).exp());
    let gyro = -k.g_gyro * k.j_w * (omega[0].abs() + omega[1].abs()) * delta_dot / r;
    let tau = st.y_mz + gyro;
    let k_ls = k.m_ls + (1.0 - k.m_ls) * (v.abs() / k.v_ls.max(1.0e-9)).clamp(0.0, 1.0);
    let x = tau * k_ls * k.g_ffb / k.tau_ref;
    (sign(x) * x.abs().min(1.0).powf(k.p_curve), tau)
}
