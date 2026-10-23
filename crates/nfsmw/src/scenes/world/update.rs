//! The per-frame update of the world scene, kept out of `mod.rs` so the scene's trait impl stays short.

use blackbox_gfx::RenderBackend;

use super::{View, WorldScene, ground, space};
use crate::input::{Action, ActionState};

impl WorldScene {
    /// Advance the scene by one frame: input, streaming, simulation, effects and captures.
    pub(super) fn update_frame(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
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
        let frozen = self.garage_open || self.nis.is_playing() || self.story_movie.is_playing();
        let input = match frozen {
            true => &idle,
            false => input,
        };
        let dt = match frozen {
            true => 0.0,
            false => dt,
        };
        self.update_car_shading(renderer);
        if let Some(traffic) = self.traffic.as_ref() {
            traffic.update_damaged_windows(renderer, self.sun.to_sun());
        }
        if let Some(drive) = self.players.lead_mut() {
            drive.set_damaged_windows(renderer, self.sun.to_sun());
            drive.set_car_scale(self.tuning.car_scale);
            drive.set_no_rev_limiter(self.tuning.no_rev_limiter);
            drive.set_look_back(input.pressed(Action::LookBack));
        }
        if input.just_pressed(Action::ToggleHeadlights)
            && let Some(drive) = self.players.lead_mut()
        {
            let on = drive.toggle_headlights();
            log::info!("headlights {}", if on { "on" } else { "off" });
        }
        let camera = match (self.view, self.players.lead()) {
            (View::Chase, Some(drive)) => drive.camera().position,
            _ => self.camera.position,
        };
        if input.just_pressed(Action::ToggleCamera) && self.players.has_lead() {
            log::info!("{}", self.toggle_view());
        }
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
        self.prepare_drive();
        self.start_queued_coop(renderer);
        self.place_guests();
        self.feed_second_player();
        // Single player runs the world's rules; a multiplayer client predicts its car and draws the rest from the
        // server (`frame_systems.rs`).
        let systems = self.frame_systems();
        let sim_dt = match (frozen, systems.world_simulation) {
            (true, _) => 0.0,
            (false, true) => self.update_simulation(renderer, input, dt),
            (false, false) => self.update_net_client(renderer, input, dt),
        };
        if systems.catch_up {
            self.update_catch_up(input);
        }
        self.follow_second_player(dt);
        if systems.races {
            self.update_race(renderer, input, dt);
        }
        if systems.entry {
            self.update_entry(input, dt);
        }
        if systems.career {
            self.update_career(dt);
        }
        self.feed_focus(renderer, sim_dt);
        self.rain.update(sim_dt, camera);
        self.wet_road.step(dt, self.rain.intensity() > 0.0);
        self.update_atmosphere(sim_dt);
        self.clock += sim_dt * self.tuning.animation_speed;
        self.environment_lights.blink.advance(sim_dt);
        self.update_sky(renderer);

        self.update_day(sim_dt, renderer.render_experiments().freeze_sun);

        self.residency.animate(renderer, self.clock);
        if self.view == View::Fly {
            self.camera.update(input, sim_dt);
        }
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
        self.upload_effects(renderer);
        self.original_shadows = renderer.render_experiments().original_shadows;
        self.update_reflections(renderer);
        self.update_shadow_casters(renderer);
    }
}
