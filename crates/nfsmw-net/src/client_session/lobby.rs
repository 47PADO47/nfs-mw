//! The client's mirror of the lobby: what the server said about the roster, the settings, the phase, the countdown, the
//! chat, votes and notices. Plain data the lobby screens read.

use std::collections::VecDeque;

use nfsmw_sim::Tick;

use crate::drop::{LoadoutRow, SessionData};
use crate::message::{
    ChatChannel, ChatMessage, Kick, LoadWorld, LobbyChange, LobbyPlayer, LobbyState, Role, SessionPhase, Settings,
    StartCountdown, SystemNotice, VoteState,
};

/// A chat line as the lobby shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatLine {
    pub from_slot: u8,
    pub channel: ChatChannel,
    pub text: String,
    pub tick: Tick,
}

impl From<&ChatMessage> for ChatLine {
    fn from(message: &ChatMessage) -> Self {
        Self {
            from_slot: message.from_slot,
            channel: message.channel,
            text: message.text.as_str().to_owned(),
            tick: message.tick,
        }
    }
}

/// The lobby as the server described it.
#[derive(Debug, Clone, PartialEq)]
pub struct LobbyView {
    /// The roster, sorted by slot.
    pub players: Vec<LobbyPlayer>,
    pub settings: Option<Settings>,
    pub phase: SessionPhase,
    pub host_slot: u8,
    /// This client's slot, once admitted.
    pub my_slot: Option<u8>,
    /// The running start countdown, until it is cancelled or the race starts.
    pub countdown: Option<StartCountdown>,
    /// The last `LoadWorld`.
    pub load: Option<LoadWorld>,
    /// The race start tick of the last `StartAt` or `RaceStart`.
    pub start_tick: Option<Tick>,
    /// The newest chat lines, oldest first.
    pub chat: VecDeque<ChatLine>,
    /// The newest system notices, oldest first.
    pub notices: VecDeque<SystemNotice>,
    /// The vote in progress.
    pub vote: Option<VoteState>,
    /// Every member's loadout, from the late-join session data.
    pub loadouts: Vec<LoadoutRow>,
    /// The kick that ended this client's session, if one did.
    pub kicked: Option<Kick>,
    keep: usize,
}

impl LobbyView {
    /// An empty lobby that keeps `keep` chat lines and notices.
    pub fn new(keep: usize) -> Self {
        Self {
            players: Vec::new(),
            settings: None,
            phase: SessionPhase::Lobby,
            host_slot: 0,
            my_slot: None,
            countdown: None,
            load: None,
            start_tick: None,
            chat: VecDeque::new(),
            notices: VecDeque::new(),
            vote: None,
            loadouts: Vec::new(),
            kicked: None,
            keep: keep.max(1),
        }
    }

    /// The row of `slot`.
    pub fn player(&self, slot: u8) -> Option<&LobbyPlayer> {
        self.players.iter().find(|row| row.slot == slot)
    }

    /// This client's row.
    pub fn me(&self) -> Option<&LobbyPlayer> {
        self.player(self.my_slot?)
    }

    /// This client's role (a racer until the roster says otherwise).
    pub fn my_role(&self) -> Role {
        self.me().map_or(Role::Racer, |row| row.role)
    }

    /// Whether this client is the host.
    pub fn is_host(&self) -> bool {
        self.my_slot == Some(self.host_slot)
    }

    /// The slots of the racers, ascending: what a spectator camera may follow.
    pub fn racers(&self) -> Vec<u8> {
        self.players.iter().filter(|row| row.role == Role::Racer).map(|row| row.slot).collect()
    }

    /// The loadout of `slot`, from the session data.
    pub fn loadout(&self, slot: u8) -> Option<&LoadoutRow> {
        self.loadouts.iter().find(|row| row.slot == slot)
    }

    /// Replaces the whole lobby.
    pub fn apply_state(&mut self, state: &LobbyState) {
        self.players = state.players.iter().cloned().collect();
        self.players.sort_by_key(|row| row.slot);
        self.settings = Some(state.settings.clone());
        self.phase = state.phase;
        self.host_slot = state.host_slot;
    }

    /// Applies one change of the roster.
    pub fn apply_change(&mut self, change: &LobbyChange) {
        match change {
            LobbyChange::Add(row) | LobbyChange::Update(row) => self.upsert(row.clone()),
            LobbyChange::Remove { slot } => self.remove(*slot),
        }
    }

    /// Adds or replaces the row of `row.slot`.
    pub fn upsert(&mut self, row: LobbyPlayer) {
        match self.players.binary_search_by_key(&row.slot, |other| other.slot) {
            Ok(index) => self.players[index] = row,
            Err(index) => self.players.insert(index, row),
        }
    }

    /// Removes the row of `slot` and its loadout.
    pub fn remove(&mut self, slot: u8) {
        self.players.retain(|row| row.slot != slot);
        self.loadouts.retain(|row| row.slot != slot);
    }

    /// Sets the role of `slot`, if it has a row.
    pub fn set_role(&mut self, slot: u8, role: Role) {
        if let Some(row) = self.players.iter_mut().find(|row| row.slot == slot) {
            row.role = role;
        }
    }

    /// Takes the late-join session data: the lobby and every loadout.
    pub fn apply_session_data(&mut self, data: &SessionData) {
        self.apply_state(&data.lobby);
        self.loadouts = data.loadouts.iter().cloned().collect();
        self.loadouts.sort_by_key(|row| row.slot);
    }

    pub fn add_chat(&mut self, line: ChatLine) {
        if self.chat.len() >= self.keep {
            self.chat.pop_front();
        }
        self.chat.push_back(line);
    }

    pub fn add_notice(&mut self, notice: SystemNotice) {
        if self.notices.len() >= self.keep {
            self.notices.pop_front();
        }
        self.notices.push_back(notice);
    }
}
