//! The lean help's factor and its split between a force and a couple (`docs/specs/ride6-chassis-round6.md`, "Lean help").
//!
//! The help pulls the front tyre's side force towards the one the lean controller wants. How much of the gap it closes is the
//! help factor: the speed curve, cut in a skid, blended up to 1 at a standstill. How the resulting force acts on the bike is the
//! balance: above 30 km/h it is the force at the front contact; slower it turns into the same force's moment about the centre of
//! gravity with no net force, which is what holds a slow or standing bike up.

use glam::Vec3;

use crate::bike::curve::{curve, factor, ramp};
use crate::bike::spec::{LeanSpec, StabilitySpec};

/// The speeds (m/s) between which the balance goes from the couple to its tuned share, whatever the tuning: 0 at 1 m/s, 1 at 3 m/s
/// (a recovered constant of the reference game, `ride6-chassis-round6.md`).
pub const BALANCE_FADE: [f32; 2] = [1.0, 3.0];

/// What the help factor depends on this step.
#[derive(Clone, Copy, Debug)]
pub struct HelpInput {
    /// The speed (m/s, absolute).
    pub speed: f32,
    /// The angle between the velocity and the heading (rad, absolute).
    pub heading_error: f32,
    /// The throttle less the brake, -1 to 1.
    pub pedals: f32,
}

/// The help factor, 0 to 1: the share of the gap between the front tyre's side force and the wanted one that the help closes.
pub fn help_factor(spec: &LeanSpec, input: &HelpInput) -> f32 {
    let extra = &spec.help_extra;
    let tuned = curve(&spec.help_speed, input.speed).clamp(0.0, 1.0);
    let skid = ramp(input.heading_error, extra.skid_range).powf(extra.skid_linearity.max(0.0));
    let cut = curve(&extra.skid_reduction, input.speed) * factor(&extra.skid_input, input.pedals);
    let value = tuned * (1.0 - cut * skid).clamp(0.0, 1.0);
    // From the stopped speed range upwards the tuned factor, below it all the way to 1 at a standstill.
    let value = match extra.stopped_range[1] > 0.0 {
        true => 1.0 + ramp(input.speed, extra.stopped_range) * (value - 1.0),
        false => value,
    };
    value.clamp(0.0, 1.0)
}

/// The share of the help force that acts as a force at the front contact (the rest is a couple), 0 to 1.
pub fn balance(spec: &LeanSpec, speed: f32) -> f32 {
    let tuned = factor(&spec.help_extra.balance_speed, speed).clamp(0.0, 1.0);
    match speed < BALANCE_FADE[1] {
        true => tuned * ramp(speed, BALANCE_FADE),
        false => tuned,
    }
}

/// The couple (N m, world frame) of the part of `force` (applied at the contact `point`) that is not left as a force: the moment of
/// the whole force about the centre of gravity `cog`, times `1 - balance`. It is what the reference game applies; with the force part it
/// makes the moment `(point - cog) x force` whatever the balance.
pub fn couple(point: Vec3, cog: Vec3, force: Vec3, balance: f32) -> Vec3 {
    (point - cog).cross(force) * (1.0 - balance)
}

/// What the stability help depends on this step.
#[derive(Clone, Copy, Debug)]
pub struct StabilityInput {
    /// The speed (m/s, absolute) and the lean (rad, positive right).
    pub speed: f32,
    pub lean: f32,
    /// The lean over its limit, -1 to 1, and the two levers.
    pub normalised_lean: f32,
    pub throttle: f32,
    pub brake: f32,
}

/// The stability help factor, 0 to 1 (`ride6-chassis-round7.md`, section 3): the speed curve times the lean term, cut by the levers.
pub fn stability_factor(spec: &StabilitySpec, input: &StabilityInput) -> f32 {
    let lean = (input.normalised_lean.abs().powf(spec.lean_sensitivity.max(0.0)) * spec.lean_influence + 1.0).max(0.0);
    let cut = |curve_points: &[(f32, f32)], lever: f32| (curve(curve_points, input.speed) * lever).clamp(0.0, 1.0);
    let kept = (1.0 - cut(&spec.throttle_reduction, input.throttle)) * (1.0 - cut(&spec.brake_reduction, input.brake));
    (curve(&spec.speed, input.speed) * lean * kept).clamp(0.0, 1.0)
}

/// The yaw rate (rad/s, positive for a turn to the right) a steady turn at `lean` needs at `speed`: `g tan(lean) / v`, the lean held
/// to `max_lean` when that is set.
pub fn desired_yaw_rate(spec: &StabilitySpec, lean: f32, speed: f32) -> f32 {
    let held = match spec.max_lean > 0.0 {
        true => lean.clamp(-spec.max_lean, spec.max_lean),
        false => lean,
    };
    held.tan() * 9.81 / speed.max(1e-3)
}

/// The angular acceleration (rad/s^2, positive to the right) the help asks of the yaw: `factor (desired - actual) / dt`, in at the
/// speed range.
pub fn stability_acceleration(
    spec: &StabilitySpec,
    factor: f32,
    input: &StabilityInput,
    actual_yaw_rate: f32,
    dt: f32,
) -> f32 {
    if input.speed <= spec.speed_range[0] {
        return 0.0;
    }
    let wanted = desired_yaw_rate(spec, input.lean, input.speed);
    factor * (wanted - actual_yaw_rate) / dt.max(0.001) * ramp(input.speed, spec.speed_range)
}
