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
        let lens = self.gameplay_lens(self.physics_aspect);
        let (position, forward, fov) = (lens.position, lens.forward, lens.fov_y_degrees);
        let Some(drive) = self.drive.as_mut() else {
            renderer.set_effects(&EffectLayer::default());
            return;
        };
        drive.maintain_flames(renderer, &self.dir, self.physics.database());
        self.explosions.load(renderer, &self.dir, self.physics.database());
        drive.effects.retain_sections(|s| self.residency.collision().pack(s).is_some());
        let car = drive.position();
        let (pose, bounds) = drive.spark_body();
        drive.vehicle_effects.set_body(pose, bounds);
        let (lamps, flares, lamp_frame, lamp_state) = drive.lamp_source();
        let trails_visible = self.view == View::Chase
            && crate::viewer::camera::original::renders_car(
                self.camera_mode,
                drive.camera_looking_back(self.camera_mode),
            );
        if !trails_visible {
            drive.tail_trails.clear();
        }
        let damage = drive.damage_quads();
        let layer = drive.effects.build(position, forward);
        if self.view == View::Chase {
            entry_marks::push_discs(&self.entry.zones, car, self.clock, &mut layer.surfaces);
        }
        drive.vehicle_effects.geometry(position, forward, matches!(self.view, View::Chase), &mut layer.streaks);
        drive.vehicle_effects.glows(position, forward, &mut layer.glows);
        let flare_view = super::super::car_light_flares::FlareView {
            camera: position,
            forward,
            fov_degrees: fov,
            height: renderer.surface_size()[1],
        };
        flares.draw(&lamps, lamp_frame, lamp_state, flare_view, &mut layer.textured);
        if trails_visible {
            flares.draw_trails(&lamps, &drive.tail_trails, lamp_frame, lamp_state, flare_view, &mut layer.textured);
        }
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
            traffic.car_glows(self.tuning.turn_signals, position, forward, &mut layer.glows);
            traffic.car_beams(&mut layer.streaks);
            traffic.cop_particles(position, forward, &mut layer.textured);
            traffic.lamp_glows(position, forward, self.residency.props(), &mut layer.glows);
        }
        self.cop_health.emit(renderer, self.tuning.cop_health_bars, &mut layer.textured);
        drive.vehicle_effects.textured(forward, &self.residency.shared.materials, &mut layer.textured);
        drive.flames.geometry(position, forward, layer);
        self.explosions.textured(position, forward, &mut layer.textured);
        renderer.set_effects(layer);
        drive.flames.reclaim(layer);
    }
}
