//! The sound side of a traffic car: the cooldown that keeps its horn and near-miss sounds from repeating.

use super::AiCar;

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
        self.quiet = (self.quiet - self.step_delta).max(0.0);
    }
}

impl AiCar {
    pub fn audio_emitter(&self) -> crate::audio::traffic::TrafficEmitter {
        use crate::scenes::world::space;
        let body = self.sim.body();
        crate::audio::traffic::TrafficEmitter {
            id: self.render_id,
            profile: self.audio_profile,
            position: space::to_render(body.position.to_array()),
            forward: space::to_render((body.orientation * glam::Vec3::Z).to_array()),
            velocity: space::to_render(body.linear_velocity.to_array()),
        }
    }
}
