//! Stage casters are explicit; the safe-house high-resolution proxy stays out of colour views.
use super::{MenuScene, Part, car_part, fe_shadows, shadowed};
use blackbox_gfx::RenderBackend;

impl MenuScene {
    /// The room keeps casting while its car is hidden for a change of car: only the car's own casters leave, so the
    /// stage's shadows do not disappear and return with every switch.
    pub(super) fn update_menu_casters(&self, renderer: &mut dyn RenderBackend) {
        let options = fe_shadows::active().options;
        if !options.enabled || !renderer.capabilities().separate_shadow_casters {
            renderer.set_shadow_casters(None);
            return;
        }
        let mut casters = Vec::new();
        if let Some(room) = &self.room {
            casters.extend(room.shadow_casters().iter().map(|&part| shadowed(part, Part::RoomCaster, options)));
        }
        if let Some(car) = self.car.as_ref().filter(|_| !self.car_hidden()) {
            casters.extend(car.snapshot(1.0).1.iter().map(|&part| shadowed(part, car_part(&part), options)));
        }
        renderer.set_shadow_casters(Some(&casters));
    }
}
