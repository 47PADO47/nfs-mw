//! A world that runs its own race: the server asks for it when the lobby starts loading, holds its load barrier until the
//! race stands on its grid, and schedules GO on the tick its countdown begins.

use nfsmw_net::input::InputCode;
use nfsmw_net::message::{Message, SessionPhase};
use nfsmw_server::{RaceRequest, WorldControl};
use nfsmw_sim::toy::{ToyConfig, ToyWorld};
use nfsmw_sim::{EntityView, OwnState, PlayerId, PlayerJoin, SimError, SimEvent, SimInputs, Tick, WorldSim};

use super::harness::{Harness, test_config};

/// Ticks the pretend race takes to stand on its grid after it is asked for.
const LOADING_TICKS: u32 = 240;

/// The toy world with a race that stages after [`LOADING_TICKS`] ticks and notes what the server asked.
struct StagedWorld {
    toy: ToyWorld,
    asked_at: Option<Tick>,
    request: Option<RaceRequest>,
    go: Option<(Tick, Tick)>,
    stopped: u32,
}

impl WorldSim for StagedWorld {
    fn tick(&self) -> Tick {
        self.toy.tick()
    }
    fn add_player(&mut self, id: PlayerId, join: &PlayerJoin) -> Result<(), SimError> {
        self.toy.add_player(id, join)
    }
    fn remove_player(&mut self, id: PlayerId) {
        self.toy.remove_player(id);
    }
    fn step(&mut self, inputs: &SimInputs, events: &mut Vec<SimEvent>) {
        self.toy.step(inputs, events);
    }
    fn entities(&self, out: &mut Vec<EntityView>) {
        self.toy.entities(out);
    }
    fn own_state(&self, id: PlayerId) -> Option<OwnState> {
        self.toy.own_state(id)
    }
    fn state_hash(&self) -> u64 {
        self.toy.state_hash()
    }
}

impl WorldControl for StagedWorld {
    fn start_race(&mut self, request: &RaceRequest) -> Result<(), String> {
        self.asked_at = Some(self.toy.tick());
        self.request = Some(request.clone());
        Ok(())
    }

    fn race_staged(&self) -> bool {
        self.asked_at.is_none_or(|asked| self.toy.tick() >= asked + LOADING_TICKS)
    }

    fn schedule_race_go(&mut self, go_tick: Tick) {
        self.go = Some((self.toy.tick(), go_tick));
    }

    fn stop_race(&mut self) {
        self.stopped += 1;
    }
}

fn phase_of(h: &Harness<StagedWorld>, index: usize) -> Option<SessionPhase> {
    h.clients[index].messages.iter().rev().find_map(|message| match message {
        Message::PhaseChange(change) => Some(change.phase),
        _ => None,
    })
}

#[test]
fn the_load_barrier_waits_for_the_worlds_race_and_go_is_scheduled_on_the_countdown() {
    let mut config = test_config();
    for (key, value) in [("mode", "circuit"), ("laps", "1"), ("track", "4242")] {
        nfsmw_net::settings::set(&mut config.settings, key, value).expect("a valid setting");
    }
    let world =
        StagedWorld { toy: ToyWorld::new(7, ToyConfig::bare()), asked_at: None, request: None, go: None, stopped: 0 };
    let mut h = Harness::over(config, world);
    let racers: Vec<usize> = (0..2).map(|seed| h.connect(130 + seed)).collect();
    h.join_all(&racers);
    for &index in &racers {
        h.clients[index].ready(true);
    }
    h.until("the world is asked for the race", 600, InputCode::NEUTRAL, |h| h.server.world().request.is_some());
    let request = h.server.world().request.clone().expect("a request");
    assert_eq!((request.track, request.laps), (4242, 1));
    assert_eq!(request.players.len(), 2);
    let asked = h.server.world().asked_at.expect("asked");

    // While the race loads the barrier holds: nobody is counted down.
    h.run(LOADING_TICKS / 2, InputCode::NEUTRAL);
    assert!(h.server.world().go.is_none(), "no GO is scheduled while the race loads");
    assert!(
        !matches!(phase_of(&h, racers[0]), Some(SessionPhase::Countdown | SessionPhase::Running)),
        "the clients are not counted down over a race that is still loading"
    );

    // Once the race stands on its grid the barrier opens, and GO is scheduled when the countdown begins.
    h.until("the countdown", 900, InputCode::NEUTRAL, |h| h.server.world().go.is_some());
    let (scheduled_at, go) = h.server.world().go.expect("a GO");
    assert!(scheduled_at >= asked + LOADING_TICKS, "GO was scheduled only after the grid stood");
    assert!(go > scheduled_at, "GO falls after the countdown begins: {go} after {scheduled_at}");
    h.until("the race start", 900, InputCode::NEUTRAL, |h| h.clients[racers[0]].got(super::harness::is_running));
    assert!(h.server.world().tick() >= go, "the round starts at the scheduled tick");
}

#[test]
fn a_world_that_runs_no_races_leaves_the_flow_alone() {
    let mut config = test_config();
    for (key, value) in [("mode", "circuit"), ("laps", "1"), ("track", "4242")] {
        nfsmw_net::settings::set(&mut config.settings, key, value).expect("a valid setting");
    }
    let mut h = Harness::new(config, ToyConfig::bare());
    let me = h.connect(140);
    h.join_all(&[me]);
    h.start_race(&[me]);
}
