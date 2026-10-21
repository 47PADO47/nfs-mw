//! The client's estimate of the server tick (sync-model.md section 5, ADR 0006 section 11).
//!
//! CONTRACT (to implement):
//!
//! - `pub struct ClockSync` with `new(tick_hz: u32) -> Self`.
//! - `add_sample(&mut self, sent_ms: u64, server_tick: f64, received_ms: u64)`: one ping exchange. `server_tick` is the
//!   server's tick (with fraction) when it answered. RTT = `received_ms - sent_ms`; samples with `received_ms < sent_ms`
//!   are ignored. Keeps the last 10 samples; the estimate uses the one with the lowest RTT, assuming the server answered
//!   at the middle of the round trip.
//! - `rtt_ms(&self) -> Option<f64>` (lowest of the kept samples), `server_tick_at(&self, now_ms: u64) -> Option<f64>`.
//! - `pub enum Adjust { Slew(f64), Step(f64) }` and `pub fn adjust(error_ticks: f64) -> Adjust`: the local tick clock is
//!   `error_ticks` behind the estimate (negative = ahead). Above [`STEP_TICKS`] in magnitude it steps by the whole error;
//!   otherwise it slews: a rate factor `1 + clamp(error_ticks * 0.01, -MAX_SLEW, MAX_SLEW)`.
//! - `pub struct LocalClock` that turns `now_ms` into a fractional tick using a rate factor: `new(tick_hz)`,
//!   `advance(&mut self, now_ms: u64)`, `tick(&self) -> f64`, `apply(&mut self, adjust: Adjust)`. Monotonic: a slew never
//!   moves it backward; only a `Step` with a negative value can.
//! - Tests: a simulated server clock with a fixed offset and RTT jitter converges within 0.5 tick in 10 samples; an
//!   outlier with a long RTT does not move the estimate; slewing a 3-tick error takes about 300 ticks; a 20-tick error
//!   steps.

/// Error above which the local clock jumps instead of slewing (100 ms at 60 Hz).
pub const STEP_TICKS: f64 = 6.0;
/// Largest rate change while slewing.
pub const MAX_SLEW: f64 = 0.02;
