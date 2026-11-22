//! The edge between the host's `f32` query and result and the model's `f64` types.
//!
//! A result with a number the host would refuse (not finite, or past the ranges of docs/modding/api-handling.md) is never
//! sent: the call answers "not handled" and the built-in tyre runs for that wheel.

use blackbox_modding::sdk::{SuspensionQuery, SuspensionResult, SuspensionWheelResult, TyreQuery, TyreResult};

use crate::suspension::{AxleIn, WheelIn, WheelOut};
use crate::wheel::{WheelInput, WheelOutput};

/// Largest force the host accepts, N.
pub const MAX_FORCE_N: f64 = 1.0e6;
/// Largest aligning torque the host accepts, N m.
pub const MAX_TORQUE_NM: f64 = 1.0e5;
/// Largest wheel spin the host accepts, rad/s.
pub const MAX_SPIN: f64 = 5000.0;
/// Largest `slip` the host accepts, m/s.
pub const MAX_SLIP: f64 = 1000.0;

/// The tyre force's camber (the wheel frame's sign: the top towards +x is positive) from the query's outward-positive one.
pub fn wheel_frame_camber(wheel_index: u32, camber_rad: f32) -> f64 {
    let outward = if camber_rad.is_finite() { f64::from(camber_rad) } else { 0.0 };
    if wheel_index % 2 == 1 { outward } else { -outward }
}

/// The model's input for one wheel. `grip` is the product of the player's gain, the tyre condition and the thermal factor.
pub fn to_input(q: &TyreQuery, grip: f64) -> WheelInput {
    WheelInput {
        gamma: wheel_frame_camber(q.wheel_index, q.camber_rad),
        radius: f64::from(q.wheel_radius_m),
        inertia: f64::from(q.wheel_inertia_kgm2),
        fz: f64::from(q.vertical_load_n),
        vx: f64::from(q.patch_forward_velocity),
        vy: f64::from(q.patch_lateral_velocity),
        spin: f64::from(q.wheel_spin),
        drive_torque: f64::from(q.drive_torque_nm),
        brake_torque: f64::from(q.brake_torque_nm),
        dt: f64::from(q.dt),
        corner_mass: f64::from(q.corner_mass_kg),
        surf_lat: f64::from(q.surface_lateral),
        surf_drive: f64::from(q.surface_drive),
        surf_rolling: f64::from(q.surface_rolling),
        grip,
        loaded: q.loaded,
    }
}

/// The host's answer for a model output, or `None` when any field is not finite or out of the host's range.
pub fn to_result(o: &WheelOutput) -> Option<TyreResult> {
    let within = |value: f64, limit: f64| value.is_finite() && value.abs() <= limit;
    let valid = within(o.fx, MAX_FORCE_N)
        && within(o.fy, MAX_FORCE_N)
        && within(o.mz, MAX_TORQUE_NM)
        && within(o.spin, MAX_SPIN)
        && within(o.slip, MAX_SLIP)
        && within(o.slip_angle, 1.0)
        && o.traction.is_finite()
        && (0.0..=1.0).contains(&o.traction);
    if !valid {
        return None;
    }
    Some(TyreResult {
        longitudinal_force_n: o.fx as f32,
        lateral_force_n: o.fy as f32,
        aligning_torque_nm: o.mz as f32,
        wheel_spin: o.spin as f32,
        traction: o.traction as f32,
        slip: o.slip as f32,
        slip_angle: o.slip_angle as f32,
    })
}

/// The suspension model's input from the host's axle query.
pub fn to_axle_input(q: &SuspensionQuery) -> AxleIn {
    let wheel = |w: &blackbox_modding::sdk::SuspensionWheelQuery| WheelIn {
        loaded: w.loaded,
        c: f64::from(w.compression_m),
        c_dot: f64::from(w.compression_speed_mps),
        steer: f64::from(w.steer_angle_rad),
        a_up: f64::from(w.body_accel_up_mps2),
        fy: f64::from(w.tyre_lateral_force_n),
        load_est: f64::from(w.static_load_n),
    };
    AxleIn {
        dt: f64::from(q.dt),
        c_rest: f64::from(q.rest_travel_m),
        c_max: f64::from(q.max_travel_m),
        m_corner: f64::from(q.corner_mass_kg),
        wheels: [wheel(&q.wheels[0]), wheel(&q.wheels[1])],
    }
}

/// The host's answer for one axle.
pub fn to_axle_result(out: &[WheelOut; 2]) -> SuspensionResult {
    let wheel = |w: &WheelOut| SuspensionWheelResult {
        body_force_n: w.body_force as f32,
        tyre_load_n: w.tyre_load as f32,
        camber_rad: w.camber as f32,
    };
    SuspensionResult { wheels: [wheel(&out[0]), wheel(&out[1])] }
}
