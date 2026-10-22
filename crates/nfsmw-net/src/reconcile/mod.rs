//! Reconciliation of the owner's predicted car with the server (sync-model.md sections 2.3, 2.4 and 2.6, MP-80).
//!
//! Every snapshot carries, in its own-car block, the server's reduced state of the car at the last input tick it
//! applied (`ack_input`). [`Reconciler::apply`] compares it with the prediction stored for that tick:
//!
//! 1. Older than a block already applied: [`Outcome::Stale`], nothing is touched.
//! 2. Every error term under its threshold ([`Tuning`]: 2 cm, 0.2°, 0.05 m/s): [`Outcome::Confirmed`]. The prediction was
//!    right; the confirmed ticks leave the ring.
//! 3. Otherwise rewind to the server's state, replay the unacknowledged inputs ([`crate::predict::Predictor`]) and
//!    measure the correction at the newest tick. Up to [`Tuning::snap_distance_m`] it is [`Outcome::Corrected`]: the
//!    drawn pose keeps still and the [`VisualOffset`] hides the jump, decaying with a 50 ms half-life. Beyond it, or
//!    when the server says `reset` or `teleport`, it is [`Outcome::Snapped`]: the offset is cleared and the drawn pose
//!    jumps (the camera should be told).
//! 4. `freeze` holds the inputs neutral until a block without it arrives; `contact` reconciles normally.
//!
//! [`CorrectionStats`] keeps the correction distances (median, 95th percentile) and the corrections and snaps per
//! minute for the network overlay.

mod offset;
mod reconciler;
mod stats;
mod tuning;

#[cfg(test)]
mod tests;

pub use offset::VisualOffset;
pub use reconciler::Reconciler;
pub use stats::CorrectionStats;
pub use tuning::Tuning;

use nfsmw_sim::Tick;

use crate::geom;
use crate::predict::CarState;
use crate::schema::{CorrectionCause, OwnBlock};

/// The server's word on the owner's car: its state after applying the input of `tick`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuthState {
    /// `ack_input`: the last input tick the server applied.
    pub tick: Tick,
    pub state: CarState,
    pub cause: CorrectionCause,
}

impl AuthState {
    /// The authoritative state of a snapshot's own-car block, acknowledged at `ack_input`.
    pub fn from_own_block(ack_input: Tick, block: &OwnBlock) -> Self {
        Self { tick: ack_input, state: CarState::from_own_block(block), cause: block.cause }
    }
}

/// Why the drawn pose jumped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapReason {
    /// The correction was longer than the snap distance.
    Distance,
    /// The server reset the car.
    Reset,
    /// The server teleported the car (a spawn, a race start).
    Teleport,
    /// The server's tick was not predicted (past the newest predicted tick): the car was put where the server says.
    Unpredicted,
    /// The clock stepped and the prediction was thrown away.
    ClockStep,
}

/// What [`Reconciler::apply`] did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Older than a block already applied, or before the first predicted tick: ignored.
    Stale,
    /// The prediction was within the thresholds; nothing moved.
    Confirmed,
    /// Rewound and replayed; the drawn pose was kept still. `distance` is the correction in metres.
    Corrected { distance: f32 },
    /// Rewound and replayed with a visible jump.
    Snapped { distance: f32, reason: SnapReason },
}

/// How far a prediction is from the server's state.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StateError {
    /// Metres.
    pub position: f32,
    /// Degrees.
    pub orientation: f32,
    /// Metres per second.
    pub velocity: f32,
}

impl StateError {
    pub fn between(predicted: &CarState, server: &CarState) -> Self {
        Self {
            position: geom::distance(predicted.position, server.position),
            orientation: geom::angle_between(predicted.orientation, server.orientation).to_degrees(),
            velocity: geom::distance(predicted.linear_velocity, server.linear_velocity),
        }
    }

    /// Whether every term is under its threshold. A NaN term is never within.
    pub fn is_within(&self, tuning: &Tuning) -> bool {
        self.position < tuning.position_threshold_m
            && self.orientation < tuning.orientation_threshold_deg
            && self.velocity < tuning.velocity_threshold_mps
    }
}
