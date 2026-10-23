//! One connection's state on the server: who it is, where it stands in the admission, its input window, its
//! replication views, its strikes and its counters.

use std::collections::VecDeque;

use blackbox_net_core::ratelimit::TokenBucket;
use blackbox_net_core::replicate::ClientState;
use blackbox_net_core::secure::KeyId;
use blackbox_net_core::session::PeerId;
use nfsmw_net::drop::BulkSender;
use nfsmw_net::input::InputWindow;
use nfsmw_net::message::{Auth, InvalidMessages};
use nfsmw_net::schema::{CorrectionCause, InputReport};
use nfsmw_sim::Tick;

use super::strikes::Suspicion;

/// Ticks the input report covers.
const REPORT_TICKS: usize = 32;
/// Reliable messages per second a client may send, and the burst (security-gameplay.md 5.1).
const MESSAGES_PER_S: u32 = 20;
const MESSAGE_BURST: u32 = 40;
/// Input frame datagrams per second, and the burst.
const INPUTS_PER_S: u32 = 66;
const INPUT_BURST: u32 = 20;

/// How the inputs of the last [`REPORT_TICKS`] ticks arrived, for the own block's input report.
#[derive(Debug, Clone, Default)]
pub(crate) struct InputLog {
    /// Per simulated tick: whether its input was received in time.
    ticks: VecDeque<bool>,
    /// Late and early inputs seen, with the tick they were seen at.
    late: VecDeque<Tick>,
    early: VecDeque<Tick>,
    /// Mean earliness in ticks, smoothed.
    earliness: f32,
}

impl InputLog {
    /// Tick `tick` was simulated with a received input or without one.
    pub(crate) fn simulated(&mut self, tick: Tick, received: bool) {
        self.ticks.push_back(received);
        if self.ticks.len() > REPORT_TICKS {
            self.ticks.pop_front();
        }
        let oldest = tick.saturating_sub(REPORT_TICKS as u32);
        self.late.retain(|seen| *seen > oldest);
        self.early.retain(|seen| *seen > oldest);
    }

    /// A frame brought `late` inputs for simulated ticks and `early` ones too far ahead, at `tick`; its newest input
    /// was `ahead` ticks ahead of the server.
    pub(crate) fn frame(&mut self, tick: Tick, late: u8, early: u8, ahead: i64) {
        for _ in 0..late.min(REPORT_TICKS as u8) {
            self.late.push_back(tick);
        }
        for _ in 0..early.min(REPORT_TICKS as u8) {
            self.early.push_back(tick);
        }
        self.late.truncate(REPORT_TICKS);
        self.early.truncate(REPORT_TICKS);
        self.earliness += (ahead.clamp(0, 15) as f32 - self.earliness) * 0.1;
    }

    /// The report of the own block.
    pub(crate) fn report(&self) -> InputReport {
        let on_time = self.ticks.iter().filter(|received| **received).count();
        let saturate = |count: usize| count.min(31) as u8;
        InputReport {
            early: saturate(self.early.len()),
            on_time: saturate(on_time),
            late: saturate(self.late.len()),
            missing: saturate(self.ticks.len() - on_time),
            earliness: self.earliness.round().clamp(0.0, 15.0) as u8,
        }
    }
}

/// Where a connection stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Connected; waiting for `Auth`.
    Hello,
    /// `Auth` read; waiting for `Join`.
    Authed,
    /// Seated in a slot.
    Joined(u8),
    /// Being closed at `at_ms` with this reason byte; everything it sends is ignored.
    Closing { at_ms: u64, reason: u8 },
}

/// One connection.
#[derive(Debug)]
pub(crate) struct Peer {
    pub(crate) id: PeerId,
    pub(crate) key: KeyId,
    /// The admission's key for the source address.
    pub(crate) source: u64,
    pub(crate) stage: Stage,
    pub(crate) auth: Option<Auth>,
    pub(crate) window: InputWindow,
    pub(crate) inputs: InputLog,
    /// The last tick simulated with this client's own input.
    pub(crate) last_input: Option<Tick>,
    pub(crate) replication: ClientState,
    /// The snapshot ticks in flight, so a `Delivered` names a tick the client was sent.
    pub(crate) in_flight: VecDeque<u32>,
    pub(crate) invalid: InvalidMessages,
    pub(crate) suspicion: Suspicion,
    pub(crate) messages: TokenBucket,
    pub(crate) input_rate: TokenBucket,
    pub(crate) input_drops: u32,
    /// The next event sequence number.
    pub(crate) event_seq: u32,
    /// The late-join session data still to send.
    pub(crate) bulk: Option<BulkSender>,
    /// Why the server moved the car since the last snapshot.
    pub(crate) cause: CorrectionCause,
    /// Snapshots and snapshot bytes sent.
    pub(crate) snapshots: u64,
    pub(crate) snapshot_bytes: u64,
    /// Snapshot bytes per second, over the last status period.
    pub(crate) window_bytes: u64,
}

impl Peer {
    /// A new connection at server tick `tick`.
    pub(crate) fn new(id: PeerId, key: KeyId, source: u64, tick: Tick) -> Self {
        Self {
            id,
            key,
            source,
            stage: Stage::Hello,
            auth: None,
            window: InputWindow::new(tick),
            inputs: InputLog::default(),
            last_input: None,
            replication: ClientState::new(),
            in_flight: VecDeque::new(),
            invalid: InvalidMessages::new(),
            suspicion: Suspicion::default(),
            messages: TokenBucket::new(MESSAGE_BURST, MESSAGES_PER_S, 1000),
            input_rate: TokenBucket::new(INPUT_BURST, INPUTS_PER_S, 1000),
            input_drops: 0,
            event_seq: 0,
            bulk: None,
            cause: CorrectionCause::None,
            snapshots: 0,
            snapshot_bytes: 0,
            window_bytes: 0,
        }
    }

    /// The slot, once joined.
    pub(crate) fn slot(&self) -> Option<u8> {
        match self.stage {
            Stage::Joined(slot) => Some(slot),
            _ => None,
        }
    }

    /// Whether the connection is being closed.
    pub(crate) fn closing(&self) -> bool {
        matches!(self.stage, Stage::Closing { .. })
    }

    /// Starts a fresh snapshot stream: the next snapshot has no baseline.
    pub(crate) fn resync(&mut self) {
        self.replication.reset();
        self.in_flight.clear();
    }
}
