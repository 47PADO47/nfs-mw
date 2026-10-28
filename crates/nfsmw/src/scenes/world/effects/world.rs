//! Upload world effects against the active camera, independent of UI/HUD rendering.

use blackbox_gfx::{EffectLayer, RenderBackend};

use super::super::{View, WorldScene};
use super::entry_marks;

impl WorldScene {
    pub(in crate::scenes::world) fn upload_effects(&mut self, renderer: &mut dyn RenderBackend) {
        let _trace = blackbox_trace::scope("World: effects / uploads");
        let lens = self.gameplay_lens(self.physics_aspect);
        let (position, forward, fov) = (lens.position, lens.forward, lens.fov_y_degrees);
        let icons = self.activity_icons(position);
        let icon_placements = entry_marks::placements(&icons);
        let mut interests = vec![(position, forward)];
        if let Some(guest) = self.guest_lens(self.physics_aspect) {
            interests.push((guest.position, guest.forward));
        }
        let actors: Vec<_> = self
            .players
            .iter()
            .filter(|player| !player.parked)
            .map(|player| player.drive.world_effect_interest())
            .collect();
        self.ambient_fx.maintain(
            renderer,
            &self.dir,
            self.physics.database(),
            self.residency.emitter_placements().chain(icon_placements.iter().map(|(key, placement)| (*key, placement))),
            &interests,
            &actors,
        );
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
        let environment = self.environment_effects(renderer);
        let rain = self.rain_effects(position);
        let Some(drive) = self.players.lead_mut() else {
            let mut layer = EffectLayer::default();
            layer.textured.extend(environment);
            self.ambient_fx.textured(position, forward, &mut layer.textured);
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
        drive.flames.set_enhanced(self.tuning.enhanced_exhaust && renderer.info().renderer == "blackbox");
        drive.maintain_flames(renderer, &self.dir, self.physics.database());
        drive.maintain_tire_smoke(renderer, &self.dir);
        drive.effects.set_tint(self.tuning.smoke_tint);
        let mut appearance = self.garage.applied.exhaust.clone();
        appearance.set_style(self.tuning.exhaust_style);
        drive.flames.set_appearance(&appearance);
        drive.flames.set_preview(self.garage_open && !self.garage.shop && self.garage.exhaust_preview);
        drive.flames.set_backfire(
            self.tuning.backfire_style,
            self.garage.backfire_preview.take(),
            self.garage_open && !self.garage.shop,
        );
        let rendered_pose = drive.pose().transform();
        drive.flames.set_render_pose(rendered_pose);
        self.explosions.load(renderer, &self.dir, self.physics.database());
        self.heli_wash.load(renderer, &self.dir, self.physics.database());
        self.finish_flares.maintain(renderer, &self.dir, self.physics.database());
        drive.effects.retain_sections(|s| self.residency.collision().pack(s).is_some());
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
        // A cutscene shows the trail only when it is asked for: the world stands still under it, so a trail from the
        // drive before it would hang in the air.
        let show_trails = self.view == View::Chase && (!cutscene || self.tuning.xenon_effects.nis_contrails);
        drive.vehicle_effects.geometry(position, forward, show_trails, &mut layer.streaks);
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
        self.cop_health.emit(
            renderer,
            self.tuning.cop_health_bars && !self.extra.hide_ui && !self.extra.testing.police_off,
            &mut layer.textured,
        );
        drive.vehicle_effects.textured(forward, &self.residency.shared.materials, &mut layer.textured);
        drive.flames.geometry_in_world(position, forward, self.residency.collision(), layer);
        self.explosions.textured(position, forward, &mut layer.textured);
        self.heli_wash.textured(position, forward, &mut layer.textured);
        self.finish_flares.textured(position, forward, &mut layer.textured);
        self.ambient_fx.textured(position, forward, &mut layer.textured);
        self.object_fx.textured(position, forward, &mut layer.textured);
        self.car_damage_fx.textured(position, forward, &mut layer.textured);
        self.impact_fx.textured(forward, &mut layer.textured);
        renderer.set_effects(layer);
        drive.flames.reclaim(layer);
        drive.effects.reclaim_built();
    }
}
