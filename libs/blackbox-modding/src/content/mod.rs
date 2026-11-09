//! Content a mod declares in files rather than code (ADR 0005 section 5): a manifest per car in `cars/`, the `[music]`
//! table of `mod.toml` for the `music/` folder, a manifest per scenery pack in `scenery/`, and a manifest per interior in
//! `interiors/`, and the `[racers]` table of `mod.toml` for extra racer names.
//!
//! This module checks the schema only. Whether a section number is free, and whether
//! the mod's files exist, is for the game to decide, because only the game knows its car tables and its index.

mod bike;
mod bike_burnout;
mod bike_camera;
mod bike_check;
mod bike_check_parts;
mod bike_dash;
mod bike_help;
mod bike_lean;
mod bike_parts;
mod bike_reduction;
#[cfg(test)]
mod bike_ride_air;
#[cfg(test)]
mod bike_ride_burnout;
#[cfg(test)]
mod bike_ride_flick;
#[cfg(test)]
mod bike_ride_hazards;
#[cfg(test)]
mod bike_ride_radius;
#[cfg(test)]
mod bike_ride_real;
#[cfg(test)]
mod bike_ride_steer;
#[cfg(test)]
mod bike_ride_steer_arcade;
#[cfg(test)]
mod bike_ride_sweep;
#[cfg(test)]
mod bike_ride_turn_table;
#[cfg(test)]
mod bike_ride_wheelie_blip;
#[cfg(test)]
mod bike_ride_wheelie_script;
#[cfg(test)]
mod bike_ride_wheelie_steer;
#[cfg(test)]
mod bike_ride_wheelie_throttle;
mod bike_rider;
#[cfg(test)]
mod bike_tests;
mod bike_thermal;
mod bike_transmission;
#[cfg(test)]
mod bike_weight_sweep;
mod bike_wheelie;
mod car;
mod components;
mod cop;
#[cfg(test)]
mod cop_tests;
mod customization;
#[cfg(test)]
mod customization_tests;
mod damage_parts;
#[cfg(test)]
mod damage_parts_tests;
mod engine_curves;
mod engine_sound;
#[cfg(test)]
mod engine_sound_tests;
mod error;
mod grading;
mod grading_cube;
mod interior;
#[cfg(test)]
mod interior_tests;
mod lamp_flares;
mod manufacturer;
#[cfg(test)]
mod manufacturer_tests;
mod model_swap;
mod music;
mod nitrous_effects;
mod nitrous_fx;
mod overrun_sound;
mod plate;
mod racers;
#[cfg(test)]
mod racers_tests;
mod rain_splash;
mod replace;
mod scenery;
#[cfg(test)]
mod scenery_tests;
mod shared_sound;
mod skid_marks;
mod sky_variant;
mod spark_gate;
mod spark_variant;
mod stats;
#[cfg(test)]
mod tests;
mod tire_smoke;
mod traffic;
#[cfg(test)]
mod traffic_tests;
mod variant;
mod wet_road;
mod wheels;
mod wind_fx;
mod world;
mod world_patch;
mod xenon_variant;

pub use bike::{
    Bike, Brakes as BikeBrakes, Camera as BikeCamera, CameraChase as BikeCameraChase, CameraFront as BikeCameraFront,
    CameraHelmet as BikeCameraHelmet, Chassis as BikeChassis, Damper as BikeDamper, Dash as BikeDash,
    DashAlign as BikeDashAlign, DashChannel as BikeDashChannel, DashColors as BikeDashColors,
    DashField as BikeDashField, Engine as BikeEngine, MF_A, MF_B, MF_C, Rider as BikeRider, Steering as BikeSteering,
    Suspension as BikeSuspension, Transmission as BikeTransmission, Tyre as BikeTyre, Wheel as BikeWheel,
};
pub use car::{CarManifest, Kind as CarKind, Role, parse as parse_car};
pub use components::{
    COMPONENTS_FOLDER, Components, MAX_PART_MARKERS, MAX_PARTS as MAX_COMPONENT_PARTS,
    MAX_SLOTS as MAX_COMPONENT_SLOTS, PartBinding, PartMarker, PartRole, is_component_id, merge_catalog, parse_catalog,
};
pub use cop::{CopClass, CopWave, DEFAULT_CHANCE, DEFAULT_COUNT, MAX_CHANCE, MAX_COUNT};
pub use customization::{Customization, Finish, Livery, MAX_PER_KIND, Paint, Part, RimPaint, Tint, Vinyl, VinylColor};
pub use damage_parts::{
    Active as ActivePart, DEFAULT_MASS_KG, DamagePart, Drive as PartDrive, DriveInput, DriveOp, DriveTest,
    MAX_DAMAGE_PARTS, MAX_DRIVES, MAX_LOCK_DEGREES, MAX_STAGES, Steering as SteeredPart, Track as WingTrack,
    WING_SLOTS, Wing, Zone as DamageZone,
};
pub use engine_curves::{
    Curve as EngineCurve, Input as EngineInput, LayerKind as EngineLayerKind, Pitch as EnginePitch,
};
pub use engine_sound::{
    CarEffects, EngineSound, Layer as EngineLayer, MAX_EFFECT_FILES, MAX_LAYERS as MAX_ENGINE_LAYERS,
};
pub use error::ContentError;
pub use grading::{
    GRADING_FOLDER, GradingCube, GradingKey, GradingPair, GradingProfile, GradingSet, GradingWeather,
    MAX_CUBES as MAX_GRADING_CUBES, grading_profile_path, is_cube_path, parse as parse_grading_profile,
};
pub use grading_cube::{CUBE_EDGE, CubeData, decode_cube, encode_lut};
pub use interior::{Exit, InteriorManifest, Pose, Shape, Suspend, TimeOverride, parse as parse_interior};
pub use lamp_flares::{
    LampFlare, LampLit, LampSide, LightBar, MAX_BARS as MAX_LIGHT_BARS, MAX_FLARES as MAX_LAMP_FLARES,
    MAX_PATTERN as MAX_LIGHT_PATTERN, SirenColour, Sprite as LampSprite,
};
pub use manufacturer::{Manufacturer, brand_key, parse as parse_manufacturer};
pub use model_swap::{
    Fit as SwapFit, FitLimits, MAX_SWAPS, ModelSwapProfile, Swap, TREES_FOLDER, model_swap_profile_path,
    parse as parse_model_swap_profile,
};
pub use music::{MusicSection, parse as parse_music};
pub use nitrous_effects::{EffectOn, EffectProfile, MAX_EFFECTS};
pub use nitrous_fx::{
    AudioProfile, BurnFollower, BurnStep, FlameProfile, NITROUS_FOLDER, NitrousProfile, PROFILE_FILE, ScreenProfile,
    parse as parse_nitrous_profile, profile_path as nitrous_profile_path,
};
pub use overrun_sound::Overrun as CarOverrun;
pub use plate::{MAX_CELLS as MAX_PLATE_CELLS, MAX_SLOTS as MAX_PLATE_SLOTS, Plate};
pub use racers::{RacersSection, parse as parse_racers};
pub use rain_splash::{
    MAX_LAYERS as MAX_SPLASH_LAYERS, RAINSPLASH_FOLDER, RainSplashProfile, SplashKind, SplashLayer,
    parse as parse_rain_splash_profile, rain_splash_profile_path,
};
pub use replace::{
    Convert as TextureConvert, LEGACY_KIND, MAX_ENTRIES as MAX_REPLACE_ENTRIES, MAX_PRIORITY as MAX_REPLACE_PRIORITY,
    ModelEntry, REPLACE_FOLDER, ReplaceSet, StateRule as TextureState, Target as ReplaceTarget, TextureEntry,
    TextureSource, from_model_swap, parse as parse_replace_set,
};
pub use scenery::{Collision, MAX_COMPONENT, Orientation, Place, SceneryManifest, parse as parse_scenery};
pub use shared_sound::SharedSound;
pub use skid_marks::{
    MAX_SETS as MAX_MARK_SETS, MarkSet, MarkSurface, SKIDMARKS_FOLDER, SkidMarksProfile,
    parse as parse_skid_marks_profile, skid_marks_profile_path,
};
pub use sky_variant::{
    Dome as SkyDome, Moon as SkyMoon, Pose as SkyPose, SKY, SKY_FOLDER, SkyProfile, Weather as SkyWeather,
    parse as parse_sky_profile, sky_profile_path,
};
pub use spark_gate::{HitChannel, HitGate, ScrapeChannel, ScrapeGate};
pub use spark_variant::{
    MAX_SPARK_COUNT, MAX_SPARK_KEYS, MAX_SPARK_LAYERS, SPARKS_FOLDER, SparkKind, SparkLayer, SparksProfile,
    parse as parse_sparks_profile, sparks_profile_path,
};
pub use stats::{DriftStats, Drive, Stats};
pub use tire_smoke::{
    MAX_LAYERS as MAX_SMOKE_LAYERS, TIRESMOKE_FOLDER, TireSmokeProfile, parse as parse_tire_smoke_profile,
    tire_smoke_profile_path,
};
pub use traffic::{MAX_INSTANCES as MAX_TRAFFIC_INSTANCES, MAX_WEIGHT as MAX_TRAFFIC_WEIGHT, Traffic, TrafficClass};
pub use variant::{
    COLLISION_SPARKS, FORMAT, GRADING, NITROUS, RAIN_SPLASH, SKID_MARKS, TIRE_SMOKE, TREES, VariantKind, WET_ROAD,
    WIND, XENON_SPARKS,
};
pub use wet_road::{WETROAD_FOLDER, WetRoadProfile, parse as parse_wet_road_profile, wet_road_profile_path};
pub use wheels::{Tyre, WheelLayout};
pub use wind_fx::{
    MAX_LAYERS as MAX_WIND_LAYERS, WIND_FOLDER, WindLayer, WindProfile, parse as parse_wind_profile, wind_profile_path,
};
pub use world::{
    CITY_BASE, Environment as WorldEnvironment, MAX_GRID_SLOTS, Slot as WorldSlot, WORLDS_FOLDER, WorldManifest,
    parse as parse_world,
};
pub use world_patch::{
    Applied as PatchedWorld, PATCHES_FOLDER, PackInput, PackOrigin, PackPlan, PatchInput, WorldPatch,
    apply as apply_world_patches, order as order_world_patches, parse as parse_world_patch, plan_packs,
};
pub use xenon_variant::{
    MAX_XENON_SPARKS, XENON_FOLDER, XenonProfile, XenonSpark, parse as parse_xenon_profile, xenon_profile_path,
};

/// The folder inside a mod that holds one car manifest per car.
pub const CARS_FOLDER: &str = "cars";
/// The folder inside a mod that holds one manufacturer manifest per brand.
pub const MANUFACTURERS_FOLDER: &str = "manufacturers";
/// The folder inside a mod that holds its music.
pub const MUSIC_FOLDER: &str = "music";
/// The folder inside a mod that holds one scenery manifest per pack.
pub const SCENERY_FOLDER: &str = "scenery";
/// The folder inside a mod that holds one interior manifest per interior.
pub const INTERIORS_FOLDER: &str = "interiors";

/// Longest car id, in bytes.
const MAX_ID_BYTES: usize = 32;

/// A car id: `A-Z a-z 0-9 _ -`, 1 to 32 bytes.
fn is_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_ID_BYTES
        && text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
