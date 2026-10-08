//! Per-step tire setup: the speed-dependent scales, brake commands, the rear-wheel stability boost and the
//! handling tuning (spec sections 3.7 and 4.1). Burnout, drift and traction control are not modelled.

use super::Vehicle;
use crate::brakes::{BrakeContext, wheel_command};
use crate::drivetrain::GEAR_REVERSE;
use crate::math::table;
use crate::tires::StepScales;

impl Vehicle {
    /// Starts the step on every tire and applies the assists. Call before the drive torque is added.
    pub(super) fn begin_tires(&mut self, speed: f32) {
        let scales = StepScales::new(speed, self.powertrain.gear() == GEAR_REVERSE);
        for t in &mut self.tires {
            t.begin_frame(scales.max_slip, scales.grip_scale, scales.traction_scale);
        }
    }

    /// Brake commands, stability boost and handling tuning. Call after the drive torque is added.
    pub(super) fn tune_tires(&mut self, dt: f32, speed: f32) {
        let c = self.controls;
        // The yaw-control gain drops while the handbrake is held and recovers when it is released.
        if c.handbrake >= 0.5 {
            self.yaw_control = (self.yaw_control - 20.0 * dt).max(0.5);
        } else {
            self.yaw_control = (self.yaw_control + dt).min(1.0);
        }
        let max_speed = self.powertrain.max_speed().max(1.0);
        let yaw_limit = table(&self.spec.tires.yaw_control, 0.0, 1.0, speed / max_speed);
        let grade = self.body.rotation().z_axis.y.abs();
        let speed_factor = speed / 30.0;
        let driven_front = self.spec.transmission.front_driven();
        let driven_rear = self.spec.transmission.rear_driven();
        let brake_ctx = BrakeContext {
            brake: c.brake,
            ebrake: c.handbrake,
            gas: c.gas,
            speed: self.forward_speed(),
            slip_angle: self.body_slip,
            tuning: self.tunings.brakes,
        };
        let nos_boost = self.powertrain.nos.boost;
        let h = self.tunings.handling;
        for i in 0..4 {
            let driven = if i < 2 { driven_front } else { driven_rear };
            let cmd = wheel_command(&brake_ctx, i, driven, false);
            let t = &mut self.tires[i];
            t.brake = cmd.brake;
            t.ebrake = cmd.ebrake;
            if i >= 2 {
                let mut boost = 1.0 + grade;
                if !(c.handbrake > 0.5 && self.body_slip.abs().to_degrees() < 20.0) {
                    boost += (self.body_slip.abs() * yaw_limit * speed_factor).min(0.35);
                }
                t.traction_boost *= self.yaw_control * (boost - 1.0) + 1.0;
            }
            t.traction_boost *= nos_boost;
            t.traction_circle = (1.0 + 0.2 * h, 1.0 - 0.2 * h);
        }
    }
}
