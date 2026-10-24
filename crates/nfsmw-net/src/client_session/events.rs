//! What the session tells the game ([`SessionEvent`]), and the gate gameplay events pass: duplicates by `seq` are
//! dropped, and an event with a target tick waits until the server clock reaches it (sync-model.md section 5.4).

use nfsmw_sim::{EntityId, Tick};

use super::lobby::ChatLine;
use super::status::EndReason;
use crate::message::{
    CancelReason, EventHead, LoadWorld, Message, QuickRelay, RaceResults, StartCountdown, SystemNotice, VoteState,
};
use crate::reconcile::SnapReason;

/// Events with a target tick held at once; past this the oldest is released early.
const MAX_HELD: usize = 256;

/// Something the game should react to, in the order it happened.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    /// The server gave the seat: the slot and the entity id of the player's car. `resumed` after a reconnect.
    Admitted {
        slot: u8,
        entity: EntityId,
        resumed: bool,
    },
    /// The connection dropped; the session tries to resume the slot.
    ConnectionLost,
    /// The session is over.
    Ended(EndReason),
    /// The lobby roster, settings, phase or host changed ([`super::ClientSession::lobby`]).
    LobbyChanged,
    /// The late-join session data arrived (every member's loadout).
    SessionData,
    Chat(ChatLine),
    /// A preset line another player (or this one) said: the slot and the id into the game's table of lines.
    Quick(QuickRelay),
    Notice(SystemNotice),
    Countdown(StartCountdown),
    CountdownCancelled(CancelReason),
    /// Load the world for the next event; answer with [`super::ClientSession::load_complete`].
    LoadWorld(LoadWorld),
    /// The race starts at this server tick; the owner's input is held neutral until then.
    StartAt(Tick),
    Vote(VoteState),
    /// A gameplay event (protocol-messages.md 7.6) at its target tick: race, pursuit, damage, respawn, phase, players.
    Game(Message),
    /// A race result record (also kept in [`super::ClientSession::last_results`]).
    Results(RaceResults),
    /// The own car's drawn pose jumped (the camera should cut, not smear).
    Snapped(SnapReason),
}

/// The head of a gameplay event message, or `None` for any other message.
pub fn event_head(message: &Message) -> Option<EventHead> {
    let head = match message {
        Message::RaceStart(m) => m.head,
        Message::Checkpoint(m) => m.head,
        Message::LapComplete(m) => m.head,
        Message::RaceFinish(m) => m.head,
        Message::RaceResults(m) => m.head,
        Message::KnockedOut(m) => m.head,
        Message::TrapScore(m) => m.head,
        Message::TagPassed(m) => m.head,
        Message::Penalty(m) => m.head,
        Message::PursuitStart(m) => m.head,
        Message::PursuitEnd(m) => m.head,
        Message::HeatChange(m) => m.head,
        Message::Infraction(m) => m.head,
        Message::BustedProgress(m) => m.head,
        Message::RoadblockEvent(m) => m.head,
        Message::SpikeEvent(m) => m.head,
        Message::Collision(m) => m.head,
        Message::Damage(m) => m.head,
        Message::Puncture(m) => m.head,
        Message::Repair(m) => m.head,
        Message::Respawn(m) => m.head,
        Message::PhaseChange(m) => m.head,
        Message::EntityRemoved(m) => m.head,
        Message::PlayerJoined(m) => m.head,
        Message::PlayerLeft(m) => m.head,
        Message::RoleChanged(m) => m.head,
        Message::ClockEvent(m) => m.head,
        _ => return None,
    };
    Some(head)
}

/// Gameplay events by `seq` (per connection) and target tick.
#[derive(Debug, Clone, Default)]
pub struct EventGate {
    last_seq: Option<u32>,
    held: Vec<(Tick, Message)>,
    duplicates: u64,
}

impl EventGate {
    /// Takes event `message` with `head`. Returns it if it is due now (no target tick, or the server clock at
    /// `server_tick` has reached it); holds it otherwise; drops a `seq` already seen.
    pub fn accept(&mut self, head: EventHead, message: Message, server_tick: Option<f64>) -> Option<Message> {
        if self.last_seq.is_some_and(|last| head.seq <= last) {
            self.duplicates += 1;
            return None;
        }
        self.last_seq = Some(head.seq);
        if head.target_tick == 0 || is_due(head.target_tick, server_tick) {
            return Some(message);
        }
        if self.held.len() >= MAX_HELD {
            self.held.remove(0);
        }
        self.held.push((head.target_tick, message));
        None
    }

    /// The held events whose target tick the server clock has reached, oldest target first.
    pub fn release(&mut self, server_tick: Option<f64>, out: &mut Vec<Message>) {
        if self.held.is_empty() {
            return;
        }
        self.held.sort_by_key(|(target, _)| *target);
        let due = self.held.iter().take_while(|(target, _)| is_due(*target, server_tick)).count();
        out.extend(self.held.drain(..due).map(|(_, message)| message));
    }

    /// A new connection numbers its events from the start again; held events are kept.
    pub fn new_connection(&mut self) {
        self.last_seq = None;
    }

    /// Events dropped as duplicates.
    pub fn duplicates(&self) -> u64 {
        self.duplicates
    }

    /// Events waiting for their target tick.
    pub fn held(&self) -> usize {
        self.held.len()
    }
}

fn is_due(target: Tick, server_tick: Option<f64>) -> bool {
    server_tick.is_some_and(|now| now >= f64::from(target))
}
