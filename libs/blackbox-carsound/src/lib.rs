//! Car sound controllers of EA Black Box games: per-frame car telemetry in, engine mix and sound commands out.
//! Specs: `docs/specs/engine-sound.md`, `engine-sound-ginsu.md` and `engine-sound-effects.md`.
//!
//! Pure and deterministic: no audio output, no file access, no game knowledge. The tuning values come in
//! as plain structs the game fills from its data; the sounds go out as plain values (volumes, pitches and
//! [`SoundCommand`]s) that the game maps to its banks and to the Ginsu synthesiser (`blackbox-ginsu`).

mod avg;
mod effects;
mod engine;
mod input;
mod interp;
mod math;
mod rng;
mod tuning;

#[cfg(test)]
mod tests;

pub use effects::{
    EffectSignals, EffectsMixer, ImpactPlay, ImpactRequest, Landing, LoopId, ScrapeKind, SoundCommand, SoundRef,
    SweetenerKind,
};
pub use engine::{
    EngineEvents, EngineMixer, EngineOutput, LoopDrive, MAX_TICKS_PER_UPDATE, ShiftDirection, ShiftState, TICK_HZ,
    TICK_SECONDS,
};
pub use input::{CarInput, GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE, NO_ROAD_NOISE, WheelInput};
pub use tuning::{
    AccelTransition, CarSoundTuning, CollisionTuning, DecelWindow, EngineMix, EngineMode, EngineTuning, MixLevels,
    NitrousTuning, ShiftStage, ShiftTuning, SkidTuning, TurboTuning, Wobble,
};
