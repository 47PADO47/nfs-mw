//! The tire of the simple chassis the original gives police cars: lateral force linear in the slip angle up to 12
//! degrees, a friction circle, plain brakes and slip only when the drive allows it. Spec: `docs/specs/cop-simple-chassis.md`.

use std::f32::consts::TAU;

use super::angle::rolling_angle;
use super::state::WHEEL_INERTIA;
use super::{LoadedInput, Tire, TireParams};
use crate::brakes::BRAKING_TORQUE_SCALE;
use crate::math::{mph_to_ms, ramp};

/// The slip angle (turns) at which the lateral force stops growing: 12 degrees.
const FULL_GRIP_ANGLE: f32 = 12.0 / 360.0;
/// Lateral force per unit of load at full slip angle, before the grip scales.
const LATERAL_GAIN: f32 = 1.54;
/// Brake torque per unit of pedal, newton metres per foot-pound of the brake spec.
const BRAKE_SCALE: f32 = 10.0;
/// How a locked rear tire shares its skid between the lateral and the longitudinal force.
const LOCK_SKID_RATIO: f32 = 1.5;
/// The wheel's resistance to a change of slip while it may slip.
const SLIP_INERTIA: f32 = 10.0;

impl Tire {
    /// The loaded step (spec section 2); returns the lateral force, newtons. `drag_reduction` is the cornering drag share.
    pub fn update_simple(&mut self, p: &TireParams, i: &LoadedInput, drag_reduction: f32) -> f32 {
        let (lat, fwd, r, dt) = (i.lat_vel, i.fwd_vel, p.radius, i.dt.max(1e-6));
        if self.load <= 0.0 && !self.brake_locked {
            self.av = fwd / r;
        }
        let fwd_acc = (fwd - self.road_speed) / dt;
        self.road_speed = fwd;
        self.lateral_speed = lat;
        self.load = i.load.max(0.0);
        let load = self.load;
        self.traction = 1.0;
        self.brake_locked = self.ebrake > 0.0 && !p.front;
        let braking = self.brake + self.ebrake > 0.0;
        let mut torque = self.drive_torque;
        if braking {
            torque += match fwd.abs() < 1.0 {
                true => -(self.brake + self.ebrake) * load * fwd / r,
                false => {
                    let pedal =
                        self.brake * p.brake_spec / BRAKING_TORQUE_SCALE * BRAKE_SCALE + self.ebrake * p.ebrake_spec;
                    match self.av <= 0.0 {
                        true => pedal,
                        false => -pedal,
                    }
                }
            };
        }
        self.slip_angle = rolling_angle(lat, fwd.abs());
        self.slip = match self.brake_locked || self.allow_slip {
            true => self.av * r - fwd,
            false => 0.0,
        };
        let mut slip_ratio = 1.0;
        let slipping = self.allow_slip && self.slip.abs() > self.max_slip;
        if slipping {
            slip_ratio = self.max_slip / self.slip.abs();
            self.traction *= slip_ratio;
        }
        let skid = self.slip.hypot(lat);
        let mut fy = self.grip_boost
            * ramp(self.slip_angle.abs(), 0.0, FULL_GRIP_ANGLE)
            * LATERAL_GAIN
            * load
            * p.grip_scale
            * self.traction_boost;
        if self.slip_angle > 0.0 {
            fy = -fy;
        }
        let skidding = self.brake_locked && skid > 1.0;
        let mut fx = match (skidding, slipping) {
            (true, _) => {
                fy = -lat * load * self.traction_boost * p.dynamic_grip * LOCK_SKID_RATIO / skid;
                self.slip * load * self.traction_boost * p.dynamic_grip / (skid * LOCK_SKID_RATIO)
            }
            (false, true) => {
                let pilot = 0.85 + 0.15 * ramp(i.body_speed, mph_to_ms(30.0), mph_to_ms(50.0));
                let dynamic = p.dynamic_grip * self.traction_boost * load * pilot / skid * self.slip;
                let ground = (torque / r).abs();
                dynamic.clamp(-ground, ground)
            }
            (false, false) => torque / r,
        };
        if self.allow_slip && !slipping && !self.brake_locked {
            fx += (self.angular_acc * r - fwd_acc) * SLIP_INERTIA / r;
        }
        let limit = load * p.static_grip * self.traction_boost * (self.brake * 2.5 + 1.0);
        let total = fx * fx + fy * fy;
        if total > limit * limit && total > 1e-6 {
            let scale = limit / total.sqrt();
            self.traction *= scale;
            fx *= scale;
            fy *= scale;
            slip_ratio *= scale * scale;
        }
        match fwd.abs() > 1.0 {
            true => fx -= (self.slip_angle * TAU).sin() * fy * drag_reduction / p.grip_scale.max(1e-6),
            false => fy *= lat.abs().min(1.0),
        }
        if !braking && !p.front {
            fx *= i.surface.drive;
            fy *= i.surface.lateral;
        }
        self.longitudinal_force = fx;
        self.lateral_force = fy;
        self.spin(torque, fwd_acc, slip_ratio, r, dt, i.surface.rolling);
        fy
    }

    /// The wheel speed after a loaded step: stopped when locked, its own while it slips, the road's otherwise.
    fn spin(&mut self, torque: f32, fwd_acc: f32, slip_ratio: f32, r: f32, dt: f32, rolling: f32) {
        if self.brake_locked {
            self.angular_acc = 0.0;
            self.av = 0.0;
            return;
        }
        if !(self.allow_slip && slip_ratio < 1.0) {
            self.av = self.road_speed / r;
            return;
        }
        let t = ((torque - self.longitudinal_force * r) + self.last_torque) * 0.5;
        self.last_torque = t;
        let effective = t - self.av * 1.5 * rolling;
        let free = effective * 0.1 - slip_ratio * self.slip / (r * dt);
        self.angular_acc = free + (fwd_acc / r - free) * slip_ratio;
        self.av += self.angular_acc * dt;
    }

    /// A simple tire off the ground (spec section 3).
    pub fn update_simple_free(&mut self, p: &TireParams, dt: f32) {
        self.traction = 0.0;
        self.load = 0.0;
        self.slip = 0.0;
        self.slip_angle = 0.0;
        self.brake_locked = self.ebrake > 0.0 && !p.front;
        match self.brake_locked {
            true => self.av = 0.0,
            false => self.av += self.drive_torque / WHEEL_INERTIA * dt,
        }
        self.lateral_force = 0.0;
        self.longitudinal_force = 0.0;
        self.last_torque = 0.0;
        self.angular_acc = 0.0;
    }
}

#[cfg(test)]
#[path = "simple_tests.rs"]
mod tests;
