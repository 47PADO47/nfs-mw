//! Starting, holding, loading, running and ending a race.

use blackbox_net_core::session::{Phase, SessionEvent};

use super::roster::peer;
use super::{Action, Lobby, LobbyError, NO_SPAWN_SLOT};
use crate::message::{CancelReason, LoadWorld, Message, Role, SessionPhase, StartAt, StartCountdown};
use crate::roles::{Command, permits};

/// Server ticks per second.
const TICKS_PER_SECOND: u32 = 60;

impl Lobby {
    /// The host or an admin starts the countdown: in the lobby, not held, with enough racers, all ready.
    pub fn start(&mut self, slot: u8, tick: u32) -> Result<Vec<Action>, LobbyError> {
        self.require(slot, Command::Start)?;
        if self.countdown.is_some() {
            return Err(LobbyError::CountdownRunning);
        }
        self.can_start()?;
        Ok(self.begin_countdown(tick))
    }

    /// The host or an admin cancels the countdown.
    pub fn cancel(&mut self, slot: u8) -> Result<Vec<Action>, LobbyError> {
        self.require(slot, Command::Cancel)?;
        if self.countdown.is_none() {
            return Err(LobbyError::NoCountdown);
        }
        Ok(self.cancel_countdown(CancelReason::HostCancelled))
    }

    /// The host or an admin turns "everyone wait" on or off (MP-54), in the lobby or on the results screen. The sim
    /// never pauses: a hold only stops the race from starting. Holding cancels a countdown.
    pub fn hold(&mut self, slot: u8, on: bool) -> Result<Vec<Action>, LobbyError> {
        self.require(slot, Command::Hold)?;
        if !matches!(self.session.phase(), Phase::Lobby | Phase::Results) {
            return Err(LobbyError::WrongPhase);
        }
        if self.hold.is_some() == on {
            return Ok(Vec::new());
        }
        let previous = self.hold.take();
        let mut actions = self.cancel_countdown(CancelReason::HostCancelled);
        if on {
            self.hold = Some(slot);
        }
        actions.push(Action::Hold { on });
        actions.push(self.update_row(previous.unwrap_or(slot)));
        Ok(actions)
    }

    /// Advances the start countdown and the phase machine. Call every tick.
    pub fn update(&mut self, now_ms: u64, tick: u32) -> Vec<Action> {
        let mut actions = Vec::new();
        if self.countdown.is_some_and(|start| tick >= start) {
            self.countdown = None;
            actions.extend(self.begin_loading());
        }
        self.session.update(now_ms, tick);
        while let Some(event) = self.session.poll_event() {
            actions.extend(self.on_session_event(event));
        }
        actions
    }

    /// `LoadComplete` from a racer.
    pub fn load_complete(&mut self, slot: u8) -> Result<Vec<Action>, LobbyError> {
        let member = self.members.get(&slot).ok_or(LobbyError::UnknownSlot)?;
        if !member.is_player() {
            return Ok(Vec::new());
        }
        self.session.peer_loaded(peer(slot)).map_err(LobbyError::Session)?;
        let member = self.members.get_mut(&slot).ok_or(LobbyError::UnknownSlot)?;
        member.loading = false;
        Ok(vec![self.update_row(slot)])
    }

    /// The server's own world finished loading: `WaitLoads`, and the load barrier's deadline starts.
    pub fn server_loaded(&mut self, now_ms: u64) -> Result<Vec<Action>, LobbyError> {
        self.session.host_loaded(now_ms).map_err(LobbyError::Session)?;
        Ok(self.drain())
    }

    /// The race ended: `Running` to `Results`.
    pub fn finish(&mut self) -> Result<Vec<Action>, LobbyError> {
        self.session.finish().map_err(LobbyError::Session)?;
        Ok(self.drain())
    }

    /// `Results` to `Lobby`: everyone is un-readied, and spectators waiting for a race take their seats while there
    /// are racer places, in slot order. Everyone gets the whole lobby again.
    pub fn back_to_lobby(&mut self) -> Result<Vec<Action>, LobbyError> {
        self.session.back_to_lobby().map_err(LobbyError::Session)?;
        let mut actions = self.drain();
        let waiting: Vec<u8> = self
            .members
            .values()
            .filter(|member| !member.is_player() && member.wants != Role::Spectator && !member.away)
            .map(|member| member.slot)
            .collect();
        for slot in waiting {
            if self.racers() >= usize::from(self.settings.max_players) {
                break;
            }
            if self.session.join(peer(slot)).is_err() {
                continue;
            }
            let Some(member) = self.members.get_mut(&slot) else {
                continue;
            };
            member.role = member.wants;
            actions.push(Action::RoleChanged { slot, role: member.role });
        }
        for member in self.members.values_mut() {
            member.ready = false;
            member.loading = false;
        }
        actions.push(Action::Broadcast(Message::LobbyState(self.state())));
        Ok(actions)
    }

    /// Ends the race and starts it again (a host command or a passed vote): back to the lobby, every racer with a car
    /// ready, and the countdown begins. Not while held.
    pub(super) fn restart(&mut self, tick: u32) -> Result<Vec<Action>, LobbyError> {
        if self.hold.is_some() {
            return Err(LobbyError::Held);
        }
        let mut actions = Vec::new();
        if self.session.phase() == Phase::Running {
            actions.extend(self.finish()?);
        }
        if self.session.phase() != Phase::Results {
            return Err(LobbyError::WrongPhase);
        }
        actions.extend(self.back_to_lobby()?);
        for member in self.members.values_mut() {
            member.ready = member.is_player() && member.loadout.is_some() && !member.away;
        }
        actions.push(Action::Broadcast(Message::LobbyState(self.state())));
        actions.extend(self.begin_countdown(tick));
        Ok(actions)
    }

    /// Whether the lobby may start: the phase, the hold, the racer count and everyone ready.
    pub(super) fn can_start(&self) -> Result<(), LobbyError> {
        if self.session.phase() != Phase::Lobby {
            return Err(LobbyError::WrongPhase);
        }
        if self.hold.is_some() {
            return Err(LobbyError::Held);
        }
        let racers: Vec<_> = self.members.values().filter(|member| member.is_player() && !member.away).collect();
        if racers.len() < usize::from(self.config.min_racers.max(1)) {
            return Err(LobbyError::TooFewRacers);
        }
        if !racers.iter().all(|member| member.ready) {
            return Err(LobbyError::NotAllReady);
        }
        Ok(())
    }

    /// Fixes the end of the start countdown at `tick` plus the configured seconds and tells everyone.
    pub(super) fn begin_countdown(&mut self, tick: u32) -> Vec<Action> {
        let seconds = self.config.countdown_seconds;
        let start_tick = tick.saturating_add(u32::from(seconds) * TICKS_PER_SECOND);
        self.countdown = Some(start_tick);
        vec![Action::Broadcast(Message::StartCountdown(StartCountdown { start_tick, seconds }))]
    }

    /// The sender's rank allows `command`.
    pub(super) fn require(&self, slot: u8, command: Command) -> Result<(), LobbyError> {
        let member = self.members.get(&slot).ok_or(LobbyError::UnknownSlot)?;
        if !permits(member.rank, command) {
            return Err(LobbyError::NotPermitted);
        }
        Ok(())
    }

    /// `Lobby` to `Loading`: every connected member gets `LoadWorld`, racers with their grid slot in slot order.
    fn begin_loading(&mut self) -> Vec<Action> {
        if self.session.start_loading().is_err() {
            return Vec::new();
        }
        let mut actions = self.drain();
        let mut grid = 0u8;
        let slots: Vec<u8> = self.members.keys().copied().collect();
        for slot in slots {
            let Some(member) = self.members.get_mut(&slot) else {
                continue;
            };
            if member.away {
                continue;
            }
            member.loading = member.is_player();
            let spawn_slot = match member.is_player() {
                true => grid,
                false => NO_SPAWN_SLOT,
            };
            grid += u8::from(member.is_player());
            actions.push(Action::Send { to: slot, message: Message::LoadWorld(self.load_world(spawn_slot)) });
        }
        actions
    }

    fn load_world(&self, spawn_slot: u8) -> LoadWorld {
        let settings = &self.settings;
        LoadWorld {
            track: settings.track,
            mode: settings.mode,
            laps: settings.laps,
            time_of_day: self.config.time_of_day % (24 * 60),
            weather: self.config.weather,
            traffic_density: settings.traffic_density,
            cop_level_cap: settings.cop_level_cap,
            pursuits: settings.pursuits,
            spawn_slot,
        }
    }

    /// Every queued phase-machine event as actions.
    fn drain(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();
        while let Some(event) = self.session.poll_event() {
            actions.extend(self.on_session_event(event));
        }
        actions
    }

    fn on_session_event(&mut self, event: SessionEvent) -> Vec<Action> {
        match event {
            SessionEvent::Phase(phase) => {
                let wire = SessionPhase::from(phase);
                let Phase::Countdown { start_tick } = phase else {
                    return vec![Action::Phase { phase: wire, target_tick: 0 }];
                };
                vec![
                    Action::Phase { phase: wire, target_tick: start_tick },
                    Action::Broadcast(Message::StartAt(StartAt { start_tick })),
                ]
            }
            SessionEvent::PeerDropped(peer) => self.load_timed_out(peer.0),
            SessionEvent::PeerLeft(_) => Vec::new(),
        }
    }

    /// A racer still loading at the barrier's deadline watches this race and races the next one.
    fn load_timed_out(&mut self, peer: u16) -> Vec<Action> {
        let Ok(slot) = u8::try_from(peer) else {
            return Vec::new();
        };
        let Some(member) = self.members.get_mut(&slot) else {
            return Vec::new();
        };
        member.wants = member.role;
        member.role = Role::Spectator;
        member.loading = false;
        let cancel = crate::message::CancelCountdown { reason: CancelReason::LoadTimeout };
        vec![
            Action::Send { to: slot, message: Message::CancelCountdown(cancel) },
            Action::Despawn { slot },
            Action::RoleChanged { slot, role: Role::Spectator },
            self.update_row(slot),
        ]
    }
}
