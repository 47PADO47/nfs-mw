//! Contact-driven escape yaw; settings and state order: vehicle-steering-assists-aero §6.3.

use glam::Vec3;

use super::Vehicle;
use crate::drivetrain::GEAR_REVERSE;
use crate::rigid_body::ContactParams;

/// Optional escape assistance, disabled by neutral defaults.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WallSteeringConfig {
    pub maximum_normal_y: f32,
    pub maximum_closing_speed_squared: f32,
    pub maximum_nose_speed_squared: f32,
    pub end_fraction: f32,
    pub side_epsilon: f32,
    pub yaw_gain: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct WallSteering {
    nose: f32,
    side: f32,
}

impl WallSteering {
    fn yaw(self, steer: f32, gas: f32, grounded: usize, reverse: bool, gain: f32) -> f32 {
        if grounded <= 2 {
            return 0.0;
        }
        let command = steer * gas;
        if self.nose != 0.0 && gas > 0.0 && self.nose * command < 0.0 {
            return 0.0;
        }
        if reverse && self.side * command > 0.0 {
            return -command * gain * self.side.abs();
        }
        if self.nose != 0.0 && gas > 0.0 {
            return command * gain * self.nose.abs();
        }
        0.0
    }
}

impl Vehicle {
    /// Queue a resolved static-world contact using its velocities before the response.
    /// Call between steps; prop/object and ground contacts must use their own callbacks.
    pub fn notify_wall_contact(&mut self, point: Vec3, normal: Vec3, closing_velocity: Vec3, body_velocity: Vec3) {
        let c = self.wall_steering_config;
        if c.yaw_gain == 0.0
            || normal.y.abs() >= c.maximum_normal_y
            || closing_velocity.length_squared() >= c.maximum_closing_speed_squared
        {
            return;
        }
        let rot = self.body.rotation();
        let lateral = normal.dot(rot.x_axis);
        let reach = (point - self.body.position).dot(rot.z_axis);
        let end = self.spec.dimension.z * c.end_fraction;
        if self.wall_steering.nose == 0.0
            && body_velocity.length_squared() < c.maximum_nose_speed_squared
            && normal.dot(rot.z_axis) <= 0.0
            && reach > end
        {
            let sign = match lateral > 0.0 {
                true => 1.0,
                false => -1.0,
            };
            self.wall_steering.nose = sign - lateral;
        }
        if self.wall_steering.side == 0.0
            && body_velocity.dot(rot.z_axis) < 0.0
            && lateral.abs() > c.side_epsilon
            && reach.abs() > end
        {
            self.wall_steering.side = lateral;
        }
    }

    /// The vehicle tweaks to a static wall reaction (`vehicle-rigid-body.md` §4.3, `RBVehicle`): the
    /// impact is taken at the body's height of the origin rather than the centre of gravity, and it is
    /// flattened to the origin. Caller-admitted head-on damping also removes restitution and adds yaw inertia.
    /// `normal` faces the body. Returns the lever-arm centre and the adjusted parameters.
    pub fn wall_reaction(&self, normal: Vec3, params: ContactParams, head_on_damping: bool) -> (Vec3, ContactParams) {
        let cog = self.body.world_cog();
        let centre = Vec3::new(cog.x, self.body.position.y, cog.z);
        if !head_on_damping {
            return (centre, params);
        }
        let forward = self.body.rotation().z_axis;
        let nose = -normal.dot(forward);
        let closing = (-normal.dot(self.body.linear_velocity) / 5.0).clamp(0.0, 1.0);
        let damp = ((nose + 1.0) / 2.0).clamp(0.0, 1.0);
        let mut adjusted = params;
        adjusted.restitution = 0.0;
        adjusted.inertia_scale.y *= 1.0 + closing * damp * 8.0;
        (centre, adjusted)
    }

    pub(super) fn apply_wall_steering(&mut self) {
        let pending = std::mem::take(&mut self.wall_steering);
        let yaw = pending.yaw(
            self.controls.steering,
            self.controls.gas,
            self.wheels_on_ground_prev,
            self.powertrain.gear() == GEAR_REVERSE,
            self.wall_steering_config.yaw_gain,
        );
        self.body.angular_velocity += self.body.rotation().y_axis * yaw;
    }
}

#[cfg(test)]
mod tests;
