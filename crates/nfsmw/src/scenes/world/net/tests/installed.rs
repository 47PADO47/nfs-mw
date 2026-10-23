//! The host over the installed world: two hosts from one seed stay equal, sixteen players come and go, and hostile
//! input does not break the world. Ignored without `NFSMW_GAME_DIR`.

use std::time::Duration;

use game_install::GameDir;
use nfsmw_sim::{EntityKind, EntityView, InputState, PlayerId, PlayerJoin, SimError, SimEvent, SimInputs, WorldSim};

use super::super::catalog::car_type_id;
use super::super::host::{HostConfig, WorldSimHost};
use super::super::ids::FIRST_WORLD_ID;

/// The car every player drives.
pub const CAR: &str = "ELISE";

pub fn dir() -> GameDir {
    GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap()
}

/// A host on the determinism test's road, waiting for streaming before every tick so that runs compare.
pub fn open(seed: u64, traffic: Option<(u32, u8)>) -> WorldSimHost {
    let mut config = HostConfig::new([2151.0, 1399.0], CAR, seed);
    config.heading = 45.0;
    config.traffic = traffic;
    config.settle_streaming = Some(Duration::from_secs(60));
    let mut host = WorldSimHost::open(&dir(), config).unwrap();
    host.add_player(PlayerId(0), &PlayerJoin::default()).unwrap();
    host
}

pub fn join(car: &str) -> PlayerJoin {
    PlayerJoin { car_type: car_type_id(car), ..PlayerJoin::default() }
}

/// A scripted driver: on the throttle, weaving, now and then a brake, a handbrake or a shift; each player its own phase.
pub fn script(tick: u32, player: u8) -> InputState {
    let t = tick + u32::from(player) * 37;
    let phase = (t / 90) % 6;
    let braking = phase == 4;
    let pedal = |on: bool| f32::from(u8::from(on));
    InputState {
        throttle: pedal(!braking),
        brake: pedal(braking),
        steer: ((t % 240) as f32 / 120.0 - 1.0) * 0.6,
        handbrake: pedal(phase == 5 && t % 90 < 10),
        nos: phase == 2,
        shift_up: t % 300 == 0,
        ..InputState::default()
    }
}

/// One tick of `script` for `players`.
pub fn inputs(tick: u32, players: &[u8]) -> SimInputs {
    let mut inputs = SimInputs::default();
    for &player in players {
        inputs.set(PlayerId(player), script(tick, player));
    }
    inputs
}

/// The parts of the state by name (each player's car, the traffic's parts), so a failure names what parted.
fn parts(host: &WorldSimHost) -> Vec<(String, u64)> {
    let scene = host.scene();
    let mut parts = Vec::new();
    for player in scene.players.iter() {
        let mut hash = crate::scenes::world::StateHasher::default();
        if let Some(sim) = player.drive.sim() {
            hash.car(sim);
        }
        parts.push((format!("player {}", player.id.0), hash.finish()));
    }
    parts.extend(scene.traffic.iter().flat_map(|traffic| traffic.hash_parts()));
    parts
}

fn entities(host: &WorldSimHost) -> Vec<EntityView> {
    let mut out = Vec::new();
    host.entities(&mut out);
    out
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about a minute)"]
fn two_hosts_fed_the_same_inputs_from_the_same_seed_stay_equal() {
    let traffic = Some((16, 20));
    let (mut a, mut b) = (open(0x5EED, traffic), open(0x5EED, traffic));
    assert_eq!(a.state_hash(), b.state_hash(), "the same opening");
    a.add_player(PlayerId(1), &join(CAR)).unwrap();
    b.add_player(PlayerId(1), &join(CAR)).unwrap();
    let start = a.scene().players.lead().unwrap().position();
    let (mut events_a, mut events_b) = (Vec::new(), Vec::new());
    for tick in 0..60 * 40 {
        let inputs = inputs(tick, &[0, 1]);
        events_a.clear();
        events_b.clear();
        a.step(&inputs, &mut events_a);
        b.step(&inputs, &mut events_b);
        assert_eq!(events_a, events_b, "the events of tick {tick}");
        if tick % 60 == 59 {
            let second = (tick + 1) / 60;
            assert_eq!(parts(&a), parts(&b), "the parts of the state after {second} seconds");
            assert_eq!(a.state_hash(), b.state_hash(), "the hash after {second} seconds");
            assert_eq!(entities(&a), entities(&b), "the entities after {} seconds", (tick + 1) / 60);
        }
    }
    assert_eq!(a.tick(), 60 * 40);
    let driven = a.scene().players.lead().unwrap().position().distance(start);
    assert!(driven > 50.0, "the host's car drives from its input ({driven:.0} m)");
    assert_eq!((a.stalled_ticks(), b.stalled_ticks()), (0, 0));
    let traffic = entities(&a).iter().filter(|view| view.id >= FIRST_WORLD_ID).count();
    assert!(traffic > 0, "the drive has traffic around it");
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about a minute)"]
fn sixteen_players_join_are_exported_in_id_order_and_leave() {
    let mut host = open(1, Some((12, 25)));
    assert_eq!(host.add_player(PlayerId(0), &PlayerJoin::default()), Err(SimError::AlreadyJoined(PlayerId(0))));
    for n in 1..16 {
        host.add_player(PlayerId(n), &join(CAR)).unwrap();
    }
    assert_eq!(host.add_player(PlayerId(16), &join(CAR)), Err(SimError::Full));
    assert_eq!(host.add_player(PlayerId(3), &join(CAR)), Err(SimError::AlreadyJoined(PlayerId(3))));
    let unknown = PlayerJoin { car_type: 0xDEAD_BEEF, ..PlayerJoin::default() };
    assert_eq!(host.remove_player_then_add(PlayerId(5), &unknown), Err(SimError::UnknownCar(0xDEAD_BEEF)));
    host.add_player(PlayerId(5), &join(CAR)).unwrap();
    let mut events = Vec::new();
    for tick in 0..240 {
        host.step(&inputs(tick, &[]), &mut events);
    }
    let views = entities(&host);
    assert!(views.windows(2).all(|pair| pair[0].id < pair[1].id), "ascending ids");
    let players: Vec<&EntityView> = views.iter().filter(|view| view.kind == EntityKind::PlayerCar).collect();
    assert_eq!(players.len(), 16, "every player's car is on the road");
    for (n, view) in players.iter().enumerate() {
        assert_eq!((view.id, view.owner), (n as u16, Some(PlayerId(n as u8))));
        assert_eq!(view.type_id, host.catalog().type_number(CAR));
    }
    for view in views.iter().filter(|view| view.kind != EntityKind::PlayerCar) {
        assert!(view.id >= FIRST_WORLD_ID && view.owner.is_none(), "{view:?}");
    }
    for view in &views {
        let length = view.orientation.iter().map(|q| q * q).sum::<f32>().sqrt();
        assert!((length - 1.0).abs() < 1e-3 && view.position.iter().all(|p| p.is_finite()), "{view:?}");
    }
    for n in 0..16 {
        let own = host.own_state(PlayerId(n)).expect("every player has an own-car block");
        assert!(own.rpm.is_finite() && (0.0..=1.0).contains(&own.nitrous));
    }
    for n in 1..16 {
        host.remove_player(PlayerId(n));
    }
    host.step(&inputs(0, &[]), &mut events);
    let views = entities(&host);
    assert_eq!(views.iter().filter(|view| view.kind == EntityKind::PlayerCar).count(), 1, "only the host's car");
    assert!(host.own_state(PlayerId(7)).is_none());
    assert!(host.scene().coop.is_none(), "the last remote player closed the co-op session");
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about 30 s)"]
fn hostile_input_does_not_break_the_world() {
    let mut host = open(2, Some((8, 25)));
    host.add_player(PlayerId(1), &join(CAR)).unwrap();
    let wild = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, -1.0e30, 7.0];
    let mut events = Vec::new();
    for tick in 0..600u32 {
        let x = wild[tick as usize % wild.len()];
        let mut inputs = SimInputs::default();
        let input = InputState {
            throttle: x,
            brake: -x,
            steer: x,
            handbrake: x,
            clutch: x,
            nos: tick % 2 == 0,
            shift_up: true,
            shift_down: tick % 3 == 0,
            gear_select: Some(usize::MAX - tick as usize),
        };
        inputs.set(PlayerId(0), input);
        inputs.set(PlayerId(1), input);
        inputs.set(PlayerId(9), input);
        host.step(&inputs, &mut events);
    }
    for view in entities(&host) {
        assert!(view.position.iter().chain(&view.linear_velocity).all(|v| v.is_finite()), "{view:?}");
    }
    let finite = |event: &SimEvent| match *event {
        SimEvent::Contact { speed, .. } => speed.is_finite(),
        _ => true,
    };
    assert!(events.iter().all(finite));
    let _ = host.state_hash();
}

impl WorldSimHost {
    /// Remove `id` and add it again with `join`, the way a seat that changed its car rejoins.
    fn remove_player_then_add(&mut self, id: PlayerId, join: &PlayerJoin) -> Result<(), SimError> {
        self.remove_player(id);
        self.add_player(id, join)
    }
}
