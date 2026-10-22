//! The installed world under a session's rules gives the same state twice from the same start
//! (`docs/specs/multiplayer/determinism.md`, step 5; `integration.md` section 4, step 6).
//!
//! A scripted free-roam drive with traffic and patrol cops runs twice in one process, each run in a world of its own, and
//! the state hash (`state_hash.rs`) must agree after every simulated second. Streaming runs in lockstep with the ticks:
//! before each frame the test waits until every tile the world wants is resident, so the loader threads' timing does not
//! decide what the tick sees. A server does the same through the session's residency rule.

use std::time::{Duration, Instant};

use game_install::GameDir;

use super::drive::STEP;
use super::session_rules::SessionRules;
use super::{DriveOptions, Options, WorldScene};
use crate::app::test_backend::NullBackend;
use crate::input::ActionState;
use crate::settings::{Percent, TrafficSettings};
use crate::viewer::Scene;

/// Simulated seconds per run: as many as keep both runs (with the world's opening and streaming) near 30 s of test time
/// on a desktop PC in the dev profile.
const SECONDS: usize = 150;
/// Frames before the drive counts: the car is placed and the traffic is asked for.
const SETTLE_FRAMES: usize = 120;

/// One lap of the script: full throttle, a long left, a run, a long right, a brake, and a reset that puts the car back
/// on the nearest road. Repeated for the whole run.
const LAP: &str = "4:throttle=1;3:throttle=0.7,steer=0.4;4:throttle=1;3:throttle=0.7,steer=-0.5;2:brake=1;0.1:reset;";

/// Waits until every tile the world wants is resident and no request is in flight, so a tick sees the same world in
/// every run.
fn settle_streaming(scene: &mut WorldScene, renderer: &mut NullBackend) {
    let start = Instant::now();
    loop {
        let centres = scene.foci();
        scene.residency.update_centres(renderer, &centres);
        // Every wanted tile in, and nothing in flight that could land in the middle of a frame.
        if scene.residency.complete() && scene.residency.counts().1 == 0 {
            return;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "streaming timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// The parts of the state by name, for the message when two runs part.
fn parts(scene: &WorldScene) -> Vec<(String, u64)> {
    let mut parts = Vec::new();
    for player in scene.players.iter() {
        let mut hash = super::StateHasher::default();
        if let Some(sim) = player.drive.sim() {
            hash.car(sim);
        }
        parts.push((format!("player {}", player.id.0), hash.finish()));
    }
    parts.extend(scene.traffic.iter().flat_map(|traffic| traffic.hash_parts()));
    parts
}

/// One run: the state hash and its parts after every simulated second of the drive.
fn run(dir: &GameDir) -> Vec<(u64, Vec<(String, u64)>)> {
    let script = LAP.repeat(SECONDS / 16 + 2);
    let options = Options {
        at: Some([2151.0, 1399.0]),
        height: 40.0,
        heading: 45.0,
        pitch: -20.0,
        fog_distance: 1500.0,
        wait_for_load: true,
        drive: Some(DriveOptions { car: "ELISE".into(), script: Some(script) }),
    };
    let mut scene = WorldScene::open(dir, options).unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    scene.set_session_rules(SessionRules::session(0x5EED));
    let traffic = TrafficSettings { cars: 16, cop_share: Percent(20), ..scene.traffic_setting };
    scene.apply_traffic_setting(traffic);
    let input = ActionState::default();
    for _ in 0..SETTLE_FRAMES {
        settle_streaming(&mut scene, &mut renderer);
        scene.update(&mut renderer, &input, STEP);
    }
    assert!(scene.players.lead().is_some_and(|drive| drive.has_sim()), "the car must be on the road");
    let mut hashes = Vec::with_capacity(SECONDS);
    for tick in 1..=SECONDS * 60 {
        settle_streaming(&mut scene, &mut renderer);
        scene.update(&mut renderer, &input, STEP);
        if tick % 60 == 0 {
            hashes.push((scene.state_hash(), parts(&scene)));
        }
    }
    let cars = scene.traffic.as_ref().map_or(0, |traffic| traffic.entity_ids().count());
    assert!(cars > 0, "the drive must have traffic around it");
    hashes
}

#[test]
#[ignore = "needs the installed game; set NFSMW_GAME_DIR (about 30 s)"]
fn a_free_roam_drive_gives_the_same_state_hash_every_second_twice() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let started = Instant::now();
    let first = run(&dir);
    let second = run(&dir);
    eprintln!("determinism: 2 x {SECONDS} simulated seconds in {:.1} s", started.elapsed().as_secs_f32());
    let player = |second: &(u64, Vec<(String, u64)>)| second.1.first().map(|part| part.1);
    assert_ne!(player(&first[0]), player(&first[SECONDS - 1]), "the car must move");
    let diverged = first.iter().zip(&second).position(|(a, b)| a.0 != b.0);
    let Some(at) = diverged else {
        assert_eq!(first.len(), SECONDS);
        return;
    };
    let apart: Vec<_> = first[at].1.iter().filter(|part| !second[at].1.contains(part)).map(|(name, _)| name).collect();
    panic!("the runs part at second {}: {apart:?} ({} and {} parts)", at + 1, first[at].1.len(), second[at].1.len());
}
