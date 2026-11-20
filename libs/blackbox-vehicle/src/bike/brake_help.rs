//! The arcade help of braking on the body (`arcade_brake.rs`, `docs/specs/ride6-bike-braking.md`): the force at the centre of gravity that makes up
//! the deceleration the levers aim for, and the torque that holds the nose up against the tyres' braking force.

use glam::Vec3;

use crate::bike::curve::ramp;
use crate::bike::{Bike, BikeInput, FRONT, REAR, brakes, choices, lean_of};

impl Bike {
    /// The lever mix the help follows `(front, rear)`: the levers as the joined brakes make them, less the relief of a bike in a lean
    /// (`choices::BRAKE_LEAN_RELIEF`, as the brake torque has it).
    fn brake_levers(&self, speed: f32, input: &BikeInput) -> (f32, f32) {
        let moving = speed.abs() > choices::STANDING_SPEED;
        let (front, rear) = brakes::joined(&self.spec.brakes, input.front_brake, input.rear_brake, moving);
        let share = self.lean_share();
        let relief = 1.0 - choices::BRAKE_LEAN_RELIEF * share * share;
        (front * relief, rear * relief)
    }

    /// The bike's lean as a share of its limit, 0 upright to 1.
    fn lean_share(&self) -> f32 {
        (lean_of(self.body.rotation()).abs() / self.plan.limit.max(choices::MIN_LEAN_LIMIT)).min(1.0)
    }

    /// The braking force of the tyres now (N, positive against the heading): the sum of what the two tyres put along the road in the last sub-step.
    fn tyre_braking_force(&self) -> f32 {
        -(self.wheels[FRONT].traction + self.wheels[REAR].traction)
    }

    /// The nose-down torque (N m, about the right axis, positive lowers the nose) that the tyres' longitudinal forces put on the body about its
    /// centre of gravity.
    fn braking_pitch_moment(&self, forward: Vec3, right: Vec3) -> f32 {
        let cog = self.body.world_cog();
        let moment: Vec3 = self
            .wheels
            .iter()
            .filter(|wheel| wheel.on_ground)
            .map(|wheel| (wheel.contact - cog).cross(forward * wheel.traction))
            .sum();
        moment.dot(right)
    }

    /// The help of braking (`ArcadeSteering::brake`): the force that makes up the deceleration the levers aim for, and the pitch hold. Nothing without the
    /// rider on the bike, with a wheel on the road, in a burnout or while the bike is pushed back.
    pub(super) fn apply_brake_help(&mut self, speed: f32, input: &BikeInput) {
        let help = self.spec.steering.arcade.brake;
        if help.gain <= 0.0 && help.pitch_hold <= 0.0 {
            return;
        }
        if self.fallen || self.burnout || self.departure.off || self.reverse.pushing() {
            return;
        }
        let Some(normal) = self.ground_normal() else { return };
        let (front, rear) = self.brake_levers(speed, input);
        if front.max(rear) <= 0.0 {
            return;
        }
        let velocity = self.body.linear_velocity;
        let along_road = velocity - normal * velocity.dot(normal);
        let Some(heading) = along_road.try_normalize() else { return };
        let fade = help.fade(speed, input.throttle);
        // The force: what the levers aim for less what the tyres give, against the way the bike goes.
        let class = self.spec.terrain_class().index();
        let target = help.target(class, self.spec.is_scooter(), front, rear, self.surface_factor());
        let mass = self.body.mass();
        let tyres = self.tyre_braking_force() / mass;
        let extra = help.extra(target, tyres) * fade;
        if extra > 0.0 {
            self.body.apply_force(-heading * (extra * mass));
        }
        // The hold: a share of the tyres' nose-down moment, taken off while the rider's weight is not forward.
        // A leaned bike is not braked hard, and its pitch is the lean controller's: the hold goes with the lean.
        let share = help.hold_share(self.trick.weight) * ramp(speed.abs(), help.speed) * (1.0 - self.lean_share());
        if share <= 0.0 {
            return;
        }
        // The axis of the pitch is the road's, across the heading: a leaned bike's own right axis points into the road and a torque about it would yaw.
        let right = normal.cross(heading);
        let moment = self.braking_pitch_moment(heading, right);
        // Only a nose-down moment is held (the tyres driving the bike lower the tail, not the nose).
        if moment <= 0.0 {
            return;
        }
        self.body.apply_torque(-right * (share * moment));
    }
}
