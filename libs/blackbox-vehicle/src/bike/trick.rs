//! The rider help around a wheelie or a stoppie, and how the bike uses it each step (`docs/specs/ride6-tricks.md`, sections 3
//! to 5): the throttle and front brake interventions, the auto-rider that moves the rider's weight, the pitch estimate they
//! share, and the pitch torque that holds the angle. Only a bike has any of it: a four-wheel vehicle has no code path here.

use glam::Vec3;

use super::curve::{curve, ramp};
use super::pitch_assist::{self, FREE_PEDAL, PitchInput, approach};
use super::spec::{RiderHelpSpec, WheelieSpec};
use super::trick_spec::AutoTuckSpec;
use super::{Bike, BikeInput, FRONT, REAR, lean_of, pitch_of};

/// Below this the share of the weight shift counts as none (the reference compares with a small constant it does not name).
const NO_SHIFT: f32 = 1e-4;

/// The state the help keeps from step to step.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TrickState {
    /// The rider's weight aimed at by the help (-1 back to 1 forward), after its own filter.
    pub aid: f32,
    /// The pedals after the interventions that act on the pedal itself: they set the strength of a wheelie or a stoppie.
    pub throttle: f32,
    pub front_brake: f32,
    /// The tuck-in action (0 or 1 for a rider who tucks in by the rule; the input's own value when it is held) that scales his
    /// air drag, and the tuck level that follows it at the rate of the input handler and goes to the animation.
    pub tuck: f32,
    pub tuck_level: f32,
}

/// The rate (1/s) at which the tuck level rises while the rider tucks and falls when he does not: the constructor defaults of
/// the input handler (`TuckInRaiseRate`, `TuckInFallRate`, both 2; no bike's data sets them).
pub const TUCK_LEVEL_RATE: f32 = 2.0;

/// The action counts as tucked above this (the reference compares the action with one half).
pub const TUCK_THRESHOLD: f32 = 0.5;

/// What the help looks at.
#[derive(Clone, Copy, Debug)]
pub(super) struct Situation {
    /// The lift (rad, positive nose up) the help estimates and its rate (rad/s).
    pub lift: f32,
    pub rate: f32,
    /// The speed along the bike (m/s).
    pub speed: f32,
    /// The weight, throttle and front brake the player asks, before any intervention.
    pub weight: f32,
    pub throttle: f32,
    pub front_brake: f32,
}

/// What the interventions leave of the pedals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Help {
    /// The throttle after the interventions on the pedal, and the cut (0 to 1) the engine's modulation adds on top.
    pub throttle: f32,
    pub engine_cut: f32,
    /// The front brake after the interventions on the pedal, and after all of them.
    pub brake_pedal: f32,
    pub brake: f32,
}

/// A range that is `[0, 0]` is off.
fn ramp_on(x: f32, range: [f32; 2]) -> f32 {
    match range == [0.0, 0.0] {
        true => 0.0,
        false => ramp(x, range),
    }
}

/// -1, 0 or 1.
fn sign(x: f32) -> f32 {
    if x > 0.0 {
        return 1.0;
    }
    if x < 0.0 {
        return -1.0;
    }
    0.0
}

fn blend(from: [f32; 2], to: [f32; 2], t: f32) -> [f32; 2] {
    [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t]
}

/// The throttle and front brake interventions of the rider help. The front lifting (a positive lift) cuts the throttle; the
/// rear lifting eases the front brake.
pub(super) fn interventions(spec: &WheelieSpec, s: &Situation) -> Help {
    let help = &spec.help;
    let mut out = Help { throttle: s.throttle, engine_cut: 0.0, brake_pedal: s.front_brake, brake: s.front_brake };
    if s.lift > 0.0 {
        let enabled = [spec.throttle_pitch, spec.forced_throttle_pitch, spec.throttle_pitch_speed] != [[0.0; 2]; 3];
        if !enabled {
            return out;
        }
        let forced = s.weight < -help.weight_deadzone && s.throttle > FREE_PEDAL;
        let angle = match forced {
            true => (ramp_on(s.lift, spec.forced_throttle_pitch), false),
            false => {
                let along = match help.launch_speed > 0.0 {
                    true => (s.speed / help.launch_speed).clamp(0.0, 1.0),
                    false => 1.0,
                };
                let range = blend(help.launch_throttle_pitch, spec.throttle_pitch, along);
                let gain = help.launch_throttle_multiplier
                    + (help.throttle_multiplier - help.launch_throttle_multiplier) * along;
                (ramp(s.lift, range) * gain, help.throttle_pitch_electronic)
            }
        };
        let speed = (ramp_on(s.rate.max(0.0), spec.throttle_pitch_speed), help.throttle_speed_electronic);
        let mut cut = 0.0;
        for (value, electronic) in [angle, speed] {
            let value = (value * help.intensity).clamp(0.0, 1.0);
            match electronic {
                true => cut += value,
                false => out.throttle *= 1.0 - value,
            }
        }
        out.engine_cut = cut.clamp(0.0, 1.0);
        return out;
    }
    let enabled = spec.brake_pitch != [0.0; 2] || spec.brake_pitch_speed != [0.0; 2];
    if !enabled {
        return out;
    }
    let forced = s.weight > help.weight_deadzone && s.front_brake > FREE_PEDAL;
    let angle = match forced {
        true => (ramp(-s.lift, help.forced_brake_pitch), false),
        false => (ramp_on(-s.lift, spec.brake_pitch) * help.brake_multiplier, help.brake_pitch_electronic),
    };
    let speed = (ramp_on((-s.rate).max(0.0), spec.brake_pitch_speed), help.brake_speed_electronic);
    let mut electronic = 1.0;
    for (value, on_engine) in [angle, speed] {
        let value = (value * help.intensity).clamp(0.0, 1.0);
        match on_engine {
            true => electronic *= 1.0 - value,
            false => out.brake_pedal *= 1.0 - value,
        }
    }
    out.brake = out.brake_pedal * electronic;
    out
}

/// The rider's weight the help aims at (-1 back to 1 forward) and, after its filter, where it is now. The player's own weight
/// beyond the dead zone is the weight (filtered at the manual speed); within it the auto-rider rests at a position the
/// throttle and brake give and shifts away from a lift, forward against a wheelie and back against a stoppie.
pub(super) fn rider_position(help: &RiderHelpSpec, s: &Situation, previous: f32, dt: f32) -> f32 {
    let Some(auto) = &help.auto_rider else { return s.weight };
    let (aim, rate) = match s.weight.abs() > help.weight_deadzone {
        true => (s.weight, auto.manual_engage_speed),
        false => {
            let rest = curve(&auto.throttle_curve, s.throttle) * auto.throttle_gain
                + curve(&auto.brake_curve, s.front_brake) * auto.brake_gain;
            let range = match s.lift > 0.0 {
                true => auto.wheelie_range,
                false => auto.stoppie_range,
            };
            let by_angle = ramp(s.lift.abs(), range) * sign(s.lift) * auto.multiplier;
            let by_speed = ramp(s.rate.abs(), auto.pitch_speed_range) * sign(s.rate);
            let shift = ((by_angle + by_speed).clamp(-1.0, 1.0) * help.intensity).clamp(-1.0, 1.0);
            let aim = match shift.abs() <= NO_SHIFT {
                true => rest * help.intensity,
                false => rest + (shift.signum() - rest) * shift.abs(),
            };
            (aim, auto.engage_speed)
        }
    };
    match rate > 0.0 {
        true => approach(previous, aim, rate * dt),
        false => aim,
    }
}

/// What the tuck-in rule looks at (`docs/specs/ride6-rider-round6.md`, section 2).
#[derive(Clone, Copy, Debug)]
pub(super) struct TuckSituation {
    /// The speed along the bike (m/s) and the roll over the steepest lean the model allows (-1 to 1, positive to the right).
    pub speed: f32,
    pub normalised_lean: f32,
    /// The player's weight, throttle and front brake, as he asks them.
    pub weight: f32,
    pub throttle: f32,
    pub front_brake: f32,
}

/// Whether the rider tucks in: a held tuck input (above one half) tucks him; otherwise, with the rule on, he tucks when he is
/// faster than `min_speed`, the lean is under its limit (a limit under 0 switches the test off), the player does not hold the
/// weight beyond the dead zone, the throttle is open beyond its minimum and the front brake under its maximum.
pub(super) fn tuck_requested(auto: Option<&AutoTuckSpec>, weight_deadzone: f32, held: f32, s: &TuckSituation) -> bool {
    if held > TUCK_THRESHOLD {
        return true;
    }
    let Some(auto) = auto else { return false };
    let leaning = auto.max_normalised_lean >= 0.0 && auto.max_normalised_lean <= s.normalised_lean.abs();
    let held_back = weight_deadzone <= s.weight.abs();
    !(s.speed.abs() <= auto.min_speed
        || leaning
        || held_back
        || s.throttle <= auto.min_throttle
        || auto.max_brake <= s.front_brake)
}

/// The tuck level one step on: it rises towards 1 while the action is tucked, falls towards 0 otherwise.
pub(super) fn tuck_level(level: f32, tucked: bool, rise: f32, fall: f32, dt: f32) -> f32 {
    match tucked {
        true => (level + rise * dt).min(1.0),
        false => (level - fall * dt).max(0.0),
    }
}

impl Bike {
    /// The road's inclination along the heading (rad, positive uphill) under the grounded wheel (the front when both are down).
    fn terrain_inclination(&self) -> f32 {
        let normal = match (self.wheels[FRONT].on_ground, self.wheels[REAR].on_ground) {
            (true, _) => self.wheels[FRONT].normal,
            (false, true) => self.wheels[REAR].normal,
            (false, false) => return 0.0,
        };
        let forward = self.body.rotation() * Vec3::Z;
        let flat = Vec3::new(forward.x, 0.0, forward.z).try_normalize().unwrap_or(Vec3::Z);
        (-normal.dot(flat)).atan2(normal.y)
    }

    /// The lift (rad, positive nose up) the rider help works from: the body's pitch, or the angle of the line through the hubs
    /// when that is smaller (it is nothing with both wheels down, on any slope), looked ahead along its rate; zero while the
    /// wheel that should lift still touches the road.
    fn help_lift(&self, lookahead: f32, rate: f32) -> f32 {
        let body = pitch_of(self.body.rotation());
        let line = self.lift_angle();
        let angle = match body.abs() > line.abs() {
            true => line,
            false => body,
        };
        let ahead = rate * lookahead;
        let ahead = match angle > 0.0 {
            true => ahead.max(0.0),
            false => ahead.min(0.0),
        };
        let lift = angle + ahead;
        let lifted = match lift > 0.0 {
            true => FRONT,
            false => REAR,
        };
        match self.wheels[lifted].on_ground {
            true => 0.0,
            false => lift,
        }
    }

    /// The nose-up rate (rad/s) of the body.
    pub(super) fn pitch_rate(&self) -> f32 {
        // Rotating about the right axis lowers the nose.
        -self.body.angular_velocity.dot(self.body.rotation() * Vec3::X)
    }

    /// Run the rider help for this step: the interventions on the pedals and the auto-rider's weight. Returns what is left of
    /// the pedals; the weight is kept for [`Bike::shift_weight`].
    pub(super) fn rider_help(&mut self, dt: f32, input: &BikeInput, speed: f32) -> Help {
        let (throttle, brake) = (input.throttle.clamp(0.0, 1.0), input.front_brake.clamp(0.0, 1.0));
        let passthrough = Help { throttle, engine_cut: 0.0, brake_pedal: brake, brake };
        let assist = self.spec.wheelie.clone();
        let (auto, deadzone, rise, fall) = match &assist {
            Some(a) => (a.help.auto_tuck, a.help.weight_deadzone, a.help.tuck_raise_rate, a.help.tuck_fall_rate),
            None => (None, 1.0, TUCK_LEVEL_RATE, TUCK_LEVEL_RATE),
        };
        let max_lean = self.spec.steering.max_lean.max(f32::EPSILON);
        let situation = TuckSituation {
            speed,
            normalised_lean: (lean_of(self.body.rotation()) / max_lean).clamp(-1.0, 1.0),
            weight: self.weight_input,
            throttle,
            front_brake: brake,
        };
        let tucked = tuck_requested(auto.as_ref(), deadzone, self.tuck, &situation);
        let tuck = f32::from(tucked);
        let tuck_level = tuck_level(self.trick.tuck_level, tucked, rise, fall, dt);
        let Some(mut assist) = assist else {
            self.trick = TrickState { aid: self.weight_input, throttle, front_brake: brake, tuck, tuck_level };
            return passthrough;
        };
        // The rider's ABS level shrinks the start of the brake relief range (`SETTINGS_ABS`, `AutoRiderStoppieBrakeControl.X`).
        assist.brake_pitch[0] *= self.spec.abs_level.relief(self.abs_level);
        let rate = self.pitch_rate();
        let situation = Situation {
            lift: self.help_lift(assist.help.lookahead, rate),
            rate,
            speed,
            weight: self.weight_input,
            throttle,
            front_brake: brake,
        };
        let mut help = interventions(&assist, &situation);
        if speed.abs() < super::choices::STANDING_SPEED {
            // The brake help waits for the bike to move; the throttle help does not.
            help.brake_pedal = brake;
            help.brake = brake;
        }
        let aid = rider_position(&assist.help, &situation, self.trick.aid, dt);
        self.trick = TrickState { aid, throttle: help.throttle, front_brake: help.brake_pedal, tuck, tuck_level };
        help
    }

    /// The tuck-in action now (0 upright, 1 tucked): it scales the rider's air drag by the curve of his posture.
    pub fn tuck_action(&self) -> f32 {
        self.trick.tuck
    }

    /// How far the rider is tucked in, 0 to 1: the action followed at the input handler's rate. The animation's `TuckedIn`.
    pub fn tuck_level(&self) -> f32 {
        self.trick.tuck_level
    }

    /// The pitch torque that holds the angle the rider's weight and pedals ask for, about the bike's right axis.
    pub(super) fn apply_pitch_assist(&mut self, dt: f32, speed: f32) {
        let Some(assist) = &self.spec.wheelie else { return };
        let rotation = self.body.rotation();
        let limit = self.plan.limit.max(0.05);
        let request = PitchInput {
            weight: self.weight_input,
            throttle: self.trick.throttle,
            front_brake: self.trick.front_brake,
            pitch: pitch_of(rotation),
            pitch_rate: self.pitch_rate(),
            terrain: self.terrain_inclination(),
            speed,
            normalised_lean: (lean_of(rotation).abs() / limit).min(1.0),
            front_on_ground: self.wheels[FRONT].on_ground,
            rear_on_ground: self.wheels[REAR].on_ground,
            // The pitch inertia of the bike without the rider.
            inertia: self.spec.chassis.mass * self.spec.chassis.tensor_scale.x,
            dt,
        };
        let torque = pitch_assist::torque(assist, &request);
        self.body.apply_torque(rotation * Vec3::X * -torque);
    }
}

#[cfg(test)]
#[path = "trick_tests.rs"]
mod tests;
