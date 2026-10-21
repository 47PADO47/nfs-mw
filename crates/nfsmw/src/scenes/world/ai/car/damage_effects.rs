use super::*;
use crate::scenes::world::damage_effects::Source;

impl AiCar {
    pub(in crate::scenes::world::ai) fn damage_effect_source(&self) -> Option<Source<'_>> {
        let health = self.damage_health()?;
        let owner = Rc::downgrade(&self.sim.events.as_ref()?.owner);
        Some(Source {
            id: self.render_id,
            owner,
            fx: &self.rig.model().driving_lods.damage_fx,
            health,
            frame: blackbox_particles::Frame {
                to_world: self.current.transform(),
                inherit_velocity: crate::scenes::world::space::to_render(self.sim.velocity().to_array()),
                intensity: 1.0,
            },
        })
    }
}
