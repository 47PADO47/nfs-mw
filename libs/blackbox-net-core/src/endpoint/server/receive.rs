//! The server's receive side: game-port datagrams sorted by their first byte (handshake to the handshake server, data
//! to the peer its connection id names), the in-memory link of the host's own client, and the discovery port.
//!
//! Every check before opening a packet is cheap and silent: the length, the first byte, the clear header, the
//! connection id, the source address (no path validation yet, so a packet from another address is dropped) and the
//! peer's inbound budget. Only then does the connection check the tag.

use std::net::SocketAddr;
use std::time::Duration;

use super::super::event::ServerEvent;
use super::super::link::PeerLink;
use super::super::transport::Transport;
use super::worker::{Peer, ServerWorker};
use crate::MAX_DATAGRAM;
use crate::conn::{Connection, Sealer, parse_clear_header};
use crate::link::Link;
use crate::secure::{Established, PacketKind, ServerAction, classify};

impl<T: Transport> ServerWorker<T> {
    /// Takes datagrams from the game port, waiting up to `wait` for the first.
    pub(super) fn receive_net(&mut self, buf: &mut [u8], wait: Duration) -> bool {
        let mut wait = wait;
        let mut work = false;
        for _ in 0..self.config.max_datagrams_per_pass.max(1) {
            let received = self.wire.net.recv_from(buf, wait);
            let (from, len) = match received {
                Ok(Some(got)) => got,
                Ok(None) => return work,
                Err(_) => {
                    self.counters.recv_errors += 1;
                    return work;
                }
            };
            wait = Duration::ZERO;
            work = true;
            let Some(datagram) = buf.get_mut(..len) else {
                continue;
            };
            let now = self.clock.now_ms();
            self.on_net_datagram(now, from, datagram);
        }
        work
    }

    /// Takes datagrams from the in-memory link of the host's own client.
    pub(super) fn receive_local(&mut self, buf: &mut [u8]) -> bool {
        let mut work = false;
        for _ in 0..self.config.max_datagrams_per_pass.max(1) {
            let Some(link) = self.wire.local.as_mut() else {
                return work;
            };
            let Ok(Some((from, len))) = link.recv(buf) else {
                return work;
            };
            work = true;
            let now = self.clock.now_ms();
            // The client may have sent before this thread took the attach command.
            if self.find(true, from, 0).is_none() {
                self.apply_control(now);
            }
            let Some(datagram) = buf.get_mut(..len) else {
                continue;
            };
            self.on_data(now, from, datagram, true);
        }
        work
    }

    /// Answers discovery requests (security-ops.md 2.3): never more bytes than the request, rate limited by the same
    /// gate as the handshake, only from allowed sources, only while a reply is set.
    pub(super) fn receive_discovery(&mut self, buf: &mut [u8]) -> bool {
        let Some(discovery) = self.discovery.as_mut() else {
            return false;
        };
        let mut work = false;
        for _ in 0..self.config.max_datagrams_per_pass.max(1) {
            let Ok(Some((from, len))) = discovery.recv_from(buf, Duration::ZERO) else {
                return work;
            };
            work = true;
            let now = self.clock.now_ms();
            let Some(datagram) = buf.get(..len) else {
                continue;
            };
            let Some(reply) = self.responder.answer(now, from, datagram, self.handshake.gate_mut()) else {
                continue;
            };
            if discovery.send_to(from, &reply).is_err() {
                self.counters.send_errors += 1;
            }
        }
        work
    }

    /// The peer a data packet belongs to: by source address on the in-memory link, by connection id on the network.
    pub(super) fn find(&self, local: bool, from: SocketAddr, conn_id: u16) -> Option<usize> {
        self.peers.iter().position(|peer| {
            let matches = match local {
                true => peer.link.addr() == from,
                false => peer.link.local_id() == conn_id,
            };
            peer.local == local && matches
        })
    }

    /// One datagram from the game port.
    fn on_net_datagram(&mut self, now_ms: u64, from: SocketAddr, datagram: &mut [u8]) {
        self.counters.datagrams_in += 1;
        self.counters.bytes_in += datagram.len() as u64;
        if datagram.len() > MAX_DATAGRAM {
            self.counters.garbage += 1;
            return;
        }
        match classify(datagram) {
            PacketKind::Handshake(_) => self.on_handshake(now_ms, from, datagram),
            PacketKind::Data => self.on_data(now_ms, from, datagram, false),
            PacketKind::Invalid => self.counters.garbage += 1,
        }
    }

    /// A handshake datagram: the handshake server decides, and its reply (never more than its limits allow) goes back.
    fn on_handshake(&mut self, now_ms: u64, from: SocketAddr, datagram: &[u8]) {
        let action = self.handshake.handle(now_ms, from, datagram, &mut self.out);
        match action {
            ServerAction::None => {}
            ServerAction::Reply(len) => {
                if let Some(bytes) = self.out.get(..len) {
                    self.wire.send(false, from, bytes);
                }
            }
            ServerAction::Established(established) => self.establish(*established),
        }
    }

    /// A finished handshake becomes a peer. Its first flush sends the packet that confirms the client's keys.
    fn establish(&mut self, established: Established) {
        let Established { info, sealer } = established;
        let Some(id) = self.allocate_peer() else {
            self.handshake.connection_closed(info.local_conn_id, info.addr);
            return;
        };
        let sealer: Box<dyn Sealer + Send> = Box::new(sealer);
        let conn = Connection::with_sealer(self.config.conn(info.local_conn_id, info.remote_conn_id), sealer);
        let link = PeerLink::new(conn, info.addr, info.local_conn_id, Some(self.limits.budget()), true);
        self.peers.push(Peer { id, local: false, link });
        self.events.push(ServerEvent::Connected { peer: id, key: info.remote_id(), addr: info.addr, local: false });
    }

    /// A data packet: routed, checked against the address and budget, opened, and its contents handed on.
    pub(super) fn on_data(&mut self, now_ms: u64, from: SocketAddr, datagram: &mut [u8], local: bool) {
        let Ok(clear) = parse_clear_header(datagram) else {
            self.counters.garbage += 1;
            return;
        };
        let Some(index) = self.find(local, from, clear.conn_id) else {
            self.counters.unknown_connection += 1;
            return;
        };
        let Some(peer) = self.peers.get_mut(index) else {
            return;
        };
        if peer.link.addr() != from {
            self.counters.wrong_address += 1;
            return;
        }
        if !peer.link.admit(now_ms, datagram.len()) {
            self.counters.over_budget += 1;
            return;
        }
        let id = peer.id;
        let events = &mut self.events;
        let read = peer.link.read(now_ms, datagram, |frame_type, payload| {
            events.push(ServerEvent::Unreliable { peer: id, frame_type, bytes: payload.to_vec() });
        });
        if read.is_err() {
            self.counters.rejected += 1;
        }
        self.drain_peer(index);
    }
}
