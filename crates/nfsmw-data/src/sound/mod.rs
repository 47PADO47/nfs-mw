//! A car's engine sound set: which `.gin` loops and banks it plays and how they are mixed, read from the
//! AttribSys classes `pvehicle`, `engineaudio`, `shiftpattern`, `turbosfx` and `acceltrans` of
//! `GLOBAL/ATTRIBUTES.BIN`. Spec: `docs/specs/engine-sound.md` §2 and `docs/formats/audio.md`.
//!
//! [`car_sound`] takes the parsed database and returns plain structs; [`CarSound::files`] lists the
//! install-relative paths a car loads, and [`load_loop`] / [`EngineLoops::load`] read and decode the Ginsu
//! loops into `blackbox-ginsu` data.

mod accel;
mod banks;
mod car;
mod collision;
mod engine;
mod fields;
mod load;
mod shift;
mod stitch;
mod turbo;

pub use accel::AccelTransition;
pub use banks::GlobalBanks;
pub use car::{CarSound, SoundUpgrades, audio_engine_level, car_sound, upgrade_entry};
pub use collision::{CollisionSounds, EventKind, ImpactSound};
pub use engine::{DecelWindow, EngineGroup, EngineMix, EngineSound, MixLevels};
pub use load::{EngineLoops, load_loop};
pub use shift::{ShiftSound, ShiftStage, Wobble};
pub use stitch::{Stitch, StitchPiece, collision_stitches};
pub use turbo::TurboSound;

/// Install folder of the `.gin` loops, the engine banks and the sweetener banks.
pub const ENGINE_DIR: &str = "SOUND/ENGINE";
/// Install folder of the gear-shift banks.
pub const SHIFTING_DIR: &str = "SOUND/SHIFTING";
/// Install folder of the turbo and supercharger banks.
pub const TURBO_DIR: &str = "SOUND/TURBO";
/// Install folder of the nitrous banks.
pub const NOS_DIR: &str = "SOUND/NOS";
/// Install folder of the tire-skid banks.
pub const SKIDS_DIR: &str = "SOUND/SKIDS";

#[cfg(test)]
mod tests;
