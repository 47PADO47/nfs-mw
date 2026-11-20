//! The NIS shell race of the prologue (16.8.1) in the installed world, headless: its scene `EndingNis03` drives the
//! player on `car1` and the two racers the race lists on `car2` and `car3`, and the scene's own fade to black
//! (`BLACK_FADE_OUT.fng`, `15_OUT`) leaves the screen black when it ends. Needs the game (set `NFSMW_GAME_DIR`).

use game_install::GameDir;

use super::drive::STEP;
use super::nis::NisKind;
use super::race::RaceOptions;
use super::{DriveOptions, Options, WorldScene};
use crate::app::test_backend::NullBackend;
use crate::input::ActionState;
use crate::viewer::Scene;

#[test]
#[ignore = "needs the installed game, the road network and world collision; set NFSMW_GAME_DIR"]
fn the_nis_shell_scene_drives_its_listed_racers_and_ends_black() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let options = Options {
        at: Some([2151.0, 1399.0]),
        height: 40.0,
        heading: 45.0,
        pitch: -20.0,
        fog_distance: 1500.0,
        wait_for_load: true,
        drive: Some(DriveOptions { car: "BMWM3GTRE46:M3GTRCAREERSTART".into(), script: None }),
    };
    let mut scene = WorldScene::open(&dir, options).unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    let input = ActionState::default();
    let mut step = |scene: &mut WorldScene| scene.update(&mut renderer, &input, STEP);
    let mut frames = 0;
    while !scene.ready() {
        step(&mut scene);
        frames += 1;
        assert!(frames < 20_000, "the world never loaded");
    }
    // Started directly: the queue would also build the career's car, which needs a loaded profile.
    let started = scene.start_race("16.8.1", RaceOptions { career: true, ..Default::default() });
    started.expect("the NIS shell race starts");
    let linked = loop {
        step(&mut scene);
        frames += 1;
        assert!(
            frames < 40_000,
            "EndingNis03 never linked its cars: race {:?}, queued {:?}, {}",
            scene.race.as_ref().map(|race| (&race.name, race.plan.len())),
            scene.queued_race.as_ref().map(|(query, _)| query),
            scene.nis.status()
        );
        if scene.nis.playing_name() == Some("EndingNis03") && !scene.nis.needs_links() {
            break scene.nis.car_spots();
        }
    };
    assert_eq!(scene.nis.playing_kind(), Some(NisKind::Ending));
    let traffic = scene.traffic.as_ref().expect("the road network");
    for racer in [1, 2] {
        let index = traffic.racer_car_index(racer).unwrap_or_else(|| panic!("racer {racer} is on the road"));
        assert!(
            linked.iter().any(|(target, _)| *target == super::nis::Target::Traffic(index)),
            "racer {racer} drives a channel of the scene"
        );
    }
    while scene.nis.is_playing() {
        step(&mut scene);
        frames += 1;
        assert!(frames < 80_000, "EndingNis03 never ended");
    }
    assert_eq!(scene.screen_fade.alpha(), 1.0, "the scene's fade to black holds after it");
}
