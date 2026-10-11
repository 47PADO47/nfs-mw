//! Upload world effects against the active camera, independent of UI/HUD rendering.

use crate::scenes::world::sign_lamps;
use blackbox_gfx::{EffectLayer, RenderBackend};
use blackbox_hash::bstring_hash;
use blackbox_render::{BlendMode, TexturedEffect};

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
        let (lamps, lamp_frame, lamp_state) = drive.lamp_source();
        let layer = drive.effects.build(position, forward);
        drive.vehicle_effects.geometry(position, forward, matches!(self.view, View::Chase), &mut layer.streaks);
        drive.vehicle_effects.glows(position, forward, &mut layer.glows);
        lamps.glows(lamp_frame, lamp_state, position, forward, &mut layer.glows);
        let mut cones = Vec::new();
        lamps.cones(lamp_frame, &mut cones);
        if let Some(traffic) = &self.traffic {
            traffic.car_cones(&mut cones);
        }
        // The headlight cookie is projected additively: a light, not a stain.
        let cookie = self.residency.shared.materials.get(&bstring_hash("HEADLIGHTS")).map(|m| m.0);
        if let (Some(texture), false) = (cookie, cones.is_empty()) {
            layer.textured.push(TexturedEffect { texture, blend: BlendMode::Additive, vertices: cones });
        }
        let signs = self.residency.placed().filter_map(|p| p.curve_lamp);
        sign_lamps::glows(signs, drive.lamp_time, position, forward, &mut layer.glows);
        if let Some(traffic) = &self.traffic {
            traffic.car_glows(position, forward, &mut layer.glows);
            traffic.cop_particles(position, forward, &mut layer.textured);
            traffic.lamp_glows(position, forward, &mut layer.glows);
        }
        drive.vehicle_effects.textured(forward, &self.residency.shared.materials, &mut layer.textured);
        drive.flames.geometry(position, forward, layer);
        renderer.set_effects(layer);
        drive.flames.reclaim(layer);
    }
}
