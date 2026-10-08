//! The `shiftpattern` and `acceltrans` fields. Spec: `docs/specs/engine-sound-effects.md` §1 and §2.

/// One RPM move of an up shift: `rpm` and `time_ms` scale a unit Bezier curve whose four control points
/// `(x, y)` are in `curve` (`x` is the fraction of the time, `y` the fraction of the RPM).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShiftStage {
    pub rpm: i32,
    pub time_ms: i32,
    pub curve: [[f32; 2]; 4],
}

/// A decaying sine: `amplitude * sin(2 pi t / period)` falling to 0 over `decay_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Wobble {
    pub amplitude: u32,
    pub period_ms: u32,
    pub decay_ms: u32,
}

/// One `shiftpattern` collection. A pattern with no `up_disengage_fall` stage or no `up_engage` plays no up
/// shift.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShiftTuning {
    /// Seconds after the shift starts that the gear clunk plays (`Up_Shift_Sound_Delay`, `Down_Shift_Sound_Delay`).
    pub up_sound_delay: f32,
    pub down_sound_delay: f32,
    /// Clunk volumes, 0 to 32767 (`Up_Vol_Shift`, `Down_Vol_Shift`).
    pub up_volume: u32,
    pub down_volume: u32,
    /// The extra engine volume right after an up shift engages (a fraction) and how long it takes to fade.
    pub up_engage_attack_volume: f32,
    pub up_engage_attack_ms: u32,
    /// Up shift: the drop while the clutch is out (one or two stages) and the engage.
    pub up_disengage_fall: Vec<ShiftStage>,
    pub up_engage: Option<ShiftStage>,
    /// Down shift: `(rpm, ms)` of the disengage fall, the engage rise and the engage fall, and the reattach
    /// scale (ms per RPM of difference).
    pub down_disengage_fall: (u32, u32),
    pub down_engage_rise: (u32, u32),
    pub down_engage_fall: (u32, u32),
    pub down_reattach_scale: f32,
    /// The wobble after an up shift: RPM and volume.
    pub lfo_rpm: Wobble,
    pub lfo_volume: Wobble,
    /// Audio upgrade level of the transmission, 0 to 3 (the shortened first-gear fall applies to 0 and 1).
    pub upgrade_level: u32,
}

/// The `acceltrans` collection of the engine's set. Times in milliseconds, RPM on the sound scale.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AccelTransition {
    /// Rev from idle: time to the peak and RPM added at the peak (`AccelFromIdle_PEAK_T`, `_PEAK_RPM`).
    pub peak_ms: u32,
    pub peak_rpm: u32,
    /// The engine volume pulse of the rev (`AccelFromIdle_PEAK_VOL`); carried, not applied (spec decision).
    pub peak_volume: f32,
    /// Time to settle (`AccelFromIdle_RESUME_T`).
    pub resume_ms: u32,
    /// Time to settle when the throttle is released during the transition (`AccelFromIdle_INTERUPT_T`).
    pub interrupt_ms: u32,
}
