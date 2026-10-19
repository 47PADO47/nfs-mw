//! The streamed city: a free-fly camera, or a car to drive with a chase camera.
mod ai;
mod breaker_visuals;
mod camera_choice;
mod camera_cycle;
mod camera_original;
mod car_light_flares;
mod car_lights;
mod car_loader;
mod career;
mod cargo;
mod commands;
mod day;
mod detail;
mod drive;
mod drive_input;
mod effects;
mod entry;
mod entry_zones;
#[cfg(test)]
mod entry_zones_tests;
mod environment;
mod events;
mod exhaust;
mod explosion_fx;
mod frame;
mod garage;
mod gps;
mod ground;
mod hud;
mod keys;
mod lighting;
mod materials;
mod mod_hooks;
mod tail_light_trails;
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
mod scene;
mod scenery_groups;
mod shadow_feed;
mod sign_lamps;
mod simulation;
mod sky;
mod sound;
mod space;
mod speed_trails;
mod start;
mod sun;
mod teardown;
mod update;
mod vehicle_effects;
mod visibility;
mod wet_reflections;
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

pub struct WorldScene {
    detail: detail::DetailState,
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
    shadow_feed: shadow_feed::ShadowFeed,
    original_shadows: bool,
    original_sky: bool,
    colour_treatment: Option<blackbox_gfx::ColourTreatment>,
    time_of_day: Option<nfsmw_data::time_of_day::Presets>,
    atmosphere: lighting::Clouds,
    /// The seven original gameplay views, from the install's `camerainfo` data.
    camera_environment: camera_original::Environment,
    original_camera: crate::viewer::camera::original::Timeline,
    camera_ground: blackbox_collision::ground_reference::GroundReferenceCache,
    original_player_step: Option<u32>,
    gain_map: Option<blackbox_gfx::RgbaImage>,
    breaker_visuals: breaker_visuals::Presentation,
    reflection_faces: [Vec<Instance>; 6],
    reflection_last: Option<reflections::CaptureStamp>,
    wet_road: wet_reflections::WetRoad,
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
    pending_car: Option<open::PendingCar>,
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
    /// The career being played, if any, and what it asks of the front end (`career/`).
    career: career::WorldCareer,
    /// The scenery groups switched on (`scenery_groups.rs`); the career opens and closes the region gates through it.
    scenery_groups: scenery_groups::SceneryGroups,
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
    /// The GPS route to a point on the road, and the arrow that shows it.
    guide: gps::Guide,
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
