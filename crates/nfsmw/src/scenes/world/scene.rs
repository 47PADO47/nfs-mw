//! Viewer integration, controls and world updates.

use super::*;

impl Scene for WorldScene {
    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        renderer.set_projected_reflection_scene(None);
        self.wet_road.instances.clear();
        self.residency.sky.release(renderer);
        self.residency.release_normal_materials(renderer);
    }

    fn garage(&mut self) -> Option<&mut crate::garage::Model> {
        Some(&mut self.garage)
    }

    fn apply_garage(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.apply_garage_selection(renderer, selection)
    }

    fn set_garage_open(&mut self, open: bool) {
        self.garage_open = open;
    }
    fn paused(&self) -> bool {
        self.garage_open
    }
    fn captures_mouse(&self) -> bool {
        // The mouse looks round the car too, on the chase camera.
        true
    }

    fn title(&self) -> String {
        format!("nfsmw — {}", self.track)
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        renderer.set_colour_treatment(self.colour_treatment.as_ref(), self.gain_map.as_ref());
        self.breaker_visuals.init(renderer);
        if let Some(PendingCar { name, model, script }) = self.pending_car.take() {
            self.start_driving(renderer, name, model, script)?;
        }
        Ok(())
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.original_sky = renderer.render_experiments().original_world_materials;
        self.shadow_quality = renderer.graphics().shadows;
        let idle = ActionState::default();
        let input = match self.garage_open {
            true => &idle,
            false => input,
        };
        let dt = match self.garage_open {
            true => 0.0,
            false => dt,
        };
        self.update_car_shading(renderer);
        if let Some(drive) = self.drive.as_mut() {
            drive.set_damaged_windows(renderer, self.sun.to_sun());
            drive.set_car_scale(self.tuning.car_scale);
            drive.set_no_rev_limiter(self.tuning.no_rev_limiter);
            drive.set_look_back(input.pressed(Action::LookBack));
        }
        if input.just_pressed(Action::ToggleHeadlights)
            && let Some(drive) = self.drive.as_mut()
        {
            let on = drive.toggle_headlights();
            log::info!("headlights {}", if on { "on" } else { "off" });
        }
        let camera = match (self.view, self.drive.as_ref()) {
            (View::Chase, Some(drive)) => drive.camera().position,
            _ => self.camera.position,
        };
        if input.just_pressed(Action::ToggleCamera) && self.drive.is_some() {
            log::info!("{}", self.toggle_view());
        }
        let [x, y] = self.focus();
        let lens = self.gameplay_lens(renderer.aspect_ratio());
        let tan_y = (lens.fov_y_degrees.to_radians() * 0.5).tan();
        let extent = (1.0 + tan_y * tan_y * (1.0 + renderer.aspect_ratio().powi(2))).sqrt();
        self.residency.set_shadow_sun(self.sun.to_sun());
        self.residency.set_shadow_receiver_extent(extent, lens.position.truncate().distance(glam::Vec2::new(x, y)));
        self.residency.update(renderer, x, y);
        self.prepare_drive();
        let sim_dt = match self.garage_open {
            true => 0.0,
            false => self.update_simulation(renderer, input, dt),
        };
        self.update_entry(input, dt);
        renderer.set_focus_filter(self.breaker_visuals.update(self.sound_speedbreaker_active, sim_dt));
        self.rain.update(sim_dt, camera);
        self.wet_road.step(dt, self.rain.intensity() > 0.0);
        self.update_atmosphere(sim_dt);
        self.clock += sim_dt * self.tuning.animation_speed;
        renderer.set_sky_state(sky::state(self.atmosphere.intensity(), self.clock));

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

    fn ui(&mut self, ctx: &egui::Context) -> f32 {
        crate::hud::show_interact_prompt(ctx, &self.entry.popup);
        self.show_pursuit_end(ctx);
        0.0
    }

    fn set_tire_effects(&mut self, smoke: bool, skid_marks: bool) {
        self.tire_effects = [smoke, skid_marks];
        if let Some(drive) = self.drive.as_mut() {
            drive.effects.set_enabled(smoke, skid_marks);
        }
    }

    fn refresh_effects(&mut self, renderer: &mut dyn RenderBackend) {
        self.update_car_shading(renderer);
        self.update_shadow_casters(renderer);
        self.update_reflections(renderer);
        self.upload_effects(renderer);
    }

    fn set_spark_style(&mut self, style: crate::settings::SparkStyle) {
        self.spark_style = style;
        if let Some(drive) = self.drive.as_mut() {
            drive.vehicle_effects.set_style(style);
        }
    }

    fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
        self.vehicle_effects = [sparks, trails];
        self.apply_vehicle_effects();
    }

    fn set_speed_trail_style(&mut self, style: crate::settings::SpeedTrailStyle) {
        self.speed_trail_style = style;
        self.apply_vehicle_effects();
    }

    fn set_exhaust_flames(&mut self, on: bool) {
        self.exhaust_flames = on;
        if let Some(drive) = self.drive.as_mut() {
            drive.flames.set_enabled(on);
        }
    }

    fn note_sputters(&mut self, pops: u32) {
        if let Some(drive) = self.drive.as_mut() {
            drive.flames.note_pops(pops);
        }
    }

    fn set_camera_mode(&mut self, mode: CameraMode) {
        self.change_camera_mode(mode);
    }

    fn environment(&self) -> Option<crate::viewer::Environment> {
        Some(self.environment_state())
    }

    fn set_environment(&mut self, value: crate::viewer::Environment) {
        self.configure_environment(value);
    }

    fn next_camera_mode(&self, current: CameraMode) -> CameraMode {
        camera_cycle::next_mode(current, self.camera_environment.views.as_ref())
    }

    fn takes_camera_setting(&self) -> bool {
        self.drive.is_some()
    }

    fn set_car_shading(&mut self, shading: crate::settings::CarShading) {
        self.car_shading = shading;
    }

    fn set_rain(&mut self, always: bool, amount: f32) {
        self.rain.configure(always, amount);
    }

    fn set_world_tuning(&mut self, tuning: crate::settings::WorldTuning) {
        self.tuning = tuning;
    }

    fn set_smoke_quality(&mut self, quality: crate::settings::SmokeQuality) {
        self.smoke_quality = quality;
        if let Some(drive) = self.drive.as_mut() {
            drive.effects.set_quality(quality);
        }
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        self.world_frame(aspect)
    }

    fn prepare_frame(&mut self, renderer: &mut dyn RenderBackend, aspect: f32) {
        self.original_sky = renderer.render_experiments().original_world_materials;
        self.update_wet_reflections(renderer, aspect);
    }

    fn set_road_reflections(
        &mut self,
        quality: crate::settings::RoadReflections,
        resolution: crate::settings::RoadReflectionResolution,
    ) {
        self.wet_road.quality = quality;
        self.wet_road.resolution = resolution;
    }

    fn rear_view(&mut self, aspect: f32) -> Option<(FrameParams, &[Instance])> {
        self.rear_frame(aspect)
    }

    fn ready(&self) -> bool {
        self.ready_for_capture()
    }
    fn benchmark_ready(&self) -> bool {
        self.ready_for_benchmark()
    }

    fn status(&self) -> Option<String> {
        let (resident, loading, shared) = self.residency.counts();
        let (eligible, selected) = self.residency.shadow_support_counts();
        let p = match (&self.drive, self.view) {
            (Some(drive), View::Chase) => drive.position(),
            _ => self.camera.position,
        };
        let zone = self.residency.zone().unwrap_or_else(|| "-".into());
        let markers = match (&self.drive, self.markers_on) {
            (Some(drive), true) => {
                let (walls, tyres) = drive.marker_counts();
                format!(", {walls} contacts, {tyres} tyre rays")
            }
            _ => String::new(),
        };
        Some(format!(
            "{}zone {zone}, tiles {resident} (+{loading} loading), shadows {selected}/{eligible} extra, {} drawn, {} props ({} knocked over), at ({:.0}, {:.0}, {:.0}){markers}",
            if shared { "" } else { "loading shared sets… " },
            self.visible.len(),
            self.residency.props().len(),
            self.residency.props().knocked_count(),
            p.x,
            p.y,
            p.z
        ))
    }

    fn set_transmission(&mut self, transmission: crate::settings::Transmission) {
        if let Some(drive) = self.drive.as_mut() {
            drive.set_transmission(transmission);
        }
    }

    fn set_wheel_options(&mut self, wheel: crate::settings::WheelOptions) {
        self.drive.iter_mut().for_each(|drive| drive.set_wheel_options(wheel));
    }

    fn set_traffic(&mut self, traffic: crate::settings::TrafficSettings) {
        self.apply_traffic_setting(traffic);
    }

    fn speed_streaks(&self) -> f32 {
        self.speed_streak_strength()
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        let looking_back = self.drive.as_ref().is_some_and(|d| d.camera_looking_back(self.camera_mode));
        if looking_back && !self.tuning.show_hud_on_look_back {
            return None;
        }
        self.hud_frame()
    }

    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        self.audio_state()
    }

    fn take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
        std::mem::take(&mut self.common_sounds)
    }

    fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        let rain = self.rain.intensity();
        self.traffic.as_ref()?.pursuit_feed().map(|feed| crate::audio::PursuitFeed { rain_intensity: rain, ..feed })
    }

    fn take_traffic_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        self.traffic.as_mut().map(|t| t.take_sounds()).unwrap_or_default()
    }

    fn take_explosions(&mut self) -> u32 {
        self.explosions.take_started()
    }

    fn readout(&self, level: crate::devtools::ShowReadout) -> Option<String> {
        let drive = self.drive.as_ref()?;
        let lines = drive.readout(level);
        Some(match self.view {
            View::Chase => lines,
            View::Fly => format!("free camera, the car waits (F to return)\n{lines}"),
        })
    }

    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        commands::LIST
    }

    fn command(
        &mut self,
        renderer: &mut dyn RenderBackend,
        name: &str,
        args: &[&str],
    ) -> Option<Result<String, String>> {
        commands::run(self, renderer, name, args)
    }
}
