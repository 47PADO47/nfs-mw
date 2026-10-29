//! Reading a bike: its pose, its speed, its lean and pitch, its wheels and its tyres (`docs/specs/vehicle-two-wheel.md`).

use glam::{Quat, Vec3};

use super::{Bike, FRONT, REAR, WheelState};

impl Bike {
    pub fn position(&self) -> Vec3 {
        self.body.position
    }

    pub fn orientation(&self) -> Quat {
        self.body.orientation
    }

    pub fn velocity(&self) -> Vec3 {
        self.body.linear_velocity
    }

    /// Speed along the bike (m/s).
    pub fn forward_speed(&self) -> f32 {
        self.body.linear_velocity.dot(self.body.rotation() * Vec3::Z)
    }

    /// The lean from upright (rad), positive when the bike leans right.
    pub fn lean(&self) -> f32 {
        lean_of(self.body.rotation())
    }

    /// The pitch (rad), positive when the nose is up.
    pub fn pitch(&self) -> f32 {
        pitch_of(self.body.rotation())
    }

    /// The lean limit the rider's controller used in the last step, and the target lean it aimed at (rad, positive right).
    pub fn lean_limit(&self) -> f32 {
        self.plan.limit
    }

    pub fn target_lean(&self) -> f32 {
        self.plan.target
    }

    /// The handlebar angle (rad), positive right.
    pub fn steer_angle(&self) -> f32 {
        self.steer.output()
    }

    pub fn rpm(&self) -> f32 {
        self.powertrain.rpm()
    }

    pub fn gear(&self) -> usize {
        self.powertrain.gear()
    }

    pub fn wheel(&self, index: usize) -> &WheelState {
        &self.wheels[index.min(REAR)]
    }

    pub fn wheels_on_ground(&self) -> usize {
        self.wheels.iter().filter(|w| w.on_ground).count()
    }

    /// The temperature (deg C, mean of the strips), the efficiency (0 to 1) and the worn tread of wheel `index`.
    pub fn tyre_temperature(&self, index: usize) -> f32 {
        self.thermal[index.min(REAR)].temperature()
    }

    pub fn tyre_efficiency(&self, index: usize) -> f32 {
        self.thermal[index.min(REAR)].efficiency()
    }

    pub fn tyre_wear(&self, index: usize) -> f32 {
        let wheel = match index {
            FRONT => &self.spec.front,
            _ => &self.spec.rear,
        };
        match &wheel.thermal {
            Some(thermal) => self.thermal[index.min(REAR)].wear(thermal),
            None => 0.0,
        }
    }

    /// Whether the rider has let go.
    pub fn fallen(&self) -> bool {
        self.fallen
    }

    /// The pose of a wheel assembly now, for drawing: the hub in the world and its orientation.
    pub fn wheel_pose(&self, index: usize) -> (Vec3, Quat) {
        let pose = self.pose(index, self.wheels[index.min(REAR)].compression);
        let (_, turn) = match index {
            FRONT => self.geometry.front_pose(self.steer.output(), 0.0),
            _ => self.geometry.rear_pose(0.0),
        };
        (pose.hub, self.body.orientation * turn)
    }
}

/// The lean (rad) of a body with orientation `rotation`: positive when its top leans to its right. The world's up, seen
/// from a body that leans right, points to the body's left.
pub(super) fn lean_of(rotation: glam::Mat3) -> f32 {
    let up = rotation.transpose() * Vec3::Y;
    (-up.x).atan2(up.y)
}

/// The pitch (rad) of a body: positive when its forward axis points above the horizon.
pub(super) fn pitch_of(rotation: glam::Mat3) -> f32 {
    (rotation * Vec3::Z).y.clamp(-1.0, 1.0).asin()
}
