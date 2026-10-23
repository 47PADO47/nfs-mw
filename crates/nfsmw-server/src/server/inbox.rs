//! Draining the endpoint: connections, unreliable frames (input, ping) and the delivery reports of snapshots.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::SocketAddr;

use blackbox_net_core::endpoint::{EndpointStats, ServerEvent};
use blackbox_net_core::session::PeerId;
use nfsmw_net::clock::Pong;
use nfsmw_net::input::{FRAME_INPUT, InputFrame, expand_tick};
use nfsmw_net::spectate::sends_input;

use super::Server;
use super::peer::{Peer, Stage};
use super::strikes::Violation;
use crate::control::WorldControl;
use nfsmw_net::frames::{FRAME_PING, FRAME_PONG, decode_ping, encode_pong};

/// Input frames dropped over the rate per strike.
const DROPS_PER_STRIKE: u32 = 10;
/// Snapshot ticks remembered per connection for delivery reports.
const IN_FLIGHT: usize = 64;

impl<W: WorldControl> Server<W> {
    /// Handles every event the net thread queued.
    pub(super) fn drain_inbox(&mut self) {
        while let Some(event) = self.endpoint.poll_event() {
            match event {
                ServerEvent::Connected { peer, key, addr, local } => self.on_connected(peer, key, addr, local),
                ServerEvent::Disconnected { peer, reason } => self.on_disconnected(peer, reason),
                ServerEvent::Reliable { peer, bytes, .. } => self.on_reliable(peer, &bytes),
                ServerEvent::Unreliable { peer, frame_type, bytes } => self.on_unreliable(peer, frame_type, &bytes),
                ServerEvent::Delivered { peer, tag } => self.on_delivered(peer, tag),
                ServerEvent::Lost { peer, tag } => self.on_lost(peer, tag),
                ServerEvent::Stats(stats) => self.on_endpoint_stats(&stats),
            }
        }
    }

    fn on_connected(&mut self, peer: PeerId, key: blackbox_net_core::secure::KeyId, addr: SocketAddr, local: bool) {
        let mut hasher = DefaultHasher::new();
        addr.hash(&mut hasher);
        self.peers.insert(peer, Peer::new(peer, key, hasher.finish(), self.world.tick()));
        self.stats.connections += 1;
        let local = match local {
            true => " (local)",
            false => "",
        };
        log::info!("connection {} from {addr}{local} ({})", peer.0, key.fingerprint());
    }

    fn on_unreliable(&mut self, peer: PeerId, frame_type: u8, bytes: &[u8]) {
        match frame_type {
            FRAME_INPUT => self.on_input(peer, bytes),
            FRAME_PING => self.on_ping(peer, bytes),
            _ => self.strike(peer, Violation::Malformed),
        }
    }

    /// An `INPUT` frame: rate-checked, decoded and stored in the player's window.
    fn on_input(&mut self, peer: PeerId, bytes: &[u8]) {
        let now = self.now_ms;
        let tick = self.world.tick();
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let Stage::Joined(slot) = state.stage else {
            return;
        };
        self.presence.heard(slot, now);
        if !state.input_rate.try_take(now, 1) {
            state.input_drops += 1;
            if state.input_drops % DROPS_PER_STRIKE == 0 {
                self.strike(peer, Violation::InputFlood);
            }
            return;
        }
        let frame = match InputFrame::decode(bytes) {
            Ok(frame) => frame,
            Err(_) => return self.invalid_frame(peer),
        };
        let role_drives = self.lobby.member(slot).is_some_and(|member| sends_input(member.role));
        if !role_drives {
            return;
        }
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let accepted = state.window.accept(&frame, tick);
        let last = expand_tick(frame.first_tick(), tick).wrapping_add(frame.len().saturating_sub(1) as u32);
        state.inputs.frame(tick, accepted.late, accepted.early, i64::from(last) - i64::from(tick));
        self.stats.inputs_in += 1;
        if accepted.early > 0 {
            self.strike(peer, Violation::EarlyInput);
        }
    }

    /// A `PING`, answered with a `PONG` in the same step.
    fn on_ping(&mut self, peer: PeerId, bytes: &[u8]) {
        let Some(state) = self.peers.get(&peer) else {
            return;
        };
        if state.closing() {
            return;
        }
        if let Some(slot) = state.slot() {
            self.presence.heard(slot, self.now_ms);
        }
        let ping = match decode_ping(bytes) {
            Ok(ping) => ping,
            Err(_) => return self.invalid_frame(peer),
        };
        let tick = f64::from(self.world.tick());
        let pong =
            Pong { seq: ping.seq, client_time_ms: ping.client_time_ms, server_recv_tick: tick, server_send_tick: tick };
        let Ok(bytes) = encode_pong(&pong) else {
            return;
        };
        self.endpoint.send_unreliable(peer, FRAME_PONG, bytes.to_vec(), None);
        self.stats.pongs += 1;
    }

    /// A frame that did not decode: it counts toward the invalid-message limit and the score.
    fn invalid_frame(&mut self, peer: PeerId) {
        let now = self.now_ms;
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        self.stats.invalid_messages += 1;
        if state.invalid.record(now) {
            return self.kick_for_violations(peer);
        }
        self.strike(peer, Violation::Malformed);
    }

    /// The snapshot of `tick` reached the client: it becomes the delta baseline.
    fn on_delivered(&mut self, peer: PeerId, tick: u32) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let Some(index) = state.in_flight.iter().position(|sent| *sent == tick) else {
            return;
        };
        state.in_flight.drain(..=index);
        state.replication.on_delivered(tick);
        self.stats.snapshots_delivered += 1;
    }

    fn on_lost(&mut self, peer: PeerId, tick: u32) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        state.in_flight.retain(|sent| *sent != tick);
        self.stats.snapshots_lost += 1;
    }

    /// Records the endpoint's counters and each seated player's round trip for the lobby rows.
    fn on_endpoint_stats(&mut self, stats: &EndpointStats) {
        for entry in &stats.peers {
            let Some(slot) = self.peers.get(&entry.peer).and_then(Peer::slot) else {
                continue;
            };
            self.lobby.set_ping(slot, entry.rtt_ms.clamp(0.0, f64::from(u16::MAX)) as u16);
        }
        self.stats.endpoint = Some(stats.clone());
    }

    /// Remembers that the snapshot of `tick` went to `peer`.
    pub(super) fn sent_snapshot(peer: &mut Peer, tick: u32) {
        peer.in_flight.push_back(tick);
        if peer.in_flight.len() > IN_FLIGHT {
            peer.in_flight.pop_front();
        }
    }
}
