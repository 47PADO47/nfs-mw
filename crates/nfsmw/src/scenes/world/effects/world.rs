//! Upload world effects against the active camera, independent of UI/HUD rendering.

use blackbox_render::{EffectLayer, Renderer};

use super::super::{View, WorldScene};

impl WorldScene {
    pub(in crate::scenes::world) fn upload_effects(&mut self, renderer: &mut Renderer) {
        let Some(drive) = self.drive.as_mut() else {
            renderer.set_effects(&EffectLayer::default());
            return;
        };
        drive.effects.retain_sections(|s| self.residency.collision().pack(s).is_some());
        let (position, forward) = match self.view {
            View::Chase => (drive.camera().position, drive.camera().forward()),
            View::Fly => (self.camera.position, self.camera.forward()),
        };
        let layer = drive.effects.build(position, forward);
        drive.vehicle_effects.geometry(position, forward, matches!(self.view, View::Chase), &mut layer.streaks);
        renderer.set_effects(layer);
    }
}
