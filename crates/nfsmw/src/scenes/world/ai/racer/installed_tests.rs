//! Production custom-race loading must retain the extended grid and use the native action estimate.

use crate::app::test_backend::NullBackend;
use crate::input::ActionState;
use crate::scenes::world::drive::STEP;
use crate::scenes::world::race::RaceOptions;
use crate::scenes::world::{DriveOptions, Options, WorldScene};
use crate::viewer::Scene;
#[path = "installed_route_assertions.rs"]
mod route_assertions;
use route_assertions::assert_shortcuts_respect_walls;

#[test]
#[ignore = "needs installed cars, roads and world collision; renderer-free six-opponent custom race"]
fn installed_custom_race_uses_action_estimates_and_six_opponents_drive() {
    run_custom_race(false, false, "14.2.4");
}

#[test]
#[ignore = "needs installed cars, roads and world collision; renderer-free full course with six opponents"]
fn installed_custom_race_six_opponents_finish_without_catch_up() {
    run_custom_race(true, false, "14.2.4");
}

#[test]
#[ignore = "needs installed world collision; follows rivals to exercise physics throughout the course"]
fn installed_custom_race_followed_opponents_finish_on_physics() {
    for catch_up in [false, true] {
        for event in ["15.2.1", "14.2.4"] {
            run_custom_race_setting(true, true, event, catch_up);
        }
    }
}

pub(super) fn loaded_world() -> (WorldScene, NullBackend) {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: Some([2151.0, 1399.0]),
            height: 40.0,
            heading: 45.0,
            pitch: -20.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: "CARRERAGT".into(), script: None }),
        },
    )
    .unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    let input = ActionState::default();
    let mut ready = false;
    for _ in 0..20_000 {
        scene.update(&mut renderer, &input, STEP);
        if scene.ready() {
            ready = true;
            break;
        }
    }
    assert!(ready, "installed world did not finish loading");
    scene.set_session_rules(crate::scenes::world::session_rules::SessionRules {
        seed: Some(super::super::streams::WORLD_SEED),
        ..Default::default()
    });
    (scene, renderer)
}

#[test]
#[ignore = "needs installed cars, roads and race; verifies production solo option"]
fn installed_custom_race_zero_opponents_stays_a_solo_race_until_the_finish() {
    let (mut scene, mut renderer) = loaded_world();
    let settings =
        crate::frontend::custom_race::settings::Settings { opponents: 0, difficulty: 2, heat: 0, ..Default::default() };
    let options = settings.race_options(crate::frontend::custom_race::Mode::Sprint);
    assert_eq!(options.opponents, Some(0));
    scene.queue_race("14.2.4".into(), options);
    let mut green = 0;
    for _ in 0..20_000 {
        scene.update(&mut renderer, &ActionState::default(), STEP);
        let Some(race) = scene.race.as_ref().filter(|race| race.engine.is_some()) else { continue };
        assert_eq!(race.plan.len(), 1);
        assert_eq!(race.plan[0].grid_slot, 0);
        let engine = race.engine.as_ref().unwrap();
        assert_eq!(engine.racers().len(), 1);
        if engine.phase() == blackbox_race::Phase::Racing {
            green += 1;
            if green == 600 {
                break;
            }
        }
    }
    assert_eq!(green, 600, "a solo race must start without declaring an immediate win");
}

#[test]
#[ignore = "needs installed world and finish camera; renderer-free winning flow"]
fn installed_race_play_path_win_opens_photo_on_the_next_update() {
    let (mut scene, mut renderer) = loaded_world();
    scene.queue_race(
        "14.2.4".into(),
        RaceOptions { opponents: Some(0), traffic: Some(0), heat: Some(0), demo: true, ..Default::default() },
    );
    let mut finish_frame = None;
    let mut photo_frame = None;
    for frame in 0..20_000 {
        // Demo jumps can cross residency tiles. Load the requested gate before its exact placement can finish.
        if let Some(mut spawn) = scene.players.lead().and_then(|d| d.waiting_for_road()).and_then(|r| r.exact) {
            scene.view = crate::scenes::world::View::Fly;
            scene.camera.position = spawn.position + glam::Vec3::Z * 8.0;
            let [x, y, z] = crate::scenes::world::space::to_physics(spawn.position);
            if let Some(hit) = crate::scenes::world::drive::drivable_ground_below(
                scene.residency.collision(),
                x,
                z,
                y + 20.0,
                y - 40.0,
            ) {
                spawn.position.z = hit.point[1];
                scene.players.lead_mut().unwrap().place_exactly(spawn, 20.0);
            }
        }
        scene.update(&mut renderer, &ActionState::default(), STEP);
        let finished = scene
            .race
            .as_ref()
            .and_then(|race| race.engine.as_ref())
            .is_some_and(|engine| engine.racers()[0].finished_at.is_some());
        if finished {
            finish_frame.get_or_insert(frame);
        }
        if let Some(report) = scene.take_race_report() {
            assert!(report.photo.is_some(), "a solo sprint win must show its photo");
            photo_frame = Some(frame);
            break;
        }
    }
    assert!(
        finish_frame.is_some(),
        "demo must cross the finish: race={:?} player={:?} waiting={:?}",
        scene.race.as_ref().map(|r| (&r.stage, r.engine.as_ref().map(|e| e.racers()))),
        scene.players.lead().map(|d| d.position()),
        scene.players.lead().and_then(|d| d.waiting_for_road())
    );
    let finish = finish_frame.unwrap();
    let photo = photo_frame.expect("the win must hand its photo to the frontend");
    assert!(photo - finish <= 1, "winning must not keep driving under a three-second result banner");
    assert!(scene.photo_shot.is_some(), "the report must start its finish camera");
    // The demo's residency helper used Fly to load remote gates; normal race finishes are driven in Chase.
    scene.view = crate::scenes::world::View::Chase;
    let at_finish = scene.players.lead().unwrap().position();
    assert!(scene.runs_while_paused(), "the photo's first interval must keep moving");
    for _ in 0..100 {
        if !scene.photo_driving() {
            break;
        }
        scene.update(&mut renderer, &ActionState::default(), STEP);
    }
    assert!(!scene.photo_driving(), "the camera must cut after the slowdown");
    assert!(
        scene.players.lead().unwrap().position().distance(at_finish) > 0.5,
        "the first interval must not be a frozen driving image"
    );
    assert!(scene.photo_lens(16.0 / 9.0).is_some());
}

#[test]
#[ignore = "needs installed cars, scene bundles, roads and world collision"]
fn installed_custom_race_intro_and_police_off_with_traffic() {
    for event in ["15.2.1", "14.2.4"] {
        let (mut scene, mut renderer) = loaded_world();
        let input = ActionState::default();
        scene.queue_race(
            event.into(),
            RaceOptions {
                opponents: Some(3),
                difficulty: Some(100),
                traffic: Some(100),
                catch_up: Some(false),
                heat: Some(0),
                ..Default::default()
            },
        );
        let mut samples = 0;
        let mut rev_samples = 0;
        let mut green_frames = 0;
        for frame in 0..20_000 {
            scene.update(&mut renderer, &input, STEP);
            if let Some(race) = scene.race.as_ref() {
                assert!(!race.has_police(), "{event}: explicit None must override the track's police flag");
            }
            if let Some(time) = scene.nis.scene_time() {
                samples += 1;
                rev_samples += usize::from(scene.nis.player_rev().is_some());
                if frame % 30 == 0 {
                    let lens = scene.gameplay_lens(16.0 / 9.0);
                    let (collision, _) = scene.residency.world_parts();
                    let at = lens.position;
                    let ground =
                        crate::scenes::world::space::ground_below(collision, at.x, at.y, at.z + 3.0, at.z - 20.0);
                    if let Some(ground) = ground {
                        assert!(at.z - ground >= 0.49, "{event}: intro camera clips into the road at {time}");
                    }
                    let traffic = scene.traffic.as_ref().unwrap();
                    let mut actors = vec![scene.players.lead().unwrap().render_pose().position];
                    actors.extend(
                        traffic.cars.iter().filter(|car| car.is_racer()).map(|car| car.render_pose(1.0).position),
                    );
                    assert_eq!(actors.len(), scene.race.as_ref().unwrap().plan.len(), "every entrant stays visible");
                    for (i, at) in actors.iter().enumerate() {
                        assert!(
                            actors[i + 1..].iter().all(|other| at.distance(*other) > 1.5),
                            "intro actors must not occupy the same place"
                        );
                    }
                    let expected: Vec<_> = scene
                        .nis
                        .car_slots()
                        .iter()
                        .filter_map(|(slot, _)| {
                            let plan = &scene.race.as_ref()?.plan;
                            let human_slot = plan[0].grid_slot;
                            let grid_slot = match *slot {
                                0 => human_slot,
                                slot if slot == human_slot => 0,
                                slot => slot,
                            };
                            let id = plan.iter().position(|plan| plan.grid_slot == grid_slot)?;
                            (id > 0).then(|| traffic.racer_car_index(id)).flatten()
                        })
                        .collect();
                    assert_eq!(scene.nis.linked_cars(), expected, "primary actor must be the human");
                    let primary = scene.nis.car_slots().into_iter().find(|(slot, _)| *slot == 0).unwrap().1;
                    let drawn = scene.players.lead().unwrap().render_pose().position;
                    assert!(
                        drawn.truncate().distance(primary.position.truncate()) < 0.01,
                        "camera must frame the human"
                    );
                    assert!(scene.owns_soundtrack(), "the intro must keep radio music out of its soundtrack");
                    let audio = scene.audio_state().expect("intro must keep the player's engine voice");
                    let rev = scene.nis.player_rev();
                    assert_eq!(audio.input.scripted_rpm, rev.map(|curve| curve.rpm));
                    if let Some(curve) = rev {
                        assert!((audio.input.throttle - curve.torque / 100.0).abs() < 1e-5);
                    }
                    eprintln!(
                        "RACE_INTRO event={event} time={time:.3} eye={at:?} ground={ground:?} rev={rev:?} rpm_fraction={} throttle={} player={:?}",
                        audio.input.rpm_pct,
                        audio.input.throttle,
                        scene.players.lead().unwrap().render_pose().position
                    );
                }
            }
            if scene
                .race
                .as_ref()
                .and_then(|r| r.engine.as_ref())
                .is_some_and(|e| e.phase() == blackbox_race::Phase::Racing)
            {
                let traffic = scene.traffic.as_ref().unwrap();
                assert!(
                    traffic
                        .cars
                        .iter()
                        .filter(|car| car.is_racer())
                        .all(|car| car.render_pose(1.0).position == car.pose(1.0).position),
                    "all race actors restore physical poses at countdown/GO"
                );
                assert!(!traffic.cops.spawning_enabled());
                assert!(!traffic.is_pursuing());
                assert_eq!(traffic.cars.iter().filter(|car| car.is_police()).count(), 0);
                assert!(scene.roam_feed().is_none(), "police None must also suppress the 911 radio feed");
                green_frames += 1;
                if green_frames == 600 {
                    break;
                }
            }
        }
        assert!(samples > 30, "{event}: must exercise a loaded intro, not skip it");
        assert!(rev_samples > 30, "{event}: must exercise the installed engine curve without extending its tail");
        assert_eq!(green_frames, 600, "{event}: must reach GO and exercise normal traffic");
    }
}

fn run_custom_race(full_course: bool, follow: bool, event: &str) {
    run_custom_race_setting(full_course, follow, event, !full_course);
}

fn run_custom_race_setting(full_course: bool, follow: bool, event: &str, catch_up: bool) {
    // Full-course mode leaves the player on the grid. Off disables physical catch-up; native emulation can force it.
    let (mut scene, mut renderer) = loaded_world();
    let input = ActionState::default();
    scene.queue_race(
        event.into(),
        RaceOptions {
            opponents: Some(6),
            difficulty: Some(100),
            traffic: Some(if follow { 25 } else { 0 }),
            catch_up: Some(catch_up),
            heat: Some(0),
            ..Default::default()
        },
    );
    let mut started = false;
    for _ in 0..20_000 {
        scene.update(&mut renderer, &input, STEP);
        if scene
            .race
            .as_ref()
            .and_then(|race| race.engine.as_ref())
            .is_some_and(|race| race.phase() == blackbox_race::Phase::Racing)
        {
            started = true;
            break;
        }
    }
    assert!(started, "custom race did not reach GO");
    let session = scene.race.as_ref().unwrap();
    assert_eq!(session.catch_up, catch_up, "the selected switch must survive production loading");
    for car in &scene.traffic.as_ref().unwrap().cars {
        if let Some(id) = car.racer_id() {
            let up = car.pose(1.0).rotation * glam::Vec3::Z;
            assert!(up.z > 0.5, "racer {id} must reach GO upright: {up:?}");
        }
    }
    let diagnostic = std::env::var_os("NFSMW_RACER_DIAGNOSTICS").is_some();
    if diagnostic {
        eprintln!("RACER_GRID event={event} plan={:?}", session.plan);
        for car in &scene.traffic.as_ref().unwrap().cars {
            if let Some(id) = car.racer_id() {
                eprintln!("RACER_GRID_CAR id={id} {}", car.race_diagnostic());
            }
        }
    }
    assert_eq!(session.engine.as_ref().unwrap().racers().len(), 7, "six opponents must survive production loading");
    let player = scene.players.lead().unwrap().ai_performance();
    let models: Vec<_> = session.plan.iter().skip(1).map(|plan| plan.model.clone().unwrap()).collect();
    for (id, name) in models.iter().enumerate() {
        let package =
            nfsmw_data::car::physics::PerformanceCatalog::read(scene.physics.database(), name).unwrap().maximum();
        assert_eq!(package.junkman, 0, "ordinary race hardware does not grant Junkman");
        let physics = scene.physics.car_with_performance(name, package).unwrap();
        let expected = blackbox_vehicle::performance::estimate_ai(&physics.spec)
            .unwrap_or_else(|| panic!("{name}: fixture must exercise analytical estimation, not its fallback"));
        let state = scene.traffic.as_mut().unwrap().racer_mut(id + 1).unwrap();
        eprintln!(
            "RACER_ACTION_ESTIMATE id={} model={name} actual={:?} expected={expected:?}",
            id + 1,
            state.performance
        );
        assert_eq!(state.performance, expected, "{name}: the planner must use the native estimate");
        assert_eq!(state.player, player, "{name}: player matching must use the same estimate domain");
        let skill = match catch_up {
            true => 1.0,
            false => 0.8,
        };
        assert_eq!(state.base_skill, skill, "hard follows the chosen catch-up option");
        assert!(!state.held, "GO releases every opponent");
        assert_eq!(
            scene.traffic.as_ref().unwrap().racer_hud(id + 1).unwrap().has_nos,
            physics.spec.nos.present(),
            "the physical car must receive its hardware, not just higher action estimates"
        );
    }
    assert_shortcuts_respect_walls(&scene);
    let mut progress = vec![0.0_f32; 6];
    if diagnostic {
        let session = scene.race.as_ref().unwrap();
        let base = session.route.as_ref().unwrap();
        let leg = &base.legs[9];
        eprintln!("RACER_BASE pieces={:?}", &base.steps[leg.steps.clone()]);
        for shortcut in &session.shortcuts {
            eprintln!("RACER_SHORTCUT {shortcut:?}");
        }
        let traffic = scene.traffic.as_mut().unwrap();
        for id in 1..=6 {
            let route = traffic.racer_mut(id).unwrap().route.clone().unwrap();
            let leg = &route.legs[9];
            eprintln!("RACER_PLAN id={id} leg={leg:?} pieces={:?}", &route.steps[leg.steps.clone()]);
        }
    }
    let mut finished = vec![None; 6];
    let mut followed_ticks = 0_u32;
    let mut physical_ticks = 0_u32;
    let mut losses = [0_u32; 6];
    let mut previous_speed = [0_f32; 6];
    let seconds = match full_course {
        true => 240.0,
        false => 15.0,
    };
    for tick in 0..(seconds / STEP) as usize {
        if follow {
            let id = (1..=6).rev().find(|&id| finished[id - 1].is_none()).unwrap_or(6);
            let traffic = scene.traffic.as_ref().unwrap();
            let car = traffic.cars.iter().find(|car| car.racer_id() == Some(id)).unwrap();
            followed_ticks += 1;
            physical_ticks += u32::from(car.race_uses_physics());
            let followed = traffic.racer_followed(id, 1.0).unwrap();
            scene.view = crate::scenes::world::View::Fly;
            scene.camera.position = followed.position + glam::Vec3::Z * 4.0;
            scene.camera.yaw = followed.heading;
            if tick % 300 == 0 {
                eprintln!(
                    "RACER_FOLLOW event={event} time={:.2} id={id} {}",
                    tick as f32 * STEP,
                    car.race_diagnostic()
                );
                if diagnostic && tick <= 600 {
                    for car in &traffic.cars {
                        if let Some(id) = car.racer_id() {
                            eprintln!("RACER_EARLY time={} id={id} {}", tick as f32 * STEP, car.race_diagnostic());
                            if tick > 0 && car.race_speed() < 1.0 {
                                eprintln!("RACER_DRIVETRAIN id={id} {}", car.telemetry_debug());
                                car.race_terrain_diagnostic(scene.residency.collision());
                            }
                        }
                    }
                }
            }
        }
        scene.update(&mut renderer, &input, STEP);
        let engine = scene.race.as_ref().and_then(|race| race.engine.as_ref()).expect("race must stay active");
        if !catch_up {
            for car in &scene.traffic.as_ref().unwrap().cars {
                let Some(id) = car.racer_id() else { continue };
                if car.race_uses_physics() && !engine.racers()[id].done() {
                    let super::super::car::Role::Racer(state) = &car.role else { unreachable!() };
                    assert_eq!(state.glue, 0.0, "physical Off must suppress boost and slowdown for racer {id}");
                    assert_eq!(state.cheat(), 0.0);
                }
            }
        }
        for ((best, at), racer) in progress.iter_mut().zip(&mut finished).zip(engine.racers().iter().skip(1)) {
            *best = best.max(racer.percent);
            *at = racer.finished_at;
        }
        if follow {
            for car in &scene.traffic.as_ref().unwrap().cars {
                if let Some(id) = car.racer_id() {
                    let speed = car.race_speed();
                    if car.race_uses_physics() && previous_speed[id - 1] > 15.0 && previous_speed[id - 1] - speed > 10.0
                    {
                        losses[id - 1] += 1;
                    }
                    previous_speed[id - 1] = speed;
                }
            }
        }
        if full_course && (tick + 1) % (30.0 / STEP) as usize == 0 {
            eprintln!("RACER_ROUTE time={:.1} progress={progress:?} finished={finished:?}", (tick + 1) as f32 * STEP);
            if tick as f32 * STEP > 150.0 {
                for car in &scene.traffic.as_ref().unwrap().cars {
                    let Some(id) = car.racer_id() else { continue };
                    if finished[id - 1].is_none() {
                        let racer = &engine.racers()[id];
                        eprintln!("RACER_DETAIL id={id} reached={} {}", racer.progress.reached, car.race_diagnostic());
                        if diagnostic && id == 6 {
                            car.race_terrain_diagnostic(scene.residency.collision());
                        }
                    }
                }
            }
        }
        if finished.iter().all(Option::is_some) {
            break;
        }
    }
    eprintln!("RACER_ACTION_PROGRESS window={seconds} full_course={full_course}: {progress:?} finished={finished:?}");
    if follow {
        eprintln!(
            "RACER_PHYSICS event={event} followed_ticks={followed_ticks} physical_ticks={physical_ticks} severe_losses={losses:?}"
        );
        assert!(
            physical_ticks as f32 / followed_ticks as f32 > 0.95,
            "the followed rival must exercise live physics, not a stand-in"
        );
    }
    for id in 1..=6 {
        let state = scene.traffic.as_mut().unwrap().racer_mut(id).unwrap();
        assert!((state.think_delta - 2.0 * STEP).abs() < 1e-5, "racer {id} must decide at 30 Hz");
    }
    for (id, percent) in progress.iter().enumerate() {
        assert!(*percent > 0.5, "racer {} did not leave the grid and drive the actual course: {percent}%", id + 1);
    }
    if full_course {
        assert!(
            finished.iter().all(Option::is_some),
            "every extended-grid opponent must drive the full course: {progress:?}"
        );
    }
}
