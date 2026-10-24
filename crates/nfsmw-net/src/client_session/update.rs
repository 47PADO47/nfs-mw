//! The once-a-frame update: drain the link, run the clock and the ping timer, reconnect after a drop, release events
//! whose target tick came, and age the smoothing.

use blackbox_net_core::endpoint::{ClientEvent, DisconnectReason, reason};

use super::ClientLink;
use super::events::SessionEvent;
use super::session::ClientSession;
use super::status::{EndReason, Status};
use crate::clock::ClockUpdate;
use crate::frames::{FRAME_PING, FRAME_PONG, FRAME_SNAPSHOT, decode_pong, encode_ping};
use crate::predict::PredictedCar;
use crate::reconcile::Outcome;

impl<L: ClientLink, C: PredictedCar + Clone> ClientSession<L, C> {
    /// Runs one frame at the caller's monotonic `now_ms`, `dt_ms` after the last. Call once per rendered frame,
    /// before stepping the owner's car ([`ClientSession::due_ticks`]) and drawing.
    pub fn update(&mut self, now_ms: u64, dt_ms: f32) {
        self.now_ms = now_ms;
        self.flush_input();
        self.poll_link(now_ms);
        self.reconnect(now_ms);
        if !self.status.is_joined() {
            return;
        }
        if let ClockUpdate::Stepped(_) = self.clock.update(now_ms)
            && let Some(server_tick) = self.clock.server_tick()
            && let Some(outcome) = self.own.clock_step(server_tick, now_ms)
        {
            self.record_outcome(outcome, now_ms);
        }
        if let Some(ping) = self.clock.ping_due(now_ms) {
            self.send_frame(FRAME_PING, encode_ping(&ping).to_vec());
        }
        let mut due = Vec::new();
        self.gate.release(self.clock.server_tick(), &mut due);
        for message in due {
            self.apply_game_event(message);
        }
        self.own.advance(dt_ms);
        self.remotes.advance(f64::from(dt_ms));
    }

    /// Hands the session a new link to the same server while it is reconnecting (when no connector was given).
    /// Ignored in any other state.
    pub fn resume_with(&mut self, link: L) {
        if !matches!(self.status, Status::Reconnecting { .. }) || self.link.is_some() {
            return;
        }
        self.counters.reconnects += 1;
        self.link = Some(link);
    }

    fn poll_link(&mut self, now_ms: u64) {
        while let Some(event) = self.link.as_mut().and_then(ClientLink::poll_event) {
            self.on_link_event(event, now_ms);
        }
    }

    fn on_link_event(&mut self, event: ClientEvent, now_ms: u64) {
        match event {
            ClientEvent::Connected { handshake_hash, .. } => {
                self.pake.hash = Some(handshake_hash);
                self.on_connected();
            }
            ClientEvent::Disconnected(why) => self.on_disconnected(why, now_ms),
            ClientEvent::Reliable { channel, bytes } => {
                self.stats.on_bytes_in(bytes.len(), now_ms);
                self.on_reliable(channel, &bytes);
            }
            ClientEvent::Unreliable { frame_type, bytes } => {
                self.stats.on_bytes_in(bytes.len(), now_ms);
                self.on_unreliable(frame_type, &bytes, now_ms);
            }
            ClientEvent::Delivered(_) | ClientEvent::Lost(_) => {}
            ClientEvent::Stats(stats) => self.endpoint_stats = Some(*stats),
        }
    }

    fn on_unreliable(&mut self, frame_type: u8, bytes: &[u8], now_ms: u64) {
        match frame_type {
            FRAME_SNAPSHOT => self.on_snapshot_part(bytes, now_ms),
            FRAME_PONG => {
                let Ok(pong) = decode_pong(bytes) else {
                    self.counters.bad_frames += 1;
                    return;
                };
                if let Some(rtt) = self.clock.on_pong(&pong, now_ms) {
                    self.stats.on_rtt(rtt);
                }
            }
            _ => self.counters.bad_frames += 1,
        }
    }

    fn on_disconnected(&mut self, why: DisconnectReason, now_ms: u64) {
        self.link = None;
        match self.status {
            Status::Ended(_) | Status::Reconnecting { .. } => {}
            Status::Joined if resumable(why) && self.seat.resume_token.is_some() => {
                let deadline_ms = now_ms + self.config.resume_window_ms;
                self.status = Status::Reconnecting { since_ms: now_ms, deadline_ms };
                self.last_reconnect_ms = Some(now_ms);
                self.events.push(SessionEvent::ConnectionLost);
            }
            _ => self.end(EndReason::Link(why)),
        }
    }

    /// While reconnecting without a link: give up at the deadline, else try the connector once per interval.
    fn reconnect(&mut self, now_ms: u64) {
        let Status::Reconnecting { deadline_ms, .. } = self.status else {
            return;
        };
        if now_ms >= deadline_ms {
            self.end(EndReason::ResumeExpired);
            return;
        }
        if self.link.is_some() {
            return;
        }
        let interval = self.config.reconnect_interval_ms;
        if self.last_reconnect_ms.is_some_and(|last| now_ms.saturating_sub(last) < interval) {
            return;
        }
        let Some(connector) = self.connector.as_mut() else {
            return;
        };
        self.last_reconnect_ms = Some(now_ms);
        self.counters.reconnects += 1;
        self.link = connector();
    }

    /// Ends the session for `why`: closes the link and reports it.
    pub(super) fn end(&mut self, why: EndReason) {
        if self.status.is_ended() {
            return;
        }
        let code = match why {
            EndReason::Left => reason::NORMAL,
            _ => reason::PROTOCOL,
        };
        if let Some(mut link) = self.link.take() {
            link.disconnect(code);
        }
        self.status = Status::Ended(why.clone());
        self.events.push(SessionEvent::Ended(why));
    }

    /// Counts a reconciliation outcome and reports a snap.
    pub(super) fn record_outcome(&mut self, outcome: Outcome, now_ms: u64) {
        match outcome {
            Outcome::Corrected { .. } => self.stats.on_correction(now_ms),
            Outcome::Snapped { reason, .. } => {
                self.stats.on_snap(now_ms);
                self.events.push(SessionEvent::Snapped(reason));
            }
            Outcome::Stale | Outcome::Confirmed => {}
        }
    }

    /// Sends an unreliable frame and counts it.
    pub(super) fn send_frame(&mut self, frame_type: u8, bytes: Vec<u8>) {
        let Some(link) = self.link.as_mut() else {
            return;
        };
        self.stats.on_bytes_out(bytes.len(), self.now_ms);
        link.send_unreliable(frame_type, bytes, None);
    }
}

/// Whether a drop for `why` may be resumed: a silence or an overflow, not a close by either side or a failed handshake.
fn resumable(why: DisconnectReason) -> bool {
    matches!(why, DisconnectReason::Timeout | DisconnectReason::Overflow | DisconnectReason::Remote(reason::TIMEOUT))
}
