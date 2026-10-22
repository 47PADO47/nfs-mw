//! The server's net thread: the loop, the peers, the timers and the sends. Receiving is in `receive.rs`, commands in
//! `control.rs`.
//!
//! One pass: control commands, send commands, up to [`EndpointConfig::max_datagrams_per_pass`] datagrams from the game
//! port (waiting up to [`EndpointConfig::poll_ms`] only when the last pass had nothing to do), the in-memory link, the
//! discovery port, then the timers (handshake expiry, budgets, idle timeouts, stats) and the sends.

use std::time::Duration;

use super::super::clock::Clock;
use super::super::command::{PeerLimits, ServerCommand, ServerControl};
use super::super::config::EndpointConfig;
use super::super::discovery::{DiscoveryResponder, SourcePolicy};
use super::super::event::{DisconnectReason, ServerEvent};
use super::super::link::{LinkEvent, PeerLink};
use super::super::reason;
use super::super::stats::EndpointStats;
use super::super::transport::Transport;
use super::super::wire::Wire;
use super::super::worker::Stop;
use crate::MAX_DATAGRAM;
use crate::handoff::{RingReceiver, RingSender};
use crate::link::MemoryLink;
use crate::secure::HandshakeServer;
use crate::session::PeerId;

/// The queues and settings the handle hands to the thread.
pub(crate) struct Shared {
    pub(crate) config: EndpointConfig,
    pub(crate) limits: PeerLimits,
    pub(crate) events: RingSender<ServerEvent>,
    pub(crate) commands: RingReceiver<ServerCommand>,
    pub(crate) control: RingReceiver<ServerControl>,
}

/// One connected peer.
#[derive(Debug)]
pub(super) struct Peer {
    pub(super) id: PeerId,
    pub(super) local: bool,
    pub(super) link: PeerLink,
}

/// Everything the server's net thread owns.
pub(crate) struct ServerWorker<T: Transport> {
    pub(super) config: EndpointConfig,
    pub(super) limits: PeerLimits,
    pub(super) wire: Wire<T>,
    pub(super) discovery: Option<Box<dyn Transport>>,
    pub(super) handshake: HandshakeServer,
    pub(super) responder: DiscoveryResponder,
    pub(super) peers: Vec<Peer>,
    pub(super) next_peer: u16,
    pub(super) events: RingSender<ServerEvent>,
    pub(super) commands: RingReceiver<ServerCommand>,
    pub(super) control: RingReceiver<ServerControl>,
    pub(super) clock: Clock,
    pub(super) counters: EndpointStats,
    pub(super) out: Vec<u8>,
    next_tick_ms: u64,
    next_stats_ms: u64,
}

impl<T: Transport> ServerWorker<T> {
    /// The thread's state, before it starts.
    pub(crate) fn new(
        shared: Shared,
        net: T,
        local: MemoryLink,
        discovery: Option<Box<dyn Transport>>,
        handshake: HandshakeServer,
        sources: SourcePolicy,
        clock: Clock,
    ) -> Self {
        Self {
            config: shared.config,
            limits: shared.limits,
            wire: Wire::new(net, Some(local)),
            discovery,
            handshake,
            responder: DiscoveryResponder::new(sources),
            peers: Vec::new(),
            next_peer: 1,
            events: shared.events,
            commands: shared.commands,
            control: shared.control,
            clock,
            counters: EndpointStats::default(),
            out: vec![0u8; MAX_DATAGRAM],
            next_tick_ms: 0,
            next_stats_ms: shared.config.stats_ms,
        }
    }

    /// Runs until the handle asks it to stop, then closes every connection.
    pub(crate) fn run(mut self, stop: &Stop) {
        let mut buf = vec![0u8; MAX_DATAGRAM + 1];
        let mut idle = false;
        while !stop.requested() {
            let now = self.clock.now_ms();
            self.wire.net.set_now(now);
            self.apply_control(now);
            self.apply_commands(now);
            let wait = Duration::from_millis(self.config.poll_ms.max(1) * u64::from(idle));
            let mut work = self.receive_net(&mut buf, wait);
            work |= self.receive_local(&mut buf);
            work |= self.receive_discovery(&mut buf);
            let now = self.clock.now_ms();
            self.timers(now);
            work |= self.flush(now);
            idle = !work;
        }
        self.shut_down();
    }

    /// The index of `peer`.
    pub(super) fn index_of(&self, peer: PeerId) -> Option<usize> {
        self.peers.iter().position(|candidate| candidate.id == peer)
    }

    /// A peer id no connected peer has, preferring the next in sequence so a stale command rarely names a new peer.
    pub(super) fn allocate_peer(&mut self) -> Option<PeerId> {
        for _ in 0..=u16::MAX {
            let candidate = self.next_peer;
            self.next_peer = self.next_peer.wrapping_add(1).max(1);
            if !self.peers.iter().any(|peer| peer.id.0 == candidate) {
                return Some(PeerId(candidate));
            }
        }
        None
    }

    /// Sends DISCONNECT with `byte` to the peer at `index`, then forgets it and reports `reason`.
    pub(super) fn close(&mut self, index: usize, now_ms: u64, byte: u8, reason: DisconnectReason) {
        let Some(peer) = self.peers.get_mut(index) else {
            return;
        };
        let (local, addr) = (peer.local, peer.link.addr());
        let wire = &mut self.wire;
        peer.link.close_now(now_ms, byte, &mut self.out, |bytes| wire.send(local, addr, bytes));
        self.remove(index, reason);
    }

    /// Forgets the peer at `index` and reports `reason`.
    pub(super) fn remove(&mut self, index: usize, reason: DisconnectReason) {
        if index >= self.peers.len() {
            return;
        }
        let peer = self.peers.swap_remove(index);
        if !peer.local {
            self.handshake.connection_closed(peer.link.local_id(), peer.link.addr());
        }
        self.events.push(ServerEvent::Disconnected { peer: peer.id, reason });
    }

    /// Hands the peer's complete messages and connection events to the game; a DISCONNECT from it removes it.
    pub(super) fn drain_peer(&mut self, index: usize) {
        let Some(peer) = self.peers.get_mut(index) else {
            return;
        };
        let id = peer.id;
        while let Some((channel, bytes)) = peer.link.take_message() {
            self.events.push(ServerEvent::Reliable { peer: id, channel, bytes });
        }
        let mut closed = None;
        let events = &mut self.events;
        peer.link.drain_events(|event| match event {
            LinkEvent::Delivered(tag) => {
                events.push(ServerEvent::Delivered { peer: id, tag });
            }
            LinkEvent::Lost(tag) => {
                events.push(ServerEvent::Lost { peer: id, tag });
            }
            LinkEvent::Closed(reason) => closed = Some(reason),
        });
        if let Some(reason) = closed {
            self.remove(index, DisconnectReason::Remote(reason));
        }
    }

    /// Handshake expiry, budget ticks, idle timeouts and the stats event.
    fn timers(&mut self, now_ms: u64) {
        self.handshake.expire(now_ms);
        if now_ms >= self.next_tick_ms {
            self.next_tick_ms = now_ms + self.config.tick_ms.max(1);
            for peer in &mut self.peers {
                peer.link.start_tick();
            }
        }
        let mut index = 0;
        while index < self.peers.len() {
            let timed_out = self.peers.get(index).is_some_and(|peer| peer.link.is_timed_out(now_ms));
            if timed_out {
                self.close(index, now_ms, reason::TIMEOUT, DisconnectReason::Timeout);
                continue;
            }
            index += 1;
        }
        if self.config.stats_ms == 0 || now_ms < self.next_stats_ms {
            return;
        }
        self.next_stats_ms = now_ms + self.config.stats_ms;
        let stats = self.stats(now_ms);
        self.events.push(ServerEvent::Stats(Box::new(stats)));
    }

    /// Sends what every peer has due, then reports the events the sends raised (losses found while writing).
    fn flush(&mut self, now_ms: u64) -> bool {
        let mut work = false;
        let Self { peers, wire, out, counters, .. } = self;
        for peer in peers.iter_mut() {
            let (local, addr) = (peer.local, peer.link.addr());
            let bad = &mut counters.bad_commands;
            work |= peer.link.flush(now_ms, out, bad, |bytes| wire.send(local, addr, bytes));
        }
        let mut index = 0;
        while index < self.peers.len() {
            let before = self.peers.len();
            self.drain_peer(index);
            if self.peers.len() == before {
                index += 1;
            }
        }
        work
    }

    /// The counters with the send, handshake, discovery and queue numbers and one entry per peer.
    fn stats(&self, now_ms: u64) -> EndpointStats {
        let mut stats = self.counters.clone();
        stats.now_ms = now_ms;
        stats.datagrams_out = self.wire.stats.datagrams;
        stats.bytes_out = self.wire.stats.bytes;
        stats.send_errors += self.wire.stats.errors;
        stats.events_dropped = self.events.dropped();
        stats.discovery_replies = self.responder.replies();
        stats.discovery_refused = self.responder.refused();
        stats.handshake = *self.handshake.stats();
        stats.peers = self.peers.iter().map(|peer| peer.link.stats(peer.id)).collect();
        stats
    }

    /// Refuses new handshakes and closes every connection.
    fn shut_down(&mut self) {
        self.handshake.set_shutting_down(true);
        let now = self.clock.now_ms();
        while let Some(last) = self.peers.len().checked_sub(1) {
            self.close(last, now, reason::SHUTTING_DOWN, DisconnectReason::Shutdown);
        }
    }
}
