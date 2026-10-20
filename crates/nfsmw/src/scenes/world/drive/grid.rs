//! The race grid: before GO the player's car is held with the brakes on, and a race can put the car at an exact place.

use super::{Drive, DriveInput, SpawnRequest};
use crate::scenes::world::road::Spawn;

/// Below this speed (m/s) the assist leaves the steering to the player.
const ASSIST_MIN_SPEED: f32 = 1.0;

impl Drive {
    /// Holds the car on the grid (`true`) or releases it (`false`). A held car keeps the player's throttle and steering,
    /// so the engine can be revved, but the brakes and the handbrake are on.
    pub fn hold_on_grid(&mut self, held: bool) {
        self.gridded = held;
    }

    /// Sets the point the car steers at by itself (physics space), or clears it. While it is set, the car's steering
    /// follows the point above 1 m/s and the player's own steering is ignored (`docs/specs/race-mode.md` section 8).
    pub fn set_steer_assist(&mut self, aim: Option<glam::Vec3>) {
        self.assist_aim = aim;
    }

    /// The steering the player asks for now, -1 to 1 (positive to the right), whatever the assist does with it.
    pub fn steer_request(&self) -> f32 {
        self.frame_input.steer
    }

    /// The input with the assist's steering, when it is on and the car is moving.
    pub(super) fn assisted(aim: Option<glam::Vec3>, sim: &super::sim::CarSim, input: DriveInput) -> DriveInput {
        let Some(aim) = aim else { return input };
        let view = sim.driver_view();
        if view.forward_speed.abs() < ASSIST_MIN_SPEED {
            return input;
        }
        let (steer, _) = blackbox_driver::simple_steering(view.position, view.forward, aim, view.max_steer, false);
        DriveInput { steer, ..input }
    }

    /// Asks for the car to be put exactly at `spawn` (the race's start), rolling at `speed` m/s (0 for a standing
    /// start). The placement happens when the world around it has loaded, as for any respawn.
    pub fn place_exactly(&mut self, spawn: Spawn, speed: f32) {
        self.pending_speed = speed;
        self.tail_trails.clear();
        self.effects.disconnect();
        self.vehicle_effects.clear();
        self.request = Some(SpawnRequest {
            near: [spawn.position.x, spawn.position.y],
            heading: Some(spawn.heading),
            exact: Some(spawn),
        });
    }
}

impl DriveInput {
    /// The input of a car held on the grid: the pedals and the steering stay, the brakes come on.
    pub(super) fn on_grid(self, held: bool) -> Self {
        if !held {
            return self;
        }
        Self { brake: 1.0, handbrake: true, nos: false, speedbreaker: false, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_car_keeps_throttle_and_steering_but_brakes() {
        let input = DriveInput { throttle: 1.0, steer: 0.5, nos: true, ..Default::default() };
        let held = input.on_grid(true);
        assert_eq!((held.throttle, held.steer, held.brake, held.handbrake, held.nos), (1.0, 0.5, 1.0, true, false));
        assert_eq!(input.on_grid(false), input);
    }
}
