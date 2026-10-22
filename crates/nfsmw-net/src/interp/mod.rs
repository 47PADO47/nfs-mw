//! Remote entities drawn from snapshots (sync-model.md section 3, MP-81): other players, traffic and police.
//!
//! - [`Interpolator`]: one snapshot buffer per entity (at most [`InterpConfig::buffer_len`] records, at most
//!   [`InterpConfig::max_entities`] entities), the display time and the drawn pose of each entity.
//! - Display time `R = S_est - transit - delay`, in server ticks: `S_est` from [`crate::clock::NetClock`], `transit`
//!   the smallest snapshot age seen lately (how old a snapshot is when it arrives), and `delay` two snapshot intervals
//!   plus an [`AdaptiveDelay`] allowance of up to one more interval when arrival jitter (a running 95th percentile of
//!   lateness) exceeds half an interval. The allowance falls again one tick per second; every change is slewed over
//!   a second.
//! - Between two records: position by cubic Hermite with the records' velocities as tangents, orientation by slerp
//!   along the shorter arc, velocities linearly, flags and the kind's discrete fields from the first record until
//!   halfway. Two records further apart than the car could have driven are a teleport: no smoothing, the first record
//!   is held until the second's tick.
//! - Past the newest record: extrapolation by velocity and angular velocity for up to 250 ms, the velocity scaled to
//!   zero linearly from 100 ms on; then the entity is held and marked [`DrawMode::Lagging`], and traffic and police
//!   fade out after a further second. When records arrive again the drawn pose blends back over 100 ms.
//! - An entity appears at `R >= ` its first record's tick (traffic fades in over 0.5 s) and goes at `R >=` its removal
//!   tick.

mod curve;
mod delay;
mod interpolator;
mod track;

#[cfg(test)]
mod tests;

pub use delay::AdaptiveDelay;
pub use interpolator::Interpolator;

use nfsmw_sim::EntityView;

use crate::reconcile::Tuning;

/// The interpolation settings, in milliseconds and server ticks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterpConfig {
    /// Physics ticks between two snapshots (3 at 20 Hz).
    pub snapshot_interval_ticks: f64,
    /// The base delay, in snapshot intervals.
    pub delay_intervals: f64,
    /// The largest jitter allowance, in snapshot intervals.
    pub max_allowance_intervals: f64,
    /// Extrapolation at full velocity up to here, milliseconds.
    pub extrapolate_full_ms: f64,
    /// Extrapolation stops here, milliseconds.
    pub extrapolate_limit_ms: f64,
    /// The blend back after extrapolation, milliseconds.
    pub blend_ms: f64,
    /// Traffic fades in over this, milliseconds.
    pub fade_in_ms: f64,
    /// Lagging traffic and police fade out after this, milliseconds.
    pub fade_out_after_ms: f64,
    /// Two records further apart than the distance the faster of them covers in the span, times 1.5, plus this, are a
    /// teleport, metres.
    pub teleport_margin_m: f32,
    /// Records kept per entity.
    pub buffer_len: usize,
    /// Entities tracked at once; records of further entities are refused and counted.
    pub max_entities: usize,
    /// An entity without a record for this long (past its newest) is forgotten, milliseconds.
    pub forget_after_ms: f64,
}

impl Default for InterpConfig {
    fn default() -> Self {
        Self::from_tuning(&Tuning::default())
    }
}

impl InterpConfig {
    /// The settings of `tuning` (sync-model.md section 9) and the fixed values of section 3.
    pub fn from_tuning(tuning: &Tuning) -> Self {
        Self {
            snapshot_interval_ticks: f64::from(tuning.snapshot_interval_ticks()),
            delay_intervals: f64::from(tuning.interp_delay_intervals),
            max_allowance_intervals: f64::from(tuning.delay_allowance_intervals),
            extrapolate_full_ms: f64::from(tuning.extrapolation_decay_ms),
            extrapolate_limit_ms: f64::from(tuning.extrapolation_limit_ms),
            blend_ms: 100.0,
            fade_in_ms: 500.0,
            fade_out_after_ms: 1000.0,
            teleport_margin_m: 5.0,
            buffer_len: 16,
            max_entities: 1024,
            forget_after_ms: 5000.0,
        }
    }
}

/// How an entity's drawn pose was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrawMode {
    /// Between two records.
    Interpolated,
    /// Past the newest record, within the extrapolation limit.
    Extrapolated,
    /// Past the extrapolation limit: held, marked lagging in the player list.
    Lagging,
}

/// An entity as it is drawn now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drawn {
    /// The record at display time: pose, motion, flags.
    pub view: EntityView,
    pub mode: DrawMode,
    /// 1 fully drawn, 0 hidden: the traffic fade-in and the fade-out of lagging traffic and police.
    pub fade: f32,
    /// Milliseconds past the newest record, 0 while interpolating.
    pub late_ms: f64,
}

/// Milliseconds per server tick.
const TICK_MS: f64 = 1000.0 / nfsmw_sim::TICK_HZ as f64;
