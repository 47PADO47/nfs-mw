//! Upload world effects against the active camera, independent of UI/HUD rendering.

use blackbox_gfx::{EffectLayer, RenderBackend};

use super::super::{View, WorldScene};
use super::entry_marks;

impl WorldScene {
    pub(in crate::scenes::world) fn upload_effects(&mut self, renderer: &mut dyn RenderBackend) {
        let _trace = blackbox_trace::scope("World: effects / uploads");
        let lens = self.gameplay_lens(self.physics_aspect);
        let (position, forward, fov) = (lens.position, lens.forward, lens.fov_y_degrees);
        let flare_view = super::super::car_light_flares::FlareView {
            camera: position,
            forward,
            fov_degrees: fov,
            height: renderer.surface_size()[1],
        };
        self.object_fx.maintain(renderer, &self.dir, self.physics.database());
        self.car_damage_fx.maintain(renderer, &self.dir);
        self.impact_fx.maintain(renderer, &self.dir);
        let cutscene = self.cutscene_playing();
        let shut_races = self.shut_race_zones();
        let environment = self.environment_effects(renderer);
        let rain = self.rain_effects(position);
        let Some(drive) = self.players.lead_mut() else {
            let mut layer = EffectLayer::default();
            layer.textured.extend(environment);
            self.object_fx.textured(position, forward, &mut layer.textured);
            self.car_damage_fx.textured(position, forward, &mut layer.textured);
            self.impact_fx.textured(forward, &mut layer.textured);
            if let Some(traffic) = &self.traffic {
                traffic.car_flares(flare_view, &mut layer.textured);
                traffic.car_glows(self.tuning.turn_signals, position, forward, &mut layer.glows);
                traffic.cop_particles(position, forward, &mut layer.textured);
                traffic.rain_particles(forward, &self.residency.shared.materials, &mut layer.textured);
                traffic.lamp_glows(position, forward, self.residency.props(), &mut layer.glows);
            }
            rain.append(&mut layer);
            renderer.set_effects(&layer);
            return;
        };
        drive.maintain_flames(renderer, &self.dir, self.physics.database());
        self.explosions.load(renderer, &self.dir, self.physics.database());
        self.heli_wash.load(renderer, &self.dir, self.physics.database());
        self.finish_flares.maintain(renderer, &self.dir, self.physics.database());
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
        let layer = drive.effects.build(position, forward);
        layer.textured.extend(environment);
        // No entry discs during a race or a cutscene: the garages and the other races' starts are not offered then.
        if self.view == View::Chase && self.race.is_none() && !cutscene {
            entry_marks::push_discs(&self.entry.zones, car, self.clock, &shut_races, &mut layer.surfaces);
        }
        drive.vehicle_effects.geometry(position, forward, matches!(self.view, View::Chase), &mut layer.streaks);
        drive.vehicle_effects.glows(position, forward, &mut layer.glows);
        flares.draw(&lamps, lamp_frame, lamp_state, flare_view, &mut layer.textured);
        if trails_visible {
            flares.draw_trails(&lamps, &drive.tail_trails, lamp_frame, lamp_state, flare_view, &mut layer.textured);
        }
        rain.append(layer);
        if let Some(traffic) = &self.traffic {
            traffic.spike_geometry(&mut layer.streaks);
            traffic.car_glows(self.tuning.turn_signals, position, forward, &mut layer.glows);
            traffic.car_flares(flare_view, &mut layer.textured);
            traffic.cop_particles(position, forward, &mut layer.textured);
            traffic.rain_particles(forward, &self.residency.shared.materials, &mut layer.textured);
            traffic.lamp_glows(position, forward, self.residency.props(), &mut layer.glows);
        }
        self.cop_health.emit(renderer, self.tuning.cop_health_bars, &mut layer.textured);
        drive.vehicle_effects.textured(forward, &self.residency.shared.materials, &mut layer.textured);
        drive.flames.geometry(position, forward, layer);
        self.explosions.textured(position, forward, &mut layer.textured);
        self.heli_wash.textured(position, forward, &mut layer.textured);
        self.finish_flares.textured(position, forward, &mut layer.textured);
        self.object_fx.textured(position, forward, &mut layer.textured);
        self.car_damage_fx.textured(position, forward, &mut layer.textured);
        self.impact_fx.textured(forward, &mut layer.textured);
        renderer.set_effects(layer);
        drive.flames.reclaim(layer);
    }
}
