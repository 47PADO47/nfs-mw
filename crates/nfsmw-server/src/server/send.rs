//! Sending reliable messages: to one connection, to a slot, to every member, to a chat audience; and the late-join
//! bulk transfer.

use blackbox_net_core::session::PeerId;
use nfsmw_net::chat::{Audience, Relay};
use nfsmw_net::drop::BulkSender;
use nfsmw_net::message::{BulkKind, Message, encode};

use super::Server;
use super::stamp::stamp;
use crate::control::WorldControl;

/// Bulk bytes sent to one connection per tick: 2 KiB, 120 KiB/s, so a late joiner's session data is there in well
/// under a second without crowding its snapshots.
const BULK_BYTES_PER_TICK: usize = 2048;

impl<W: WorldControl> Server<W> {
    /// Sends `message` to connection `peer`, stamping an event's sequence number.
    pub(super) fn send_to_peer(&mut self, peer: PeerId, mut message: Message) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        if stamp(&mut message, state.event_seq) {
            state.event_seq = state.event_seq.wrapping_add(1);
        }
        let mut bytes = Vec::new();
        if let Err(err) = encode(&message, &mut bytes) {
            log::warn!("message {:#06x} not sent: {err}", message.type_id());
            return;
        }
        self.stats.messages_out += 1;
        self.endpoint.send_reliable(peer, message.channel().index(), bytes);
    }

    /// Sends `message` to the connection seated in `slot`, if any.
    pub(super) fn send_to_slot(&mut self, slot: u8, message: Message) {
        let Some(peer) = self.slots.get(&slot).copied() else {
            return;
        };
        self.send_to_peer(peer, message);
    }

    /// Sends `message` to every seated connection, and records it in the replay.
    pub(super) fn broadcast(&mut self, message: Message) {
        self.record_event(&message);
        let slots: Vec<u8> = self.slots.keys().copied().collect();
        for slot in slots {
            self.send_to_slot(slot, message.clone());
        }
    }

    /// Sends a relayed chat line to its audience.
    pub(super) fn relay_chat(&mut self, relay: Relay) {
        let message = Message::ChatMessage(relay.message);
        let targets: Vec<u8> = match relay.audience {
            Audience::All => self.slots.keys().copied().collect(),
            Audience::Team(team) => {
                self.lobby.members().filter(|member| member.team == team).map(|member| member.slot).collect()
            }
            Audience::Whisper { from, to } => vec![from, to],
        };
        for slot in targets {
            self.send_to_slot(slot, message.clone());
        }
    }

    /// Writes an event to the replay, when one is recorded.
    fn record_event(&mut self, message: &Message) {
        let Some(replay) = self.replay.as_mut() else {
            return;
        };
        let mut bytes = Vec::new();
        if encode(message, &mut bytes).is_ok() {
            replay.event(self.world.tick(), &bytes);
        }
    }

    /// Starts the late-join transfer of the session data to `slot` (MP-53).
    pub(super) fn start_bulk(&mut self, slot: u8) {
        let data = self.lobby.session_data().encode();
        let Some(peer) = self.slots.get(&slot).and_then(|peer| self.peers.get_mut(peer)) else {
            return;
        };
        match BulkSender::new(BulkKind::SessionData, u16::from(slot), data) {
            Ok(sender) => peer.bulk = Some(sender),
            Err(err) => log::warn!("slot {slot}: no session data: {err:?}"),
        }
    }

    /// Sends the next chunks of every running bulk transfer.
    pub(super) fn send_bulk(&mut self) {
        let mut out = Vec::new();
        for peer in self.peers.values_mut() {
            let Some(sender) = peer.bulk.as_mut() else {
                continue;
            };
            for message in sender.next(BULK_BYTES_PER_TICK) {
                out.push((peer.id, message));
            }
            if sender.is_done() {
                peer.bulk = None;
            }
        }
        for (peer, message) in out {
            self.send_to_peer(peer, message);
        }
    }
}
