//! The per-step tire rules of the police simple chassis: drive slip and traction, the rear oversteer correction, plain
//! brakes and the cornering drag. Spec: `docs/specs/cop-simple-chassis.md` section 1.

use glam::Vec3;

use super::Vehicle;
use crate::drivetrain::GEAR_FIRST;
use crate::math::ramp;

/// The oversteer at which the rear correction is full, radians (35 degrees).
const OVERSTEER_FULL: f32 = 35.0 * std::f32::consts::PI / 180.0;
/// The speed at which the rear correction is full, m/s.
const OVERSTEER_SPEED: f32 = 20.0;
/// The largest rear traction gain of the correction.
const OVERSTEER_GAIN: f32 = 0.15;
/// The share of the lateral force that slows a cornering police car.
const STEER_DRAG_REDUCTION: f32 = 0.5;
/// Traction of a driven wheel while staging.
const STAGING_TRACTION: f32 = 0.25;

impl Vehicle {
    /// Gives the car the simple chassis' own tires (police cars). Takes effect while the simple chassis is selected.
    pub fn set_simple_tires(&mut self, on: bool) {
        self.simple_tires = on;
    }

    /// Whether the simple tires were asked for (they apply only on a simple chassis).
    pub fn simple_tires(&self) -> bool {
        self.simple_tires
    }

    pub(super) fn simple_tires_on(&self) -> bool {
        self.simple_tires && self.simple_chassis.is_some() && self.rolling_chassis.is_none()
    }

    /// The rear oversteer correction and the drive: half of each axle's share to each grounded wheel, which may slip
    /// in the low gears under throttle and takes the nitrous boost. False when the simple tires are off.
    pub(super) fn simple_drive(&mut self, torque: f32, grounded: [bool; 4], angular_velocity: Vec3) -> bool {
        if !self.simple_tires_on() {
            return false;
        }
        let up = self.body.rotation().y_axis;
        let oversteer = (self.body_slip - 0.25 * angular_velocity.dot(up)).abs();
        let speed = self.body.linear_velocity.length();
        let grip = ramp(oversteer, 0.0, OVERSTEER_FULL) * ramp(speed, 0.0, OVERSTEER_SPEED) * OVERSTEER_GAIN;
        self.tires[2].traction_boost = 1.0 + grip;
        self.tires[3].traction_boost = 1.0 + grip;
        if torque.abs() <= f32::EPSILON {
            return true;
        }
        let staging = self.launch.staging();
        let slip = staging || (self.powertrain.gear() < GEAR_FIRST + 2 && self.controls.gas > 0.5);
        let mut boost = self.powertrain.nos.boost;
        if staging {
            boost *= STAGING_TRACTION;
        }
        let front = self.spec.transmission.torque_split;
        for (i, tire) in self.tires.iter_mut().enumerate() {
            let share = match i < 2 {
                true => front,
                false => 1.0 - front,
            };
            if share == 0.0 || !grounded[i] {
                continue;
            }
            tire.drive_torque = torque * share * 0.5;
            tire.allow_slip = slip;
            tire.traction_boost *= boost;
        }
        true
    }

    /// Plain brakes: the pedal on every wheel, the handbrake on the rear ones; and the cornering drag share. False when
    /// the simple tires are off.
    pub(super) fn simple_tire_controls(&mut self) -> bool {
        if !self.simple_tires_on() {
            return false;
        }
        let c = self.controls;
        for (i, tire) in self.tires.iter_mut().enumerate() {
            tire.brake = c.brake;
            tire.ebrake = match i >= 2 {
                true => c.handbrake,
                false => 0.0,
            };
            tire.drag_reduction = STEER_DRAG_REDUCTION;
        }
        true
    }
}

#[cfg(test)]
#[path = "simple_tires_tests.rs"]
mod tests;
