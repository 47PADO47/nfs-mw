//! The force of the arcade steering that acts on the body (`arcade.rs`; `docs/specs/ride6-arcade-steering.md`): the side force at the centre
//! of gravity.

use glam::Vec3;

use super::{Bike, REAR, TurnInput, choices};

impl Bike {
    /// The share of the lean limit the cornering assist follows (-1 to 1, positive right): the target lean of the controller, which is
    /// ahead of the bike's own lean while it rolls, so the path turns as soon as the stick is moved and the lean catches up.
    fn assist_share(&self) -> f32 {
        (self.plan.target / self.plan.limit.max(choices::MIN_LEAN_LIMIT)).clamp(-1.0, 1.0)
    }

    /// What the turn law reads (`turn_law.rs`): the shares of the lean limit the stick (through the rider's hand filter) and the controller's
    /// target ask for, and the target lean.
    fn turn_input(&self) -> TurnInput {
        TurnInput {
            target_share: self.assist_share(),
            stick_share: self.plan.stick.clamp(-1.0, 1.0),
            target_lean: self.plan.target,
        }
    }

    /// The side acceleration the cornering assist gives now (m/s^2, positive right): the turn law's, or the older assist's without one.
    fn assist_acceleration(&self, speed: f32) -> f32 {
        self.spec.steering.arcade.assist_acceleration(self.assist_share(), &self.turn_input(), speed, self.lean.trim)
    }

    /// The turn law's trim for this sub-step: the shortfall of the turn the bike makes (the speed times the yaw rate about the road's normal, not the leaned bike's up) from the
    /// circle the stick asks for, integrated (`TurnLaw::trim_step`). Nothing without a turn law.
    fn update_turn_trim(&mut self, dt: f32, speed: f32) {
        let Some(law) = self.spec.steering.arcade.turn else { return };
        let up = self.wheels[REAR].normal;
        let measured = self.body.angular_velocity.dot(up) * speed.abs();
        let scale = self.surface_factor();
        self.lean.trim = law.trim_step(self.lean.trim, &self.turn_input(), speed, measured, scale, dt);
    }

    /// The cornering assist (`ArcadeSteering::cornering_assist`): a side force at the centre of gravity, along the road towards the
    /// lean the rider asks for, with no roll torque. The rear wheel must be on the road, and there is none in a burnout or a fall.
    pub(super) fn apply_cornering_assist(&mut self, dt: f32, speed: f32) {
        if self.fallen || self.burnout || !self.wheels[REAR].on_ground {
            self.lean.trim = 0.0;
            return;
        }
        self.update_turn_trim(dt, speed);
        let rotation = self.body.rotation();
        let acceleration = self.cornering_acceleration(speed);
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
        // The assist is a force the tyres do not give: on loose ground it is scaled by the grip the bike believes in (`rough.rs`).
        self.assist_acceleration(speed) * self.surface_factor()
    }
}
