//! The session phases on the server side: loading spawns the racers' cars, the race start builds the mode rules, the
//! end of a round sends the results, and the results screen returns to the lobby on its timer.

use nfsmw_net::fairness::ResetState;
use nfsmw_net::lobby::Action;
use nfsmw_net::message::{EventHead, List, Message, RaceStart, Respawn, RespawnCause, SessionPhase};
use nfsmw_net::modes::{AnyMode, CustomRules, ModeEvent, ModeRules, ModeSetup, messages};
use nfsmw_net::results::{RaceRecord, RecordMeta, Roster, Tally, build_record};
use nfsmw_sim::{PlayerId, SimEvent};

use super::Server;
use crate::control::WorldControl;

/// The running round: its rules, tally and roster, and what the server remembers of it.
#[derive(Debug, Default)]
pub(crate) struct RaceState {
    pub(crate) mode: Option<AnyMode>,
    pub(crate) tally: Tally,
    pub(crate) roster: Roster,
    /// The race start tick from the countdown.
    pub(crate) start_tick: Option<u32>,
    /// When the results screen goes back to the lobby.
    pub(crate) results_until_ms: Option<u64>,
    /// Mode events not handled yet.
    pub(crate) events: Vec<ModeEvent>,
    pub(crate) finished: Vec<PlayerId>,
    pub(crate) busted: Vec<PlayerId>,
    pub(crate) last_record: Option<RaceRecord>,
    pub(crate) rounds: u32,
}

impl RaceState {
    /// Whether the mode has a race clock (resets cost time, a drop is a DNF).
    fn is_race(&self) -> bool {
        matches!(self.mode, Some(AnyMode::Race(_) | AnyMode::Speedtrap(_) | AnyMode::Knockout(_)))
    }

    /// Whether `player` races in the running round and has not finished.
    pub(crate) fn is_racing(&self, player: PlayerId) -> bool {
        self.is_race() && !self.finished.contains(&player)
    }

    /// The reset rule's view of `player` while the round runs.
    pub(crate) fn reset_state(&self, player: PlayerId) -> ResetState {
        if self.busted.contains(&player) {
            return ResetState::Busted;
        }
        if !self.is_race() {
            return ResetState::Roaming;
        }
        if self.finished.contains(&player) {
            return ResetState::Finished;
        }
        ResetState::Racing
    }
}

impl<W: WorldControl> Server<W> {
    /// The lobby's timers, the votes' end, and the results screen's end.
    pub(super) fn update_lobby(&mut self) {
        let tick = self.world.tick();
        let actions = self.lobby.update(self.now_ms, tick);
        self.apply_actions(actions);
        let events = self.votes.update(tick);
        self.apply_vote_events(events);
        if self.race.results_until_ms.is_some_and(|until| self.now_ms >= until) {
            self.race.results_until_ms = None;
            match self.lobby.back_to_lobby() {
                Ok(actions) => self.apply_actions(actions),
                Err(err) => log::warn!("cannot return to the lobby: {err:?}"),
            }
        }
    }

    /// The lobby entered `phase`; returns the lobby actions that follow.
    pub(super) fn on_phase(&mut self, phase: SessionPhase, target_tick: u32) -> Vec<Action> {
        match phase {
            SessionPhase::Loading => {
                for slot in self.lobby.racer_slots() {
                    self.spawn_car(slot, false);
                }
                return self.lobby.server_loaded(self.now_ms).unwrap_or_else(|err| {
                    log::warn!("the load barrier refused the server: {err:?}");
                    Vec::new()
                });
            }
            SessionPhase::Countdown => self.race.start_tick = Some(target_tick),
            SessionPhase::Running => self.start_round(),
            SessionPhase::Results => self.end_round(),
            SessionPhase::Lobby => {
                self.race.mode = None;
                self.race.start_tick = None;
                self.race.results_until_ms = None;
            }
            SessionPhase::WaitLoads | SessionPhase::Intro => {}
        }
        Vec::new()
    }

    /// Puts `slot`'s car in the world, unless it has one. `announce` sends the late joiner's `Respawn`.
    pub(super) fn spawn_car(&mut self, slot: u8, announce: bool) {
        if self.frames.car_of(slot).is_some() {
            return;
        }
        let Some(loadout) = self.lobby.member(slot).and_then(|member| member.loadout.clone()) else {
            return;
        };
        let player = PlayerId(slot);
        if let Err(err) = self.world.add_player(player, &loadout.to_join(None)) {
            return log::warn!("slot {slot}: no car: {err}");
        }
        self.frames.capture(&self.world);
        let Some(view) = self.frames.car_view(slot).copied() else {
            return;
        };
        self.race.roster.insert(nfsmw_net::results::Seat { player, slot, entity: view.id });
        if !announce {
            return;
        }
        let head = EventHead { seq: 0, tick: self.world.tick(), target_tick: 0 };
        let yaw = nfsmw_net::schema::yaw_of(view.orientation);
        let cause = RespawnCause::JoinInProgress;
        self.broadcast(Message::Respawn(Respawn { head, entity_id: view.id, position: view.position, yaw, cause }));
    }

    /// The race starts: the mode's rules for the racers, and `RaceStart` with the grid.
    fn start_round(&mut self) {
        let tick = self.world.tick();
        let start = self.race.start_tick.unwrap_or(tick);
        let players: Vec<PlayerId> = self.lobby.racer_slots().into_iter().map(PlayerId).collect();
        let settings = self.lobby.settings();
        let rules = CustomRules::from_settings(settings, nfsmw_net::lobby::LobbyConfig::default().time_of_day);
        let setup = ModeSetup::new(settings.mode, rules, players.clone(), start);
        let mut mode = match AnyMode::build(&setup) {
            Ok(mode) => mode,
            Err(err) => {
                log::error!("the mode cannot run: {err}");
                if let Ok(actions) = self.lobby.finish() {
                    self.apply_actions(actions);
                }
                return;
            }
        };
        self.race.tally = Tally::new();
        self.race.finished.clear();
        self.race.busted.clear();
        mode.begin(&mut self.race.events);
        self.race.mode = Some(mode);
        let mut order = List::empty();
        for entity in players.iter().filter_map(|player| self.race.roster.get(*player)).map(|seat| seat.entity) {
            let _ = order.push(entity);
        }
        let head = EventHead { seq: 0, tick, target_tick: start };
        self.broadcast(Message::RaceStart(RaceStart { head, order }));
        log::info!("round {} starts at tick {start}: {:?}, {} racers", self.race.rounds + 1, setup.mode, players.len());
    }

    /// The round is over: build its record, send `RaceResults`, start the results timer.
    fn end_round(&mut self) {
        self.race.results_until_ms = Some(self.now_ms + u64::from(self.config.results_seconds) * 1000);
        let Some(mode) = self.race.mode.take() else {
            return;
        };
        self.race.rounds += 1;
        let settings = self.lobby.settings();
        let tick = self.world.tick();
        let meta = RecordMeta {
            mode: settings.mode,
            track: settings.track,
            laps: settings.laps,
            start_tick: self.race.start_tick.unwrap_or(tick),
            end_tick: tick,
        };
        let record = build_record(meta, &mode.placings(), &self.race.tally, &self.race.roster);
        let head = EventHead { seq: 0, tick, target_tick: 0 };
        self.broadcast(Message::RaceResults(record.to_message(head)));
        if let Some(replay) = self.replay.as_mut() {
            replay.save_record(&record, self.race.rounds);
        }
        log::info!("round {} over: {} rows", self.race.rounds, record.rows.len());
        self.race.last_record = Some(record);
    }

    /// `ResultRequest`: the last record again.
    pub(super) fn resend_result(&mut self, slot: u8) {
        let Some(record) = self.race.last_record.as_ref() else {
            return;
        };
        let head = EventHead { seq: 0, tick: self.world.tick(), target_tick: 0 };
        let message = Message::RaceResults(record.to_message(head));
        self.send_to_slot(slot, message);
    }

    /// The world's events of a tick to the mode and the tally, then every car's sample.
    pub(super) fn feed_round(&mut self, events: &[SimEvent]) {
        let tick = self.world.tick().saturating_sub(1);
        for event in events {
            if let SimEvent::Busted { player } = event {
                self.race.busted.push(*player);
            }
        }
        let Some(mode) = self.race.mode.as_mut() else {
            return;
        };
        for event in events {
            let input = nfsmw_net::modes::ModeInput::Sim(*event);
            mode.feed(tick, &input, &mut self.race.events);
            self.race.tally.feed(&input);
        }
        for sample in self.frames.car_samples() {
            let input = nfsmw_net::modes::ModeInput::Car(sample);
            mode.feed(tick, &input, &mut self.race.events);
            self.race.tally.feed(&input);
        }
        mode.end_tick(tick, &mut self.race.events);
        self.race.tally.end_tick();
    }

    /// Sends the mode's events and acts on them: spectators, finishes, the end of the round.
    pub(super) fn handle_mode_events(&mut self) {
        let events = std::mem::take(&mut self.race.events);
        let tick = self.world.tick().saturating_sub(1);
        for event in events {
            for message in messages(&event, tick, &self.race.roster) {
                self.broadcast(message);
            }
            match event {
                ModeEvent::Finished { player, .. } => self.race.finished.push(player),
                ModeEvent::Spectate { player } => {
                    let reason = nfsmw_net::spectate::SpectateReason::Eliminated;
                    if let Ok(actions) = self.lobby.make_spectator(player.0, reason) {
                        self.apply_actions(actions);
                    }
                }
                ModeEvent::Over { reason } => {
                    log::info!("the round is over: {reason:?}");
                    match self.lobby.finish() {
                        Ok(actions) => self.apply_actions(actions),
                        Err(err) => log::warn!("cannot finish the round: {err:?}"),
                    }
                }
                _ => {}
            }
        }
    }
}
