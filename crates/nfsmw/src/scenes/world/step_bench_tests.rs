//! The cost of one world tick at 1, 8 and 16 players with traffic: the sim benchmark of
//! `docs/specs/multiplayer/integration.md` section 4, step 1, and the "world step" budget of `bandwidth.md` section 5.
//!
//! Each run opens the installed city, puts the lead in a scripted car and the other players in cars of their own beside
//! it (every player holds the throttle), keeps 16 traffic cars with a fifth of them patrol cops, and times
//! `WorldScene::update` with one 60 Hz tick per frame on a renderer that draws nothing. Streaming runs in lockstep and is
//! not timed. The time is the whole frame of the scene, so it is an upper bound of the sim step. Indicative numbers:
//! `cargo test -p nfsmw world_step_costs -- --ignored --nocapture`.

use std::time::{Duration, Instant};

use bevy_input::keyboard::KeyCode;
use game_install::GameDir;

use super::drive::STEP;
use super::players::PlayerId;
use super::session_rules::SessionRules;
use super::{DriveOptions, Options, WorldScene};
use crate::app::test_backend::NullBackend;
use crate::input::{ActionState, Bindings};
use crate::settings::{Percent, TrafficSettings};
use crate::viewer::Scene;

/// Ticks before timing starts: the guests are placed and the traffic fills in.
const WARM_UP: usize = 2400;
/// Timed ticks per run.
const MEASURED: usize = 1200;
/// The script of the lead: a loop of throttle and turns.
const LAP: &str = "4:throttle=1;3:throttle=0.7,steer=0.4;4:throttle=1;3:throttle=0.7,steer=-0.5;2:brake=1;0.1:reset;";

/// Waits until every tile the world wants is resident and nothing is in flight.
fn settle(scene: &mut WorldScene, renderer: &mut NullBackend) {
    let start = Instant::now();
    loop {
        let centres = scene.foci();
        scene.residency.update_centres(renderer, &centres);
        if scene.residency.complete() && scene.residency.counts().1 == 0 {
            return;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "streaming timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// The per-tick times (ms) of `players` players.
fn measure(dir: &GameDir, players: u8) -> Vec<f32> {
    let at = [2151.0, 1399.0];
    let options = Options {
        at: Some(at),
        height: 40.0,
        heading: 45.0,
        pitch: -20.0,
        fog_distance: 1500.0,
        wait_for_load: true,
        drive: Some(DriveOptions { car: "ELISE".into(), script: Some(LAP.repeat(8)) }),
    };
    let mut scene = WorldScene::open(dir, options).unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    scene.set_session_rules(SessionRules::session(1));
    let traffic = TrafficSettings { cars: 16, cop_share: Percent(20), ..scene.traffic_setting };
    scene.apply_traffic_setting(traffic);
    let mut throttle = ActionState::default();
    throttle.update_for_test(&Bindings::default(), &[KeyCode::KeyW], &[], false);
    let idle = ActionState::default();
    settle(&mut scene, &mut renderer);
    scene.update(&mut renderer, &idle, STEP);
    for id in 1..players {
        let (name, model) = super::car_loader::load_named_car(dir, "ELISE").unwrap();
        let near = [at[0] + 25.0 * f32::from(id % 4), at[1] + 25.0 * f32::from(id / 4)];
        scene.add_player(&mut renderer, PlayerId(id), name, model, near, None).unwrap();
        scene.inputs.set(PlayerId(id), &throttle);
    }
    let mut times = Vec::with_capacity(MEASURED);
    for tick in 0..WARM_UP + MEASURED {
        settle(&mut scene, &mut renderer);
        let start = Instant::now();
        scene.update(&mut renderer, &idle, STEP);
        if tick >= WARM_UP {
            times.push(start.elapsed().as_secs_f32() * 1000.0);
        }
    }
    let placed = scene.players.iter().filter(|player| player.drive.has_sim()).count();
    let cars = scene.traffic.as_ref().map_or(0, |traffic| traffic.entity_ids().count());
    eprintln!("world-step {players} players: {placed} on the road, {cars} traffic and cop cars");
    times
}

fn percentile(times: &mut [f32], percent: usize) -> f32 {
    times.sort_by(f32::total_cmp);
    times[(times.len() - 1) * percent / 100]
}

#[test]
#[ignore = "needs the game; diagnostic benchmark"]
fn world_step_costs() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    for players in [1, 8, 16] {
        let mut times = measure(&dir, players);
        let mean = times.iter().sum::<f32>() / times.len() as f32;
        let (p50, p95, p99) = (percentile(&mut times, 50), percentile(&mut times, 95), percentile(&mut times, 99));
        eprintln!("world-step {players} players: mean {mean:.3} ms, p50 {p50:.3} ms, p95 {p95:.3} ms, p99 {p99:.3} ms");
    }
}
