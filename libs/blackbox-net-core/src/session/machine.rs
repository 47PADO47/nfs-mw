//! The session state machine: phase transitions, peer bookkeeping and the event queue.

use std::collections::{BTreeMap, VecDeque};

use super::{PeerId, PeerState, Phase, SessionConfig, SessionError, SessionEvent};

/// One session: the phase, the peers in id order, and the queue of events not yet polled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    config: SessionConfig,
    phase: Phase,
    peers: BTreeMap<PeerId, PeerState>,
    events: VecDeque<SessionEvent>,
    intro_start_ms: u64,
    load_deadline_ms: u64,
}

impl Session {
    /// A session in `Lobby` with no peers.
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            phase: Phase::Lobby,
            peers: BTreeMap::new(),
            events: VecDeque::new(),
            intro_start_ms: 0,
            load_deadline_ms: 0,
        }
    }

    /// The current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// The state of one peer, or `None` if it is not in the session.
    pub fn peer(&self, id: PeerId) -> Option<PeerState> {
        self.peers.get(&id).copied()
    }

    /// Every peer and its state, in id order.
    pub fn peers(&self) -> impl Iterator<Item = (PeerId, PeerState)> + '_ {
        self.peers.iter().map(|(id, state)| (*id, *state))
    }

    /// Adds a peer. In `Lobby` it enters `Lobby`. In `Running` it joins in progress and enters `InGame`. In any other
    /// phase the join is refused with `Closed`. A known id is `Duplicate`.
    pub fn join(&mut self, id: PeerId) -> Result<(), SessionError> {
        if self.peers.contains_key(&id) {
            return Err(SessionError::Duplicate);
        }
        let state = match self.phase {
            Phase::Lobby => PeerState::Lobby,
            Phase::Running => PeerState::InGame,
            _ => return Err(SessionError::Closed),
        };
        self.peers.insert(id, state);
        Ok(())
    }

    /// Removes a peer and queues `PeerLeft`. Allowed in every phase. An unknown id is `UnknownPeer`.
    pub fn leave(&mut self, id: PeerId) -> Result<(), SessionError> {
        if self.peers.remove(&id).is_none() {
            return Err(SessionError::UnknownPeer);
        }
        self.events.push_back(SessionEvent::PeerLeft(id));
        Ok(())
    }

    /// `Lobby` to `Loading`. Every peer enters `Loading`. The load deadline starts at `host_loaded`.
    pub fn start_loading(&mut self) -> Result<(), SessionError> {
        self.require(Phase::Lobby)?;
        for state in self.peers.values_mut() {
            *state = PeerState::Loading;
        }
        self.set_phase(Phase::Loading);
        Ok(())
    }

    /// The host has loaded. `Loading` to `WaitLoads`, with the load deadline at `now_ms + load_timeout_ms`.
    pub fn host_loaded(&mut self, now_ms: u64) -> Result<(), SessionError> {
        self.require(Phase::Loading)?;
        self.load_deadline_ms = now_ms.saturating_add(self.config.load_timeout_ms);
        self.set_phase(Phase::WaitLoads);
        Ok(())
    }

    /// A peer has loaded. Allowed in `Loading` and `WaitLoads`, since a peer can finish before the host does. A repeat
    /// is accepted and changes nothing.
    pub fn peer_loaded(&mut self, id: PeerId) -> Result<(), SessionError> {
        if !matches!(self.phase, Phase::Loading | Phase::WaitLoads) {
            return Err(SessionError::WrongPhase { phase: self.phase });
        }
        let Some(state) = self.peers.get_mut(&id) else {
            return Err(SessionError::UnknownPeer);
        };
        *state = PeerState::Loaded;
        Ok(())
    }

    /// Advances the timed transitions. Call every tick with the caller's time and the server tick.
    ///
    /// - `WaitLoads`: once every peer has loaded, or the deadline has passed (dropping the peers that have not loaded
    ///   with `PeerDropped`), the session goes to `Intro`, or straight to `Countdown` when `intro_ms` is zero. With zero
    ///   peers this happens at the first call.
    /// - `Intro`: after `intro_ms` it goes to `Countdown`.
    /// - `Countdown`: once `tick` reaches `start_tick` the session goes to `Running` and loaded peers become `InGame`.
    ///
    /// Each call makes at most one transition.
    pub fn update(&mut self, now_ms: u64, tick: u32) {
        match self.phase {
            Phase::WaitLoads => self.update_wait_loads(now_ms, tick),
            Phase::Intro => self.update_intro(now_ms, tick),
            Phase::Countdown { start_tick } => self.update_countdown(start_tick, tick),
            _ => {}
        }
    }

    /// `Running` to `Results`.
    pub fn finish(&mut self) -> Result<(), SessionError> {
        self.require(Phase::Running)?;
        self.set_phase(Phase::Results);
        Ok(())
    }

    /// `Results` to `Lobby`. Every peer enters `Lobby`.
    pub fn back_to_lobby(&mut self) -> Result<(), SessionError> {
        self.require(Phase::Results)?;
        for state in self.peers.values_mut() {
            *state = PeerState::Lobby;
        }
        self.set_phase(Phase::Lobby);
        Ok(())
    }

    /// The oldest queued event, if any.
    pub fn poll_event(&mut self) -> Option<SessionEvent> {
        self.events.pop_front()
    }

    fn require(&self, phase: Phase) -> Result<(), SessionError> {
        if self.phase != phase {
            return Err(SessionError::WrongPhase { phase: self.phase });
        }
        Ok(())
    }

    fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.events.push_back(SessionEvent::Phase(phase));
    }

    fn update_wait_loads(&mut self, now_ms: u64, tick: u32) {
        let all_loaded = self.peers.values().all(|state| *state == PeerState::Loaded);
        if !all_loaded && now_ms < self.load_deadline_ms {
            return;
        }
        self.drop_unloaded();
        self.begin_intro(now_ms, tick);
    }

    fn update_intro(&mut self, now_ms: u64, tick: u32) {
        if now_ms.saturating_sub(self.intro_start_ms) < self.config.intro_ms {
            return;
        }
        self.enter_countdown(tick);
    }

    fn update_countdown(&mut self, start_tick: u32, tick: u32) {
        if tick < start_tick {
            return;
        }
        for state in self.peers.values_mut() {
            if *state == PeerState::Loaded {
                *state = PeerState::InGame;
            }
        }
        self.set_phase(Phase::Running);
    }

    /// Removes every peer that has not loaded, in id order, queuing `PeerDropped` for each.
    fn drop_unloaded(&mut self) {
        let dropped: Vec<PeerId> =
            self.peers.iter().filter(|(_, state)| **state != PeerState::Loaded).map(|(id, _)| *id).collect();
        for id in dropped {
            self.peers.remove(&id);
            self.events.push_back(SessionEvent::PeerDropped(id));
        }
    }

    fn begin_intro(&mut self, now_ms: u64, tick: u32) {
        if self.config.intro_ms > 0 {
            self.intro_start_ms = now_ms;
            self.set_phase(Phase::Intro);
            return;
        }
        self.enter_countdown(tick);
    }

    fn enter_countdown(&mut self, tick: u32) {
        let start_tick = tick.saturating_add(self.config.countdown_ticks);
        self.set_phase(Phase::Countdown { start_tick });
    }
}
