//! Upload world effects against the active camera, independent of UI/HUD rendering.

use blackbox_gfx::{EffectLayer, RenderBackend};

use super::super::{View, WorldScene};

impl WorldScene {
    pub(in crate::scenes::world) fn upload_effects(&mut self, renderer: &mut dyn RenderBackend) {
        let Some(drive) = self.drive.as_mut() else {
            renderer.set_effects(&EffectLayer::default());
            return;
        };
        drive.maintain_flames(renderer, &self.dir, self.physics.database());
        drive.effects.retain_sections(|s| self.residency.collision().pack(s).is_some());
        let (position, forward) = match self.view {
            View::Chase => (drive.camera().position, drive.camera().forward()),
            View::Fly => (self.camera.position, self.camera.forward()),
        };
        let (pose, bounds) = drive.spark_body();
        drive.vehicle_effects.set_body(pose, bounds);
        let layer = drive.effects.build(position, forward);
        drive.vehicle_effects.geometry(position, forward, matches!(self.view, View::Chase), &mut layer.streaks);
        drive.vehicle_effects.glows(position, forward, &mut layer.glows);
        drive.vehicle_effects.textured(forward, &self.residency.shared.materials, &mut layer.textured);
        drive.flames.geometry(position, forward, layer);
        renderer.set_effects(layer);
        drive.flames.reclaim(layer);
    }
}
