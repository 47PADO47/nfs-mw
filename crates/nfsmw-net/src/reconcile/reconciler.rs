//! The reconcile step of each snapshot and the drawn pose between them.

use nfsmw_sim::Tick;

use super::{AuthState, CorrectionStats, Outcome, SnapReason, StateError, Tuning, VisualOffset};
use crate::geom;
use crate::predict::{Pose, PredictedCar, Predictor};
use crate::schema::CorrectionCause;

/// The client's reconciliation state: tuning, the drawn offset and the statistics.
#[derive(Debug, Clone, Default)]
pub struct Reconciler {
    tuning: Tuning,
    offset: VisualOffset,
    stats: CorrectionStats,
    /// The newest `ack_input` applied.
    last: Option<Tick>,
    /// The last error measured against a server state.
    last_error: StateError,
}

impl Reconciler {
    pub fn new(tuning: Tuning) -> Self {
        Self { tuning, ..Self::default() }
    }

    pub fn tuning(&self) -> &Tuning {
        &self.tuning
    }

    pub fn offset(&self) -> &VisualOffset {
        &self.offset
    }

    pub fn stats(&self) -> &CorrectionStats {
        &self.stats
    }

    /// The error of the last server state compared.
    pub fn last_error(&self) -> StateError {
        self.last_error
    }

    /// Reconcile `predictor` with the server's state `auth`, received at `now_ms` (used for the per-minute rates only).
    /// See the module documentation for the steps.
    pub fn apply<C: PredictedCar>(&mut self, predictor: &mut Predictor<C>, auth: &AuthState, now_ms: u64) -> Outcome {
        if self.last.is_some_and(|last| auth.tick <= last) {
            return Outcome::Stale;
        }
        let snap_cause = match auth.cause {
            CorrectionCause::Reset => Some(SnapReason::Reset),
            CorrectionCause::Teleport => Some(SnapReason::Teleport),
            _ => None,
        };
        let predicted = predictor.entry(auth.tick).map(|entry| entry.state);
        let unpredicted = auth.tick >= predictor.next_tick();
        if predicted.is_none() && !unpredicted && snap_cause.is_none() {
            // Before the first predicted tick, or confirmed already: nothing to compare.
            return Outcome::Stale;
        }
        self.last = Some(auth.tick);
        predictor.set_frozen(auth.cause == CorrectionCause::Freeze);
        self.stats.record_check();
        if let Some(predicted) = predicted {
            self.last_error = StateError::between(&predicted, &auth.state);
            if snap_cause.is_none() && self.last_error.is_within(&self.tuning) {
                predictor.confirm(auth.tick);
                return Outcome::Confirmed;
            }
        }
        let before = predictor.car().state().pose();
        predictor.rewind_replay(auth.tick, &auth.state);
        predictor.confirm(auth.tick);
        let after = predictor.car().state().pose();
        let distance = geom::distance(before.position, after.position);
        let reason = snap_cause
            .or_else(|| unpredicted.then_some(SnapReason::Unpredicted))
            .or_else(|| (distance > self.tuning.snap_distance_m).then_some(SnapReason::Distance));
        if let Some(reason) = reason {
            self.offset.clear();
            self.stats.record_snap(distance, now_ms);
            return Outcome::Snapped { distance, reason };
        }
        self.offset.absorb(before, after);
        self.stats.record_correction(distance, now_ms);
        Outcome::Corrected { distance }
    }

    /// The clock stepped (sync-model.md section 5.2): forget the prediction, continue from `next`, and snap.
    pub fn clock_step<C: PredictedCar>(&mut self, predictor: &mut Predictor<C>, next: Tick, now_ms: u64) -> Outcome {
        predictor.clear(next);
        self.offset.clear();
        self.stats.record_snap(0.0, now_ms);
        Outcome::Snapped { distance: 0.0, reason: SnapReason::ClockStep }
    }

    /// Let `dt_ms` of render time pass: the offset decays.
    pub fn advance(&mut self, dt_ms: f32) {
        self.offset.decay(dt_ms, self.tuning.offset_half_life_ms);
    }

    /// The pose to draw for the physics pose `physics`: physics plus the unapplied part of past corrections.
    pub fn drawn(&self, physics: Pose) -> Pose {
        self.offset.apply(physics)
    }

    /// The drawn pose of the predictor's car.
    pub fn drawn_pose<C: PredictedCar>(&self, predictor: &Predictor<C>) -> Pose {
        self.drawn(predictor.car().state().pose())
    }
}
