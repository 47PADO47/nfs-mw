//! Viewer integration, controls and world updates.

use super::*;

impl Scene for WorldScene {
    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        self.release_gpu(renderer);
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
        Self::note_radial_blur_support(renderer);
        if let Some(open::PendingCar { name, model, script }) = self.pending_car.take() {
            self.start_driving(renderer, name, model, script)?;
        }
        Ok(())
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.update_frame(renderer, input, dt);
        self.update_route(dt);
    }

    fn set_waypoint(&mut self, target: Option<[f32; 2]>) -> bool {
        self.route_to(target)
    }

    fn ui(&mut self, ctx: &egui::Context) -> f32 {
        crate::hud::show_interact_prompt(ctx, &self.entry.popup);
        crate::hud::show_mod_message(ctx);
        self.show_pursuit_end(ctx);
        self.show_race_card(ctx);
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
        if self.spark_style != style {
            self.impact_fx.clear();
        }
        self.spark_style = style;
        if let Some(drive) = self.drive.as_mut() {
            drive.vehicle_effects.set_style(style);
        }
    }

    fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
        self.vehicle_effects = [sparks, trails];
        self.impact_fx.set_enabled(sparks);
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

    fn traffic_status(&self) -> Option<String> {
        self.traffic
            .as_ref()
            .map(|traffic| format!("{}\n{}", traffic.population_status(self.traffic_focus()), traffic.pursuit_status()))
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

    fn set_detail(&mut self, detail: crate::settings::DetailSettings) {
        self.detail.configure(detail);
        self.residency.capture_texture_detail(detail.world.mip_discard());
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
        // Each half of a split screen has no mirror; the picture is the two views.
        match self.split.is_some() {
            true => None,
            false => self.rear_frame(aspect),
        }
    }

    fn set_second_input(&mut self, input: &ActionState) {
        if let Some(split) = self.split.as_mut() {
            split.actions = input.clone();
        }
    }

    fn split_active(&self) -> bool {
        self.split.is_some()
    }

    fn split_view(&mut self, player: usize, aspect: f32) -> Option<(FrameParams, &[Instance])> {
        self.split_frame(player, aspect)
    }

    fn split_hud_state(&self, player: usize) -> Option<crate::hud::HudState> {
        self.split_hud(player)
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
        if self.moment.hides_hud() {
            return None;
        }
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
        let look = self.player_look(&self.speech_look).unwrap_or_default();
        let mut feed = self.traffic.as_ref()?.pursuit_feed()?;
        feed.rain_intensity = rain;
        feed.speech.player.car_type = look.car_type;
        feed.speech.player.car_colour = look.colour;
        Some(feed)
    }

    fn roam_feed(&mut self) -> Option<crate::audio::RoamFeed> {
        let look = self.player_look(&self.speech_look).unwrap_or_default();
        let traffic = self.traffic.as_mut()?;
        let tuning = traffic.roam_tuning();
        let mut feed = traffic.roam_feed(&tuning)?;
        feed.player.car_type = look.car_type;
        feed.player.car_colour = look.colour;
        Some(feed)
    }

    fn start_911_call(&mut self, secs: f32) {
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.start_911_call(secs);
        }
    }

    fn pursuit_ended(&self) -> blackbox_pursuit::EndReason {
        self.traffic.as_ref().map_or(blackbox_pursuit::EndReason::None, |traffic| traffic.end_reason())
    }

    fn take_traffic_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        self.traffic.as_mut().map(|t| t.take_sounds()).unwrap_or_default()
    }

    fn take_explosions(&mut self) -> Vec<glam::Vec3> {
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
    fn time_held(&self) -> bool {
        self.day.held()
    }
    fn set_time_held(&mut self, held: bool) {
        self.day.set_held(held);
    }
    fn set_mod_blips(&mut self, blips: &[crate::hud::MapBlip]) {
        self.mod_blips.clear();
        self.mod_blips.extend_from_slice(blips);
    }
    fn player_car(&mut self) -> Option<crate::viewer::PlayerCar> {
        self.player_car_state()
    }
    fn camera_readout(&self) -> Option<crate::viewer::CameraReadout> {
        Some(self.camera_now())
    }
    fn world_info(&self) -> Option<crate::viewer::WorldInfo> {
        Some(self.world_now())
    }
    fn move_player(&mut self, at: [f32; 2], heading: f32) -> bool {
        self.move_player_to(at, heading)
    }
    fn repair_player(&mut self) -> bool {
        self.repair_player_car()
    }
    fn set_traffic_density(&mut self, cars: u32, cop_share: u8) {
        self.set_traffic_density_to(cars, cop_share);
    }
    fn spawn_traffic_at(&mut self, at: [f32; 2], heading: f32) -> bool {
        self.spawn_traffic_to(at, heading)
    }
    fn take_bust_ticket(&mut self) -> Option<Vec<(blackbox_pursuit::Infraction, u32)>> {
        self.traffic.as_mut()?.take_bust_ticket()
    }
    fn take_race_report(&mut self) -> Option<crate::viewer::RaceReport> {
        self.take_race_report()
    }
    fn take_trap_photo(&mut self) -> Option<crate::viewer::PhotoReport> {
        self.career.trap_photo.take()
    }
    fn take_rewards_report(&mut self) -> Option<crate::viewer::RewardsReport> {
        self.career.rewards.take()
    }
    fn answer_race(&mut self, answer: crate::viewer::RaceAnswer) -> bool {
        self.answer_race(answer)
    }
    fn race_active(&self) -> bool {
        self.race_in_progress()
    }
    fn restart_race(&mut self) -> bool {
        self.restart_race_now()
    }
    fn pay_bust(&mut self, marker: bool) -> bool {
        self.career_pay_bust(marker)
    }
    fn take_career_request(&mut self) -> Option<crate::career::CareerRequest> {
        self.career.requests.pop_front()
    }
    fn rotor_frame(&mut self, dt: f32) -> Option<crate::audio::HeliFrame> {
        self.rotor_frame_now(dt)
    }
    fn siren_frame(&mut self, dt: f32) -> Option<crate::audio::SirenFrame> {
        self.traffic.as_mut()?.siren_frame(dt)
    }
}
