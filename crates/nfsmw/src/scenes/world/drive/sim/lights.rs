//! The vehicle FX task admits sustained NPC braking without changing physical controls.
use super::CarSim;

impl CarSim {
    pub(super) fn advance_lamps(&mut self, brake: f32, dt: f32) {
        if self.collision_role.player {
            return;
        }
        self.brake_lamp_age = match brake > 0.0 {
            true => self.brake_lamp_age + dt,
            false => 0.0,
        };
    }

    pub fn lamp_brake(&self) -> f32 {
        let on = match self.collision_role.player {
            true => self.vehicle.controls().brake > 0.0,
            false => self.brake_lamp_age > 0.5,
        };
        f32::from(u8::from(on))
    }

    pub fn lamp_damage(&self) -> u32 {
        self.events.as_ref().map_or(0, |events| events.broken_lights())
    }
}
