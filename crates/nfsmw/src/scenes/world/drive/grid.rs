//! The race grid: before GO the player's car is held with the brakes on, and a race can put the car at an exact place.

use super::{Drive, DriveInput, SpawnRequest};
use crate::scenes::world::road::Spawn;

impl Drive {
    /// Holds the car on the grid (`true`) or releases it (`false`). A held car keeps the player's throttle and steering,
    /// so the engine can be revved, but the brakes and the handbrake are on.
    pub fn hold_on_grid(&mut self, held: bool) {
        self.gridded = held;
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
