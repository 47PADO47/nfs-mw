//! Routing a reliable message: the rate and decode checks, the admission messages, then the lobby, chat, vote and
//! reset commands of a seated player. Server-to-client messages from a client are violations.

use blackbox_net_core::session::PeerId;
use nfsmw_net::chat::Sender;
use nfsmw_net::fairness::ResetState;
use nfsmw_net::lobby::LobbyError;
use nfsmw_net::message::{ChatChannel, EventHead, Message, Respawn, RespawnCause, SessionPhase, VoteKind, decode};
use nfsmw_net::modes::ModeInput;
use nfsmw_net::vote::VoteRoll;
use nfsmw_sim::PlayerId;

use super::Server;
use super::peer::Stage;
use super::strikes::{Verdict, Violation};
use crate::catalog::CatalogRef;
use crate::control::WorldControl;

impl<W: WorldControl> Server<W> {
    /// One reliable message from `peer`.
    pub(super) fn on_reliable(&mut self, peer: PeerId, bytes: &[u8]) {
        let now = self.now_ms;
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        if state.closing() {
            return;
        }
        if let Some(slot) = state.slot() {
            self.presence.heard(slot, now);
        }
        if !state.messages.try_take(now, 1) {
            self.stats.messages_dropped += 1;
            return self.strike(peer, Violation::MessageFlood);
        }
        self.stats.messages_in += 1;
        let message = match decode(bytes) {
            Ok(message) => message,
            Err(err) => return self.on_bad_message(peer, err.is_skippable()),
        };
        let stage = state.stage;
        match (stage, message) {
            (Stage::Hello, Message::Auth(auth)) => {
                if let Some(state) = self.peers.get_mut(&peer) {
                    state.auth = Some(auth);
                    state.stage = Stage::Authed;
                }
                self.begin_password(peer);
            }
            (Stage::Authed, Message::Join(join)) => self.on_join_request(peer, join),
            (Stage::Authed, Message::PakeMsg(message)) => self.on_pake_msg(peer, message),
            (Stage::Authed, Message::PakeConfirm(confirm)) => self.on_pake_confirm(peer, confirm),
            (Stage::Joined(slot), message) => self.on_member_message(peer, slot, message),
            _ => self.strike(peer, Violation::OutOfOrder),
        }
    }

    /// A message that did not decode. An unknown type is counted without a strike of its own; past the limit of
    /// protocol.md section 8.2 the connection is closed.
    fn on_bad_message(&mut self, peer: PeerId, skippable: bool) {
        let now = self.now_ms;
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        self.stats.invalid_messages += 1;
        if state.invalid.record(now) {
            return self.kick_for_violations(peer);
        }
        if !skippable {
            self.strike(peer, Violation::Malformed);
        }
    }

    /// Adds a violation to `peer`'s score and acts on the verdict.
    pub(super) fn strike(&mut self, peer: PeerId, violation: Violation) {
        let now = self.now_ms;
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        if state.closing() {
            return;
        }
        self.stats.strikes += 1;
        match state.suspicion.add(violation, now) {
            Verdict::Nothing => {}
            Verdict::Warn => log::warn!("connection {}: suspicious ({violation:?})", peer.0),
            Verdict::Resync => {
                log::warn!("connection {}: resync after violations", peer.0);
                state.resync();
            }
            Verdict::Kick => self.kick_for_violations(peer),
        }
    }

    /// A seated player's message.
    fn on_member_message(&mut self, peer: PeerId, slot: u8, message: Message) {
        let now = self.now_ms;
        let tick = self.world.tick();
        let catalog = CatalogRef(self.setup.catalog.as_ref());
        let result = match message {
            Message::SetReady(body) => self.lobby.set_ready(slot, body.ready, now, tick),
            Message::SelectLoadout(body) => self.lobby.select_loadout(slot, body.loadout, &catalog, now),
            Message::SetRole(body) => self.lobby.set_role(slot, body.role, now),
            Message::UpdateSettings(body) => self.lobby.update_settings(slot, &body.patch, &catalog, now),
            Message::StartCountdown(_) => self.lobby.start(slot, tick),
            Message::CancelCountdown(_) => self.lobby.cancel(slot),
            Message::LoadComplete(_) => self.lobby.load_complete(slot),
            Message::HostCommand(body) => self.lobby.command(slot, body.command, &catalog, now, tick),
            Message::Vote(body) => return self.on_vote(peer, slot, &body),
            Message::ChatSend(body) => return self.on_chat(slot, &body),
            Message::QuickMessage(body) => return self.on_quick(slot, &body),
            Message::ResetRequest(_) => return self.on_reset(slot),
            Message::BaselineLost(_) => return self.on_baseline_lost(peer),
            Message::ResultRequest(_) => return self.resend_result(slot),
            Message::StreamingStall(body) => {
                log::debug!("slot {slot}: streaming stall {}", body.stalled);
                return;
            }
            Message::BulkStart(_) | Message::BulkChunk(_) | Message::BulkEnd(_) => return,
            _ => return self.strike(peer, Violation::OutOfOrder),
        };
        match result {
            Ok(actions) => self.apply_actions(actions),
            Err(err) => self.on_lobby_error(peer, slot, err),
        }
    }

    /// A refused lobby command: a spoof is a strike, anything else is logged.
    fn on_lobby_error(&mut self, peer: PeerId, slot: u8, err: LobbyError) {
        if err.is_violation() {
            return self.strike(peer, Violation::Spoof);
        }
        log::debug!("slot {slot}: lobby refused: {err:?}");
    }

    fn on_vote(&mut self, peer: PeerId, slot: u8, vote: &nfsmw_net::message::Vote) {
        let electorate = self.lobby.voters();
        let members = self.lobby.slots();
        let protected = self.lobby.protected();
        let roll = VoteRoll { electorate: &electorate, members: &members, protected: &protected };
        match self.votes.submit(slot, vote, &roll, self.world.tick(), self.now_ms) {
            Ok(events) => self.apply_vote_events(events),
            Err(err) => {
                log::debug!("slot {slot}: vote refused: {err:?}");
                if vote.kind == VoteKind::Kick && err == nfsmw_net::vote::VoteError::BadTarget {
                    self.strike(peer, Violation::OutOfOrder);
                }
            }
        }
    }

    /// `BaselineLost`: the client's views are gone, so the next snapshot is full.
    fn on_baseline_lost(&mut self, peer: PeerId) {
        if let Some(state) = self.peers.get_mut(&peer) {
            state.resync();
        }
    }

    fn sender(&self, slot: u8) -> Option<Sender> {
        let member = self.lobby.member(slot)?;
        Some(Sender { slot, team: member.team, muted: member.muted })
    }

    fn on_chat(&mut self, slot: u8, send: &nfsmw_net::message::ChatSend) {
        let Some(sender) = self.sender(slot) else {
            return;
        };
        let members = self.lobby.slots();
        let tick = self.world.tick();
        match self.chat.send(sender, send, |to| members.contains(&to), tick, self.now_ms) {
            Ok(relay) => {
                if send.channel == ChatChannel::All {
                    log::info!("chat {slot}: {}", relay.message.text.as_str());
                }
                self.relay_chat(relay);
            }
            Err(refusal) => log::debug!("slot {slot}: chat refused: {refusal:?}"),
        }
    }

    /// A quick message. The catalogue has no server-to-client quick message yet, so it is checked and counted only.
    fn on_quick(&mut self, slot: u8, quick: &nfsmw_net::message::QuickMessage) {
        let Some(sender) = self.sender(slot) else {
            return;
        };
        if let Err(refusal) = self.chat.quick(sender, quick, self.now_ms) {
            log::debug!("slot {slot}: quick message refused: {refusal:?}");
        }
    }

    /// `ResetRequest`: the reset rule decides; a grant moves the car and is announced as `Respawn`.
    fn on_reset(&mut self, slot: u8) {
        let player = PlayerId(slot);
        let tick = self.world.tick();
        let state = self.reset_state(slot);
        let grant = match self.resets.request(player, tick, state) {
            Ok(grant) => grant,
            Err(refusal) => return log::debug!("slot {slot}: reset refused: {refusal}"),
        };
        if !self.world.reset_player(player) {
            return log::debug!("slot {slot}: the world cannot reset this car");
        }
        if let Some(mode) = self.race.mode.as_mut() {
            let input = ModeInput::Reset { player, penalty_ms: grant.penalty_ms };
            nfsmw_net::modes::ModeRules::feed(mode, tick, &input, &mut self.race.events);
            self.race.tally.feed(&input);
        }
        if let Some(peer) = self.slots.get(&slot).and_then(|peer| self.peers.get_mut(peer)) {
            peer.cause = nfsmw_net::schema::CorrectionCause::Reset;
        }
        let mut views = Vec::new();
        self.world.entities(&mut views);
        let Some(view) = views.iter().find(|view| view.owner == Some(player)) else {
            return;
        };
        let head = EventHead { seq: 0, tick, target_tick: 0 };
        let yaw = nfsmw_net::schema::yaw_of(view.orientation);
        let respawn = Respawn { head, entity_id: view.id, position: view.position, yaw, cause: RespawnCause::Reset };
        self.broadcast(Message::Respawn(respawn));
    }

    /// What the player is doing, for the reset rule.
    fn reset_state(&self, slot: u8) -> ResetState {
        let spectating = self.lobby.member(slot).is_none_or(|member| !member.is_player());
        if spectating {
            return ResetState::Spectating;
        }
        match self.lobby.wire_phase() {
            SessionPhase::Countdown | SessionPhase::Loading | SessionPhase::WaitLoads | SessionPhase::Intro => {
                ResetState::Countdown
            }
            SessionPhase::Results => ResetState::Finished,
            SessionPhase::Lobby => ResetState::Roaming,
            SessionPhase::Running => self.race.reset_state(PlayerId(slot)),
        }
    }
}
