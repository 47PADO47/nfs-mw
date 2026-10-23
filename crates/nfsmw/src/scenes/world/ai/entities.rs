//! The entity ids of the computer-driven cars (`docs/specs/multiplayer/determinism.md`, D5).
//!
//! Each world numbers its own cars from 1 as they join it, so two worlds in one process (a server and a client, tests)
//! number theirs independently, and the same spawns give the same ids in every run. The render keys (`keys.rs`) are a
//! separate, presentation-only identity: model uploads advance them, so no gameplay rule reads them.

use super::TrafficWorld;
use super::car::AiCar;

impl TrafficWorld {
    /// Puts a placed car into the world: it takes the next entity id and goes to the end of `cars`. Every car joins
    /// through here and every removal keeps the order, so `cars` is in id order and the step, the pair contacts and
    /// the impulses that follow from them run in id order.
    pub(in crate::scenes::world::ai) fn admit(&mut self, mut car: AiCar) {
        self.next_entity += 1;
        car.set_entity(self.next_entity);
        self.cars.push(car);
    }

    /// The entity ids of the cars, in the order they step.
    #[cfg(test)]
    pub fn entity_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.cars.iter().map(AiCar::key)
    }
}
