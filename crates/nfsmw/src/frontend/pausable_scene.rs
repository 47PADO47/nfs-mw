//! Scene forwarding while menus conceal or pause the world.
use super::*;
use crate::viewer::Fullscreen;

impl Scene for Pausable {
    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        self.inner.close(renderer);
    }

    fn reattached(&mut self, renderer: &mut dyn RenderBackend) {
        self.inner.reattached(renderer);
    }

    fn garage(&mut self) -> Option<&mut crate::garage::Model> {
        self.inner.garage()
    }

    fn apply_garage(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.inner.apply_garage(renderer, selection)
    }

    fn set_garage_open(&mut self, open: bool) {
        self.inner.set_garage_open(open);
    }
    fn title(&self) -> String {
        self.inner.title()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.inner.init(renderer)
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        if self.paused.get() {
            if self.effects_dirty {
                self.inner.refresh_effects(renderer);
                self.effects_dirty = false;
            }
            return;
        }
        self.inner.update(renderer, input, dt);
        self.effects_dirty = false;
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        if !self.concealed() {
            return self.inner.frame(aspect);
        }
        // The scene still takes its frame (its camera and physics read the aspect ratio), but nothing is drawn.
        let _ = self.inner.frame(aspect);
        (flat_frame(), &[] as &[Instance])
    }

    // The rear-view mirror is the driving scene's, so the wrapper must pass it on: without this the HUD mirror
    // shows its bezel over a blank picture whenever the scene runs inside the front end (the play path).
    fn rear_view(&mut self, aspect: f32) -> Option<(FrameParams, &[Instance])> {
        if self.concealed() {
            return None;
        }
        self.inner.rear_view(aspect)
    }

    // Like the rear view, speed streaks belong to the driving scene; forward them so the play path draws them.
    fn speed_streaks(&self) -> f32 {
        self.inner.speed_streaks()
    }

    fn ready(&self) -> bool {
        self.inner.ready()
    }

    fn benchmark_ready(&self) -> bool {
        self.inner.benchmark_ready()
    }

    fn captures_mouse(&self) -> bool {
        !self.paused.get() && !self.concealed() && self.inner.captures_mouse()
    }

    fn status(&self) -> Option<String> {
        self.inner.status()
    }

    fn readout(&self, level: crate::devtools::ShowReadout) -> Option<String> {
        self.inner.readout(level)
    }

    fn set_transmission(&mut self, transmission: crate::settings::Transmission) {
        self.inner.set_transmission(transmission);
    }

    fn environment(&self) -> Option<crate::viewer::Environment> {
        self.inner.environment()
    }

    fn traffic_status(&self) -> Option<String> {
        self.inner.traffic_status()
    }

    fn set_environment(&mut self, value: crate::viewer::Environment) {
        self.inner.set_environment(value);
        self.effects_dirty = true;
    }

    fn time_held(&self) -> bool {
        self.inner.time_held()
    }

    fn set_time_held(&mut self, held: bool) {
        self.inner.set_time_held(held);
    }

    fn set_mod_blips(&mut self, blips: &[crate::hud::MapBlip]) {
        self.inner.set_mod_blips(blips);
    }

    fn set_waypoint(&mut self, target: Option<[f32; 2]>) -> bool {
        self.inner.set_waypoint(target)
    }

    fn player_car(&mut self) -> Option<crate::viewer::PlayerCar> {
        self.inner.player_car()
    }

    fn camera_readout(&self) -> Option<crate::viewer::CameraReadout> {
        self.inner.camera_readout()
    }

    fn world_info(&self) -> Option<crate::viewer::WorldInfo> {
        self.inner.world_info()
    }

    fn move_player(&mut self, at: [f32; 2], heading: f32) -> bool {
        self.inner.move_player(at, heading)
    }

    fn repair_player(&mut self) -> bool {
        self.inner.repair_player()
    }

    fn set_traffic_density(&mut self, cars: u32, cop_share: u8) {
        self.inner.set_traffic_density(cars, cop_share);
    }

    fn spawn_traffic_at(&mut self, at: [f32; 2], heading: f32) -> bool {
        self.inner.spawn_traffic_at(at, heading)
    }

    fn set_rain(&mut self, always: bool, amount: f32) {
        self.inner.set_rain(always, amount);
        self.effects_dirty = true;
    }

    fn set_world_tuning(&mut self, tuning: crate::settings::WorldTuning) {
        self.inner.set_world_tuning(tuning);
    }

    fn set_camera_mode(&mut self, mode: crate::settings::CameraMode) {
        self.inner.set_camera_mode(mode);
    }

    fn next_camera_mode(&self, current: crate::settings::CameraMode) -> crate::settings::CameraMode {
        self.inner.next_camera_mode(current)
    }

    fn takes_camera_setting(&self) -> bool {
        self.inner.takes_camera_setting()
    }

    fn set_wheel_options(&mut self, wheel: crate::settings::WheelOptions) {
        self.inner.set_wheel_options(wheel);
    }

    fn set_traffic(&mut self, traffic: crate::settings::TrafficSettings) {
        self.inner.set_traffic(traffic);
    }

    fn prepare_frame(&mut self, renderer: &mut dyn RenderBackend, aspect: f32) {
        self.inner.prepare_frame(renderer, aspect);
    }

    fn set_road_reflections(
        &mut self,
        quality: crate::settings::RoadReflections,
        resolution: crate::settings::RoadReflectionResolution,
    ) {
        self.inner.set_road_reflections(quality, resolution);
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        if self.paused.get() || self.concealed() {
            return Some(crate::hud::HudState { visible: false, ..Default::default() });
        }
        self.inner.hud_state()
    }

    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.paused.get() || self.concealed() {
            return None;
        }
        self.inner.car_sound()
    }

    fn take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
        let sounds = self.inner.take_common_sounds();
        if self.paused.get() || self.concealed() {
            return Vec::new();
        }
        sounds
    }

    fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        if self.concealed() {
            return None;
        }
        self.inner.pursuit_feed()
    }

    fn pursuit_ended(&self) -> blackbox_pursuit::EndReason {
        if self.concealed() {
            return blackbox_pursuit::EndReason::None;
        }
        self.inner.pursuit_ended()
    }

    fn take_bust_ticket(&mut self) -> Option<Vec<(blackbox_pursuit::Infraction, u32)>> {
        if self.concealed() {
            return None;
        }
        self.inner.take_bust_ticket()
    }

    fn pay_bust(&mut self, marker: bool) -> bool {
        self.inner.pay_bust(marker)
    }

    fn take_career_request(&mut self) -> Option<crate::career::CareerRequest> {
        if self.concealed() {
            return None;
        }
        self.inner.take_career_request()
    }

    fn paused(&self) -> bool {
        self.paused.get() || self.inner.paused()
    }

    fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
        self.inner.set_tire_effects(smoke, marks);
        self.effects_dirty = true;
    }

    fn set_smoke_quality(&mut self, quality: crate::settings::SmokeQuality) {
        self.inner.set_smoke_quality(quality);
        self.effects_dirty = true;
    }

    fn set_detail(&mut self, detail: crate::settings::DetailSettings) {
        self.inner.set_detail(detail);
    }
    fn set_car_shading(&mut self, shading: crate::settings::CarShading) {
        self.inner.set_car_shading(shading);
        self.effects_dirty = true;
    }

    fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
        self.inner.set_vehicle_effects(sparks, trails);
        self.effects_dirty = true;
    }

    fn set_speed_trail_style(&mut self, style: crate::settings::SpeedTrailStyle) {
        self.inner.set_speed_trail_style(style);
    }

    fn set_spark_style(&mut self, style: crate::settings::SparkStyle) {
        self.inner.set_spark_style(style);
        self.effects_dirty = true;
    }

    fn set_exhaust_flames(&mut self, on: bool) {
        self.inner.set_exhaust_flames(on);
        self.effects_dirty = true;
    }

    fn note_sputters(&mut self, pops: u32) {
        self.inner.note_sputters(pops);
    }

    fn fullscreen(&mut self) -> Option<Fullscreen> {
        self.inner.fullscreen()
    }

    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        self.inner.take_clip()
    }

    fn take_traffic_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        let sounds = self.inner.take_traffic_sounds();
        if self.concealed() {
            return Vec::new();
        }
        sounds
    }

    fn rotor_frame(&mut self, dt: f32) -> Option<crate::audio::HeliFrame> {
        let frame = self.inner.rotor_frame(dt);
        if self.concealed() {
            return None;
        }
        frame
    }

    fn take_explosions(&mut self) -> u32 {
        self.inner.take_explosions()
    }

    fn finished(&self) -> bool {
        self.inner.finished()
    }

    /// The world's panels (the entry card, the race card, the end-of-pursuit card), unless the loading screen hides the
    /// world.
    fn ui(&mut self, ctx: &egui::Context) -> f32 {
        if self.concealed() {
            return 0.0;
        }
        self.inner.ui(ctx)
    }

    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        self.inner.commands()
    }

    fn command(
        &mut self,
        renderer: &mut dyn RenderBackend,
        name: &str,
        args: &[&str],
    ) -> Option<Result<String, String>> {
        self.inner.command(renderer, name, args)
    }
}
