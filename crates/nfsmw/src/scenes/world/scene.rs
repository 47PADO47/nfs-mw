//! Viewer integration, controls and world updates.

use super::*;
use crate::viewer::Fullscreen;

impl Scene for WorldScene {
    fn extra_options(&mut self) -> Option<&mut crate::extra_options::State> {
        Some(&mut self.extra)
    }
    fn apply_extra_option(
        &mut self,
        renderer: &mut dyn RenderBackend,
        action: crate::extra_options::Action,
    ) -> anyhow::Result<()> {
        self.extra_action(renderer, action)
    }
    fn screenshot_ui_hidden(&self) -> bool {
        self.extra.hide_ui
    }
    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        renderer.set_sun_glare(None);
        self.close_garage_backdrop(renderer);
        self.release_gpu(renderer);
    }
    fn garage(&mut self) -> Option<&mut crate::garage::Model> {
        Some(&mut self.garage)
    }
    fn apply_garage(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.apply_garage_selection(renderer, selection)
    }
    fn begin_garage(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        anyhow::ensure!(self.garage_entry_allowed(), "Auto shops are unavailable during an event or pursuit");
        self.begin_garage_session(renderer)
    }
    fn begin_tuning(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.begin_garage_session(renderer)
    }
    fn tuning_available(&self) -> Option<bool> {
        self.garage_tuning_available()
    }
    fn confirm_tuning(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.confirm_garage_tuning(renderer, selection)
    }
    fn garage_view(
        &mut self,
        renderer: &mut dyn RenderBackend,
        stage: crate::garage::View,
        camera: &'static str,
    ) -> Result<()> {
        self.show_garage_view(renderer, stage, camera)
    }
    fn preview_garage(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.preview_garage_session(renderer, selection)
    }
    fn confirm_garage(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.confirm_garage_session(renderer, selection)
    }
    fn cancel_garage(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.cancel_garage_session(renderer)?;
        self.close_garage_backdrop(renderer);
        self.leave_garage_at_exit();
        Ok(())
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
        self.update_scene(renderer, input, dt);
    }
    fn set_waypoint(&mut self, target: Option<[f32; 2]>) -> bool {
        self.route_to(target)
    }

    #[cfg(feature = "multiplayer")]
    fn attach_net_client(&mut self, client: &crate::net::ClientHandle) -> bool {
        self.attach_client(client)
    }
    fn runs_while_paused(&self) -> bool {
        self.is_net_client()
    }
    fn ui(&mut self, ctx: &egui::Context) -> f32 {
        self.scene_ui(ctx)
    }

    /// Whether any of the panels above has something to draw (see `ui_panels::Panels`). The scene's readout line is the
    /// metrics overlay's, so the UI pass asks the metrics for it.
    fn wants_ui(&self) -> bool {
        self.panels().any()
    }
    fn fullscreen(&mut self) -> Option<Fullscreen> {
        self.story_movie.picture()
    }
    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        self.story_movie.take_clip()
    }

    fn set_tire_effects(&mut self, smoke: bool, skid_marks: bool) {
        self.scene_set_tire_effects(smoke, skid_marks)
    }

    fn refresh_effects(&mut self, renderer: &mut dyn RenderBackend) {
        self.update_car_shading(renderer);
        self.update_shadow_casters(renderer);
        self.update_reflections(renderer);
        self.shadow_feed.cars.invalidate();
        self.upload_effects(renderer);
    }

    fn set_spark_style(&mut self, style: crate::settings::SparkStyle) {
        self.scene_set_spark_style(style)
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
        self.scene_set_exhaust_flames(on)
    }

    fn note_sputters(&mut self, pops: u32) {
        if let Some(drive) = self.players.lead_mut() {
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
        camera_cycle::next_mode(current, self.camera_environment.views.as_ref(), &self.extra_view_keys())
    }

    fn takes_camera_setting(&self) -> bool {
        self.players.has_lead()
    }

    fn set_detail(&mut self, detail: crate::settings::DetailSettings) {
        self.detail.configure(detail);
        self.residency.capture_texture_detail(detail.world.mip_discard());
    }
    fn set_car_shading(&mut self, shading: crate::settings::CarShading) {
        self.car_shading = shading;
        if let Some(backdrop) = &mut self.garage_backdrop {
            backdrop.set_car_shading(shading);
        }
    }

    fn set_rain(&mut self, always: bool, amount: f32) {
        self.rain.configure(always, amount);
    }

    fn set_tail_preview(&mut self, preview: Option<crate::settings::TailPreview>) {
        if let Some(drive) = self.players.lead_mut() {
            drive.tail_trails.set_preview(preview);
        }
    }
    fn set_world_tuning(&mut self, tuning: crate::settings::WorldTuning) {
        self.tuning = tuning;
    }

    fn set_smoke_quality(&mut self, quality: crate::settings::SmokeQuality) {
        self.scene_set_smoke_quality(quality)
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        // Choose the owner before borrowing its returned instance slice.
        match self.garage_backdrop.is_some() {
            true => self.garage_backdrop.as_mut().expect("garage backdrop").frame(aspect),
            false => self.world_frame(aspect),
        }
    }

    fn prepare_frame(&mut self, renderer: &mut dyn RenderBackend, aspect: f32) {
        if let Some(backdrop) = &mut self.garage_backdrop {
            backdrop.prepare_frame(renderer, aspect);
            return;
        }
        self.update_sky(renderer);
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
        if self.garage_backdrop.is_some() {
            return None;
        }
        // Each half of a split screen has no mirror; the picture is the two views.
        match self.split_on() {
            true => None,
            false => self.rear_frame(aspect),
        }
    }

    fn set_second_input(&mut self, input: &ActionState) {
        if let Some(split) = self.split.as_mut() {
            split.actions = input.clone();
        }
        if self.players.has_guests() {
            self.inputs.set(super::players::PlayerId(1), input);
        }
    }

    fn split_active(&self) -> bool {
        self.split_on()
    }

    fn coop_second_device(&self) -> Option<crate::input::CoopDevice> {
        self.coop.as_ref().map(|session| session.device)
    }

    fn end_coop(&mut self) {
        self.queue_end_coop();
    }

    fn coop_end_locked(&self) -> bool {
        self.coop_end_locked()
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
    fn race_staging(&self) -> bool {
        self.race_is_staging()
    }
    fn benchmark_ready(&self) -> bool {
        self.ready_for_benchmark()
    }

    fn status(&self) -> Option<String> {
        self.status_line()
    }

    fn set_transmission(&mut self, transmission: crate::settings::Transmission) {
        if let Some(drive) = self.players.lead_mut() {
            drive.set_transmission(transmission);
        }
    }

    fn set_wheel_options(&mut self, wheel: crate::settings::WheelOptions) {
        self.players.drives_mut().for_each(|drive| drive.set_wheel_options(wheel));
    }

    fn set_traffic(&mut self, traffic: crate::settings::TrafficSettings) {
        self.apply_traffic_setting(traffic);
    }

    fn speed_streaks(&self) -> f32 {
        self.speed_streak_strength()
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        self.scene_hud()
    }

    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        self.audio_state()
    }

    fn radio_on_menus(&self) -> bool {
        self.garage_backdrop.is_some()
    }

    fn second_car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        self.guest_audio_state()
    }
    fn note_car_visual_rpm(&mut self, player: usize, fraction: Option<f32>) {
        self.accept_car_visual_rpm(player, fraction);
    }

    fn take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
        std::mem::take(&mut self.common_sounds)
    }

    fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        self.scene_pursuit_feed()
    }

    fn roam_feed(&mut self) -> Option<crate::audio::RoamFeed> {
        self.scene_roam_feed()
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
    fn take_world_effect_audio(&mut self) -> Vec<crate::audio::WorldEffectAudio> {
        self.residency.world_parts().1.take_world_effect_audio()
    }

    fn readout(&self, level: crate::devtools::ShowReadout) -> Option<String> {
        let drive = self.players.lead()?;
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
        if name == "career" {
            return Some(career::sms_popup::command(self, args));
        }
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
    fn set_effect_variants(&mut self, variants: &crate::effect_variants::EffectVariants) {
        self.scene_set_effect_variants(variants);
    }
    fn player_car(&mut self) -> Option<crate::viewer::PlayerCar> {
        self.player_car_state()
    }
    fn engine_readout(&mut self) -> Option<crate::engine_switch::EngineReadout> {
        self.engine_state()
    }
    fn engine_command(&mut self, command: crate::engine_switch::EngineCommand) {
        self.engine_request(command);
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
    fn jump_player(&mut self, at: [f32; 2]) -> bool {
        self.start_at_point(at)
    }
    fn start_career_event(&mut self, event: &str) -> bool {
        let options = super::race::RaceOptions { career: true, ..Default::default() };
        match self.start_race(event, options) {
            Ok(_) => true,
            Err(e) => {
                log::warn!("career: cannot start event {event}: {e}");
                false
            }
        }
    }
    fn add_player_nitrous(&mut self, amount: f32) -> bool {
        self.add_player_nitrous_to(amount)
    }

    fn set_player_drift_assist(&mut self, on: bool) {
        self.set_player_drift_assist_to(on);
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
        // Co-op has no ticket screen: `coop_auto_pay_ticket` pays the ticket itself.
        if self.coop.is_some() {
            return None;
        }
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
    fn nis_audio(&self) -> Option<&crate::audio::nis::NisAudioRequest> {
        self.nis.audio_request()
    }
    fn note_nis_audio(&mut self, playing: bool) {
        self.nis.note_voice(playing);
    }
    fn rotor_frame(&mut self, dt: f32) -> Option<crate::audio::HeliFrame> {
        self.rotor_frame_now(dt)
    }
    fn siren_frame(&mut self, dt: f32) -> Option<crate::audio::SirenFrame> {
        self.traffic.as_mut()?.siren_frame(dt)
    }
    fn note_police_speech(&mut self, phase: Option<blackbox_pursuit::siren::Phase>) {
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.note_police_speech(phase);
        }
    }
}
