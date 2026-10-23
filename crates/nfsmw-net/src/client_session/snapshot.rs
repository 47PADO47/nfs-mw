//! Snapshots: parts in, the whole snapshot decoded against its baseline, then the remote entities to the
//! interpolator, the own block to reconciliation and the global block to [`super::GlobalView`].
//!
//! The server learns which snapshots arrived from the packet acknowledgements of the parts' packets, so the client
//! sends no acknowledgement of its own. A snapshot whose baseline the client no longer has is reported with
//! `BaselineLost` (at most every 500 ms), and the server answers with a full one.

use blackbox_net_core::replicate::ReplicateError;

use super::ClientLink;
use super::parts::Assembled;
use super::session::ClientSession;
use crate::frames::read_snapshot;
use crate::message::BaselineLost;
use crate::predict::PredictedCar;

/// The shortest pause between two `BaselineLost` reports, milliseconds.
const BASELINE_REPORT_MS: u64 = 500;

impl<L: ClientLink, C: PredictedCar + Clone> ClientSession<L, C> {
    pub(super) fn on_snapshot_part(&mut self, bytes: &[u8], now_ms: u64) {
        let frame = match read_snapshot(&self.schema, bytes) {
            Ok(frame) => frame,
            Err(_) => {
                self.counters.bad_frames += 1;
                return;
            }
        };
        if !self.status.is_joined() {
            return;
        }
        if let Some(snapshot) = self.parts.accept(&frame) {
            self.on_snapshot(snapshot, now_ms);
        }
    }

    fn on_snapshot(&mut self, snapshot: Assembled, now_ms: u64) {
        let bodies: Vec<&[u8]> = snapshot.bodies.iter().map(Vec::as_slice).collect();
        match self.views.decode(self.schema.schema(), snapshot.tick, &bodies) {
            Ok(_) => {}
            Err(ReplicateError::MissingBaseline) => {
                self.counters.missing_baselines += 1;
                self.report_baseline_lost(snapshot.tick, now_ms);
                return;
            }
            Err(_) => {
                self.counters.bad_snapshots += 1;
                return;
            }
        }
        self.counters.snapshots += 1;
        self.stats.on_snapshot(snapshot.tick, now_ms);
        if let Some(global) = snapshot.global
            && self.global.is_none_or(|(tick, _)| snapshot.tick > tick)
        {
            self.global = Some((snapshot.tick, global));
        }
        let Some(server_tick) = self.clock.server_tick() else {
            return;
        };
        if let Some(view) = self.views.view(snapshot.tick) {
            self.remotes.on_view(&self.schema, view, snapshot.tick, server_tick, self.seat.entity);
        }
        if self.is_spectator() {
            return;
        }
        let Some(own) = snapshot.own else {
            return;
        };
        let rtt = self.clock.best_rtt_ms().unwrap_or(0.0);
        if let Some(outcome) = self.own.on_block(snapshot.tick, snapshot.ack_input, &own, server_tick, rtt, now_ms) {
            self.record_outcome(outcome, now_ms);
        }
    }

    fn report_baseline_lost(&mut self, tick: u32, now_ms: u64) {
        if self.last_baseline_report_ms.is_some_and(|last| now_ms.saturating_sub(last) < BASELINE_REPORT_MS) {
            return;
        }
        self.last_baseline_report_ms = Some(now_ms);
        self.send(BaselineLost { tick }.into());
    }
}
