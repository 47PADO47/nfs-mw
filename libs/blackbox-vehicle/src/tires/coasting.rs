//! Caller-selected rolling contact response without powered slip or handbrake.

use std::f32::consts::TAU;

use super::rolling::rolling_angle;
use super::{LoadedInput, Tire, TireParams};

#[derive(Clone, Copy, Debug)]
pub struct CoastingTireResponse {
    pub angle_range: f32,
    pub lateral_gain: f32,
    pub brake_torque_scale: f32,
    pub brake_force_gain: f32,
    pub corner_drag: f32,
}

impl Tire {
    /// Loaded, no-powered-slip contact. Caller must supply zero handbrake.
    pub fn update_coasting(&mut self, p: &TireParams, i: &LoadedInput, c: CoastingTireResponse) -> f32 {
        let (lat, fwd, radius) = (i.lat_vel, i.fwd_vel, p.radius);
        if self.load <= 0.0 && !self.brake_locked {
            self.av = fwd / radius;
        }
        self.load = i.load.max(0.0);
        self.road_speed = fwd;
        self.lateral_speed = lat;
        self.slip_angle = rolling_angle(lat, fwd.abs());
        self.slip = 0.0;
        self.brake_locked = false;
        self.traction = 1.0;
        let mut torque = self.drive_torque;
        if self.brake > 0.0 {
            torque += match fwd.abs() < 1.0 {
                true => -self.brake * i.load * fwd / radius,
                false => match self.av > 0.0 {
                    true => -self.brake * p.brake_spec * c.brake_torque_scale,
                    false => self.brake * p.brake_spec * c.brake_torque_scale,
                },
            };
        }
        let ramp = (self.slip_angle.abs() / c.angle_range).clamp(0.0, 1.0);
        let magnitude = self.grip_boost * ramp * c.lateral_gain * self.load * p.grip_scale * self.traction_boost;
        self.lateral_force = match self.slip_angle > 0.0 {
            true => -magnitude,
            false => magnitude,
        };
        self.longitudinal_force = torque / radius;
        let length = self.lateral_force.hypot(self.longitudinal_force);
        let limit = self.load * p.static_grip * self.traction_boost * (1.0 + self.brake * c.brake_force_gain);
        if length > limit && length * length > 1e-6 {
            self.traction = limit / length;
            self.lateral_force *= self.traction;
            self.longitudinal_force *= self.traction;
        }
        if fwd.abs() > 1.0 {
            self.longitudinal_force -=
                (self.slip_angle * TAU).sin() * self.lateral_force * c.corner_drag / p.grip_scale;
        } else {
            self.lateral_force *= lat.abs().min(1.0);
        }
        if self.brake <= 0.0 && !p.front {
            self.longitudinal_force *= i.surface.drive;
            self.lateral_force *= i.surface.lateral;
        }
        self.av = fwd / radius;
        self.lateral_force
    }
}

#[cfg(test)]
mod tests;
