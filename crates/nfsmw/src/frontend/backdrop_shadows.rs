//! Stage casters are explicit; the safe-house high-resolution proxy stays out of colour views.
use super::{MenuScene, Part, car_part, fe_shadows, shadowed};
use blackbox_gfx::RenderBackend;

impl MenuScene {
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
        if let Some(car) = &self.car {
            casters.extend(car.snapshot(1.0).1.iter().map(|&part| shadowed(part, car_part(&part), options)));
        }
        renderer.set_shadow_casters(Some(&casters));
    }
}
