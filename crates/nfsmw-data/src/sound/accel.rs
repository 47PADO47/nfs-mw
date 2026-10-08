//! The `acceltrans` class: the rev when the throttle is stabbed. Rules: `docs/specs/engine-sound-effects.md` §2.

use super::fields::Fields;

/// Times in milliseconds, RPM on the sound scale (1000 to 10000).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AccelTransition {
    /// Rev from idle: time to the peak, RPM added at the peak, the engine volume pulse and the time to settle.
    pub peak_ms: u32,
    pub peak_rpm: u32,
    pub peak_volume: f32,
    pub resume_ms: u32,
    /// Time to settle when the throttle is released during the transition.
    pub interrupt_ms: u32,
}

impl AccelTransition {
    pub(super) fn read(c: Fields<'_>) -> Self {
        Self {
            peak_ms: c.u32("AccelFromIdle_PEAK_T"),
            peak_rpm: c.u32("AccelFromIdle_PEAK_RPM"),
            peak_volume: c.f32("AccelFromIdle_PEAK_VOL"),
            resume_ms: c.u32("AccelFromIdle_RESUME_T"),
            interrupt_ms: c.u32("AccelFromIdle_INTERUPT_T"),
        }
    }
}
