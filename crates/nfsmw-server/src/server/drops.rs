//! Connections that end (MP-50), slots held for a resume (MP-51), kicks, and the timers that drive them.

use blackbox_net_core::endpoint::{DisconnectReason, reason};
use blackbox_net_core::session::PeerId;
use nfsmw_net::chat::notice;
use nfsmw_net::drop::{DropAction, DropCause, DropContext, on_drop, on_expiry};
use nfsmw_net::message::{EntityRemoved, EventHead, Kick, KickReason, Message, RemoveReason};
use nfsmw_net::modes::{ModeInput, ModeRules};
use nfsmw_sim::PlayerId;

use super::Server;
use super::admit::CLOSE_DELAY_MS;
use super::peer::Stage;
use crate::control::WorldControl;

impl<W: WorldControl> Server<W> {
    /// The close timers, silent players and ended resume windows.
    pub(super) fn run_timers(&mut self) {
        let now = self.now_ms;
        let due: Vec<(PeerId, u8)> = self
            .peers
            .values()
            .filter_map(|peer| match peer.stage {
                Stage::Closing { at_ms, reason } if at_ms <= now => Some((peer.id, reason)),
                _ => None,
            })
            .collect();
        for (peer, reason) in due {
            self.endpoint.disconnect(peer, reason);
            if let Some(state) = self.peers.get_mut(&peer) {
                state.stage = Stage::Closing { at_ms: u64::MAX, reason };
            }
        }
        for slot in self.presence.take_timed_out(now) {
            let Some(peer) = self.slots.get(&slot).copied() else {
                continue;
            };
            log::info!("slot {slot} timed out");
            self.stats.timeouts += 1;
            self.close(peer, reason::TIMEOUT, 0);
            self.drop_slot(slot, DropCause::Timeout);
        }
        for slot in self.book.take_expired(now) {
            if self.slots.contains_key(&slot) {
                continue;
            }
            log::info!("slot {slot}: the resume window ended");
            let context = self.drop_context(slot, false);
            self.apply_drop_actions(on_expiry(slot, &context));
        }
    }

    /// The net thread reports `peer` gone.
    pub(super) fn on_disconnected(&mut self, peer: PeerId, why: DisconnectReason) {
        let Some(state) = self.peers.remove(&peer) else {
            return;
        };
        log::info!("connection {} closed: {why}", peer.0);
        let Stage::Joined(slot) = state.stage else {
            return;
        };
        if self.slots.get(&slot) != Some(&peer) {
            return;
        }
        self.slots.remove(&slot);
        let cause = match why {
            DisconnectReason::Timeout | DisconnectReason::Overflow => DropCause::Timeout,
            _ => DropCause::Quit,
        };
        self.drop_slot(slot, cause);
    }

    /// Starts closing `peer` with `reason` after `delay_ms` (0 for now); it leaves its slot at once and everything it
    /// sends from now on is ignored.
    pub(super) fn close(&mut self, peer: PeerId, reason: u8, delay_ms: u64) {
        let Some(state) = self.peers.get_mut(&peer) else {
            return;
        };
        if let Stage::Joined(slot) = state.stage
            && self.slots.get(&slot) == Some(&peer)
        {
            self.slots.remove(&slot);
        }
        state.stage = Stage::Closing { at_ms: self.now_ms.saturating_add(delay_ms), reason };
        state.bulk = None;
        if delay_ms > 0 {
            return;
        }
        self.endpoint.disconnect(peer, reason);
        state.stage = Stage::Closing { at_ms: u64::MAX, reason };
    }

    /// Kicks `peer` for violations (security-gameplay.md 5.2): a `Kick`, a notice, and its slot released.
    pub(super) fn kick_for_violations(&mut self, peer: PeerId) {
        let Some(state) = self.peers.get(&peer) else {
            return;
        };
        if state.closing() {
            return;
        }
        let slot = state.slot();
        log::warn!("connection {} kicked for violations", peer.0);
        self.stats.kicks += 1;
        self.send_to_peer(peer, Message::Kick(Kick { reason: KickReason::Violations, ban_minutes: 0 }));
        self.close(peer, reason::KICKED, CLOSE_DELAY_MS);
        let Some(slot) = slot else {
            return;
        };
        self.broadcast(Message::SystemNotice(notice::kicked(slot, None)));
        self.drop_slot(slot, DropCause::Kicked);
    }

    /// The lobby kicked `slot` (it removed the member already): tell the player and close the connection.
    pub(super) fn on_lobby_kick(&mut self, slot: u8, kick: Kick) {
        self.stats.kicks += 1;
        if let Some(peer) = self.slots.get(&slot).copied() {
            self.send_to_peer(peer, Message::Kick(kick));
            self.close(peer, reason::KICKED, CLOSE_DELAY_MS);
        }
        self.forget_slot(slot);
    }

    /// `slot` lost its connection for `cause`: hold it for a resume or release it.
    fn drop_slot(&mut self, slot: u8, cause: DropCause) {
        let resumable = cause == DropCause::Timeout && self.book.hold(slot, self.now_ms).is_some();
        self.presence.forget(slot);
        let context = self.drop_context(slot, resumable);
        self.apply_drop_actions(on_drop(slot, cause, &context, self.now_ms));
    }

    fn drop_context(&self, slot: u8, resumable: bool) -> DropContext {
        let player = PlayerId(slot);
        DropContext {
            has_car: self.frames.car_of(slot).is_some(),
            racing: self.race.is_racing(player),
            in_pursuit: self.race.tally.get(player).is_some_and(|stats| stats.in_pursuit()),
            resumable,
        }
    }

    /// Carries out the drop rules' actions.
    pub(super) fn apply_drop_actions(&mut self, actions: Vec<DropAction>) {
        for action in actions {
            self.apply_drop_action(action);
        }
    }

    fn apply_drop_action(&mut self, action: DropAction) {
        match action {
            DropAction::GhostCar { slot } => {
                if !self.world.set_ghost(PlayerId(slot), true) {
                    log::debug!("slot {slot}: the world has no ghost cars; the car parks with neutral input");
                }
            }
            DropAction::RemoveCar { slot } => self.remove_car(slot, RemoveReason::Left),
            DropAction::LeavePursuit { slot } => log::debug!("slot {slot} leaves its pursuit"),
            DropAction::DidNotFinish { slot } => self.feed_mode(ModeInput::Left { player: PlayerId(slot) }),
            DropAction::HoldSlot { slot, until_ms } => {
                log::info!("slot {slot} held for a resume until {until_ms} ms");
                if let Ok(actions) = self.lobby.set_away(slot, true) {
                    self.apply_actions(actions);
                }
            }
            DropAction::ReleaseSlot { slot, reason } => {
                if let Ok(actions) = self.lobby.remove(slot, reason) {
                    self.apply_actions(actions);
                }
                self.forget_slot(slot);
            }
            DropAction::RestoreCar { slot } => {
                self.world.set_ghost(PlayerId(slot), false);
                self.stats.resumes += 1;
                if let Ok(actions) = self.lobby.set_away(slot, false) {
                    self.apply_actions(actions);
                }
            }
            DropAction::FullSnapshot { slot } => {
                if let Some(peer) = self.slots.get(&slot).and_then(|peer| self.peers.get_mut(peer)) {
                    peer.resync();
                }
            }
            DropAction::Notice(notice) => self.broadcast(Message::SystemNotice(notice)),
        }
    }

    /// Forgets a slot's chat bucket, vote, presence, resume token and reset timer.
    fn forget_slot(&mut self, slot: u8) {
        self.chat.forget(slot);
        let events = self.votes.leave(slot);
        self.apply_vote_events(events);
        self.presence.forget(slot);
        self.book.release(slot);
        self.resets.forget(PlayerId(slot));
    }

    /// Takes `slot`'s car out of the world and tells everyone.
    pub(super) fn remove_car(&mut self, slot: u8, why: RemoveReason) {
        let Some(entity_id) = self.frames.car_of(slot) else {
            return;
        };
        self.world.remove_player(PlayerId(slot));
        self.frames.forget_car(slot);
        self.race.roster.remove(PlayerId(slot));
        let head = EventHead { seq: 0, tick: self.world.tick(), target_tick: 0 };
        self.broadcast(Message::EntityRemoved(EntityRemoved { head, entity_id, reason: why }));
    }

    /// Feeds one session fact to the running mode and the tally.
    pub(super) fn feed_mode(&mut self, input: ModeInput) {
        let tick = self.world.tick();
        let Some(mode) = self.race.mode.as_mut() else {
            return;
        };
        mode.feed(tick, &input, &mut self.race.events);
        self.race.tally.feed(&input);
    }
}
