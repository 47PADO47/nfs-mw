//! Configurable rolling chassis: direct controls, fixed steering and simple drag.

use glam::Vec3;

use super::Vehicle;
use crate::steering::WheelAngles;
use crate::tires::RollingTireResponse;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug)]
pub struct RollingChassisConfig {
    pub tires: RollingTireResponse,
    pub steering_degrees: f32,
    /// Vertical contact lever-arm multiplier; horizontal components retain their geometry.
    pub force_arm_height: f32,
    pub front_drive_share: f32,
    pub handbrake_front: bool,
    /// Fixed body-local centre of gravity, or the normal common chassis calculation.
    pub cog: Option<Vec3>,
    pub load_from_up: bool,
}

impl Vehicle {
    pub fn set_rolling_chassis(&mut self, config: Option<RollingChassisConfig>) {
        self.rolling_chassis = config;
        let front_share = config.map_or(self.spec.transmission.torque_split, |c| c.front_drive_share);
        self.powertrain.set_front_drive_share(front_share);
        if config.is_some() {
            self.set_simple_chassis(None);
        }
    }

    pub fn set_rolling_cog(&mut self, cog: Vec3) {
        if let Some(config) = self.rolling_chassis.as_mut() {
            config.cog = Some(cog);
        }
    }

    pub(super) fn rolling_steering(&mut self) -> bool {
        let Some(config) = self.rolling_chassis else { return false };
        let angle = (self.controls.steering * config.steering_degrees).to_radians();
        self.wheel_angles = self.forced_wheel_angles.unwrap_or(WheelAngles { left: angle, right: angle });
        true
    }

    pub(super) fn rolling_tire_controls(&mut self) -> bool {
        let Some(config) = self.rolling_chassis else { return false };
        for (i, tire) in self.tires.iter_mut().enumerate() {
            tire.brake = self.controls.brake;
            tire.ebrake = match config.handbrake_front || i >= 2 {
                true => self.controls.handbrake,
                false => 0.0,
            };
        }
        true
    }

    pub(super) fn rolling_drive(&mut self, torque: f32, grounded: [bool; 4]) -> bool {
        let Some(config) = self.rolling_chassis else { return false };
        for (i, tire) in self.tires.iter_mut().enumerate() {
            if !grounded[i] {
                continue;
            }
            let share = match i < 2 {
                true => config.front_drive_share,
                false => 1.0 - config.front_drive_share,
            };
            tire.drive_torque = torque * share * 0.5;
        }
        true
    }
}
