//! Arrest through real player input, installed physics and the normal world clock.

use super::*;
use crate::input::{ActionState, Bindings};
use crate::scenes::world::{DriveOptions, Options, WorldScene, drive::STEP};
use crate::viewer::Scene;
use bevy_input::keyboard::KeyCode;
use game_install::GameDir;

fn arrest_state(scene: &WorldScene) -> String {
    let drive = scene.drive.as_ref().unwrap();
    let player = Vec3::from_array(space::to_physics(drive.player_view().position));
    let world = scene.traffic.as_ref().unwrap();
    let nearest = world.nearest_matched_cop_position(player);
    let nearest_distance = nearest.map(|cop| Vec2::new(cop.x - player.x, cop.z - player.z).length());
    let cars: Vec<_> = world
        .cars
        .iter()
        .map(|car| {
            let cop = car.physics_position();
            format!(
                "{} id{} at{cop:?} delta{:?} speed{:.3} cop{} patrol{} roadblock{} live{} wreck{} finite{}",
                car.name,
                car.audio_emitter().id,
                cop - player,
                car.speed(),
                car.is_cop(),
                car.is_patrol(),
                car.is_roadblock(),
                car.is_live_police(),
                car.is_wrecked(),
                car.is_finite(),
            )
        })
        .collect();
    format!(
        "player-sensing{player:?} player-model{:?} player-body{:?} speed{:.3} tuning-speed{:.3} bust-tuning{:?} bust{:.6} pursuit-time{:.3} raw-sight{} tracked-sight{} escape{:.3} nearest{nearest:?} nearest-xz{nearest_distance:?} pursuing{} arrest{} simulation-ready{} game-speed{} cars{cars:?}",
        Vec3::from_array(space::to_physics(drive.physics_pose().position)),
        world.player_position,
        drive.speed(),
        world.police.bust_speed(1),
        world.police.bust_tuning(),
        world.busted_meter(),
        world.pursuit_time(),
        world.police.has_contact(),
        world.police.is_perp_in_sight(),
        world.police.escape_progress(),
        world.is_pursuing(),
        world.arrest_active(),
        scene.residency.simulation_ready(scene.focus(), 20.0),
        scene.tuning.game_speed,
    )
}

#[test]
#[ignore = "needs installed player/cop physics, road network and world collision"]
fn installed_arrest_locks_player_input_until_the_ticket_is_paid() {
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
    let mut stopped = ActionState::default();
    stopped.update_for_test(&Bindings::default(), &[KeyCode::Space], &[], false);
    for _ in 0..2000 {
        scene.update(&mut renderer, &stopped, STEP);
        if scene.ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(scene.ready(), "the installed player must be placed");
    for _ in 0..60 {
        scene.update(&mut renderer, &stopped, STEP);
    }
    crate::scenes::world::ai::models::load_cop_models(&mut scene, &mut renderer, &["copmidsize".into()]).unwrap();
    crate::scenes::world::ai::models::load_requested(&mut scene, &mut renderer, false, true);
    let drive = scene.drive.as_ref().unwrap();
    let heading = drive.heading();
    let performance = drive.performance();
    let at = drive.ai_body().position;
    let physical = scene.traffic.as_ref().unwrap().player_position.expect("the real player body was simulated");
    assert!(at.distance(physical) < 1e-4, "pursuit target must use the current body origin");
    let sensed = Vec3::from_array(space::to_physics(drive.player_view().position));
    assert!(sensed.distance(physical) < 1e-4, "police sensing must use the same body origin");
    let forward = Vec3::from_array(space::to_physics(Vec3::new(heading.cos(), heading.sin(), 0.0)));
    let mut world = scene.traffic.take().unwrap();
    world.cars.clear();
    world.configure(0, 0);
    let model = world.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case("copmidsize")).unwrap();
    let spot = at - forward * 8.0;
    let mut nav = RoadNav::traffic();
    assert!(nav.init_at_point(&world.network, &world.index, spot, forward, false));
    let car = AiCar::place(
        &model.model,
        Role::Traffic,
        nav,
        Spawn { position: space::to_render(spot.to_array()), heading },
        Start { speed: 0.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces },
    )
    .expect("the installed cop must stand near the player");
    let identity = car.audio_emitter().id;
    world.cars.push(car);
    world.cars[0].set_roadblock();
    let mut rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    for row in &mut rows {
        row.cops.clear();
    }
    world.player_position = Some(at);
    world.start_pursuit(1, rows, performance);
    scene.traffic = Some(world);
    let initial = arrest_state(&scene);
    let initial_world = scene.traffic.as_ref().unwrap();
    let nearest = initial_world
        .nearest_matched_cop_position(at)
        .unwrap_or_else(|| panic!("the cop must be on the player's deck: {initial}"));
    assert!((nearest.y - at.y).abs() < 1.5, "stock vertical bust gate: {initial}");
    assert!(Vec2::new(nearest.x - at.x, nearest.z - at.z).length() < 15.0, "stock proximity gate: {initial}");
    assert!(scene.drive.as_ref().unwrap().speed().abs() < 0.5, "handbraked fixture must stand still: {initial}");
    let mut history = Vec::new();
    let mut arrest_tick = None;
    for tick in 1..=360 {
        scene.update(&mut renderer, &stopped, STEP);
        if tick % 60 == 0 {
            history.push(format!("tick{tick}: {}", arrest_state(&scene)));
        }
        if scene.traffic.as_ref().unwrap().arrest_active() {
            arrest_tick = Some(tick);
            break;
        }
    }
    assert!(
        arrest_tick.is_some(),
        "normal five-second proximity must trigger arrest; initial{initial}; history{history:#?}; final{}",
        arrest_state(&scene),
    );
    assert!(arrest_tick.unwrap() >= 300, "a normal five-second arrest must not be an instant/reset bust");
    assert!(!scene.pay_bust(false), "the hold cannot be skipped");
    let held_position = scene.drive.as_ref().unwrap().physics_pose().position;
    let mut held = ActionState::default();
    held.update_for_test(
        &Bindings::default(),
        &[KeyCode::ArrowUp, KeyCode::ArrowRight, KeyCode::ShiftLeft, KeyCode::KeyR, KeyCode::KeyE],
        &[],
        false,
    );
    for _ in 0..240 {
        scene.update(&mut renderer, &held, STEP);
    }
    let drive = scene.drive.as_mut().unwrap();
    assert!(
        drive.physics_pose().position.distance(held_position) < 1.0,
        "held throttle/steering/reset cannot drive away"
    );
    assert!(!drive.sim_mut().unwrap().telemetry().nos_burning);
    let world = scene.traffic.as_ref().unwrap();
    assert!(world.arrest_active() && world.is_busted());
    assert!(world.cars.iter().any(|car| car.audio_emitter().id == identity && !car.emergency_lights()));
    assert!(scene.take_bust_ticket().is_some(), "the normal world clock reaches the ticket");
    assert!(scene.take_bust_ticket().is_none());
    assert!(scene.pay_bust(false));
    assert!(!scene.traffic.as_ref().unwrap().arrest_active());
    assert!(!scene.traffic.as_ref().unwrap().is_busted());
    let mut accelerate = ActionState::default();
    accelerate.update_for_test(&Bindings::default(), &[KeyCode::ArrowUp], &[], false);
    for _ in 0..90 {
        scene.update(&mut renderer, &accelerate, STEP);
    }
    assert!(
        scene.drive.as_ref().unwrap().physics_pose().position.distance(held_position) > 1.0,
        "payment releases driving"
    );
    scene.close(&mut renderer);
}
