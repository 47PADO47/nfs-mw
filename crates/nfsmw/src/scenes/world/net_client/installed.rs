//! The client mode over the installed world: the real `Server` over a `WorldSimHost`, one client-mode world and one
//! scripted bot over loopback UDP, all on a fake 60 Hz clock in this thread. Ignored without `NFSMW_GAME_DIR`.

use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::{Duration, Instant};

use bevy_input::keyboard::KeyCode;
use blackbox_gfx::{Capabilities, GraphicsApi};
use blackbox_net_core::endpoint::ClientEndpoint;
use blackbox_net_core::secure::IdentityKey;
use game_install::GameDir;
use nfsmw_net::interp::DrawMode;
use nfsmw_net::message::SessionPhase;
use nfsmw_net::predict::{InputState, PredictedCar};
use nfsmw_server::bot::{Bot, BotConfig};
use nfsmw_server::{Server, ServerConfig};
use nfsmw_sim::{EntityKind, PlayerId, WorldSim};

use super::super::drive::STEP;
use super::super::net::catalog::car_type_id;
use super::super::net::host::{HostConfig, WorldSimHost, settle_streaming};
use super::super::{DriveOptions, Options, WorldScene};
use crate::app::null_backend::NullBackend;
use crate::input::{ActionState, Bindings};
use crate::net::client::{NetClient, PlayerSetup, endpoint_config};
use crate::viewer::Scene;

/// The car of every player.
const CAR: &str = "ELISE";
/// Where the session starts (`net/options.rs`): a road with room to drive (the determinism tests' road, 2151, 1399,
/// has an obstacle about 10 m away either way from where the car is placed).
const AT: [f32; 2] = [1945.0, 1850.0];
const HEADING: f32 = 45.0;
/// How long one tick may wait for streaming.
const SETTLE: Duration = Duration::from_secs(60);

fn dir() -> GameDir {
    GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap()
}

fn renderer() -> NullBackend {
    NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan))
}

/// A world that drives `CAR` from `AT`, as the front end's free roam opens it.
fn client_world(dir: &GameDir, renderer: &mut NullBackend) -> WorldScene {
    let options = Options {
        at: Some(AT),
        height: 40.0,
        heading: HEADING,
        pitch: -20.0,
        fog_distance: 1500.0,
        wait_for_load: true,
        drive: Some(DriveOptions { car: CAR.to_owned(), script: None }),
    };
    let mut scene = WorldScene::open(dir, options).unwrap();
    scene.init(renderer).unwrap();
    scene
}

/// The server over the real world, on loopback with an ephemeral port.
fn server(dir: &GameDir, cars: &[String]) -> (Server<WorldSimHost>, SocketAddr) {
    server_with(dir, cars, |_| {})
}

/// As [`server`], with the config changed by `configure` first (a race's mode and track).
fn server_with(
    dir: &GameDir,
    cars: &[String],
    configure: impl FnOnce(&mut ServerConfig),
) -> (Server<WorldSimHost>, SocketAddr) {
    let mut host = HostConfig::new(AT, CAR, 0x5EED);
    host.heading = HEADING;
    host.settle_streaming = Some(SETTLE);
    let world = WorldSimHost::open(dir, host).unwrap();
    let mut config = ServerConfig {
        bind: [127, 0, 0, 1].into(),
        port: 0,
        discovery: false,
        countdown_seconds: 1,
        status_seconds: 0,
        ..ServerConfig::default()
    };
    configure(&mut config);
    let endpoint = nfsmw_server::bind(&config, IdentityKey::generate().unwrap()).unwrap();
    let addr = endpoint.local_addr();
    let setup = crate::net::server_setup(cars, 0x5EED);
    (Server::new(config, setup, endpoint, world, 0).unwrap(), addr)
}

/// The throttle key held.
fn throttle() -> ActionState {
    let mut actions = ActionState::default();
    actions.update_for_test(&Bindings::default(), &[KeyCode::ArrowUp], &[], false);
    actions
}

/// Milliseconds of tick `tick` on the 60 Hz clock.
fn ms(tick: u64) -> u64 {
    tick * 1000 / 60
}

/// The bot `index` as the real world knows it: the same car as everyone and the install's data id.
fn bot_config(cars: &[String], index: u32) -> BotConfig {
    BotConfig {
        car_type: car_type_id(CAR),
        data_id: crate::net::data_id(cars),
        sim_build: crate::net::sim_build(),
        ..BotConfig::new(index, 7)
    }
}

/// The server, the client world and the bot, stepped together on the fake clock.
struct Rig {
    server: Server<WorldSimHost>,
    addr: SocketAddr,
    scene: WorldScene,
    renderer: NullBackend,
    client: Rc<RefCell<NetClient>>,
    bot: Option<Bot>,
    tick: u64,
    /// The wall clock the frames are paced to: the net threads run in real time, so the fake clock must not outrun
    /// them (a frame takes a few milliseconds of work, a datagram as long to cross the threads).
    started: Instant,
}

impl Rig {
    fn open() -> Self {
        let dir = dir();
        let cars = nfsmw_data::car::list(&dir);
        let (server, addr) = server(&dir, &cars);
        Self::join(&dir, &cars, server, addr, None, 0)
    }

    /// A client world that joins `server`, which has run `tick` ticks with `bot` (if any) seated before it.
    fn join(
        dir: &GameDir,
        cars: &[String],
        server: Server<WorldSimHost>,
        addr: SocketAddr,
        bot: Option<Bot>,
        tick: u64,
    ) -> Self {
        let mut renderer = renderer();
        let mut scene = client_world(dir, &mut renderer);
        let player = PlayerSetup { name: "tester".into(), car: CAR.into() };
        let endpoint = ClientEndpoint::connect(addr, endpoint_config(&IdentityKey::generate().unwrap())).unwrap();
        let client = Rc::new(RefCell::new(NetClient::new(endpoint, player.session_config(cars), true)));
        assert!(scene.attach_client(&client), "a driving world takes the client");
        let started = Instant::now() - Duration::from_micros(tick * 1_000_000 / 60);
        Self { server, addr, scene, renderer, client, bot, tick, started }
    }

    /// One 60 Hz frame of everyone, the client's devices giving `actions`.
    fn frame(&mut self, actions: &ActionState) {
        let now = ms(self.tick);
        if let Some(bot) = self.bot.as_mut() {
            bot.step(now);
        }
        self.server.tick_once(now);
        self.client.borrow_mut().set_now_ms(now);
        settle_streaming(&mut self.scene, &mut self.renderer, SETTLE);
        self.scene.update(&mut self.renderer, actions, STEP);
        self.tick += 1;
        let due = self.started + Duration::from_micros(self.tick * 1_000_000 / 60);
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
    }

    /// The bot joins as player 2, after the client has its seat (slot 0, the host's car).
    fn add_bot(&mut self, cars: &[String]) {
        self.bot = Some(Bot::connect(self.addr, bot_config(cars, 1)).unwrap());
    }
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (a few minutes)"]
fn a_client_world_drives_against_the_real_server_with_a_bot_beside_it() {
    let _turn = super::super::installed_turn::take();
    let cars = nfsmw_data::car::list(&dir());
    let mut rig = Rig::open();
    let (idle, gas) = (ActionState::default(), throttle());
    while rig.server.lobby().wire_phase() != SessionPhase::Running {
        assert!(rig.tick < 60 * 30, "the session started within 30 s (phase {:?})", rig.server.lobby().wire_phase());
        if rig.bot.is_none() && rig.client.borrow().session().slot().is_some() {
            rig.add_bot(&cars);
        }
        rig.frame(&idle);
    }
    for _ in 0..60 * 2 {
        rig.frame(&idle);
    }
    assert_eq!(rig.client.borrow().session().slot(), Some(0));
    assert!(rig.client.borrow().session().own_car().is_some(), "the first own block placed the predicted car");
    let snaps_before = rig.client.borrow().session().corrections().snaps();
    let start = rig.server.world().scene().players.lead().unwrap().stepped_pose().position;
    let (mut bot_seen, mut bot_interpolated) = (0, 0);
    for second in 0..20 {
        for _ in 0..60 {
            rig.frame(&gas);
            let mode = rig.scene.net_client().unwrap();
            let bot_car = mode.display().iter().find(|(_, drawn)| drawn.view.owner == Some(PlayerId(1)));
            let Some((_, drawn)) = bot_car else { continue };
            bot_seen += 1;
            bot_interpolated += usize::from(drawn.mode == DrawMode::Interpolated);
            assert_eq!(drawn.view.kind, EntityKind::PlayerCar);
        }
        let client = rig.client.borrow();
        let session = client.session();
        let predicted = session.own_car().unwrap().state();
        let served = rig.server.world().scene().players.lead().unwrap();
        let lead = session.lead_ticks().unwrap_or(0) as f32;
        let speed = served.speed().abs();
        let apart = served.stepped_pose().position.distance(glam::Vec3::from_array(predicted.position));
        // The prediction runs `lead` ticks ahead of the server's newest tick; the rest is what reconciliation leaves.
        let bound = speed * (lead + 3.0) / 60.0 + 1.0;
        assert!(apart <= bound, "second {second}: {apart:.2} m apart at {speed:.1} m/s, lead {lead} ticks");
    }
    let client = rig.client.borrow();
    let stats = client.session().corrections();
    let driven = rig.server.world().scene().players.lead().unwrap().stepped_pose().position.distance(start);
    eprintln!(
        "e2e: drove {driven:.0} m; checks {}, corrections {}, snaps {}, p95 {:?} m; bot seen {bot_seen} frames,          interpolated {bot_interpolated}",
        stats.checks(),
        stats.corrections(),
        stats.snaps(),
        stats.p95()
    );
    assert!(driven > 100.0, "the server's car drove from the client's input ({driven:.0} m)");
    assert_eq!(stats.snaps(), snaps_before, "no hard snaps while driving");
    assert!(stats.p95().is_none_or(|p95| p95 < 2.0), "corrections stay under the snap distance");
    assert!(bot_seen > 60 * 18, "the bot is drawn nearly every frame ({bot_seen})");
    assert!(bot_interpolated * 10 >= bot_seen * 9, "the bot is interpolated, not extrapolated");
    assert!(rig.bot.as_ref().is_some_and(|bot| bot.report().seated));
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about 30 s)"]
fn the_game_car_rewinds_the_real_drive_exactly() {
    let _turn = super::super::installed_turn::take();
    let dir = dir();
    let mut renderer = renderer();
    let mut scene = client_world(&dir, &mut renderer);
    for _ in 0..600 {
        settle_streaming(&mut scene, &mut renderer, SETTLE);
        scene.update(&mut renderer, &ActionState::default(), STEP);
        if scene.players.lead().is_some_and(|drive| drive.has_sim() && drive.waiting_for_road().is_none()) {
            break;
        }
    }
    let mut car = super::GameCar::new();
    car.set_surfaces(scene.physics.surfaces.clone());
    assert!(scene.lend_world(&car));
    let input =
        |tick: u32| InputState { throttle: 1.0, steer: (tick % 60) as f32 / 60.0 - 0.5, ..InputState::default() };
    for tick in 0..60 {
        car.step(&input(tick));
    }
    let saved = car.save();
    let first = run(&mut car, input);
    car.restore(&saved);
    let second = run(&mut car, input);
    scene.return_world(&car);
    assert_eq!(first, second, "the same inputs from the same save give the same states");
    assert!(car.steps() >= 300, "every step ran the physics ({})", car.steps());
    let moved = glam::Vec3::from_array(first[0].position).distance(glam::Vec3::from_array(first[119].position));
    assert!(moved > 5.0, "the predicted steps drive the car ({moved:.1} m in 2 s)");
}

/// Ticks 60 to 179 of `input`, the state after each.
fn run(car: &mut super::GameCar, input: impl Fn(u32) -> InputState) -> Vec<nfsmw_net::predict::CarState> {
    (60..180)
        .map(|tick| {
            car.step(&input(tick));
            car.state()
        })
        .collect()
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (a few minutes)"]
fn a_client_in_a_race_seat_follows_the_servers_race_car() {
    let _turn = super::super::installed_turn::take();
    let dir = dir();
    let cars = nfsmw_data::car::list(&dir);
    let data = nfsmw_data::gameplay::load(&dir).unwrap();
    let sprint = data
        .races
        .iter()
        .find(|race| {
            race.race_type == Some(nfsmw_data::gameplay::RaceType::Sprint)
                && !race.story.nis_shell
                && race.start.is_some()
                && !super::super::race::is_story_race(race)
        })
        .expect("a sprint in the install");
    let track = blackbox_feng::fe_hash_upper(&sprint.name).to_string();
    let (mut server, addr) = server_with(&dir, &cars, |config| {
        for (key, value) in [("mode", "sprint"), ("track", track.as_str()), ("traffic_density", "0")] {
            nfsmw_net::settings::set(&mut config.settings, key, value).unwrap();
        }
    });
    // The bot takes slot 0 (the host's car); the client then sits in slot 1, a person of the race.
    let mut bot = Bot::connect(addr, bot_config(&cars, 1)).unwrap();
    let mut tick = 0;
    while !bot.report().seated {
        assert!(tick < 60 * 10, "the bot is seated within 10 s");
        bot.step(ms(tick));
        server.tick_once(ms(tick));
        std::thread::sleep(Duration::from_millis(16));
        tick += 1;
    }
    let mut rig = Rig::join(&dir, &cars, server, addr, Some(bot), tick);
    let (idle, gas) = (ActionState::default(), throttle());
    while rig.server.lobby().wire_phase() != SessionPhase::Running {
        assert!(rig.tick < 60 * 120, "the race started within 2 min (phase {:?})", rig.server.lobby().wire_phase());
        rig.frame(&idle);
    }
    assert_eq!(rig.client.borrow().session().slot(), Some(1), "the client is a person of the race");
    assert_eq!(rig.server.world().scene().race_people, [PlayerId(1)], "the client's seat is the race's person");

    // The grid is far from where the client's world opened: its car was set down there by the first own block (a
    // teleport), and the road it stands on was streamed in first.
    let on_grid = |rig: &Rig| {
        let mut views = Vec::new();
        rig.server.world().entities(&mut views);
        let served = *views.iter().find(|view| view.owner == Some(PlayerId(1))).expect("the race car is exported");
        let predicted = glam::Vec3::from_array(rig.client.borrow().session().own_car().unwrap().state().position);
        (served, predicted)
    };
    // The streaming of a far road takes a while on a busy machine: wait for the car to arrive (15 s at most).
    let mut arrived = false;
    for frame in 0..60 * 15 {
        rig.frame(&gas);
        let (served, predicted) = on_grid(&rig);
        if frame >= 60 && glam::Vec3::from_array(served.position).distance(predicted) < 20.0 {
            arrived = true;
            break;
        }
    }
    let (served, _) = on_grid(&rig);
    let from_start = glam::Vec3::from_array(served.position).distance(glam::Vec3::new(AT[0], AT[1], 0.0));
    assert!(from_start > 500.0, "the race grid is far from where the client's world opened ({from_start:.0} m)");
    assert!(arrived, "the client's car stands at the server's race car, not where its world opened");

    // Four seconds of driving: the client's prediction of the race car is as good as in free roam.
    let snaps_before = rig.client.borrow().session().corrections().snaps();
    let mut fastest = 0.0f32;
    for second in 0..4 {
        for _ in 0..60 {
            rig.frame(&gas);
        }
        let (served, predicted) = on_grid(&rig);
        let speed = glam::Vec3::from_array(served.linear_velocity).length();
        fastest = fastest.max(speed);
        let lead = rig.client.borrow().session().lead_ticks().unwrap_or(0) as f32;
        let apart = glam::Vec3::from_array(served.position).distance(predicted);
        // The prediction runs `lead` ticks ahead of the server's newest tick; the rest is what reconciliation leaves.
        let bound = speed * (lead + 3.0) / 60.0 + 1.0;
        assert!(apart <= bound, "second {second}: {apart:.2} m apart at {speed:.1} m/s, lead {lead} ticks");
    }
    let client = rig.client.borrow();
    let stats = client.session().corrections();
    eprintln!(
        "race e2e: checks {}, corrections {}, snaps {} (before {snaps_before}), median {:?} m, p95 {:?} m, fastest {fastest:.1} m/s",
        stats.checks(),
        stats.corrections(),
        stats.snaps(),
        stats.median(),
        stats.p95()
    );
    assert!(fastest > 10.0, "the client's pedals drive the race car ({fastest:.1} m/s)");
    assert_eq!(stats.snaps(), snaps_before, "no hard snaps while driving the race car");
    assert!(stats.median().is_some_and(|median| median < 0.2), "the race car predicts as well as a free-roam one");
}
