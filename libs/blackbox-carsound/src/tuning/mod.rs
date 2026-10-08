//! Tuning values: plain structs the game fills from its data (AttribSys `engineaudio`, `shiftpattern`,
//! `acceltrans`, `turbosfx`). Names and units follow the data so the conversion is a field copy.

mod effects;
mod engine;
mod shift;

pub use effects::{CollisionTuning, NitrousTuning, SkidTuning, TurboTuning};
pub use engine::{DecelWindow, EngineMix, EngineMode, EngineTuning, MixLevels};
pub use shift::{AccelTransition, ShiftStage, ShiftTuning, Wobble};

/// Everything one car needs.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSoundTuning {
    pub engine: EngineTuning,
    pub shift: ShiftTuning,
    pub accel_transition: AccelTransition,
    /// `None` for a car without forced induction (`turbosfx/default`).
    pub turbo: Option<TurboTuning>,
    pub nitrous: NitrousTuning,
    pub skid: SkidTuning,
    /// The local player's car: it gets the RPM remap, the clutch model, the tachometer feedback, the
    /// brake-mash sound and measured landing intensities.
    pub is_local_player: bool,
    /// Seed of the random choices (compression bumps, blow-off samples).
    pub seed: u32,
}

impl Default for CarSoundTuning {
    fn default() -> Self {
        Self {
            engine: EngineTuning::default(),
            shift: ShiftTuning::default(),
            accel_transition: AccelTransition::default(),
            turbo: None,
            nitrous: NitrousTuning::default(),
            skid: SkidTuning::default(),
            is_local_player: true,
            seed: 1,
        }
    }
}
