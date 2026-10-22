//! Independently posed trailer lamps use the trailer's own current controls.

use super::TrailerCar;
use crate::scenes::world::car_light_flares::FlareView;
use crate::scenes::world::car_lights::LampState;
use crate::scenes::world::drive::{CarRig, SurfaceDamage};
use blackbox_render::TexturedEffect;
use std::rc::Rc;

impl TrailerCar {
    pub(in crate::scenes::world::ai) fn surface_source(&self) -> (Rc<CarRig>, SurfaceDamage) {
        (self.rig.clone(), SurfaceDamage::new(0, self.sim.lamp_damage()))
    }

    pub(in crate::scenes::world::ai) fn flares(&self, alpha: f32, view: FlareView, out: &mut Vec<TexturedEffect>) {
        let state = LampState {
            broken: self.sim.lamp_damage(),
            headlights: true,
            brake: self.sim.lamp_brake(),
            reverse: self.sim.telemetry().gear < 0,
            ..Default::default()
        };
        self.rig.flares().draw(self.rig.lights(), self.pose(alpha).transform(), state, view, out);
    }
}
