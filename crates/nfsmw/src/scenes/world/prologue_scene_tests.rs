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
    let (mut scene, mut renderer) = prologue_world();
    let input = ActionState::default();
    let mut step = |scene: &mut WorldScene| scene.update(&mut renderer, &input, STEP);
    let mut frames = 0;
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

/// A renderer-free world with the authored prologue BMW already loaded.
fn prologue_world() -> (WorldScene, NullBackend) {
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
    (scene, renderer)
}

#[test]
#[ignore = "needs the installed prologue cars and road; reports the second Razor launch through the world race path"]
fn installed_second_razor_launch_through_world() {
    use crate::scenes::world::session_rules::SessionRules;

    let (mut scene, mut renderer) = prologue_world();
    let rules = scene.session_rules();
    scene.set_session_rules(SessionRules { race_screens: false, ..rules });
    // This uses the same production race constructor as the career queue, with
    // the already loaded prologue BMW taking the place of a saved career car.
    scene.start_race("16.2.1", RaceOptions { career: true, ..Default::default() }).expect("second Razor race");
    let mut input = ActionState::default();
    input.update_for_test(&crate::input::Bindings::default(), &[bevy_input::keyboard::KeyCode::ArrowUp], &[], false);
    let mut green = false;
    let mut five = false;
    for _ in 0..20_000 {
        scene.update(&mut renderer, &input, STEP);
        let Some(race) = scene.race.as_ref() else { continue };
        assert_eq!(race.event, "16.2.1");
        let Some(engine) = race.engine.as_ref() else { continue };
        if engine.phase() != blackbox_race::Phase::Racing {
            continue;
        }
        let traffic = scene.traffic.as_ref().expect("race traffic world");
        let (_, _, speed) = traffic.racers().find(|(id, ..)| *id == 1).expect("Razor's car");
        let (_, driver) = traffic.racers_driving().find(|(id, _)| *id == 1).expect("Razor's controls");
        let elapsed = engine.elapsed();
        if !green || (!five && elapsed >= 5.0) {
            eprintln!(
                "Razor t={elapsed:.3} s speed={speed:.2} m/s rpm={:.0} gear={} gas={:.2} brake={:.2} target={:.2} torque={:.0}",
                driver.rpm, driver.gear, driver.throttle, driver.brake, driver.target_speed, driver.drive_torque
            );
        }
        green = true;
        if elapsed >= 5.0 {
            assert!(speed.is_finite() && speed > 0.0, "Razor is physically driving at five seconds");
            assert!(!driver.stand_in, "Razor must still be using full physics at five seconds");
            five = true;
        }
        if five && let Some(gate1) = engine.racers()[1].checkpoint_times.first() {
            eprintln!("Razor gate 1: {gate1:.3} s");
            break;
        }
        if elapsed >= 30.0 {
            eprintln!("Razor gate 1: not reached in this straight-throttle fixture");
            break;
        }
    }
    assert!(green && five, "the production world must reach GO and five seconds of racing");
}
