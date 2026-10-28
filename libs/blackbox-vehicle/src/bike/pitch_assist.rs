//! The wheelie and stoppie control (`docs/specs/ride6-tricks.md`, sections 1 to 3): a pure pitch torque that holds the bike at
//! the pitch the rider's weight and pedal ask for, so a wheelie settles at its angle instead of flipping.
//!
//! Angles are nose-up positive here; the reference works in the opposite sign and its torque follows, so the two are
//! mirror images and give the same motion.

use super::curve::{factor, ramp};
use super::spec::{PitchParams, WheelieSpec};

/// The pedal that must stay below this (0 to 1) for the other lift to be asked: a wheelie needs the front brake below it and
/// a stoppie the throttle (`ride6-tricks.md`, section 1).
pub const FREE_PEDAL: f32 = 0.1;

/// `MaxAngleFactorInputTuning`: how the strength of the pedal and lever (0 to 1) scales the target angle. The data of no bike
/// sets it; this is the constructor default, a straight line from 0 to 1.
pub const INPUT_FACTOR_RANGE: [f32; 2] = [0.0, 1.0];

/// What the assist is given.
#[derive(Clone, Copy, Debug)]
pub struct PitchInput {
    /// The rider's longitudinal weight as the player asks it, -1 (back) to 1 (forward).
    pub weight: f32,
    /// The throttle and the front brake (0 to 1) after the rider help's direct interventions.
    pub throttle: f32,
    pub front_brake: f32,
    /// The pitch of the body (rad, positive nose up) and its rate (rad/s).
    pub pitch: f32,
    pub pitch_rate: f32,
    /// The road's inclination along the heading (rad, positive uphill).
    pub terrain: f32,
    /// The speed along the bike (m/s) and the normalised lean (0 upright, 1 at the lean limit).
    pub speed: f32,
    pub normalised_lean: f32,
    pub front_on_ground: bool,
    pub rear_on_ground: bool,
    /// The pitch inertia of the bike without the rider (kg m^2): the help torque is scaled by it.
    pub inertia: f32,
    /// The step (s).
    pub dt: f32,
}

/// The lever travel the rider asks the assist for, -1 (a wheelie at full) to 1 (a stoppie at full), after the dead zones.
pub fn target_position(spec: &WheelieSpec, weight: f32) -> f32 {
    let w = weight.clamp(-1.0, 1.0);
    match w >= 0.0 {
        true => ramp(w, [spec.stoppie.deadzone, 1.0]),
        false => -ramp(-w, [spec.wheelie.deadzone, 1.0]),
    }
}

/// The time-optimal bang-bang plan from a state `(x, v)` to rest at `target` with an acceleration of at most `a`: the total
/// time (signed: negative when the plan starts by decelerating) and the time of the switch. `(0, 0)` when there is no plan.
fn plan(x: f32, v: f32, a: f32, target: f32) -> (f32, f32) {
    const NONE: f32 = 1000.0;
    // The best plan whose first leg accelerates by `a` (negative `a`: the other way).
    let leg = |a: f32| -> (f32, f32) {
        let v2 = 2.0 * v * v;
        let disc = (target - x) * a * 4.0 + v2;
        if disc < 0.0 {
            return (NONE, 0.0);
        }
        let root = disc.sqrt();
        let candidate = |s: f32| -> (f32, f32) {
            let total = (s - v) / a;
            let switch = (a * total - v) / (a + a) + 1e-5;
            match total < 0.0 || switch < 0.0 || total < switch {
                true => (NONE, 0.0),
                false => (total, switch),
            }
        };
        let (up, down) = (candidate(root), candidate(-root));
        match down.0 <= up.0 {
            true => down,
            false => up,
        }
    };
    let (plus, minus) = (leg(a), leg(-a));
    match minus.0 <= plus.0 {
        true if minus.0 >= NONE => (0.0, 0.0),
        true => (-minus.0, minus.1),
        false => (plus.0, plus.1),
    }
}

/// The angular acceleration (rad/s^2) the tracker commands for a state `(x, v)` heading for `target` (all along one sign
/// convention): the bang-bang plan's acceleration, averaged over `window` seconds so the switch is smooth.
pub fn track(x: f32, v: f32, target: f32, limit: f32, window: f32) -> f32 {
    if window <= 0.0 || limit <= 0.0 {
        return 0.0;
    }
    let (total, switch) = plan(x, v, limit, target);
    if total == 0.0 {
        return 0.0;
    }
    let (time, a) = match total < 0.0 {
        true => (-total, -limit),
        false => (total, limit),
    };
    // The plan ends inside the window: the average over its own length; a switch inside the window: over the window.
    match (window < time, switch <= window) {
        (true, false) => a,
        (true, true) => (2.0 * switch / window - 1.0) * a,
        (false, _) => (2.0 * switch / time - 1.0) * a,
    }
}

/// A value moving from `from` towards `to` with a square-root approach: it covers the distance at a speed that falls as the
/// square root of what is left. `step` is the rate times the time.
pub fn approach(from: f32, to: f32, step: f32) -> f32 {
    let left = from - to;
    let d = left.abs().sqrt() - step;
    match d > 0.0 {
        true => to + left.signum() * d * d,
        false => to,
    }
}

/// The pitch torque (N m, positive raises the nose) that holds the pitch at the angle asked for; 0 when the assist is off.
pub fn torque(spec: &WheelieSpec, input: &PitchInput) -> f32 {
    let position = target_position(spec, input.weight);
    if position == 0.0 {
        return 0.0;
    }
    let wheelie = position < 0.0;
    let (params, grounded, free_lever, pedal): (&PitchParams, bool, bool, f32) = match wheelie {
        // A wheelie: the rear tyre must be on the road and the front brake off; the throttle sets the strength.
        true => (&spec.wheelie, input.rear_on_ground, input.front_brake < FREE_PEDAL, input.throttle),
        // A stoppie: the front tyre on the road and the throttle off; the front brake sets the strength.
        false => (&spec.stoppie, input.front_on_ground, input.throttle < FREE_PEDAL, input.front_brake),
    };
    if !grounded || !free_lever || input.speed.abs() < params.min_speed {
        return 0.0;
    }
    // The strength of the request is the lever times the pedal; with the pedal released the target is level.
    let strength = (position * pedal).abs().min(1.0);
    let scale = INPUT_FACTOR_RANGE[0] + (INPUT_FACTOR_RANGE[1] - INPUT_FACTOR_RANGE[0]) * strength;
    let reach = scale * ramp(input.speed.abs(), params.speed_ramp) * params.max_angle;
    let target = -position.signum() * reach;
    let current = input.pitch - input.terrain * params.terrain_influence + input.pitch_rate * params.lookahead;
    let aim = match params.filter_timescale > 0.0 {
        true => approach(current, target, params.filter_timescale * input.dt),
        false => target,
    };
    let command = track(current, input.pitch_rate, aim, params.acceleration, params.acceleration_time);
    let lean_help = 1.0 - ramp(input.normalised_lean.abs(), params.lean_help);
    // The height of the grounded wheel above the road is nothing while it carries load.
    let ground_help = 1.0 - ramp(0.0, params.ground_range);
    let help = lean_help * ground_help * factor(&params.help_speed, input.speed.abs());
    input.inertia * command * help
}

#[cfg(test)]
#[path = "pitch_assist_tests.rs"]
mod tests;
