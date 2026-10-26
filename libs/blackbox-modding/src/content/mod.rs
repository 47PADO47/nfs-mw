//! Content a mod declares in files rather than code (ADR 0005 section 5): a manifest per car in `cars/`, the `[music]`
//! table of `mod.toml` for the `music/` folder, a manifest per scenery pack in `scenery/`, and a manifest per interior in
//! `interiors/`, and the `[racers]` table of `mod.toml` for extra racer names.
//!
//! This module checks the schema only. Whether a section number is free, and whether
//! the mod's files exist, is for the game to decide, because only the game knows its car tables and its index.

mod car;
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
mod interior;
#[cfg(test)]
mod interior_tests;
mod music;
mod nitrous_effects;
mod nitrous_fx;
mod racers;
#[cfg(test)]
mod racers_tests;
mod scenery;
#[cfg(test)]
mod scenery_tests;
mod shared_sound;
mod stats;
#[cfg(test)]
mod tests;
mod wheels;

pub use car::{CarManifest, Role, parse as parse_car};
pub use customization::{Customization, Finish, Livery, MAX_PER_KIND, Paint, Part, RimPaint, Tint, Vinyl, VinylColor};
pub use damage_parts::{
    Active as ActivePart, DEFAULT_MASS_KG, DamagePart, MAX_DAMAGE_PARTS, MAX_STAGES, Zone as DamageZone,
};
pub use engine_curves::{
    Curve as EngineCurve, Input as EngineInput, LayerKind as EngineLayerKind, Pitch as EnginePitch,
};
pub use engine_sound::{
    CarEffects, EngineSound, Layer as EngineLayer, MAX_EFFECT_FILES, MAX_LAYERS as MAX_ENGINE_LAYERS,
};
pub use error::ContentError;
pub use interior::{Exit, InteriorManifest, Pose, Shape, Suspend, TimeOverride, parse as parse_interior};
pub use music::{MusicSection, parse as parse_music};
pub use nitrous_effects::{EffectOn, EffectProfile, MAX_EFFECTS};
pub use nitrous_fx::{
    AudioProfile, BurnFollower, BurnStep, FlameProfile, NITROUS_FOLDER, NitrousProfile, PROFILE_FILE, ScreenProfile,
    parse as parse_nitrous_profile, profile_path as nitrous_profile_path,
};
pub use racers::{RacersSection, parse as parse_racers};
pub use scenery::{Collision, MAX_COMPONENT, Orientation, Place, SceneryManifest, parse as parse_scenery};
pub use shared_sound::SharedSound;
pub use stats::{Drive, Stats};
pub use wheels::{Tyre, WheelLayout};

/// The folder inside a mod that holds one car manifest per car.
pub const CARS_FOLDER: &str = "cars";
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
