//! The server's commands: sends queue messages and frames on the peers' connections; control commands close peers,
//! change budgets, set the discovery reply and attach the host's own client.

use std::net::SocketAddr;

use super::super::command::{ServerCommand, ServerControl};
use super::super::event::{DisconnectReason, ServerEvent};
use super::super::link::PeerLink;
use super::super::reason;
use super::super::transport::Transport;
use super::worker::{Peer, ServerWorker};
use super::{LOCAL_CLIENT_ID, LOCAL_SERVER_ID};
use crate::conn::{ConnError, Connection, Plain, Sealer};
use crate::secure::KeyId;
use crate::session::PeerId;

impl<T: Transport> ServerWorker<T> {
    /// Takes every waiting control command.
    pub(super) fn apply_control(&mut self, now_ms: u64) {
        for _ in 0..self.control.capacity() {
            let Some(control) = self.control.pop() else {
                return;
            };
            match control {
                ServerControl::Disconnect { peer, reason } => {
                    if let Some(index) = self.index_of(peer) {
                        self.close(index, now_ms, reason, DisconnectReason::Local(reason));
                    }
                }
                ServerControl::SetBudget { peer, limits } => {
                    if let Some(found) = self.peers.iter_mut().find(|candidate| candidate.id == peer) {
                        found.link.set_budget(limits.budget());
                    }
                }
                ServerControl::DiscoveryReply(reply) => self.responder.set_reply(reply.as_ref()),
                ServerControl::AttachLocal { addr, key } => self.attach_local(addr, key),
            }
        }
    }

    /// Takes every waiting send. A peer whose reliable queue is full is closed (security-ops.md 2.1).
    pub(super) fn apply_commands(&mut self, now_ms: u64) {
        let mut overflowed: Vec<PeerId> = Vec::new();
        for _ in 0..self.commands.capacity() {
            let Some(command) = self.commands.pop() else {
                break;
            };
            match command {
                ServerCommand::Reliable { peer, channel, bytes } => {
                    if let Some(index) = self.index_of(peer) {
                        self.reliable_to(index, channel, &bytes, &mut overflowed);
                    }
                }
                ServerCommand::Unreliable { peer, frame_type, bytes, tag } => {
                    if let Some(index) = self.index_of(peer) {
                        self.unreliable_to(index, frame_type, bytes, tag);
                    }
                }
                ServerCommand::BroadcastReliable { channel, bytes } => {
                    for index in 0..self.peers.len() {
                        self.reliable_to(index, channel, &bytes, &mut overflowed);
                    }
                }
                ServerCommand::BroadcastUnreliable { frame_type, bytes } => {
                    for index in 0..self.peers.len() {
                        self.unreliable_to(index, frame_type, bytes.clone(), None);
                    }
                }
            }
        }
        for peer in overflowed {
            if let Some(index) = self.index_of(peer) {
                self.close(index, now_ms, reason::PROTOCOL, DisconnectReason::Overflow);
            }
        }
    }

    /// Queues a reliable message on the peer at `index`.
    fn reliable_to(&mut self, index: usize, channel: u8, bytes: &[u8], overflowed: &mut Vec<PeerId>) {
        let Some(peer) = self.peers.get_mut(index) else {
            return;
        };
        match peer.link.send_reliable(channel, bytes) {
            Ok(()) => {}
            Err(ConnError::QueueFull) => overflowed.push(peer.id),
            Err(_) => self.counters.bad_commands += 1,
        }
    }

    /// Queues an unreliable frame on the peer at `index`.
    fn unreliable_to(&mut self, index: usize, frame_type: u8, bytes: Vec<u8>, tag: Option<u32>) {
        let Some(peer) = self.peers.get_mut(index) else {
            return;
        };
        if !peer.link.queue_unreliable(frame_type, bytes, tag) {
            self.counters.bad_commands += 1;
        }
    }

    /// The host's own client: a plain connection on the in-memory link, reported like any peer.
    fn attach_local(&mut self, addr: SocketAddr, key: KeyId) {
        if self.wire.local.is_none() || self.find(true, addr, 0).is_some() {
            return;
        }
        let Some(id) = self.allocate_peer() else {
            return;
        };
        let sealer: Box<dyn Sealer + Send> = Box::new(Plain);
        let conn = Connection::with_sealer(self.config.conn(LOCAL_SERVER_ID, LOCAL_CLIENT_ID), sealer);
        let link = PeerLink::new(conn, addr, LOCAL_SERVER_ID, None, false);
        self.peers.push(Peer { id, local: true, link });
        self.events.push(ServerEvent::Connected { peer: id, key, addr, local: true, handshake_hash: [0; 32] });
    }
}
