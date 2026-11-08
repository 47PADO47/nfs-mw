//! The force of the arcade steering that acts on the body (`arcade.rs`; `docs/specs/ride6-arcade-steering.md`): the side force at the centre
//! of gravity.

use glam::Vec3;

use super::{Bike, REAR, choices};

impl Bike {
    /// The share of the lean limit the cornering assist follows (-1 to 1, positive right): the target lean of the controller, which is
    /// ahead of the bike's own lean while it rolls, so the path turns as soon as the stick is moved and the lean catches up.
    fn assist_share(&self) -> f32 {
        (self.plan.target / self.plan.limit.max(choices::MIN_LEAN_LIMIT)).clamp(-1.0, 1.0)
    }

    /// The cornering assist (`ArcadeSteering::cornering_assist`): a side force at the centre of gravity, along the road towards the
    /// lean the rider asks for, with no roll torque. The rear wheel must be on the road, and there is none in a burnout or a fall.
    pub(super) fn apply_cornering_assist(&mut self, speed: f32) {
        if self.fallen || self.burnout || !self.wheels[REAR].on_ground {
            return;
        }
        let rotation = self.body.rotation();
        let acceleration = self.spec.steering.arcade.cornering_acceleration(self.assist_share(), speed);
        if acceleration == 0.0 {
            return;
        }
        let normal = self.wheels[REAR].normal;
        let right = rotation * Vec3::X;
        let Some(side) = (right - normal * right.dot(normal)).try_normalize() else { return };
        self.body.apply_force(side * (acceleration * self.body.mass()));
    }

    /// The side acceleration the cornering assist adds now (m/s^2, positive right), for the stability help's wanted yaw rate.
    pub(super) fn cornering_acceleration(&self, speed: f32) -> f32 {
        self.spec.steering.arcade.cornering_acceleration(self.assist_share(), speed)
    }
}
