//! Arrest through real player input, installed physics and the normal world clock.

use super::arrest_fixture::{ArrestFixture, handbrake, open_arrest_fixture};
use super::*;
use crate::input::{ActionState, Bindings};
use crate::scenes::world::{WorldScene, drive::STEP};
use crate::viewer::Scene;
use bevy_input::keyboard::KeyCode;

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
    let Some(ArrestFixture { mut scene, mut renderer, identity, at }) = open_arrest_fixture() else { return };
    let stopped = handbrake();
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
