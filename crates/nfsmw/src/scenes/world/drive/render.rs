//! Main-view and independent-pass car submission.
use super::*;

impl Drive {
    /// Append the car's instances.
    pub fn instances(&self, out: &mut Vec<blackbox_render::Instance>) {
        self.instances_with_detail(None, out);
    }
    pub(in crate::scenes::world) fn instances_with_detail(
        &self,
        detail: Option<&DetailView>,
        out: &mut Vec<blackbox_render::Instance>,
    ) {
        if let Some(sim) = &self.sim {
            let dark = self.lights_dark();
            let revealed = sim.events.as_ref().map(|e| e.revealed_slots()).unwrap_or_default();
            self.rig.instances_detail(
                &self.render_pose(),
                sim.events.as_ref().map(|e| &e.hidden_slots),
                &revealed,
                DrawOptions {
                    damage: sim.events.as_ref().map(|e| e.source_levels()),
                    car: crate::scenes::world::keys::PLAYER_CAR,
                    lamps: [
                        self.headlights && !dark,
                        LampState { brake: self.brake_lamp, broken: sim.lamp_damage(), dark, ..Default::default() }
                            .brake_surfaces_on(),
                    ],
                    reverse: sim.telemetry().gear < 0 && !dark,
                    detail,
                    glass: self.cracked.mask(),
                    broken_lights: sim.lamp_damage(),
                    steering: sim.steering_input(),
                },
                out,
            );
        }
    }
}
