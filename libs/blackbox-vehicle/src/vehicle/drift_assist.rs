//! The drift assist: an optional layer over the ordinary drift of `drift.rs` that makes a slide easy to start and to hold.
//!
//! It is off unless the caller turns it on with [`Vehicle::set_drift_assist`], and while it is off nothing here runs, so a
//! car is bit for bit what it was. While it is on it does three things, all driven by one small state machine (a slide that
//! is fast and crooked enough is a drift, and straightening out ends it):
//!
//! - **Entry.** Pulling the handbrake with the wheel turned at speed pushes the nose round with a yaw torque in the
//!   direction of the steering, for a short time, so the rear steps out instead of the car just braking.
//! - **Hold.** In a drift the steering sets the slip angle the car is held at (full lock asks for a share of the car's
//!   largest drift angle) and a yaw torque, strongest when the throttle is down, pulls the car to that angle from either
//!   side. Letting the wheel go straightens the car out.
//! - **Grip.** In a drift the rear tyres keep only part of their grip, so the throttle sustains the slide.
//!
//! The numbers are a [`DriftTuning`]: the car's own when its stats give them, the generic ones otherwise. The strengths are
//! in the units the physics reads (a yaw acceleration, a share of grip), not the units of any game's data; whoever fills
//! the tuning converts.

use super::Vehicle;
use crate::math::ramp;

/// A drift ends once the slip has stayed under the exit angle for this long, seconds.
const EXIT_HOLD: f32 = 0.25;
/// Longest the entry push lasts after the handbrake is pulled, seconds.
const KICK_TIME: f32 = 0.45;
/// The entry push is this share of the hold torque.
const KICK_SHARE: f32 = 0.8;
/// The wheel has to be turned this far for the handbrake to start a drift.
const KICK_STEER: f32 = 0.15;
/// The slip angle a drift is held at with full steering lock, as a share of the largest drift angle.
const TARGET_SHARE: f32 = 0.5;
/// How fast the assist fades in and out, per second.
const BLEND_RATE: f32 = 6.0;
/// The hold torque with no throttle, as a share of the full one.
const COASTING_HOLD: f32 = 0.4;
/// How much of the critical damping the pull to the target has.
const DAMPING_RATIO: f32 = 0.8;
/// The fastest change of the slip angle the damping reacts to, rad/s.
const MAX_RATE: f32 = 3.0;
/// The slip angle (in shares of the held angle) over which the hold torque rises from nothing to its full strength.
const HOLD_BAND: f32 = 0.5;

/// What the assist needs to know about a car. All angles are radians, speeds metres per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftTuning {
    /// A slide counts as a drift from this speed along the car.
    pub enter_speed: f32,
    /// ... and from this slip angle.
    pub enter_angle: f32,
    /// A drift is over once the slip stays under this angle.
    pub exit_angle: f32,
    /// The largest slip angle the assist asks for (full steering lock holds a share of it).
    pub max_angle: f32,
    /// The strongest yaw acceleration of the hold, rad/s^2. Multiplied by the car's yaw inertia to make a torque.
    pub yaw_accel: f32,
    /// The share of their grip the rear tyres keep in a drift, 0 to 1.
    pub rear_grip: f32,
}

impl DriftTuning {
    /// A car whose stats say nothing about drifting: the median numbers of the 2012 install's cars (10 m/s, 7.5 and 3 degrees,
    /// a largest angle of 40 degrees) and a yaw acceleration that holds a sports car's drift.
    pub const GENERIC: DriftTuning = DriftTuning {
        enter_speed: 10.0,
        enter_angle: 7.5 * std::f32::consts::PI / 180.0,
        exit_angle: 3.0 * std::f32::consts::PI / 180.0,
        max_angle: 40.0 * std::f32::consts::PI / 180.0,
        yaw_accel: 6.0,
        rear_grip: 0.75,
    };
}

impl Default for DriftTuning {
    fn default() -> Self {
        Self::GENERIC
    }
}

/// The assist's state between steps.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Assist {
    on: bool,
    drifting: bool,
    /// How long the conditions of a drift have been off, seconds.
    off: f32,
    /// How long the entry push has lasted, seconds (0 when the handbrake is not held).
    kick: f32,
    /// How much of the assist is on, 0 to 1.
    blend: f32,
    /// The slip angle of the step before, radians.
    last_slip: f32,
}

impl Assist {
    pub(super) fn reset(&mut self) {
        *self = Assist { on: self.on, ..Assist::default() };
    }
}

impl Vehicle {
    /// Turn the drift assist on or off. Turning it off ends the assist's own drift at once and changes nothing else.
    pub fn set_drift_assist(&mut self, on: bool) {
        self.assist.on = on;
        if !on {
            self.assist.reset();
        }
    }

    /// Whether the drift assist is on.
    pub fn drift_assist(&self) -> bool {
        self.assist.on
    }

    /// Whether the assist counts the car as drifting right now (always false with the assist off).
    pub fn assist_drifting(&self) -> bool {
        self.assist.on && self.assist.drifting
    }

    /// The assist's step, after the ordinary drift and the tyre set-up of the step. Does nothing while it is off.
    pub(super) fn update_drift_assist(&mut self, dt: f32) {
        if !self.assist.on || dt.is_nan() || dt <= 0.0 {
            return;
        }
        let tuning = self.spec.drift;
        let c = self.controls;
        let speed = self.forward_speed();
        let slip = self.body_slip;
        let grounded = self.wheels_on_ground_prev >= 2;
        let a = &mut self.assist;
        let slip_rate = if dt > 0.0 { (slip - a.last_slip) / dt } else { 0.0 };
        a.last_slip = slip;
        let fast = grounded && speed >= tuning.enter_speed;

        let sliding = fast && slip.abs() >= tuning.enter_angle;
        if !a.drifting && sliding {
            a.drifting = true;
            a.off = 0.0;
        }
        if a.drifting {
            let straight = !fast || slip.abs() < tuning.exit_angle;
            a.off = if straight { a.off + dt } else { 0.0 };
            if a.off >= EXIT_HOLD {
                a.drifting = false;
                a.off = 0.0;
            }
        }
        let provoking = fast && c.handbrake > 0.5 && c.steering.abs() > KICK_STEER;
        a.kick = if provoking { a.kick + dt } else { 0.0 };
        let kicking = provoking && a.kick <= KICK_TIME && !a.drifting;

        let wanted = if a.drifting || kicking { 1.0 } else { 0.0 };
        a.blend = if wanted > a.blend {
            (a.blend + BLEND_RATE * dt).min(wanted)
        } else {
            (a.blend - BLEND_RATE * dt).max(wanted)
        };
        if a.blend <= 0.0 {
            return;
        }
        let (drifting, blend) = (a.drifting, a.blend);
        let strength = tuning.yaw_accel * self.body.inertia().y * blend;
        let up = self.body.rotation().y_axis;

        // Entry: push the nose the way the wheel is turned.
        if kicking {
            let torque = c.steering.signum() * c.steering.abs().min(1.0) * KICK_SHARE * strength;
            self.body.apply_torque(up * torque);
        }
        if !drifting {
            return;
        }
        // Hold: a drift to the right (steering positive) is a slide with a negative slip angle, so the target slip has the
        // opposite sign of the steering. Positive torque turns the nose right, which makes the slip more negative.
        let held = tuning.max_angle * TARGET_SHARE;
        let target = -c.steering.clamp(-1.0, 1.0) * held;
        let band = held * HOLD_BAND;
        let error = ((slip - target) / band).clamp(-1.0, 1.0);
        let throttle = COASTING_HOLD + (1.0 - COASTING_HOLD) * c.gas.clamp(0.0, 1.0);
        // The pull to the target behaves like a damped spring: the damping is the rate that keeps it from swinging past.
        let damping = 2.0 * DAMPING_RATIO * (tuning.yaw_accel / band).sqrt() * self.body.inertia().y * blend;
        self.body.apply_torque(up * (error * throttle * strength + damping * slip_rate.clamp(-MAX_RATE, MAX_RATE)));
        // Grip: the rear tyres let go a part of the way, more the further into the drift the car is.
        let depth = ramp(slip.abs(), tuning.exit_angle, tuning.enter_angle) * blend;
        let rear = 1.0 - (1.0 - tuning.rear_grip.clamp(0.0, 1.0)) * depth;
        for tire in &mut self.tires[2..] {
            tire.traction_boost *= rear;
        }
    }
}
