//! Tuning of the turbo, nitrous, skid and collision sounds.

/// The `turbosfx` fields. Spec: `docs/specs/engine-sound-effects.md` §4.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TurboTuning {
    /// Spool loop volume, 0 to 32767 (`Vol_Spool`).
    pub spool_volume: u32,
    /// Ticks of full throttle to charge the spool (`ChargeTime`).
    pub charge_time: f32,
    /// Charge lost per tick off the throttle (`Leak_Rate`).
    pub leak_rate: f32,
    /// Blow-off volumes (`Vol_Blowoff1`, `Vol_Blowoff2`), 0 to 32767; 0 means none.
    pub blowoff_volume: [u32; 2],
    /// How long the blow-off sample plays, seconds (the game fills it with the decoded length).
    pub blowoff_seconds: f32,
}

impl Default for TurboTuning {
    fn default() -> Self {
        Self { spool_volume: 0, charge_time: 20.0, leak_rate: 0.5, blowoff_volume: [0, 0], blowoff_seconds: 1.5 }
    }
}

/// Nitrous sound shaping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NitrousTuning {
    /// How much the loop's pitch rises with the boost interpolator (1 + `pitch_boost` at full boost).
    pub pitch_boost: f32,
}

impl Default for NitrousTuning {
    fn default() -> Self {
        Self { pitch_boost: 0.08 }
    }
}

/// Skid sound selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkidTuning {
    /// `Aud_Skid_Type` of the surface a blown tire rolls on (`simsurface/blown_tire`).
    pub blown_tire_surface: u8,
    /// The drift race type uses the drift skid banks.
    pub drift_race: bool,
}

/// What the game looked up for one collision event (the `audioimpact` collection of the car's link).
/// Spec: `docs/specs/engine-sound-effects.md` §8.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CollisionTuning {
    /// Number of samples in each `STITCH_LEVEL_0..3` list (0 = list empty).
    pub level_lengths: [u32; 4],
    /// `Volumes.Vol1..Vol4`, 0 to 32767.
    pub volumes: [u32; 4],
    /// `StreamSweetner[].Threshold`, in list order.
    pub stream_thresholds: Vec<u32>,
}
