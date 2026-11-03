//! Upload world effects against the active camera, independent of UI/HUD rendering.

use blackbox_gfx::{EffectLayer, RenderBackend};

use super::super::{View, WorldScene};
use super::entry_marks;

fn fog_shelters(zones: &blackbox_roads::TrackZones, camera: glam::Vec3, range: f32) -> Vec<blackbox_gfx::FogShelter> {
    let point = camera.truncate();
    let mut nearby: Vec<_> = zones
        .zones
        .iter()
        .filter(|zone| [3, 4, 5, 7].contains(&zone.kind) && (3..=16).contains(&zone.polygon.len()))
        .filter_map(|zone| {
            let distance = point.distance(point.clamp(zone.bbox_min, zone.bbox_max));
            (distance < range).then_some((distance, zone))
        })
        .collect();
    nearby.sort_by(|a, b| a.0.total_cmp(&b.0));
    nearby
        .into_iter()
        .take(24)
        .map(|(_, zone)| blackbox_gfx::FogShelter {
            bounds: [zone.bbox_min.x, zone.bbox_min.y, zone.bbox_max.x, zone.bbox_max.y],
            ceiling: zone.elevation,
            polygon: zone.polygon.iter().map(|point| point.to_array()).collect(),
        })
        .collect()
}

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
        let mut street_glows = Vec::with_capacity(self.street_glows.len() * 18);
        let mut street_lenses = Vec::with_capacity(self.street_glows.len() * 6);
        let glow_strength = renderer.render_experiments().local_street_glow;
        let fog_strength = renderer.render_experiments().local_light_fog;
        let weather = self.rain.visible_intensity().max(self.atmosphere.intensity() * 0.5);
        let shelters = match fog_strength > 0.0 && weather > 0.0 {
            true => fog_shelters(
                &self.camera_environment.regions,
                position,
                renderer.render_experiments().local_light_fog_distance,
            ),
            false => Vec::new(),
        };
        for &lamp in &self.street_glows {
            super::street_lamps::append(&mut street_glows, &mut street_lenses, lamp, flare_view, glow_strength);
        }
        let rain = self.rain_effects(position);
        let helmet_camera = self.bike_helmet_active();
        let Some(drive) = self.players.lead_mut() else {
            let mut layer = EffectLayer { light_fog: [fog_strength, weather], ..EffectLayer::default() };
            layer.fog_shelters = shelters;
            layer.glows.extend_from_slice(&street_glows);
            layer.light_lenses.extend_from_slice(&street_lenses);
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
        drive.maintain_rider(renderer, &self.dir);
        let arrested = self.traffic.as_ref().is_some_and(|traffic| traffic.arrest_active());
        drive.set_rider_hidden(arrested);
        drive.set_first_person(helmet_camera);
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
        let trails_visible = drive.tail_trails.preview_active()
            || self.view == View::Chase
                && crate::viewer::camera::original::renders_car(
                    self.camera_mode,
                    drive.camera_looking_back(self.camera_mode),
                );
        if !trails_visible {
            drive.tail_trails.clear();
        }
        let layer = drive.effects.build(position, forward);
        layer.light_fog = [fog_strength, weather];
        layer.fog_shelters = shelters;
        layer.glows.extend_from_slice(&street_glows);
        layer.light_lenses.extend_from_slice(&street_lenses);
        layer.textured.extend(environment);
        // A cutscene shows the trail only when it is asked for: the world stands still under it, so a trail from the
        // drive before it would hang in the air.
        let show_trails = self.view == View::Chase && (!cutscene || self.tuning.xenon_effects.nis_contrails);
        drive.vehicle_effects.geometry(position, forward, show_trails, &mut layer.streaks);
        drive.vehicle_effects.glows(position, forward, &mut layer.glows);
        flares.draw(&lamps, lamp_frame, lamp_state, flare_view, &mut layer.textured);
        let style = self.tuning.tail_trail_style;
        if trails_visible && style.enabled {
            match style.enhanced {
                true => flares.draw_smooth_trails(
                    &lamps,
                    &drive.tail_trails,
                    lamp_frame,
                    lamp_state,
                    flare_view,
                    style,
                    bounds,
                    layer,
                ),
                false => flares.draw_trails(
                    &lamps,
                    &drive.tail_trails,
                    lamp_frame,
                    lamp_state,
                    flare_view,
                    &mut layer.textured,
                ),
            }
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

#[cfg(test)]
mod fog_shelter_tests {
    use super::*;

    #[test]
    #[ignore = "requires installed world"]
    fn reported_tunnel_has_a_usable_shelter_region() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("game install")).unwrap();
        let index = nfsmw_data::world::WorldIndex::open(&dir, "L2RA").unwrap();
        let zones = index.track_zones.expect("world has track zones");
        let camera = glam::Vec3::new(337.0, 1896.0, 211.5);
        let selected = fog_shelters(&zones, camera, 460.0);
        let original = zones
            .zones
            .iter()
            .find(|zone| {
                [3, 4, 5, 7].contains(&zone.kind) && zone.elevation > camera.z && zone.contains(camera.truncate())
            })
            .expect("reported tunnel must have a covered track zone");
        assert!(
            selected.iter().any(|shelter| {
                shelter.ceiling == original.elevation && shelter.polygon.len() == original.polygon.len()
            }),
            "the reported tunnel zone must reach the fog renderer"
        );
    }
}
