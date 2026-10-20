//! Caller-selected chassis offsets and steering range; npc-chassis-contact-state spec.

use glam::Vec3;

use super::Vehicle;
use crate::input::Controls;
use crate::steering::ABSOLUTE_MAX_STEERING;
use crate::suspension::center_of_gravity;

/// Optional chassis coefficients. The caller supplies all game-specific values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimpleChassisConfig {
    /// Extra ride height in inches for zero, one, or at least two supported wheels.
    pub ride_extra: [f32; 3],
    /// Steering range in degrees at full throttle and at full brake/coast.
    pub steering_range: [f32; 2],
    /// Multipliers on local angular velocity at the first grounded frame.
    pub landing_angular_retention: Vec3,
}

impl SimpleChassisConfig {
    fn steering_range(self, controls: Controls) -> f32 {
        let blend = controls.brake.max(controls.handbrake).max(1.0 - controls.gas).clamp(0.0, 1.0);
        self.steering_range[0] + blend * (self.steering_range[1] - self.steering_range[0])
    }
}

impl Vehicle {
    /// Selects simple chassis contact state independently of the input device and tire solver.
    pub fn set_simple_chassis(&mut self, config: Option<SimpleChassisConfig>) {
        self.simple_chassis = config;
        self.simple_max_steering = config.map_or(0.0, |c| c.steering_range[0]);
    }

    /// The suspension's current maximum range, before a simple chassis applies tire steering scale.
    pub fn maximum_steering_degrees(&self) -> f32 {
        if self.simple_chassis.is_some() {
            return self.simple_max_steering;
        }
        ABSOLUTE_MAX_STEERING * self.spec.tires.steering
    }

    pub(super) fn update_chassis_cog(&mut self) {
        let ride_extra =
            self.simple_chassis.map_or(self.tunings.ride_height, |c| c.ride_extra[self.wheels_on_ground_prev.min(2)]);
        self.body.set_cog(center_of_gravity(
            &self.spec.chassis,
            self.spec.dimension.y,
            ride_extra,
            self.wheels_on_ground_prev > 0,
        ));
    }

    pub(super) fn simple_steering_scale(&mut self) -> f32 {
        let Some(config) = self.simple_chassis else { return self.spec.tires.steering };
        self.simple_max_steering = config.steering_range(self.controls);
        self.spec.tires.steering * self.simple_max_steering / ABSOLUTE_MAX_STEERING
    }
}
