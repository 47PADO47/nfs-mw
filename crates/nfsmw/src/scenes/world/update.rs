//! The per-frame update of the world scene, kept out of `mod.rs` so the scene's trait impl stays short.

use blackbox_gfx::RenderBackend;

use super::{View, WorldScene, ground, space};
use crate::input::{Action, ActionState};
use crate::viewer::Scene;

impl WorldScene {
    /// Advance the scene by one frame: input, streaming, simulation, effects and captures.
    pub(super) fn update_frame(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.prepare_extra();
        let camera_dt = dt;
        let dt = match self.extra.testing.paused {
            true => 0.0,
            false => dt,
        };
        self.update_garage_camera(input, dt);
        let options = renderer.render_experiments();
        self.original_sky = options.original_world_materials || options.enhanced_sky;
        let _span = bevy_log::info_span!("world::update").entered();
        let _trace = blackbox_trace::scope("World: update");
        self.shadow_quality = renderer.graphics().shadows;
        self.detail.mirror_far = renderer.graphics().reflection_distance.mirror_far();
        // The bust's arrest scene starts before the NIS is advanced, so its first frame cuts and freezes the world.
        self.update_arrest_nis();
        self.update_nis(input, dt);
        self.update_nis_characters(renderer);
        self.update_nis_models(renderer);
        self.update_story_movie(input, dt);
        self.update_coop_suspend();
        // The garage, a playing NIS scene and a playing story movie freeze the world: no input reaches it and no time
        // passes in it.
        let idle = ActionState::default();
        let frozen =
            self.extra.testing.paused || self.garage_open || self.nis.is_playing() || self.story_movie.is_playing();
        let camera_input = input.clone();
        let input = match frozen {
            true => &idle,
            false => input,
        };
        let dt = match frozen {
            true => 0.0,
            false => dt,
        };
        // The free camera looks with the right stick, which also shifts the car: the stick moves one or the other.
        let mut free_camera_input;
        let input = if self.view == View::Fly {
            free_camera_input = input.clone();
            free_camera_input.consume(Action::ShiftUp);
            free_camera_input.consume(Action::ShiftDown);
            &free_camera_input
        } else {
            input
        };
        let trace = blackbox_trace::scope("World: update / shading and effects set-up");
        self.update_car_shading(renderer);
        if let Some(traffic) = self.traffic.as_ref() {
            traffic.update_damaged_windows(renderer, self.sun.to_sun());
        }
        if let Some(drive) = self.players.lead_mut() {
            drive.set_damaged_windows(renderer, self.sun.to_sun());
            drive.set_car_scale(self.tuning.car_scale);
            drive.vehicle_effects.set_tuning(self.tuning.xenon_effects);
            drive.set_no_rev_limiter(self.tuning.no_rev_limiter);
            drive.set_look_back(input.pressed(Action::LookBack));
        }
        let camera_shake = self.tuning.camera_shake;
        for drive in self.players.drives_mut() {
            drive.set_camera_shake(camera_shake);
        }
        if input.just_pressed(Action::ToggleHeadlights)
            && let Some(drive) = self.players.lead_mut()
        {
            let on = drive.toggle_headlights();
            log::info!("headlights {}", if on { "on" } else { "off" });
        }
        if input.just_pressed(Action::ToggleCamera) && self.players.has_lead() {
            log::info!("{}", self.toggle_view());
        }
        drop(trace);
        let trace = blackbox_trace::scope("World: update / streaming");
        let [x, y] = self.focus();
        let lens = self.gameplay_lens(renderer.aspect_ratio());
        let tan_y = (lens.fov_y_degrees.to_radians() * 0.5).tan();
        let extent = (1.0 + tan_y * tan_y * (1.0 + renderer.aspect_ratio().powi(2))).sqrt();
        self.residency.set_shadow_sun(self.sun.to_sun());
        self.residency.set_shadow_receiver_extent(extent, lens.position.truncate().distance(glam::Vec2::new(x, y)));
        self.prepare_traffic_support();
        // Section overrides expire on reload only outside an active race (prop-lifetime.md).
        let racing = self.race.as_ref().is_some_and(|race| {
            matches!(race.stage, super::race::Stage::Intro { .. })
                || (race.stage == super::race::Stage::Running
                    && (race.challenge.is_some()
                        || race.engine.as_ref().is_some_and(|engine| engine.phase() == blackbox_race::Phase::Racing)))
        });
        self.residency.world_parts().1.set_streaming_reset_locked(racing);
        let centres = self.foci();
        self.residency.update_centres(renderer, &centres);
        drop(trace);
        let trace = blackbox_trace::scope("World: update / players and guests");
        self.prepare_drive();
        self.start_queued_coop(renderer);
        self.place_guests();
        self.feed_second_player();
        self.feed_race_people();
        for drive in self.players.drives_mut() {
            let environment = self
                .audio_environment
                .as_ref()
                .map(|map| map.at(glam::Vec3::from_array(space::to_physics(drive.position()))))
                .unwrap_or_default();
            let exposed = !(environment.tunnel || environment.overpass || environment.garage || environment.dynamic);
            let enhanced = options.enhanced_wet_surfaces;
            let water = self.wet_road.water();
            let wet = (self.rain.intensity() > 0.0 || (enhanced && water.dampness > 0.05)) && exposed;
            drive.vehicle_effects.set_surface_water(enhanced.then_some((water, options)));
            drive.vehicle_effects.set_rain(wet);
            drive.effects.set_wet(wet);
        }
        // Single player runs the world's rules; a multiplayer client predicts its car and draws the rest from the
        // server (`frame_systems.rs`).
        drop(trace);
        let systems = self.frame_systems();
        let sim_dt = match (frozen, systems.world_simulation) {
            (true, _) => 0.0,
            (false, true) => self.update_simulation(renderer, input, dt),
            (false, false) => self.update_net_client(renderer, input, dt),
        };
        let trace = blackbox_trace::scope("World: update / catch-up, races, entry, career");
        if systems.catch_up {
            {
                let _t = blackbox_trace::scope("World: update / catch-up");
                self.update_catch_up(input);
            }
        }
        {
            let _t = blackbox_trace::scope("World: update / second player");
            self.follow_second_player(dt);
        }
        if systems.races {
            {
                let _t = blackbox_trace::scope("World: update / race");
                self.update_race(renderer, input, dt);
            }
        }
        if systems.entry {
            {
                let _t = blackbox_trace::scope("World: update / entry");
                self.update_entry(input, dt);
            }
        }
        if systems.career {
            {
                let _t = blackbox_trace::scope("World: update / career");
                self.update_career(dt);
            }
        }
        drop(trace);
        let trace = blackbox_trace::scope("World: update / focus, rain and atmosphere");
        self.feed_focus(renderer, sim_dt);
        let lens = self.gameplay_lens(renderer.aspect_ratio());
        let environment = self
            .audio_environment
            .as_ref()
            .map(|map| map.at(glam::Vec3::from_array(space::to_physics(lens.position))))
            .unwrap_or_default();
        let exposed = !(environment.tunnel || environment.overpass || environment.garage || environment.dynamic);
        self.rain.update_view(sim_dt, lens.position, lens.forward, exposed);
        self.wet_road.step_rain(sim_dt, self.rain.intensity());
        if let Some(traffic) = &mut self.traffic {
            traffic.step_rain(
                self.physics.database(),
                self.residency.collision(),
                super::ai::RainView {
                    camera: lens.position,
                    intensity: self.rain.intensity(),
                    water: options.enhanced_wet_surfaces.then_some((self.wet_road.water(), options)),
                    environment: self.audio_environment.as_ref(),
                    dt: sim_dt,
                },
            );
        }
        self.update_atmosphere(sim_dt);
        drop(trace);
        let trace = blackbox_trace::scope("World: update / sky, day and animation");
        self.clock += sim_dt * self.tuning.animation_speed;
        self.environment_lights.blink.advance(sim_dt);
        self.update_sky(renderer);

        self.update_day(sim_dt, renderer.render_experiments().freeze_sun);

        self.residency.animate(renderer, self.clock);
        if self.extra.testing.paused
            && self.view == View::Chase
            && let Some(drive) = self.players.lead_mut()
        {
            drive.follow(
                self.residency.collision(),
                (camera_input.value(Action::LookX), camera_input.value(Action::LookY)),
                camera_dt,
            );
            drive.set_alpha(1.0);
        }

        if self.extra.testing.paused && self.view == View::Chase {
            self.seed_original_camera();
            self.update_original_camera(camera_dt, 0.0, self.speedbreaker_clock, false, 1.0);
        }
        if self.view == View::Fly {
            let tools_camera = (self.extra.freecam || self.extra.testing.paused)
                && !self.garage_open
                && !self.nis.is_playing()
                && !self.story_movie.is_playing();
            if tools_camera {
                self.camera.update(&camera_input, camera_dt);
            }
            if !tools_camera {
                self.camera.update(input, sim_dt);
            }
        }
        drop(trace);
        if !self.grounded && self.residency.complete() {
            self.grounded = true;
            let estimate = ground::height_near(self.residency.placed(), self.start[0], self.start[1], 150.0);
            // The collision surface just under the estimate is the real road; the estimate is only a guess.
            let top = estimate.unwrap_or(0.0) + 30.0;
            let road = space::ground_below(self.residency.collision(), self.start[0], self.start[1], top, top - 120.0);
            self.camera.position.z = road.or(estimate).unwrap_or(0.0) + self.start_height;
            self.cut.arm();
            log::info!(
                "ground near the start: scenery estimate {estimate:?}, collision {road:?}; camera at z = {:.0}",
                self.camera.position.z
            );
        }
        let trace = blackbox_trace::scope("World: update / effects uploads");
        if let Some(drive) = self.players.lead_mut() {
            let pose = drive.pose().transform();
            drive.flames.advance_visual(sim_dt, pose);
        }
        self.upload_effects(renderer);
        self.original_shadows = renderer.render_experiments().original_shadows;
        drop(trace);
        self.update_reflections(renderer);
        self.update_shadow_casters(renderer);
        // Both have read the traffic cars; the next frame moves them.
        self.shadow_feed.cars.invalidate();
    }
}

impl WorldScene {
    pub(super) fn update_scene(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        if let Some(backdrop) = &mut self.garage_backdrop {
            backdrop.update(renderer, input, dt);
            return;
        }
        self.begin_hud_feed();
        self.update_frame(renderer, input, dt);
        self.update_route(dt);
        self.end_hud_feed(dt);
    }
}
