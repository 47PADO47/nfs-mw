//! The two tyre models of the probe, as pure functions of one query. They share nothing but the hook: the linear tyre is a
//! spring in the slip speed with a friction limit per axis, the friction-circle tyre is a constant friction coefficient on the
//! combined slip. Both integrate the wheel spin themselves, implicitly, because a stiff tyre on a 10 kg m^2 wheel is
//! unstable under an explicit step of 1/60 s.

use blackbox_modding::sdk::{TyreQuery, TyreResult};

/// Friction coefficient of the linear tyre.
pub const LINEAR_MU: f32 = 1.0;
/// Longitudinal stiffness of the linear tyre, newtons per metre per second of slip.
pub const LINEAR_K_LONG: f32 = 8000.0;
/// Lateral stiffness of the linear tyre, newtons per metre per second of sideways patch speed.
pub const LINEAR_K_LAT: f32 = 3000.0;
/// Friction coefficient of the friction-circle tyre.
pub const CIRCLE_MU: f32 = 1.1;
/// The slip speed (m/s) at which the friction-circle tyre reaches its full friction.
pub const CIRCLE_SLIP_SPEED: f32 = 0.5;

/// The torque on the wheel from the hub: drive, minus the brake against the spin.
fn hub_torque(q: &TyreQuery) -> f32 {
    q.drive_torque_nm - q.brake_torque_nm * q.wheel_spin.signum()
}

/// The answer for a wheel in the air: no forces, the spin advances under the torques, and a brake cannot reverse it.
fn airborne(q: &TyreQuery) -> TyreResult {
    let mut spin = q.wheel_spin + hub_torque(q) / q.wheel_inertia_kgm2 * q.dt;
    if q.brake_torque_nm > 0.0 && spin * q.wheel_spin < 0.0 {
        spin = 0.0;
    }
    TyreResult { wheel_spin: spin, traction: 0.0, ..TyreResult::default() }
}

/// The spin at the end of the tick for a tyre whose longitudinal force is `stiffness * (spin * r - forward)`, solved
/// implicitly.
fn implicit_spin(q: &TyreQuery, stiffness: f32) -> f32 {
    let (r, inertia, dt) = (q.wheel_radius_m, q.wheel_inertia_kgm2, q.dt);
    (inertia * q.wheel_spin / dt + hub_torque(q) + r * stiffness * q.patch_forward_velocity)
        / (inertia / dt + r * r * stiffness)
}

fn slip_angle_turns(q: &TyreQuery) -> f32 {
    q.patch_lateral_velocity.atan2(q.patch_forward_velocity.abs()) / std::f32::consts::TAU
}

/// A linear tyre: force proportional to slip, clamped to the friction limit on each axis.
pub fn linear(q: &TyreQuery) -> TyreResult {
    if !q.loaded {
        return airborne(q);
    }
    let (r, inertia, dt) = (q.wheel_radius_m, q.wheel_inertia_kgm2, q.dt);
    let limit = LINEAR_MU * q.vertical_load_n * q.tyre_condition;
    let mut spin = implicit_spin(q, LINEAR_K_LONG);
    let mut fx = LINEAR_K_LONG * (spin * r - q.patch_forward_velocity);
    let mut traction = 1.0;
    if fx.abs() > limit {
        fx = fx.clamp(-limit, limit);
        spin = q.wheel_spin + (hub_torque(q) - fx * r) / inertia * dt;
        traction = 0.5;
    }
    if q.brake_torque_nm > 0.0 && spin * q.wheel_spin < 0.0 {
        spin = 0.0;
    }
    let fy = (-LINEAR_K_LAT * q.patch_lateral_velocity).clamp(-limit, limit);
    TyreResult {
        longitudinal_force_n: fx,
        lateral_force_n: fy,
        aligning_torque_nm: 0.0,
        wheel_spin: spin,
        traction,
        slip: spin * r - q.patch_forward_velocity,
        slip_angle: slip_angle_turns(q),
    }
}

/// A constant-friction tyre: the force is `mu * load` against the direction of the sliding of the patch, whatever the
/// slip, ramped up over the first `CIRCLE_SLIP_SPEED` metres per second so it does not chatter around zero.
pub fn friction_circle(q: &TyreQuery) -> TyreResult {
    if !q.loaded {
        return airborne(q);
    }
    let (r, inertia, dt) = (q.wheel_radius_m, q.wheel_inertia_kgm2, q.dt);
    let cap = CIRCLE_MU * q.vertical_load_n * q.tyre_condition;
    let stiffness = cap / CIRCLE_SLIP_SPEED;
    let mut spin = implicit_spin(q, stiffness);
    let mut fx = stiffness * (spin * r - q.patch_forward_velocity);
    let mut fy = -stiffness * q.patch_lateral_velocity;
    let magnitude = fx.hypot(fy);
    let mut traction = 1.0;
    if magnitude > cap && magnitude > 0.0 {
        traction = cap / magnitude;
        fx *= traction;
        fy *= traction;
        spin = q.wheel_spin + (hub_torque(q) - fx * r) / inertia * dt;
    }
    if q.brake_torque_nm > 0.0 && spin * q.wheel_spin < 0.0 {
        spin = 0.0;
    }
    TyreResult {
        longitudinal_force_n: fx,
        lateral_force_n: fy,
        aligning_torque_nm: 0.0,
        wheel_spin: spin,
        traction,
        slip: spin * r - q.patch_forward_velocity,
        slip_angle: slip_angle_turns(q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> TyreQuery {
        TyreQuery {
            loaded: true,
            dt: 1.0 / 60.0,
            vertical_load_n: 3500.0,
            patch_forward_velocity: 20.0,
            body_speed: 20.0,
            wheel_spin: 20.0 / 0.33,
            wheel_radius_m: 0.33,
            wheel_inertia_kgm2: 10.0,
            surface_lateral: 1.0,
            surface_drive: 1.0,
            surface_rolling: 1.0,
            tyre_condition: 1.0,
            ..TyreQuery::default()
        }
    }

    #[test]
    fn a_rolling_wheel_without_torque_gets_no_force() {
        for model in [linear, friction_circle] {
            let answer = model(&query());
            assert!(answer.longitudinal_force_n.abs() < 1.0 && answer.lateral_force_n.abs() < 1.0, "{answer:?}");
            assert_eq!(answer.traction, 1.0);
        }
    }

    #[test]
    fn drive_torque_pushes_the_car_forward_and_a_sideways_patch_is_resisted() {
        let q = TyreQuery { drive_torque_nm: 400.0, patch_lateral_velocity: 0.3, ..query() };
        for model in [linear, friction_circle] {
            let answer = model(&q);
            assert!(answer.longitudinal_force_n > 100.0, "{answer:?}");
            assert!(answer.lateral_force_n < -100.0, "the force opposes the sideways motion: {answer:?}");
        }
    }

    #[test]
    fn the_models_differ_at_large_slip() {
        let q = TyreQuery { patch_lateral_velocity: 6.0, ..query() };
        let linear_answer = linear(&q);
        let circle_answer = friction_circle(&q);
        assert!((linear_answer.lateral_force_n.abs() - LINEAR_MU * 3500.0).abs() < 1.0, "per-axis limit");
        let magnitude = circle_answer.longitudinal_force_n.hypot(circle_answer.lateral_force_n);
        assert!(magnitude <= CIRCLE_MU * 3500.0 + 1.0, "the friction circle caps the combined force: {magnitude}");
        assert!(circle_answer.traction < 1.0);
    }

    #[test]
    fn an_airborne_wheel_has_no_force_and_a_brake_cannot_reverse_it() {
        let q = TyreQuery { loaded: false, brake_torque_nm: 5000.0, wheel_spin: 1.0, ..query() };
        for model in [linear, friction_circle] {
            let answer = model(&q);
            assert_eq!((answer.longitudinal_force_n, answer.lateral_force_n, answer.traction), (0.0, 0.0, 0.0));
            assert_eq!(answer.wheel_spin, 0.0);
        }
    }
}
