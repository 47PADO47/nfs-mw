//! The rider's lean controller (`docs/specs/ride6-lean-controller.md`), replacing the inverted-pendulum balance.
//!
//! The rider does not balance the bike: he prescribes its roll motion. The stick, through a filter, scales the lean limit into a
//! target lean; a second filter smooths the target; a spring-damper pulls the lean to it, which asks for a roll acceleration
//! and a roll torque; the torque becomes a desired side force at the front tyre (`-tau / h`); the tyre's lateral curve, inverted, turns the force into a slip angle; and the handlebar is set to the
//! angle that gives the front wheel that slip. `Bike` steers the handlebar and applies the lean help (`help`); no torque is applied
//! to the body for the prescribed roll itself (`docs/specs/ride6-chassis-round6.md`).

use super::choices;
use glam::{Mat3, Vec3};

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
    /// The road's bank under the bike (rad, positive when the road is lower on the right of the heading); 0 on flat ground.
    pub terrain_slope: f32,
    /// The roll inertia of the bike and the rider together (kg m^2) and the height of the combined centre of gravity (m).
    pub roll_inertia: f32,
    pub cg_height: f32,
    /// The arm the wanted front force is divided by on top of the height: the cosine of the lean of the chassis from the front
    /// contact's ground plane ([`lean_arm`]); 1 when the front wheel is off the ground or too slow to tell.
    pub front_arm: f32,
    /// The most the target lean may be (rad): the block-fall assist's hold; infinite when it is off.
    pub target_cap: f32,
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
    /// Whether the cap held the target back.
    pub capped: bool,
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
    // The road's bank: leaning into a rising road allows more, leaning off a falling one less (`ride6-chassis-round7.md`, section 4).
    if !spec.terrain.speed.is_empty() {
        let direction = if input.stick < 0.0 { -1.0 } else { 1.0 };
        let held = curve(&spec.terrain.speed, input.speed.abs()) * input.terrain_slope * direction;
        let bank = held.clamp(-spec.terrain.max_angle, spec.terrain.max_angle);
        limit += bank * if bank <= 0.0 { spec.terrain.reduction } else { spec.terrain.increment };
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
        let free = self.target.step(wanted, spec.target_k, spec.target_damping_ratio, input.dt * scale);
        let target = free.clamp(-input.target_cap, input.target_cap);
        let ratio = spec.damping_ratio * spec.damping_multiplier;
        // The spec's `a = -K (theta - target) - 2 z sqrt(K) theta'`, realised over the step by its exact solution (the damping
        // term alone is unstable held over a 60 Hz step: `filter::step_acceleration`).
        let accel = step_acceleration(input.lean - target, input.lean_rate, spec.k, ratio, input.dt);
        let torque = input.roll_inertia * accel;
        // `arm` is the distance of the front tyre's side direction from the roll axis: the height of the centre of gravity for a
        // small lean (the binary floors it at 0.05 m). The sign is that of a force to the right: a roll to the right needs a push
        // to the left first.
        let front_force = -torque / input.cg_height.max(0.05) / input.front_arm.max(choices::MIN_LEAN_ARM);
        LeanPlan { limit, target, acceleration: accel, torque, front_force, capped: target != free }
    }
}

/// The wheel's `Slip.y` (rad as the model reads it, positive when the patch moves to the right) at which a tyre at load `fz_kn` and camber `gamma_deg`
/// pushes sideways with `wanted` newtons (positive to the right). Beyond the peak of the curve it is the slip of the peak.
pub fn slip_for_force(tyre: &Mf94, fz_kn: f32, gamma_deg: f32, wanted: f32) -> f32 {
    let p = peaks(tyre, fz_kn, gamma_deg);
    let (mut low, mut high) = (p.y_negative.to_degrees(), p.y_positive.to_degrees());
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

/// The handlebar angle (rad, positive right) for a front contact that moves with the world `velocity` over ground of unit `normal`
/// (world frame), a bike of orientation `rotation` (body to world; body frame x right, y up, z forward) and the wanted lateral slip
/// `slip_y` (`ride6-feel-round9.md`, section 1; the binary's `0x1450c9320`, `0x145158890`): the velocity without its component along
/// the ground normal is the wheel's direction of travel; the wanted heading is that direction turned to the left by the slip, in the
/// plane of the ground; the angle is the heading of that vector in the chassis frame, projected **along the ground normal** onto the
/// chassis' horizontal plane. A bike leaned by `phi` therefore needs `slip / cos(phi)` of handlebar for `slip` on the ground.
pub fn handlebar_angle(velocity: Vec3, normal: Vec3, rotation: Mat3, slip_y: f32) -> f32 {
    let Some(direction) = (velocity - normal * velocity.dot(normal)).try_normalize() else { return 0.0 };
    let heading = direction - normal.cross(direction) * slip_y;
    let to_body = rotation.transpose();
    let (heading, up) = (to_body * heading, to_body * normal);
    if up.y.abs() < 1e-3 {
        return 0.0;
    }
    let flat = heading - up * (heading.y / up.y);
    flat.x.atan2(flat.z)
}

/// The share of the wanted front force the controller asks once the bike leans: the force is divided by the cosine of the lean of the
/// chassis' right axis from the ground plane (`ride6-feel-round9.md`, section 1; `0x1450c9320`), at least [`choices::MIN_LEAN_ARM`].
pub fn lean_arm(right: Vec3, normal: Vec3) -> f32 {
    (1.0 - right.dot(normal).powi(2)).max(0.0).sqrt().max(choices::MIN_LEAN_ARM)
}

/// The handlebar angle the controller aims at (rad): the plan's angle, blended to the stick's direct mapping at a walking pace.
pub fn blend_to_stick(steering: &SteeringSpec, speed: f32, stick: f32, controlled: f32) -> f32 {
    let direct = stick.clamp(-1.0, 1.0) * steering.max_angle;
    let share = ramp(speed.abs(), steering.lean.blend_range);
    direct + (controlled - direct) * share
}

#[path = "lean_spec.rs"]
pub(super) mod extra;
#[path = "lean_help.rs"]
pub(super) mod help;

#[cfg(test)]
#[path = "lean_tests.rs"]
mod tests;
