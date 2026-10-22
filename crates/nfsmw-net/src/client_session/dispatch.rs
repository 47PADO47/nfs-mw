//! Reliable messages from the server: decode, check the channel, and route to admission, the lobby mirror, chat,
//! bulk, or the gameplay event gate. Garbage, unknown types and client-only messages count toward the invalid-message
//! rule (protocol.md section 8.2), which ends the session past 16 in 10 s.

use super::ClientLink;
use super::events::{SessionEvent, event_head};
use super::lobby::ChatLine;
use super::session::ClientSession;
use super::status::EndReason;
use crate::drop::SessionData;
use crate::message::{BulkKind, Message, Role, decode};
use crate::predict::PredictedCar;

impl<L: ClientLink, C: PredictedCar + Clone> ClientSession<L, C> {
    pub(super) fn on_reliable(&mut self, channel: u8, bytes: &[u8]) {
        let Ok(message) = decode(bytes) else {
            self.bad_message();
            return;
        };
        if message.channel().index() != channel {
            self.bad_message();
            return;
        }
        if let Some(head) = event_head(&message) {
            self.on_game_event(head, message);
            return;
        }
        self.on_session_message(message);
    }

    fn on_session_message(&mut self, message: Message) {
        match message {
            Message::PakeStart(start) => self.on_pake_start(&start),
            Message::PakeConfirm(confirm) => self.on_pake_confirm(&confirm.tag),
            Message::JoinAccepted(accepted) => self.on_accepted(&accepted),
            Message::JoinRejected(rejected) => self.end(EndReason::Rejected(rejected)),
            Message::Kick(kick) => {
                self.lobby.kicked = Some(kick.clone());
                self.end(EndReason::Kicked(kick));
            }
            Message::LobbyState(state) => {
                self.lobby.apply_state(&state);
                self.after_roster_change();
            }
            Message::LobbyDelta(delta) => {
                for change in delta.changes.iter() {
                    self.lobby.apply_change(change);
                }
                self.after_roster_change();
            }
            Message::StartCountdown(countdown) => {
                self.lobby.countdown = Some(countdown.clone());
                self.events.push(SessionEvent::Countdown(countdown));
            }
            Message::CancelCountdown(cancel) => {
                self.lobby.countdown = None;
                self.events.push(SessionEvent::CountdownCancelled(cancel.reason));
            }
            Message::LoadWorld(load) => {
                self.lobby.load = Some(load.clone());
                self.events.push(SessionEvent::LoadWorld(load));
            }
            Message::StartAt(start) => {
                self.set_start_tick(start.start_tick);
                self.events.push(SessionEvent::StartAt(start.start_tick));
            }
            Message::VoteState(vote) => {
                self.lobby.vote = Some(vote.clone());
                self.events.push(SessionEvent::Vote(vote));
            }
            Message::ChatMessage(chat) => {
                let line = ChatLine::from(&chat);
                self.lobby.add_chat(line.clone());
                self.events.push(SessionEvent::Chat(line));
            }
            Message::SystemNotice(notice) => {
                self.lobby.add_notice(notice.clone());
                self.events.push(SessionEvent::Notice(notice));
            }
            Message::BulkStart(_) | Message::BulkChunk(_) | Message::BulkEnd(_) => self.on_bulk(&message),
            _ => self.bad_message(),
        }
    }

    /// One bulk message; a finished session-data transfer replaces the lobby and adds every loadout.
    fn on_bulk(&mut self, message: &Message) {
        let complete = match self.bulk.accept(message) {
            Ok(Some(complete)) => complete,
            Ok(None) => return,
            Err(_) => {
                self.bad_message();
                return;
            }
        };
        if complete.kind != BulkKind::SessionData {
            return;
        }
        let Ok(data) = SessionData::decode(&complete.data) else {
            self.bad_message();
            return;
        };
        self.lobby.apply_session_data(&data);
        self.after_roster_change();
        self.events.push(SessionEvent::SessionData);
    }

    /// A gameplay event: a race start fixes the start tick at once (so the owner's launch is held), then the gate
    /// drops duplicates and holds events until their target tick.
    fn on_game_event(&mut self, head: crate::message::EventHead, message: Message) {
        if let Message::RaceStart(_) = message
            && head.target_tick != 0
        {
            self.set_start_tick(head.target_tick);
        }
        if let Some(due) = self.gate.accept(head, message, self.clock.server_tick()) {
            self.apply_game_event(due);
        }
    }

    /// A gameplay event at its target tick: update the mirror, then hand it to the game.
    pub(super) fn apply_game_event(&mut self, message: Message) {
        match &message {
            Message::PhaseChange(change) => {
                self.lobby.phase = change.phase;
                self.events.push(SessionEvent::LobbyChanged);
            }
            Message::PlayerLeft(left) => {
                self.lobby.remove(left.slot);
                let racers = self.lobby.racers();
                self.camera.target_left(left.slot, &racers);
                self.events.push(SessionEvent::LobbyChanged);
            }
            Message::RoleChanged(changed) => {
                self.lobby.set_role(changed.slot, changed.role);
                self.after_roster_change();
            }
            Message::RaceResults(results) => {
                self.results = Some(results.clone());
                self.events.push(SessionEvent::Results(results.clone()));
            }
            _ => {}
        }
        self.events.push(SessionEvent::Game(message));
    }

    /// The roster changed: a spectator stops predicting, and a followed racer who left or watches is replaced.
    fn after_roster_change(&mut self) {
        if self.lobby.my_role() == Role::Spectator {
            self.own.forget();
        }
        let racers = self.lobby.racers();
        if let crate::spectate::Camera::Follow(slot) = self.camera.camera()
            && !racers.contains(&slot)
        {
            self.camera.target_left(slot, &racers);
        }
        self.events.push(SessionEvent::LobbyChanged);
    }

    fn set_start_tick(&mut self, tick: u32) {
        self.lobby.start_tick = Some(tick);
        self.own.set_start_tick(Some(tick));
    }

    /// Counts an invalid message; past the limit the session ends.
    pub(super) fn bad_message(&mut self) {
        self.counters.bad_messages += 1;
        if self.invalid.record(self.now_ms) {
            self.end(EndReason::Protocol);
        }
    }
}
