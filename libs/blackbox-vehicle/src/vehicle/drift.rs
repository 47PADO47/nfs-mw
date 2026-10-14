//! Ordinary racer drift state and friction/damping, chassis spec §4.4.

use super::Vehicle;
use crate::math::{graph, mph_to_ms, table};

const REAR_GRIP: [f32; 10] = [1.1, 0.95, 0.87, 0.77, 0.67, 0.6, 0.51, 0.43, 0.37, 0.34];

#[derive(Clone, Copy, Debug, Default)]
enum Phase {
    #[default]
    Out,
    Enter,
    In,
    Exit,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Drift {
    phase: Phase,
    pub value: f32,
    speed: f32,
}

impl Vehicle {
    /// Physical chassis speed sampled before drift processing, independently of wheel feedback.
    pub fn drift_speed(&self) -> f32 {
        self.drift.speed
    }

    pub(super) fn update_drift(&mut self, dt: f32, speed: f32, angular_velocity: glam::Vec3) {
        let d = &mut self.drift;
        d.speed = speed;
        let change = match d.phase {
            Phase::In | Phase::Enter => 8.0,
            Phase::Out | Phase::Exit => -2.0,
        };
        d.value = (d.value + change * dt).clamp(0.0, 1.0);
        if d.value <= 0.0 {
            d.phase = Phase::Out;
        }
        if d.value >= 1.0 {
            d.phase = Phase::In;
        }
        let slip = self.body_slip;
        let yaw = self.body.rotation().y_axis.dot(angular_velocity);
        let c = self.controls;
        let threshold = 12.0_f32.to_radians();
        let fast = speed > mph_to_ms(30.0);
        match d.phase {
            Phase::In | Phase::Exit => {
                let sustaining = yaw * slip < 0.0
                    && slip.abs() <= std::f32::consts::FRAC_PI_2
                    && fast
                    && self.steering.average_input() * slip <= 0.0
                    && slip.abs() > threshold;
                d.phase = match (
                    sustaining || (c.gas * c.steering * slip > threshold && fast),
                    c.gas * slip.abs() > threshold,
                ) {
                    (true, _) => Phase::In,
                    (_, true) => Phase::Enter,
                    _ => Phase::Exit,
                };
            }
            _ => {
                if fast && (c.handbrake > 0.5 || slip.abs() > threshold) {
                    d.phase = Phase::Enter;
                }
            }
        }
        if d.value <= 0.0 {
            return;
        }
        if yaw * slip < 0.0 && self.wheels_on_ground_prev >= 2 {
            let stabilizer =
                [(0.0, 0.0), (15.0, 0.1), (30.0, 0.45), (45.0, 0.85), (60.0, 0.95), (89.0, 1.15), (90.0, 0.0)];
            let gain = graph(&stabilizer, slip.abs().to_degrees());
            let torque = d.value * -yaw * gain * 4.0 * self.body.inertia().y;
            self.body.apply_torque(self.body.rotation().y_axis * torque);
        }
        let counter = if c.steering * slip > 0.0 { c.steering.abs() } else { 0.0 };
        let coefficient = 0.5 * slip.abs() + 4.0 * counter + 0.5 * yaw.abs();
        let grip = table(&REAR_GRIP, 0.0, 1.0, coefficient * d.value);
        for tire in &mut self.tires[2..] {
            tire.drift_friction = grip;
        }
    }
}
