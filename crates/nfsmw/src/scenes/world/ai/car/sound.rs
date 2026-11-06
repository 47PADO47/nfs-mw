//! The sound side of a computer-driven car: the cooldown that keeps a traffic car's horn and near-miss sounds from
//! repeating, and what the traffic or race-opponent sound producers read of it.

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

impl AiCar {
    /// What a race opponent's engine voice reads of it; `None` for any other car (`authored-traffic-audio.md`
    /// "Actor roles").
    pub fn racer_emitter(&self) -> Option<crate::audio::traffic::RacerEmitter> {
        use crate::scenes::world::space;
        if !self.is_racer() {
            return None;
        }
        let body = self.sim.body();
        let t = self.sim.telemetry();
        let car = self.name.split(':').next().unwrap_or(&self.name).to_ascii_lowercase();
        Some(crate::audio::traffic::RacerEmitter {
            id: self.render_id,
            car,
            position: space::to_render(body.position.to_array()),
            forward: space::to_render((body.orientation * glam::Vec3::Z).to_array()),
            velocity: space::to_render(body.linear_velocity.to_array()),
            rpm_pct: crate::scenes::world::drive::sound::physics_rpm_fraction(t.rpm, t.idle, t.red_line),
            throttle: self.controls.gas.clamp(0.0, 1.0),
            wheels_on_ground: u8::try_from(t.wheels_on_ground.min(4)).unwrap_or(4),
        })
    }
}
