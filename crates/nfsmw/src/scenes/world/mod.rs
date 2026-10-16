//! The streamed city: a free-fly camera, or a car to drive with a chase camera.

mod ai;
mod breaker_visuals;
mod camera_choice;
mod camera_cycle;
mod camera_original;
mod car_light_flares;
mod car_lights;
mod car_loader;
mod cargo;
mod commands;
mod damage_marks;
mod day;
mod drive;
mod drive_input;
mod effects;
mod entry;
mod entry_zones;
#[cfg(test)]
mod entry_zones_tests;
mod events;
mod exhaust;
mod explosion_fx;
mod frame;
mod garage;
mod ground;
mod hud;
mod keys;
mod lighting;
use car_loader::load_car;
mod focus;
mod focus_feed;
mod object_geometry;
mod open;
#[cfg(test)]
mod performance_tests;
mod props;
mod readiness;
mod rear_view;
mod reflections;
mod residency;
mod resident;
mod road;
mod rotor;
mod shadow_feed;
mod sign_lamps;
mod simulation;
mod sound;
mod space;
mod speed_trails;
mod start;
mod sun;
mod update;
mod vehicle_effects;
mod visibility;
mod zone;

use anyhow::{Context, Result};
use blackbox_gfx::{FrameParams, Instance, RenderBackend};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::physics::PhysicsData;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, Streamer, WorldIndex, load_global_textures};

use crate::input::ActionState;
use crate::settings::CameraMode;
use crate::viewer::Scene;
use crate::viewer::camera::{CameraCut, FlyCamera};
use drive::{Drive, DriveScript, MarkerMeshes};
use residency::Residency;

pub struct Options {
    pub at: Option<[f32; 2]>,
    pub height: f32,
    pub heading: f32,
    pub pitch: f32,
    /// Where the fog is complete, in metres.
    pub fog_distance: f32,
    pub wait_for_load: bool,
    /// Drive a car from the start.
    pub drive: Option<DriveOptions>,
}

pub struct DriveOptions {
    /// Car folder (a unique prefix is enough).
    pub car: String,
    /// A scripted driver (`--drive-script`).
    pub script: Option<String>,
}

/// Which camera is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    /// The free-fly camera.
    Fly,
    /// The chase camera behind the car.
    Chase,
}

/// A car waiting for the renderer to exist.
struct PendingCar {
    name: String,
    model: CarModel,
    script: Option<DriveScript>,
}

pub struct WorldScene {
    garage: crate::garage::Model,
    garage_parts: Option<nfsmw_data::car::customization::Inventory>,
    garage_open: bool,
    audio_environment: Option<crate::audio::environment_context::EnvironmentMap>,
    common_sounds: Vec<crate::audio::CommonEvent>,
    sound_speedbreaker_active: bool,
    dir: GameDir,
    track: String,
    camera: FlyCamera,
    residency: Residency,
    /// Start position, for the ground estimate.
    start: [f32; 2],
    /// Height above the ground to place the camera at once the start area has loaded.
    start_height: f32,
    grounded: bool,
    wait_for_load: bool,
    fog_distance: f32,
    visible: Vec<Instance>,
    /// What the rear-view mirror sees: the same world, without the player's car.
    rear_visible: Vec<Instance>,
    shadow_casters: Vec<Instance>,
    original_shadows: bool,
    colour_treatment: Option<blackbox_gfx::ColourTreatment>,
    time_of_day: Option<nfsmw_data::time_of_day::Presets>,
    /// The seven original gameplay views, from the install's `camerainfo` data.
    camera_environment: camera_original::Environment,
    original_camera: crate::viewer::camera::original::Timeline,
    camera_ground: blackbox_collision::ground_reference::GroundReferenceCache,
    original_player_step: Option<u32>,
    gain_map: Option<blackbox_gfx::RgbaImage>,
    breaker_visuals: breaker_visuals::Presentation,
    reflection_faces: [Vec<Instance>; 6],
    reflection_last: Option<reflections::CaptureStamp>,
    /// Seconds since the scene started, for texture animations.
    clock: f32,
    sun: sun::Sun,
    /// The day-night clock: the hour of day, the sun for it and the lights.
    day: day::DayClock,
    /// The aspect ratio the physics camera uses (set by each frame).
    physics_aspect: f32,
    /// The one fixed timeline of the player, the traffic and the object contacts (see `update_simulation`).
    physics_clock: drive::FixedClock,
    speedbreaker_clock: drive::SpeedbreakerClock,
    view: View,
    pending_car: Option<PendingCar>,
    drive: Option<Drive>,
    /// Computer-driven cars, when the track has a road network.
    traffic: Option<ai::TrafficWorld>,
    /// The police helicopter's position at the last rotor frame, for its velocity (`rotor_frame`).
    rotor_previous: Option<Vec3>,
    /// Minimap blips that do not come from the traffic: the race starts and the interiors, made once at open
    /// (`hud::static_blips`). Nothing clears this list, so a later addition stays until it is removed here.
    pub map_blips: Vec<crate::hud::MapBlip>,
    /// Minimap blips the mods placed (`bb.map`), set each frame by the mods' systems and merged after `map_blips`.
    pub mod_blips: Vec<crate::hud::MapBlip>,
    /// The entry zones (auto shops, safe houses, race starts) and the interact card (`entry.rs`).
    entry: entry::Entry,
    /// The `traffic`, `cop_share` settings as last told, and a count typed in the console that replaces them.
    traffic_setting: crate::settings::TrafficSettings,
    lamp_meshes: Option<ai::LampMeshes>,
    traffic_manual: Option<usize>,
    /// The car last driven, for `drive` without a name.
    last_car: String,
    /// Car physics data, road grips and the gameplay database.
    physics: PhysicsData,
    tire_effects: [bool; 2],
    vehicle_effects: [bool; 2],
    speed_trail_style: crate::settings::SpeedTrailStyle,
    exhaust_flames: bool,
    spark_style: crate::settings::SparkStyle,
    smoke_quality: crate::settings::SmokeQuality,
    rain: crate::weather::rain::Rain,
    /// The petrol station's explosion effect, started by the scenery physics.
    explosions: explosion_fx::ExplosionFx,
    /// The physics clock multiplier (Extra Options `GameSpeed`).
    tuning: crate::settings::WorldTuning,
    car_shading: crate::settings::CarShading,
    shadow_quality: blackbox_gfx::ShadowQuality,
    /// `debug collisions` is on: the car's contact points are drawn.
    markers_on: bool,
    /// The meshes of the contact markers, uploaded the first time they are asked for.
    marker_meshes: Option<MarkerMeshes>,
    /// The free camera jumped (a new scene, the camera toggle, `goto`, the start settling on the ground).
    cut: CameraCut,
    /// When the car started waiting for a road (no physics while it waits).
    waiting_since: Option<std::time::Instant>,
    /// The gameplay camera the setting picked; only used while driving.
    camera_mode: CameraMode,
    /// The cop health bars: their state, the bars of the last step and their texture (`effects/cop_health.rs`).
    cop_health: effects::cop_health::CopHealthBars,
}

/// Leaving the free camera more than this far (metres) from the car brings the car to the camera.
const FLOWN_AWAY: f32 = 25.0;

const CLEAR: [f32; 3] = [0.55, 0.63, 0.72];

/// The car `drive` picks when none was named.
pub const DEFAULT_CAR: &str = "BMWM3GTR";

impl Scene for WorldScene {
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
        if let Some(PendingCar { name, model, script }) = self.pending_car.take() {
            self.start_driving(renderer, name, model, script)?;
        }
        Ok(())
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.update_frame(renderer, input, dt);
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
        Some(self.day.environment())
    }

    fn set_environment(&mut self, value: crate::viewer::Environment) {
        self.configure_day(value);
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
            "{}zone {zone}, tiles {resident} (+{loading} loading), {} drawn, {} props ({} knocked over), at ({:.0}, {:.0}, {:.0}){markers}",
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

    fn take_bust_ticket(&mut self) -> Option<Vec<(blackbox_pursuit::Infraction, u32)>> {
        self.traffic.as_mut()?.take_bust_ticket()
    }

    fn pay_bust(&mut self, marker: bool) -> bool {
        self.traffic.as_mut().is_some_and(|traffic| traffic.pay_bust(marker))
    }

    fn take_traffic_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        self.traffic.as_mut().map(|t| t.take_sounds()).unwrap_or_default()
    }

    /// The helicopter's rotor for the audio this frame (see `rotor.rs`).
    fn rotor_frame(&mut self, dt: f32) -> Option<crate::audio::HeliFrame> {
        self.rotor_frame_now(dt)
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
