//! A career sprint of the prologue, run with the demo's gate-to-gate jumps in the installed world: the race must stay on
//! until the finish line is crossed, whatever the checkpoint before it did. Spec: `docs/specs/race-goals.md`.

use blackbox_race::Phase;
use game_install::GameDir;

use super::drive::STEP;
use super::race::{RaceOptions, Stage};
use super::{DriveOptions, Options, WorldScene};
use crate::app::test_backend::NullBackend;
use crate::input::ActionState;
use crate::viewer::Scene;

fn open_world() -> (WorldScene, NullBackend) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: Some([2151.0, 1399.0]),
            height: 40.0,
            heading: 45.0,
            pitch: -20.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "ELISE".into(), script: None }),
        },
    )
    .unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    (scene, renderer)
}

/// Runs frames until `until` holds; the frames in between are recorded by `watch`.
fn pump(
    scene: &mut WorldScene,
    renderer: &mut NullBackend,
    mut watch: impl FnMut(&WorldScene),
    until: impl Fn(&WorldScene) -> bool,
) {
    let input = ActionState::default();
    for _ in 0..20000 {
        scene.update(renderer, &input, STEP);
        watch(scene);
        if until(scene) {
            return;
        }
    }
    panic!("the world did not get there in time");
}

/// Runs `name` as a career race with the demo driver and returns, per checkpoint count reached, what the race did.
fn run(name: &str) {
    let (mut scene, mut renderer) = open_world();
    pump(&mut scene, &mut renderer, |_| {}, |s| s.ready());
    let options = RaceOptions { career: true, demo: true, ..Default::default() };
    scene.queue_race(name.to_owned(), options);
    pump(
        &mut scene,
        &mut renderer,
        |_| {},
        |s| s.race.as_ref().is_some_and(|r| r.stage == Stage::Running && r.engine.is_some()),
    );
    eprintln!("{name}: resolved to {}", scene.race.as_ref().unwrap().name);
    let total = scene.race.as_ref().unwrap().engine.as_ref().unwrap().course().checkpoints.len();
    eprintln!(
        "{name}: bonuses {:?} limit {:?}",
        scene.race.as_ref().unwrap().engine.as_ref().unwrap().course().bonuses,
        scene.race.as_ref().unwrap().engine.as_ref().unwrap().time_left()
    );
    {
        let course = scene.race.as_ref().unwrap().engine.as_ref().unwrap().course();
        for (i, g) in course.checkpoints.iter().enumerate() {
            eprintln!("{name}: cp{i} centre {:?} half {:?}", g.centre, g.half);
        }
        if let Some(f) = course.finish.as_ref() {
            let last = course.checkpoints.last().unwrap();
            eprintln!(
                "{name}: finish centre {:?} half {:?}; last cp -> finish {:.0} m; last cp inside finish {}; finish touches last cp {}",
                f.centre,
                f.half,
                (f.centre - last.centre).length(),
                f.contains(last.centre),
                f.touches(last.centre, last.centre)
            );
        }
    }
    let mut seen = (0, false);
    // The counter goes back to 0 when the finish closes the lap, so the best it reached is kept.
    let mut best = 0;
    let mut early = None;
    pump(
        &mut scene,
        &mut renderer,
        |s| {
            let Some(engine) = s.race.as_ref().and_then(|r| r.engine.as_ref()) else { return };
            let me = &engine.racers()[0];
            best = best.max(me.progress.reached);
            let state = (me.progress.reached, engine.is_over());
            if state == seen {
                return;
            }
            seen = state;
            eprintln!(
                "{name}: reached {}/{total} over {} finished {} phase {:?} at {:.1}s",
                me.progress.reached,
                engine.is_over(),
                me.finished_at.is_some(),
                engine.phase(),
                engine.elapsed()
            );
            if engine.is_over() && me.finished_at.is_none() && early.is_none() {
                early = Some(me.progress.reached);
            }
        },
        |s| s.race.as_ref().is_none_or(|r| r.engine.as_ref().is_none_or(|e| e.is_over() || e.phase() == Phase::Over)),
    );
    assert_eq!(early, None, "{name}: the race ended after {early:?} of {total} checkpoints, before the finish line");
    assert_eq!(best, total, "{name}: the player reached every checkpoint");
}

#[test]
#[ignore = "needs the installed game, car physics, the road network and world collision; set NFSMW_GAME_DIR"]
fn the_first_prologue_race_ends_at_its_finish_line() {
    run("16.1.0");
}

#[test]
#[ignore = "needs the installed game, car physics, the road network and world collision; set NFSMW_GAME_DIR"]
fn the_second_prologue_race_ends_at_its_finish_line() {
    run("16.2.2");
}
