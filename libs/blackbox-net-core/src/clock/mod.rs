//! The client's estimate of the server tick (sync-model.md section 5, ADR 0006 section 11).
//!
//! Three pieces: [`ClockSync`] keeps the last ping samples and estimates the server tick from the one with the lowest
//! round trip; [`LocalClock`] turns the caller's `now_ms` into a fractional tick at a rate factor; [`adjust`] decides,
//! from the error between the two, whether to slew the rate or to step the tick. Time only comes from `now_ms`.

mod local;
mod rule;
mod sync;
#[cfg(test)]
mod tests;

pub use local::LocalClock;
pub use rule::{Adjust, adjust};
pub use sync::ClockSync;

/// Error above which the local clock jumps instead of slewing (100 ms at 60 Hz).
pub const STEP_TICKS: f64 = 6.0;
/// Largest rate change while slewing.
pub const MAX_SLEW: f64 = 0.02;
