//! Carrying out what the lobby and the vote box decided.

use std::collections::VecDeque;

use nfsmw_net::chat::notice::{self, VoteNotice};
use nfsmw_net::lobby::Action;
use nfsmw_net::message::{EventHead, Message, PhaseChange, PlayerJoined, PlayerLeft, RemoveReason, RoleChanged};
use nfsmw_net::vote::VoteEvent;

use super::Server;
use crate::catalog::CatalogRef;
use crate::control::WorldControl;

impl<W: WorldControl> Server<W> {
    /// Carries out `actions` in order; actions they cause run after them.
    pub(super) fn apply_actions(&mut self, actions: Vec<Action>) {
        let mut queue: VecDeque<Action> = actions.into();
        while let Some(action) = queue.pop_front() {
            queue.extend(self.apply_action(action));
        }
    }

    fn head(&self, target_tick: u32) -> EventHead {
        EventHead { seq: 0, tick: self.world.tick(), target_tick }
    }

    /// One action; returns the lobby actions it caused.
    fn apply_action(&mut self, action: Action) -> Vec<Action> {
        match action {
            Action::Send { to, message } => self.send_to_slot(to, message),
            Action::Broadcast(message) => self.broadcast(message),
            Action::Joined { slot } => {
                let Some(member) = self.lobby.member(slot) else {
                    return Vec::new();
                };
                let joined =
                    PlayerJoined { head: self.head(0), slot, player_id: member.player_id, name: member.name.clone() };
                self.broadcast(Message::PlayerJoined(joined));
            }
            Action::Left { slot, reason } => {
                self.broadcast(Message::PlayerLeft(PlayerLeft { head: self.head(0), slot, reason }));
            }
            Action::RoleChanged { slot, role } => {
                self.broadcast(Message::RoleChanged(RoleChanged { head: self.head(0), slot, role }));
            }
            Action::Phase { phase, target_tick } => {
                log::info!("phase {phase:?} (target tick {target_tick})");
                self.broadcast(Message::PhaseChange(PhaseChange { head: self.head(target_tick), phase }));
                return self.on_phase(phase, target_tick);
            }
            Action::Kick { slot, kick } => self.on_lobby_kick(slot, kick),
            Action::Spawn { slot } => self.spawn_car(slot, true),
            Action::Despawn { slot } => self.remove_car(slot, RemoveReason::Despawned),
            Action::Hold { on } => log::info!("everyone wait: {on}"),
            Action::EndSession => {
                log::info!("the host left: the session ends");
                self.ended = true;
            }
        }
        Vec::new()
    }

    /// Tells everyone about a vote and carries out a passed one.
    pub(super) fn apply_vote_events(&mut self, events: Vec<VoteEvent>) {
        for event in events {
            match event {
                VoteEvent::State(state) => self.broadcast(Message::VoteState(state)),
                VoteEvent::Passed { kind, option } => {
                    self.broadcast(Message::SystemNotice(notice::vote(kind, option, VoteNotice::Passed)));
                    let catalog = CatalogRef(self.setup.catalog.as_ref());
                    match self.lobby.apply_vote(kind, option, &catalog, self.world.tick()) {
                        Ok(actions) => self.apply_actions(actions),
                        Err(err) => log::info!("a passed vote could not be carried out: {err:?}"),
                    }
                }
                VoteEvent::Failed { kind, option } => {
                    self.broadcast(Message::SystemNotice(notice::vote(kind, option, VoteNotice::Failed)));
                }
                VoteEvent::Cancelled { kind, option } => {
                    self.broadcast(Message::SystemNotice(notice::vote(kind, option, VoteNotice::Cancelled)));
                }
            }
        }
    }
}
