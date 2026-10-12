//! The sound side of a traffic car: the cooldown that keeps its horn and near-miss sounds from repeating.

use super::AiCar;
use crate::scenes::world::drive::STEP;

impl AiCar {
    /// Whether the car may play a sound now (its cooldown has run out).
    pub fn ready(&self) -> bool {
        self.quiet <= 0.0
    }

    /// Starts the cooldown after the car played a sound of `cooldown` seconds.
    pub fn sounded(&mut self, cooldown: f32) {
        self.quiet = cooldown;
    }

    /// Counts the cooldown down by one physics step.
    pub fn tick_quiet(&mut self) {
        self.quiet = (self.quiet - STEP).max(0.0);
    }
}
