//! The room password exchange (security.md 4.6), the server's half of `nfsmw-net`'s client admission.
//!
//! A room with a password answers `Auth` with `PakeStart` (this side's SPAKE2 message, bound to the connection's Noise
//! handshake hash). The client answers with its own message and its confirmation tag; the server checks the tag, sends
//! its own tag back when it matched, and only then judges the `Join`, with the proof the exchange gave. A `Join` that
//! comes first waits in the peer until the exchange ends. An exchange that does not end within
//! [`PAKE_TIMEOUT_MS`] counts as failed. The password never travels, and a wrong one shows only as a failed proof,
//! which the join gate counts against the guess limits and answers like a full room.

use blackbox_net_core::secure::password::{self, PakeRole};
use blackbox_net_core::session::PeerId;
use nfsmw_net::join::PasswordProof;
use nfsmw_net::message::{Bytes, Join, PakeConfirm, PakeMsg, PakeScheme, PakeStart};

use super::Server;
use super::peer::PakeProgress;
use super::strikes::Violation;
use crate::control::WorldControl;

/// How long a client has to finish the exchange after `PakeStart`, milliseconds.
pub(super) const PAKE_TIMEOUT_MS: u64 = 5_000;

impl<W: WorldControl> Server<W> {
    /// `Auth` came in: a room with a password answers with `PakeStart`.
    pub(super) fn begin_password(&mut self, peer: PeerId) {
        let Some(secret) = self.config.password.clone() else {
            return;
        };
        let now = self.now_ms;
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let start =
            password::start(&nfsmw_net::message::password_secret(&secret), &state.handshake_hash, PakeRole::Host);
        let message = start.map(|(pake, mine)| (pake, Bytes::new(mine.to_vec())));
        let Ok((pake, Ok(message))) = message else {
            log::warn!("connection {}: the room password exchange could not start", peer.0);
            state.pake = PakeProgress::Done(PasswordProof::Failed);
            return;
        };
        state.pake = PakeProgress::Started(pake);
        state.pake_deadline_ms = Some(now.saturating_add(PAKE_TIMEOUT_MS));
        self.send_to_peer(peer, PakeStart { scheme: PakeScheme::Spake2, message }.into());
    }

    /// `Join` from an authenticated connection: judged now, or held until the password exchange ends.
    pub(super) fn on_join_request(&mut self, peer: PeerId, join: Join) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let proof = match &state.pake {
            PakeProgress::Idle => PasswordProof::Missing,
            PakeProgress::Done(proof) => *proof,
            PakeProgress::Started(_) | PakeProgress::Finished(_) => {
                if state.pending_join.is_some() {
                    return self.strike(peer, Violation::OutOfOrder);
                }
                state.pending_join = Some(join);
                return;
            }
        };
        self.on_join(peer, join, proof);
    }

    /// The client's SPAKE2 message.
    pub(super) fn on_pake_msg(&mut self, peer: PeerId, message: PakeMsg) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let PakeProgress::Started(pake) = std::mem::take(&mut state.pake) else {
            return self.strike(peer, Violation::OutOfOrder);
        };
        state.pake = match pake.finish(&message.message) {
            Ok(confirm) => PakeProgress::Finished(confirm),
            Err(_) => PakeProgress::Done(PasswordProof::Failed),
        };
        self.resolve_join(peer);
    }

    /// The client's confirmation tag: it proves the client used the same password over the same handshake. A match
    /// is answered with the server's own tag, which proves the same to the client.
    pub(super) fn on_pake_confirm(&mut self, peer: PeerId, confirm: PakeConfirm) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let PakeProgress::Finished(ours) = std::mem::take(&mut state.pake) else {
            return self.strike(peer, Violation::OutOfOrder);
        };
        let passed = ours.verify(&confirm.tag);
        let proof = match passed {
            true => PasswordProof::Passed,
            false => PasswordProof::Failed,
        };
        state.pake = PakeProgress::Done(proof);
        if passed {
            self.send_to_peer(peer, PakeConfirm { tag: ours.tag() }.into());
        }
        self.resolve_join(peer);
    }

    /// Judges the held `Join`, if there is one and the exchange is over.
    fn resolve_join(&mut self, peer: PeerId) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        let PakeProgress::Done(proof) = &state.pake else {
            return;
        };
        let proof = *proof;
        let Some(join) = state.pending_join.take() else {
            return;
        };
        self.on_join(peer, join, proof);
    }

    /// Exchanges that ran out of time end as failed.
    pub(super) fn expire_password_exchanges(&mut self) {
        let now = self.now_ms;
        let due: Vec<PeerId> = self
            .peers
            .values()
            .filter(|peer| matches!(peer.pake, PakeProgress::Started(_) | PakeProgress::Finished(_)))
            .filter(|peer| peer.pake_deadline_ms.is_some_and(|at| at <= now))
            .map(|peer| peer.id)
            .collect();
        for peer in due {
            let Some(state) = self.peers.get_mut(&peer) else {
                continue;
            };
            log::info!("connection {}: the room password exchange timed out", peer.0);
            state.pake = PakeProgress::Done(PasswordProof::Failed);
            self.resolve_join(peer);
        }
    }
}
