//! The slew-or-step rule: how the local tick clock closes an error against the server estimate.

use super::{MAX_SLEW, STEP_TICKS};

/// What to do with the local tick clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Adjust {
    /// Set the rate factor. The tick keeps running and never jumps.
    Slew(f64),
    /// Add this many ticks at once. Negative moves the tick back.
    Step(f64),
}

/// The rule for an error of `error_ticks` (positive means the local clock is behind the estimate).
///
/// Above [`STEP_TICKS`] in magnitude the clock steps by the whole error. Otherwise it slews with the rate factor
/// `1 + clamp(error * 0.01, -MAX_SLEW, MAX_SLEW)`. A NaN error slews at rate 1, so it cannot poison the tick.
pub fn adjust(error_ticks: f64) -> Adjust {
    if error_ticks.is_nan() {
        return Adjust::Slew(1.0);
    }
    if error_ticks.abs() > STEP_TICKS {
        return Adjust::Step(error_ticks);
    }
    Adjust::Slew(1.0 + (error_ticks * 0.01).clamp(-MAX_SLEW, MAX_SLEW))
}
