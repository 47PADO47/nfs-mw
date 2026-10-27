//! The stand-in of a racer off the loaded world (`docs/specs/ai-simulation-lod.md` section 3): the world's collision
//! streams around every car, but a racer can still outrun it. While its surroundings are not loaded the car is not
//! simulated; it is moved along its navigator's path towards its drive target at the speed it would drive at, so it
//! keeps to its route and crosses the gates on the way. Once its surroundings are loaded again it is put on the ground
//! where it is and drives on. A switch either way waits [`SWITCH_AFTER`] seconds, so a car on the edge does not flip.

use blackbox_roads::RandomSource;
use glam::Vec3;

use super::{AiCar, Ctx, Role};
use crate::scenes::world::ai::racer::RACER_THINK_STEPS;
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;

/// Seconds a racer stays off (or back on) the loaded world before it switches to (or from) its stand-in.
pub const SWITCH_AFTER: f32 = 0.25;

/// Whether a car is on its stand-in, and what it carries between steps.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StandIn {
    pub active: bool,
    /// Seconds the car has wanted the other state.
    pending: f32,
    /// The stand-in's speed, m/s.
    pub speed: f32,
}

impl StandIn {
    /// Advances the switch by `dt` with the car's surroundings loaded (`on_world`) or not. True when the car should
    /// switch now: on to the stand-in when it is off the world, off it when it is back.
    pub fn wants_switch(&mut self, on_world: bool, dt: f32) -> bool {
        if on_world != self.active {
            self.pending = 0.0;
            return false;
        }
        self.pending += dt;
        if self.pending < SWITCH_AFTER {
            return false;
        }
        self.pending = 0.0;
        true
    }
}

impl AiCar {
    /// Whether the racer moves by its stand-in this step. A racer whose surroundings went or came back for long enough
    /// switches: on, keeping its speed; off, put on the ground where it is, unless there is no ground there yet.
    pub(in crate::scenes::world::ai) fn update_stand_in(&mut self, on_world: bool, ground: &WorldGround<'_>) -> bool {
        let eligible = matches!(&self.role, Role::Racer(racer) if racer.human.is_none() && !racer.held);
        if !eligible || self.is_wrecked() {
            return self.stand_in.active;
        }
        if !self.stand_in.wants_switch(on_world, self.step_delta) {
            return self.stand_in.active;
        }
        if !self.stand_in.active {
            self.stand_in = StandIn { active: true, pending: 0.0, speed: self.speed().abs() };
            log::debug!("race: {} is off the loaded world, on its stand-in", self.name);
            return true;
        }
        let nose = self.current.rotation * Vec3::X;
        let spawn = Spawn { position: self.current.position, heading: nose.y.atan2(nose.x) };
        if !self.sim.place_traffic_moving(ground, spawn, self.stand_in.speed) {
            return true;
        }
        self.stand_in = StandIn::default();
        let pose = self.sim.pose();
        (self.previous, self.current) = (pose, pose);
        log::debug!("race: {} is back on the loaded world", self.name);
        false
    }

    /// One step of the stand-in: the think on its turn, then the car moves towards its drive target.
    pub(in crate::scenes::world::ai) fn stand_in_step<R: RandomSource>(
        &mut self,
        ctx: &mut Ctx<'_, R>,
        collision: &blackbox_collision::CollisionWorld,
    ) {
        self.think_elapsed += self.step_delta;
        if (self.steps + self.stagger).is_multiple_of(RACER_THINK_STEPS) {
            self.think(ctx, collision);
        }
        let Role::Racer(racer) = &self.role else { return };
        let speed = racer.stand_in_speed(self.stand_in.speed, self.speed, self.step_delta);
        let road = self.physics_position() - Vec3::Y * self.sim.ride_lift();
        let (moved, forward) = stand_in_move(road, self.target, speed * self.step_delta);
        self.sim.place_kinematic(moved, forward, speed);
        self.stand_in.speed = speed;
        self.previous = self.current;
        self.current = self.sim.pose();
        self.steps += 1;
        self.elapsed += self.step_delta;
    }
}

/// The road point `step` metres from `road` towards `target` (no further than the target), and the direction of travel.
fn stand_in_move(road: Vec3, target: Vec3, step: f32) -> (Vec3, Vec3) {
    let to = target - road;
    let length = to.length();
    let forward = match to.try_normalize() {
        Some(forward) => forward,
        None => return (road, Vec3::Z),
    };
    (road + forward * step.min(length), forward)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_waits_a_quarter_second_each_way() {
        let mut stand_in = StandIn::default();
        assert!(!stand_in.wants_switch(true, 0.1), "on the world and simulated: nothing to do");
        assert!(!stand_in.wants_switch(false, 0.1));
        assert!(!stand_in.wants_switch(false, 0.1));
        assert!(!stand_in.wants_switch(true, 0.1), "back in time: the wait starts again");
        assert!(!stand_in.wants_switch(false, 0.2));
        assert!(stand_in.wants_switch(false, 0.1), "off the world for 0.3 s");
        stand_in.active = true;
        assert!(!stand_in.wants_switch(false, 1.0), "on the stand-in and off the world: nothing to do");
        assert!(!stand_in.wants_switch(true, 0.2));
        assert!(stand_in.wants_switch(true, 0.1));
    }

    #[test]
    fn the_stand_in_moves_towards_its_target_without_passing_it() {
        let (moved, forward) = stand_in_move(Vec3::ZERO, Vec3::new(0.0, 3.0, 40.0), 10.0);
        assert!((moved.length() - 10.0).abs() < 1e-4 && forward.z > 0.99 && moved.y > 0.0, "it climbs with the road");
        let (moved, _) = stand_in_move(Vec3::ZERO, Vec3::new(0.0, 0.0, 4.0), 10.0);
        assert_eq!(moved, Vec3::new(0.0, 0.0, 4.0));
        assert_eq!(stand_in_move(Vec3::ONE, Vec3::ONE, 5.0), (Vec3::ONE, Vec3::Z));
    }
}
