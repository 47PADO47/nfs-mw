//! The rider's lean controller (`docs/specs/ride6-lean-controller.md`), replacing the inverted-pendulum balance.
//!
//! The rider does not balance the bike: he prescribes its roll motion. The stick, through a filter, scales the lean limit into a
//! target lean; a second filter smooths the target; a spring-damper pulls the lean to it, which asks for a roll acceleration
//! and a roll torque; the torque becomes a desired side force at the front tyre (`-tau / h`, plus the force of a steady turn,
//! see `choices.rs`); the tyre's lateral curve, inverted, turns the force into a slip angle; and the handlebar is set to the
//! angle that gives the front wheel that slip. `Bike` applies the roll torque (within a limit) and steers the handlebar.

use super::choices;
use super::curve::{curve, factor, ramp};
use super::filter::{SecondOrder, step_acceleration};
use super::magic::Mf94;
use super::peaks::peaks;
use super::spec::SteeringSpec;

/// What the controller needs to know this step.
#[derive(Clone, Copy, Debug)]
pub struct LeanInput {
    /// The stick, -1 to 1.
    pub stick: f32,
    /// The speed along the bike (m/s).
    pub speed: f32,
    /// The lean (rad, positive right) and its rate.
    pub lean: f32,
    pub lean_rate: f32,
    /// The front tyre's lateral friction coefficient (lateral peak over load).
    pub lateral_grip: f32,
    /// The lateral acceleration in g and the angle between the velocity and the heading (rad).
    pub lateral_g: f32,
    pub heading_error: f32,
    /// A cap of the lean limit from a wheelie or a stoppie `(limit rad, share 0 to 1)`: the limit moves that share of the way.
    pub pitch_cut: (f32, f32),
    /// The roll inertia of the bike and the rider together (kg m^2) and the combined centre of gravity (kg, m).
    pub roll_inertia: f32,
    pub mass: f32,
    pub cg_height: f32,
    /// The share of the weight on the front wheel.
    pub front_share: f32,
    pub dt: f32,
}

/// What the controller asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LeanPlan {
    /// The lean limit this step (rad) and the target lean after the filters.
    pub limit: f32,
    pub target: f32,
    /// The prescribed roll acceleration (rad/s^2, positive to the right) and the roll torque it needs (N m).
    pub acceleration: f32,
    pub torque: f32,
    /// The side force wanted at the front tyre (N, positive to the right).
    pub front_force: f32,
}

/// The state of the controller's filters.
#[derive(Clone, Debug, Default)]
pub struct Lean {
    stick: SecondOrder,
    target: SecondOrder,
}

/// The lean limit (rad) for a front tyre of friction coefficient `lateral_grip` at this speed and slide.
pub fn lean_limit(steering: &SteeringSpec, input: &LeanInput) -> f32 {
    let spec = &steering.lean;
    let grip = input.lateral_grip.max(0.0).atan() * choices::LATERAL_GRIP_TOLERANCE;
    let mut limit = grip.min(steering.max_lean);
    if input.speed < spec.stopped_speed {
        let share = (input.speed / spec.stopped_speed.max(1e-6)).clamp(0.0, 1.0);
        limit = spec.stopped_limit + (limit - spec.stopped_limit) * share;
    }
    limit *= factor(&spec.g_tuning, input.lateral_g.abs());
    let skid = ramp(input.heading_error.abs(), spec.skid_range).powf(spec.skid_linearity.max(0.0));
    let cut = curve(&spec.skid_reduction, input.speed.abs());
    limit *= (1.0 - cut * skid).clamp(0.0, 1.0);
    let (pitch_limit, share) = input.pitch_cut;
    limit + (pitch_limit.min(limit) - limit) * share.clamp(0.0, 1.0)
}

impl Lean {
    pub fn new() -> Self {
        Self::default()
    }

    /// Hold the filters at rest (a bike just placed).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Work out the roll motion for this step.
    pub fn plan(&mut self, steering: &SteeringSpec, input: &LeanInput) -> LeanPlan {
        let spec = &steering.lean;
        let stick = self.stick.step(input.stick.clamp(-1.0, 1.0), spec.steer_k, spec.steer_damping_ratio, input.dt);
        let limit = lean_limit(steering, input);
        let wanted = limit * stick;
        let attenuation = 1.0 - (input.lean.abs() * spec.time_scale_attenuation).min(0.99);
        let sensitivity = 1.0 + (wanted - input.lean).abs() * spec.time_scale_sensitivity;
        let scale = attenuation
            * spec.time_scale
            * factor(&spec.time_scale_speed, input.speed.abs())
            * factor(&spec.time_scale_input, input.stick.clamp(-1.0, 1.0))
            * sensitivity;
        let target = self.target.step(wanted, spec.target_k, spec.target_damping_ratio, input.dt * scale);
        let ratio = spec.damping_ratio * spec.damping_multiplier;
        // The spec's `a = -K (theta - target) - 2 z sqrt(K) theta'`, realised over the step by its exact solution (the damping
        // term alone is unstable held over a 60 Hz step: `filter::step_acceleration`).
        let accel = step_acceleration(input.lean - target, input.lean_rate, spec.k, ratio, input.dt);
        let torque = input.roll_inertia * accel;
        // `arm` is the distance of the front tyre's side direction from the roll axis: the height of the centre of gravity for a
        // small lean (the binary floors it at 0.05 m). The sign is that of a force to the right: a roll to the right needs a push
        // to the left first.
        let mut front_force = -torque / input.cg_height.max(0.05);
        if choices::LEAN_FORCE_INCLUDES_TURN_FORCE {
            front_force += input.front_share * input.mass * 9.81 * input.lean.tan();
        }
        LeanPlan { limit, target, acceleration: accel, torque, front_force }
    }
}

/// The slip angle (rad, positive when the patch moves to the right) at which a tyre at load `fz_kn` and camber `gamma_deg`
/// pushes sideways with `wanted` newtons (positive to the right). Beyond the peak of the curve it is the slip of the peak.
pub fn slip_for_force(tyre: &Mf94, fz_kn: f32, gamma_deg: f32, wanted: f32) -> f32 {
    let p = peaks(tyre, fz_kn, gamma_deg);
    let (mut low, mut high) = (p.y_negative.asin().to_degrees(), p.y_positive.asin().to_degrees());
    let force = |alpha: f32| tyre.lateral(fz_kn.max(0.01), alpha, gamma_deg);
    // The formula's force is positive for a positive slip angle; the tyre pushes against the patch's motion.
    let target = (-wanted).clamp(force(low), force(high));
    for _ in 0..30 {
        let middle = 0.5 * (low + high);
        match force(middle) < target {
            true => low = middle,
            false => high = middle,
        }
    }
    (0.5 * (low + high)).to_radians()
}

/// The handlebar angle (rad) at which the wheel's heading has the angle `wanted` (rad, from the bike's heading) in the plane
/// of the ground. `heading_of` gives that angle for a handlebar angle; it is monotone near the answer.
pub fn solve_steer(wanted: f32, max_angle: f32, heading_of: impl Fn(f32) -> f32) -> f32 {
    const EPSILON: f32 = 1e-3;
    let mut steer = wanted.clamp(-max_angle, max_angle);
    for _ in 0..5 {
        let here = heading_of(steer);
        let slope = (heading_of(steer + EPSILON) - here) / EPSILON;
        if slope.abs() < 1e-3 {
            break;
        }
        steer = (steer + (wanted - here) / slope).clamp(-max_angle, max_angle);
    }
    steer
}

/// The handlebar angle the controller aims at (rad): the plan's angle, blended to the stick's direct mapping at a walking pace.
pub fn blend_to_stick(steering: &SteeringSpec, speed: f32, stick: f32, controlled: f32) -> f32 {
    let direct = stick.clamp(-1.0, 1.0) * steering.max_angle;
    let share = ramp(speed.abs(), steering.lean.blend_range);
    direct + (controlled - direct) * share
}

#[cfg(test)]
#[path = "lean_tests.rs"]
mod tests;
