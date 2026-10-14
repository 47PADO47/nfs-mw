//! Upload world effects against the active camera, independent of UI/HUD rendering.

use crate::scenes::world::sign_lamps;
use blackbox_gfx::{EffectLayer, RenderBackend};

use super::super::damage_marks::Mark;
use super::super::{View, WorldScene};
use super::entry_marks;

/// Rain streaks: a pale blue, dim enough to read as water rather than light.
const RAIN_COLOUR: [u8; 4] = [180, 200, 255, 90];
/// Damage marks: a dark scrape and a pale crack line.
const SCUFF_COLOUR: [u8; 4] = [40, 40, 40, 170];
const CRACK_COLOUR: [u8; 4] = [235, 240, 245, 210];

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
        let car = drive.position();
        let (pose, bounds) = drive.spark_body();
        drive.vehicle_effects.set_body(pose, bounds);
        let (lamps, lamp_frame, lamp_state) = drive.lamp_source();
        let damage = drive.damage_quads();
        let layer = drive.effects.build(position, forward);
        if self.view == View::Chase {
            entry_marks::push_discs(&self.entry.zones, car, self.clock, &mut layer.surfaces);
        }
        drive.vehicle_effects.geometry(position, forward, matches!(self.view, View::Chase), &mut layer.streaks);
        drive.vehicle_effects.glows(position, forward, &mut layer.glows);
        lamps.glows(lamp_frame, lamp_state, position, forward, &mut layer.glows);
        lamps.beams(lamp_frame, lamp_state.headlights, &mut layer.streaks);
        for mark in damage {
            let colour = match mark.mark {
                Mark::Scuff => SCUFF_COLOUR,
                Mark::Crack => CRACK_COLOUR,
            };
            blackbox_gfx::EffectLayer::quad(&mut layer.streaks, mark.corners, colour);
        }
        let mut drops = Vec::new();
        self.rain.streaks(position, &mut drops);
        for quad in drops {
            blackbox_gfx::EffectLayer::quad(&mut layer.streaks, quad, RAIN_COLOUR);
        }
        let signs = self.residency.placed().filter_map(|p| p.curve_lamp);
        sign_lamps::glows(signs, drive.lamp_time, position, forward, &mut layer.glows);
        if let Some(traffic) = &self.traffic {
            traffic.car_glows(position, forward, &mut layer.glows);
            traffic.car_beams(&mut layer.streaks);
            traffic.cop_particles(position, forward, &mut layer.textured);
            traffic.lamp_glows(position, forward, &mut layer.glows);
        }
        drive.vehicle_effects.textured(forward, &self.residency.shared.materials, &mut layer.textured);
        drive.flames.geometry(position, forward, layer);
        renderer.set_effects(layer);
        drive.flames.reclaim(layer);
    }
}
