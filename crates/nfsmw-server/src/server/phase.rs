//! The session phases on the server side: loading spawns the racers' cars, the race start builds the mode rules, the
//! end of a round sends the results, and the results screen returns to the lobby on its timer.

use nfsmw_net::fairness::ResetState;
use nfsmw_net::lobby::Action;
use nfsmw_net::message::{EventHead, List, Message, RaceStart, Respawn, RespawnCause, SessionPhase};
use nfsmw_net::modes::{AnyMode, CustomRules, ModeEvent, ModeRules, ModeSetup, messages};
use nfsmw_net::results::{RaceRecord, RecordMeta, Roster, Tally, build_record};
use nfsmw_sim::{PlayerId, SimEvent};

use super::Server;
use crate::control::{RaceRequest, WorldControl};

/// A world event as the mode rules take it: a speed trap pass is its own input, everything else passes through.
fn mode_input(event: &SimEvent) -> nfsmw_net::modes::ModeInput {
    match *event {
        SimEvent::SpeedTrapPassed { player, trap, kmh } => nfsmw_net::modes::ModeInput::TrapPass { player, trap, kmh },
        event => nfsmw_net::modes::ModeInput::Sim(event),
    }
}

/// The world race a session's settings ask for: the race modes with a track, with `slots` racing. None for free roam,
/// pursuit co-op and tag (they run in the open city) and for a race mode with no track chosen.
fn race_request(settings: &nfsmw_net::message::Settings, slots: &[u8]) -> Option<RaceRequest> {
    use nfsmw_net::message::Mode;
    let runs_a_race = matches!(settings.mode, Mode::Circuit | Mode::Sprint | Mode::Speedtrap | Mode::Knockout);
    if !runs_a_race || settings.track == 0 {
        return None;
    }
    Some(RaceRequest {
        mode: settings.mode,
        track: settings.track,
        laps: settings.laps,
        traffic_density: settings.traffic_density,
        players: slots.iter().copied().map(PlayerId).collect(),
    })
}

/// How long the server waits for the world's race to stand on its grid before it goes on without it, milliseconds.
const WORLD_RACE_PATIENCE_MS: u64 = 60_000;

/// Speed traps a speedtrap round has room for. The world checks each pass against the trap's own threshold, so the
/// round's thresholds are zero: only "better than the racer's best" is left for the rules.
const TRAP_SLOTS: usize = 32;

/// The running round: its rules, tally and roster, and what the server remembers of it.
#[derive(Debug, Default)]
pub(crate) struct RaceState {
    pub(crate) mode: Option<AnyMode>,
    pub(crate) tally: Tally,
    pub(crate) roster: Roster,
    /// The race start tick from the countdown.
    pub(crate) start_tick: Option<u32>,
    /// When the server began waiting for the world's race to stand on its grid, in server milliseconds: the load barrier
    /// is held until it does ([`WorldControl::race_staged`]).
    pub(crate) awaiting_world_since_ms: Option<u64>,
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
        self.finish_waiting_for_world();
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
                // A world race that was asked for must stand on its grid before the server counts as loaded, so the
                // countdown does not run over a race that is still loading.
                if self.start_world_race() {
                    self.race.awaiting_world_since_ms = Some(self.now_ms);
                    return Vec::new();
                }
                return self.server_loaded();
            }
            SessionPhase::Countdown => {
                self.race.start_tick = Some(target_tick);
                self.world.schedule_race_go(target_tick);
            }
            SessionPhase::Running => self.start_round(),
            SessionPhase::Results => self.end_round(),
            SessionPhase::Lobby => {
                self.race.awaiting_world_since_ms = None;
                self.world.stop_race();
                self.race.mode = None;
                self.race.start_tick = None;
                self.race.results_until_ms = None;
            }
            SessionPhase::WaitLoads | SessionPhase::Intro => {}
        }
        Vec::new()
    }

    /// Asks the world to run the session's race, for the modes that have a track. A world that cannot is not an error:
    /// the toy world has no races, and the round then runs on the events the world does raise.
    /// Returns whether the world took the race.
    fn start_world_race(&mut self) -> bool {
        let slots = self.lobby.racer_slots();
        let Some(request) = race_request(self.lobby.settings(), &slots) else {
            return false;
        };
        match self.world.start_race(&request) {
            Ok(()) => {
                log::info!("the world runs the race: track {:#010x}, {} racers", request.track, request.players.len());
                true
            }
            Err(why) => {
                log::info!("no world race: {why}");
                false
            }
        }
    }

    /// The server reports itself loaded to the lobby's barrier.
    fn server_loaded(&mut self) -> Vec<Action> {
        self.lobby.server_loaded(self.now_ms).unwrap_or_else(|err| {
            log::warn!("the load barrier refused the server: {err:?}");
            Vec::new()
        })
    }

    /// Reports the server loaded once the world's race stands on its grid, or after [`WORLD_RACE_PATIENCE_MS`].
    fn finish_waiting_for_world(&mut self) {
        let Some(since) = self.race.awaiting_world_since_ms else {
            return;
        };
        let patient = self.now_ms.saturating_sub(since) < WORLD_RACE_PATIENCE_MS;
        if patient && !self.world.race_staged() {
            return;
        }
        self.race.awaiting_world_since_ms = None;
        if !patient {
            log::warn!("the world's race did not stand on its grid in time; going on without waiting");
        }
        let actions = self.server_loaded();
        self.apply_actions(actions);
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
        let mut setup = ModeSetup::new(settings.mode, rules, players.clone(), start);
        if settings.mode == nfsmw_net::message::Mode::Speedtrap {
            setup.trap_thresholds_kmh = vec![0.0; TRAP_SLOTS];
        }
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
            let input = mode_input(event);
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

#[cfg(test)]
mod tests {
    use nfsmw_net::message::{Mode, ServerName, Settings};

    use super::*;

    fn settings(mode: Mode, track: u32) -> Settings {
        Settings { mode, track, laps: 3, traffic_density: 40, ..Settings::lan(ServerName::new("t").expect("a name")) }
    }

    #[test]
    fn a_race_mode_with_a_track_asks_the_world_for_the_race() {
        let request = race_request(&settings(Mode::Circuit, 0xABCD), &[0, 2, 5]).expect("a request");
        assert_eq!(request.mode, Mode::Circuit);
        assert_eq!((request.track, request.laps, request.traffic_density), (0xABCD, 3, 40));
        assert_eq!(request.players, [PlayerId(0), PlayerId(2), PlayerId(5)]);
        for mode in [Mode::Sprint, Mode::Speedtrap, Mode::Knockout] {
            assert!(race_request(&settings(mode, 7), &[0, 1]).is_some(), "{mode:?}");
        }
    }

    #[test]
    fn a_trap_pass_is_its_own_mode_input_and_other_events_pass_through() {
        use nfsmw_net::modes::ModeInput;
        let pass = SimEvent::SpeedTrapPassed { player: PlayerId(1), trap: 2, kmh: 190.0 };
        assert!(matches!(mode_input(&pass), ModeInput::TrapPass { player: PlayerId(1), trap: 2, .. }));
        let lap = SimEvent::LapCompleted { player: PlayerId(1), lap: 1, time_ms: 60_000 };
        assert!(matches!(mode_input(&lap), ModeInput::Sim(SimEvent::LapCompleted { .. })));
    }

    #[test]
    fn the_open_city_modes_and_a_missing_track_ask_for_nothing() {
        for mode in [Mode::FreeRoam, Mode::PursuitCoop, Mode::Tag] {
            assert!(race_request(&settings(mode, 7), &[0, 1]).is_none(), "{mode:?}");
        }
        assert!(race_request(&settings(Mode::Circuit, 0), &[0, 1]).is_none(), "track 0 is the whole city");
    }
}
