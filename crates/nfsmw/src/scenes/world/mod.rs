//! The streamed city: a free-fly camera, or a car to drive with a chase camera.
mod ai;
mod breaker_icons;
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
#[cfg(test)]
mod determinism_tests;
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
pub(crate) mod feed;
mod finish_flares;
mod frame;
mod frame_systems;
mod garage;
pub(crate) use garage::catalog as garage_catalog;
pub(crate) use garage::native_my_car_handle;
mod gps;
mod ground;
mod heli_wash;
mod hud;
mod hud_marks;
mod hud_context;
mod hud_nav;
mod hud_pursuit;
#[cfg(test)]
mod installed_turn;
mod keys;
mod lighting;
mod materials;
mod mod_hooks;
mod moment;
mod moment_world;
pub mod net;
#[cfg(feature = "multiplayer")]
pub mod net_client;
mod nis;
mod photo_shot;
pub mod players;
mod tail_light_trails;
#[cfg(test)]
pub(crate) use ai::{ArrestFixture, handbrake, open_arrest_fixture};
use car_loader::load_car;
mod focus;
mod focus_feed;
mod object_geometry;
mod open;
pub use open::WorldLoad;
mod damage_effects;
mod environment_lights;
mod impact_effects;
#[cfg(test)]
mod performance_tests;
mod prop_effects;
mod props;
mod race;
#[cfg(test)]
mod race_course_run_tests;
#[cfg(test)]
mod race_restart_tests;
mod readiness;
mod rear_view;
mod reflections;
mod reflections_visibility;
mod residency;
mod resident;
mod road;
mod rotor;
mod scene;
mod scene_effects;
mod scene_hud;
mod scene_space;
mod scenery_groups;
mod session_rules;
use session_rules::ViewSource;
mod shadow_feed;
mod simulation;
mod sky;
mod sound;
mod space;
mod state_hash;
use state_hash::StateHasher;
mod speech_look;
mod speed_trails;
mod split;
mod start;
#[cfg(test)]
mod step_bench_tests;
mod story_movie;
mod sun;
mod teardown;
mod ui_panels;
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
use drive::{DriveScript, MarkerMeshes};
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
    /// Car folder (a unique prefix is enough), with a preset ride after a colon (`BMWM3GTRE46:M3GTRCAREERSTART`).
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
    garage_backdrop: Option<crate::frontend::GarageBackdrop>,
    garage_exit: Option<nfsmw_data::gameplay::Placement>,
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
    reflection_scratch: reflections_visibility::Scratch,
    reflection_last: Option<reflections::CaptureStamp>,
    wet_road: wet_reflections::WetRoad,
    /// Seconds since the scene started, for texture animations.
    clock: f32,
    environment_lights: environment_lights::EnvironmentLights,
    sun: sun::Sun,
    /// The day-night clock: the hour of day, the sun for it and the lights.
    day: day::DayClock,
    /// The aspect ratio the physics camera uses (set by each frame).
    physics_aspect: f32,
    /// The aspect ratio of the last frame drawn (`frame.rs`), for the HUD's camera (`camera_choice.rs`).
    drawn_aspect: f32,
    /// The HUD's events of the frame (`feed/hud.rs`).
    hud_feed: feed::HudFeed,
    /// The one fixed timeline of the player, the traffic and the object contacts (see `update_simulation`).
    physics_clock: drive::FixedClock,
    speedbreaker_clock: drive::SpeedbreakerClock,
    view: View,
    pending_car: Option<open::PendingCar>,
    /// The players' cars; the lead (player 1) is the only one outside co-op (`players/`, ADR 0010).
    players: players::Players,
    /// The world's car render keys, shared with the traffic models it uploads (keys.rs).
    car_keys: std::rc::Rc<keys::CarKeys>,
    /// The rules a multiplayer session changes; the defaults are single player's (`session_rules.rs`).
    rules: session_rules::SessionRules,
    /// Computer-driven cars, when the track has a road network.
    traffic: Option<ai::TrafficWorld>,
    /// Minimap blips that do not come from the traffic: the race starts and the interiors, made once at open
    /// (`hud::static_blips`). Nothing clears this list, so a later addition stays until it is removed here.
    pub map_blips: Vec<crate::hud::MapBlip>,
    /// The races, their triggers and their opponents, when the install's gameplay data could be read.
    /// The icons of the race starts with the race each marks; which show is decided each frame (`hud.rs`).
    race_starts: Vec<(String, crate::hud::MapBlip)>,
    gameplay: Option<std::sync::Arc<nfsmw_data::gameplay::GameplayData>>,
    /// The race being run, if any (`race/`).
    race: Option<race::RaceSession>,
    /// The fade to black of the prologue's breakdown (`race/chase_fade.rs`).
    screen_fade: race::chase_fade::ScreenFade,
    /// A race to start as soon as the car is there (a quick race asked for it, `queue_race`).
    queued_race: Option<(String, race::RaceOptions)>,
    /// The race, options and course `queued_race` runs again when the post-race screen's Restart queued it: it starts as
    /// a restart, past its intro (`race/restart.rs`).
    queued_again: Option<race::RestartKit>,
    /// The two players of the split-screen race `queued_race` starts (`race/split.rs`); `None` for a one-player race.
    queued_split: Option<SplitSetup>,
    /// The network seats racing as people beside the lead, in grid order: racer `n + 1` is `race_people[n]`
    /// (`players/race.rs`). Empty outside a race with remote players.
    race_people: Vec<players::PlayerId>,
    /// The next race stages on the grid and waits for [`WorldScene::release_staged_race`] instead of counting down by
    /// itself (`race/hold.rs`). A server sets it before it starts the session's race.
    race_hold: bool,
    /// Nobody drives the lead's car in the next race (a dedicated server's idle car): it stands on the grid as a
    /// computer racer, so the race does not wait for it (`race/hold.rs`).
    race_lead_idle: bool,
    /// The split-screen match being played: the second player's actions and camera (`split/`).
    split: Option<split::SplitState>,
    /// The bottom half's draw list and cut, for a split-screen race or co-op (`split/view.rs`).
    second_view: split::SecondView,
    /// Each co-op guest's actions this frame; the lead reads the frame's own input (`players/inputs.rs`).
    inputs: players::inputs::Inputs,
    /// The co-op session in play (`players/coop.rs`); `None` outside co-op.
    coop: Option<players::coop::CoopSession>,
    /// A co-op session the front end asked for, started on the next frame.
    queued_coop: Option<players::coop::CoopSetup>,
    /// Co-op ends on the next frame, when the front end's End Co-op asked for it (`players/coop.rs`).
    end_coop_queued: bool,
    /// What the mods set about the races and what they can read of them (`race/mods.rs`).
    race_mods: race::mods::RaceMods,
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
    /// How the cops describe the car being driven (`speech_look.rs`).
    speech_look: std::cell::RefCell<speech_look::Cache>,
    /// Car physics data, road grips and the gameplay database.
    physics: PhysicsData,
    tire_effects: [bool; 2],
    vehicle_effects: [bool; 2],
    speed_trail_style: crate::settings::SpeedTrailStyle,
    exhaust_flames: bool,
    spark_style: crate::settings::SparkStyle,
    smoke_quality: crate::settings::SmokeQuality,
    rain: crate::weather::rain::Rain,
    /// The wind each player view hands to the scenery sway (`docs/specs/scenery-sway.md`).
    wind: crate::weather::wind::Wind,
    /// The petrol station's explosion effect, started by the scenery physics.
    explosions: explosion_fx::ExplosionFx,
    /// The helicopter's rotor wash on the ground (`heli_wash.rs`).
    heli_wash: heli_wash::HeliWash,
    /// The flares at the running race's finish line (`finish_flares.rs`).
    finish_flares: finish_flares::FinishFlares,
    object_fx: prop_effects::ObjectFx,
    car_damage_fx: damage_effects::DamageEffects,
    impact_fx: impact_effects::ImpactEffects,
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
    waiting_since: Option<drive_input::RoadWait>,
    /// The gameplay camera the setting picked; only used while driving.
    camera_mode: CameraMode,
    /// The cop health bars: their state, the bars of the last step and their texture (`effects/cop_health.rs`).
    cop_health: effects::cop_health::CopHealthBars,
    /// The cinematic moment of a pursuit breaker, when one plays (`moment.rs`).
    moment: moment::Moment,
    /// The NIS cutscene player: the camera of a cutscene scene while it plays (`nis/`).
    nis: nis::NisPlayer,
    /// The movie the world plays over its view, and the tutorials seen (`story_movie.rs`).
    story_movie: story_movie::StoryMovie,
    /// The camera of the win photo while the post-race screen shows it (`photo_shot.rs`).
    photo_shot: Option<photo_shot::PhotoShot>,
    /// The pursuit breakers' minimap icons (`breaker_icons.rs`).
    breaker_icons: breaker_icons::BreakerIcons,
    /// The multiplayer client this world runs for, if any (`net_client/`): the server owns its rules.
    #[cfg(feature = "multiplayer")]
    net_client: Option<net_client::ClientMode>,
}

/// Leaving the free camera more than this far (metres) from the car brings the car to the camera.
const FLOWN_AWAY: f32 = 25.0;

const CLEAR: [f32; 3] = [0.55, 0.63, 0.72];

/// The car `drive` picks when none was named.
pub use race::{MAX_RACERS, RaceOptions, SplitSetup};

pub const DEFAULT_CAR: &str = "BMWM3GTR";
