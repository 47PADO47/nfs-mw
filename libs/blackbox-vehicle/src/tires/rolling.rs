//! Caller-selected rolling tire responses, independent of the racing slip solver.

use std::f32::consts::TAU;

use super::{LoadedInput, Tire, TireParams};
use crate::math::table;

#[derive(Clone, Copy, Debug)]
pub enum RollingTireResponse {
    Linear { gain: f32, corner_drag: f32 },
    Curve { samples: [f32; 7], range: f32, rear_scale: [f32; 2], corner_drag: f32 },
}

impl Tire {
    pub fn update_rolling(&mut self, p: &TireParams, i: &LoadedInput, response: RollingTireResponse) -> f32 {
        let (lat, fwd, radius) = (i.lat_vel, i.fwd_vel, p.radius);
        if self.load <= 0.0 && matches!(response, RollingTireResponse::Linear { .. }) {
            self.av = fwd / radius;
        }
        self.load = i.load.max(0.0);
        self.road_speed = fwd;
        self.lateral_speed = lat;
        self.slip_angle = rolling_angle(lat, fwd.abs());
        let slip = self.av * radius - fwd;
        let mut torque = self.drive_torque;
        let sign_speed = match response {
            RollingTireResponse::Linear { .. } => self.av,
            RollingTireResponse::Curve { .. } => fwd,
        };
        let brake = self.brake * p.brake_spec + self.ebrake * p.ebrake_spec;
        torque += match sign_speed > 0.0 {
            true => -brake,
            false => brake,
        };
        self.longitudinal_force = torque / radius;
        self.brake_locked = self.ebrake > 0.0;
        self.traction = match self.brake_locked {
            true => 0.0,
            false => 1.0,
        };
        let corner_drag = match response {
            RollingTireResponse::Linear { gain, corner_drag } => {
                self.slip = match self.ebrake > 0.0 && fwd.abs() > 1.0 {
                    true => slip,
                    false => 0.0,
                };
                let skid = slip.hypot(lat);
                self.lateral_force = -lat * gain * self.load * p.grip_scale;
                if skid > 1.0 {
                    self.lateral_force /= skid;
                }
                if self.ebrake > 0.5 && skid > 1.0 {
                    self.longitudinal_force = -fwd * gain * self.load * p.grip_scale / skid;
                }
                self.av = (1.0 - self.ebrake) * fwd / radius;
                corner_drag
            }
            RollingTireResponse::Curve { samples, range, rear_scale, corner_drag } => {
                self.slip = slip;
                self.lateral_force = self.load * p.grip_scale * table(&samples, 0.0, range, self.slip_angle.abs());
                if lat > 0.0 {
                    self.lateral_force = -self.lateral_force;
                }
                if !p.front {
                    self.lateral_force *= rear_scale[usize::from(self.ebrake > 0.5)];
                }
                let magnitude = self.longitudinal_force.hypot(self.lateral_force);
                if magnitude > self.load * p.static_grip && magnitude > 0.001 {
                    let scale = self.load * p.dynamic_grip / magnitude;
                    self.longitudinal_force *= scale;
                    self.lateral_force *= scale;
                }
                self.av = match self.brake_locked {
                    true => 0.0,
                    false => fwd / radius,
                };
                if self.brake_locked {
                    self.slip = -fwd;
                }
                corner_drag
            }
        };
        if fwd.abs() > 1.0 {
            self.longitudinal_force -= (self.slip_angle * TAU).sin() * self.lateral_force * corner_drag;
        } else {
            self.lateral_force *= lat.abs().min(1.0);
        }
        self.angular_acc = 0.0;
        self.last_torque = 0.0;
        self.lateral_force
    }

    pub fn update_rolling_free(&mut self) {
        self.load = 0.0;
        self.slip = 0.0;
        self.slip_angle = 0.0;
        self.lateral_speed = 0.0;
        self.lateral_force = 0.0;
        self.longitudinal_force = 0.0;
        self.brake_locked = self.brake > 0.0 || self.ebrake > 0.0;
        if self.brake_locked {
            self.av = 0.0;
        }
    }
}

/// Mathematical octant samples with integer interpolation; no external lookup data.
pub(super) fn rolling_angle(lateral: f32, forward: f32) -> f32 {
    static OCTANT: std::sync::OnceLock<[u16; 257]> = std::sync::OnceLock::new();
    let nodes = OCTANT.get_or_init(|| {
        std::array::from_fn(|i| ((i as f64 / 256.0).atan() * 65536.0 / std::f64::consts::TAU).round() as u16)
    });
    let y = match lateral.abs() <= 1e-6 {
        true => 0.0,
        false => lateral.abs(),
    };
    let x = match forward.abs() <= 1e-6 {
        true => 0.0,
        false => forward.abs(),
    };
    let largest = x.max(y);
    if largest == 0.0 {
        return 0.0;
    }
    let ratio = (f64::from(x.min(y)) / f64::from(largest) * 65536.0) as u32;
    let index = (ratio >> 8) as usize;
    let base = u32::from(nodes[index]);
    let next = u32::from(nodes[(index + 1).min(256)]);
    let interpolated = base + (((next - base) * (ratio & 255)) >> 8);
    let angle = match y > x {
        true => 16384 - interpolated,
        false => interpolated,
    };
    let sign = match lateral < 0.0 {
        true => -1.0,
        false => 1.0,
    };
    sign * angle as f32 / 65536.0
}

#[cfg(test)]
mod tests;
