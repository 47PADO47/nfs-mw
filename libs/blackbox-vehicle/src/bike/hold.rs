//! The standing hold: a bike that stands on a slope with no throttle stays where it is (`docs/specs/ride6-bike-standstill-hold.md`).
//!
//! A bike on a slope with its brakes off and no throttle rolls: gravity along the slope is more than its rolling resistance above about one
//! degree. The rider of the game would put his feet down and hold it (his standing clips, `docs/specs/ride6-rider-standstill.md`), and the
//! reverse walk (the rider paddles the bike back) is for the rider who asks for it, not for one who is holding the bike. So below a small
//! speed, with no throttle and on a slope of at most [`MAX_SLOPE`], the hold is a parking brake with the rider's feet:
//!
//! - both brake levers are held at [`LEVER`] at least, which locks the wheels against the tyres' own friction;
//! - a force cancels the pull of gravity along the road (`m g sin(slope)`), so nothing is left to roll the bike;
//! - the velocity along the road is damped with the time constant [`DAMPING_TIME`], so a creep of the last step does not stay.
//!
//! It lets go at the throttle, when the bike is pushed back by the reverse walk, in a burnout, in a fall, with both wheels off the road, over
//! [`LEAVE_SPEED`] along the road (a bike that is really rolling is not held, the brakes are the rider's) and past [`MAX_SLOPE`] (the force fades out
//! over [`SLOPE_FADE`]: nobody holds a bike on a steep ramp). On a slope under [`MIN_SLOPE`] nothing changes: the bike's own rolling resistance
//! holds it.

use glam::Vec3;

use super::curve::ramp;
use super::{Bike, FRONT, REAR};

/// The speed along the road (m/s) under which a bike is taken to stand and held. 0.8 m/s is a walking pace of a rider holding the bike; the
/// reverse walk's animation starts at -0.28 m/s (-1 km/h), so a bike that creeps back from rest is held long before. A project's choice.
pub const HOLD_SPEED: f32 = 0.8;
/// The speed along the road (m/s) over which a held bike is let go again: the hold's hysteresis, so a bike at the edge does not flutter.
pub const LEAVE_SPEED: f32 = 1.2;
/// The throttle (0 to 1, after the rider help) at or over which the hold lets go.
pub const THROTTLE: f32 = 0.05;
/// The slope (rad) from which the hold works: 1 degree, about where gravity along the road (0.17 m/s^2) passes the rolling resistance of the
/// wheels (0.02 g, 0.2 m/s^2).
pub const MIN_SLOPE: f32 = 0.017_453_3;
/// The steepest slope (rad) the hold keeps in full: 30 degrees, `tan` 0.58, which the rider's feet and a locked tyre (mu 1.2) do hold, and which is
/// steeper than any road. Past it the force fades out over [`SLOPE_FADE`].
pub const MAX_SLOPE: f32 = std::f32::consts::FRAC_PI_6;
/// The width (rad, 5 degrees) over which the hold fades out past [`MAX_SLOPE`].
pub const SLOPE_FADE: f32 = 0.087_3;
/// The share of each brake lever held at least while the bike is held. Half a lever: enough to lock a wheel that no engine drives, little
/// enough that the brake torque stays a hold and not a stoppie.
pub const LEVER: f32 = 0.5;
/// The time constant (s) of the damping of the velocity along the road. A project's choice: quick enough to stop a creep within a few frames,
/// slow enough that a bike that is stopped by the hold from 0.8 m/s does not jerk its rider (0.15 s is 9 frames at 60 Hz).
pub const DAMPING_TIME: f32 = 0.15;

/// What the hold keeps from step to step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HoldState {
    /// Whether the bike is held.
    pub active: bool,
}

impl Bike {
    /// Whether the rider is holding the bike on a slope (the standing hold is on).
    pub fn held_on_slope(&self) -> bool {
        self.hold.active
    }

    /// The normal of the road under the bike: the mean of the two tyres' when both touch it, the one's when one does; `None` in the air.
    pub(super) fn ground_normal(&self) -> Option<Vec3> {
        let (front, rear) = (&self.wheels[FRONT], &self.wheels[REAR]);
        let normal = match (front.on_ground, rear.on_ground) {
            (true, true) => front.normal + rear.normal,
            (true, false) => front.normal,
            (false, true) => rear.normal,
            (false, false) => return None,
        };
        normal.try_normalize()
    }

    /// The slope (rad) of the road under the bike; 0 in the air.
    pub(super) fn ground_slope(&self) -> f32 {
        self.ground_normal().map_or(0.0, |normal| normal.y.clamp(-1.0, 1.0).acos())
    }

    /// The hold's part of one sub-step: the levers it keeps, the force and the damping it puts on the body. `throttle` is what the engine is
    /// given (0 to 1, after the rider help) and `levers` the brake levers `(front, rear)` as the rider and the reverse leave them.
    pub(super) fn step_hold(&mut self, dt: f32, throttle: f32, levers: (f32, f32)) -> (f32, f32) {
        let was_active = self.hold.active;
        self.hold.active = false;
        let Some(normal) = self.ground_normal() else { return levers };
        let velocity = self.body.linear_velocity;
        let along_road = velocity - normal * velocity.dot(normal);
        let leave = match was_active {
            true => LEAVE_SPEED,
            false => HOLD_SPEED,
        };
        let slope = normal.y.clamp(-1.0, 1.0).acos();
        let weight = 1.0 - ramp(slope, [MAX_SLOPE, MAX_SLOPE + SLOPE_FADE]);
        let rider_on = !self.fallen && !self.departure.off && !self.burnout && !self.reverse.pushing();
        let holding = rider_on && throttle < THROTTLE && slope >= MIN_SLOPE && weight > 0.0;
        if !holding || along_road.length() >= leave {
            return levers;
        }
        self.hold.active = true;
        let gravity = Vec3::NEG_Y * 9.81;
        let along_slope = gravity - normal * gravity.dot(normal);
        self.body.apply_force(-along_slope * (self.body.mass() * weight));
        let keep = (-dt / DAMPING_TIME).exp();
        self.body.linear_velocity -= along_road * ((1.0 - keep) * weight);
        (levers.0.max(LEVER), levers.1.max(LEVER))
    }
}

#[cfg(test)]
#[path = "hold_tests.rs"]
mod tests;
