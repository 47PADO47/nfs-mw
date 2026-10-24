//! Sustained speed acceptance with production-loaded police variants and unmodified heat fleets.
//! The target is kinematic and the corridor is synthetic; actual city-route acceptance is separate.

use super::*;
use crate::scenes::world::ai::PlayerView;
use crate::scenes::world::{WorldScene, drive::STEP, props::PropWorld};
use crate::viewer::{Scene, camera::Lens};

const SECONDS: f32 = 40.0;

fn copy_model(source: &CopModel) -> CopModel {
    let model = &source.model;
    assert!(model.trailer.is_none(), "regular fleet fixture has no tractor");
    CopModel {
        model: TrafficModel {
            name: model.name.clone(),
            rig: model.rig.clone(),
            physics: model.physics.clone(),
            tractor: model.tractor,
            trailer: None,
            cop_lights: model.cop_lights.clone(),
            audio_profile: model.audio_profile,
            keys: model.keys.clone(),
        },
        car: source.car,
    }
}

fn loaded() -> (WorldScene, NullBackend, Vec<CopModel>) {
    let (mut scene, mut renderer, old_world) = super::installed_route::open();
    scene.traffic = Some(old_world);
    let rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    let names: std::collections::BTreeSet<_> =
        rows.iter().take(5).flat_map(nfsmw_data::pursuit::ground_cops).map(|record| record.name.clone()).collect();
    crate::scenes::world::ai::models::load_cop_models(
        &mut scene,
        &mut renderer,
        &names.into_iter().collect::<Vec<_>>(),
    )
    .unwrap();
    crate::scenes::world::ai::models::load_requested(&mut scene, &mut renderer, false, true);
    let models = std::mem::take(&mut scene.traffic.as_mut().unwrap().cop_models);
    for model in &models {
        eprintln!(
            "FLEET_MODEL name={} performance={:?} ai={:?} nos={:?}",
            model.model.name, model.car.performance, model.car.ai, model.model.physics.spec.nos
        );
    }
    (scene, renderer, models)
}

fn player(seconds: f32, speed: f32, nitrous_boost: f32) -> (Focus, PlayerView, Lens) {
    let body = Body {
        position: Vec3::new(2.0, 0.7, seconds * speed),
        velocity: Vec3::Z * speed,
        forward: Vec2::Y,
        half_width: 1.0,
        half_length: 2.3,
    };
    let position = space::to_render(body.position.to_array());
    let forward = space::to_render([0.0, 0.0, 1.0]);
    (
        Focus { position: [position.x, position.y], heading: 0.0, speed, body: Some(body) },
        PlayerView {
            position,
            velocity: forward * speed,
            forward,
            speed,
            throttle: 1.0,
            nitrous: 0.0,
            nitrous_boost,
            nos_input: false,
        },
        Lens::look_at(position - forward * 8.0 + Vec3::Z * 3.0, position, 60.0, 0.1, 1.0),
    )
}

#[derive(Default, Debug)]
struct Catchup {
    initial: Option<u32>,
    closest: f32,
    final_gap: Option<f32>,
    matched_ticks: usize,
    late_ticks: usize,
    original_nos_ticks: usize,
    original_burning_ticks: usize,
    original_max_speed: f32,
    peak_gap: f32,
    greatest_recovery: f32,
    original_late_matched_ticks: usize,
}

fn encounter(
    scene: &WorldScene,
    models: &[CopModel],
    heat: u32,
    speed: f32,
    entry_ratio: f32,
    automatic: bool,
) -> Catchup {
    let data = &scene.physics;
    let player_physics = data.car("bmwm3gtr").unwrap();
    let performance = nfsmw_data::car::physics::measured::ai_performance_of_spec(
        &player_physics.spec,
        player_physics.tire_integration,
    );
    let boost = match player_physics.spec.nos.present() {
        true => player_physics.spec.nos.torque_boost,
        false => 0.0,
    };
    let rows = nfsmw_data::pursuit::heat_rows(data.database());
    let dominant = nfsmw_data::pursuit::ground_cops(&rows[heat as usize - 1])
        .max_by_key(|record| record.count.saturating_mul(record.chance))
        .expect("actual heat row must request a regular ground unit")
        .name
        .clone();
    let collision = super::installed_spawn_drive::corridor();
    let mut world = TrafficWorld::new(straight_road(6000.0));
    world.load_data(data.database(), None);
    world.configure(0, 0);
    for model in models {
        world.add_cop_model(copy_model(model));
    }
    world.set_career_bin(Some(16));
    world.police.set_forced_pursuit(true);
    world.start_pursuit(heat, rows, performance);
    let mut result = Catchup { closest: f32::INFINITY, ..Default::default() };
    if !automatic {
        let spot = CopSpot {
            at: Vec3::new(2.0, 0.0, -100.0),
            target: Vec3::ZERO,
            speed: speed * entry_ratio,
            player: performance,
            band: None,
        };
        assert!(world.spawn_cop_at(&dominant, spot, &collision, &data.surfaces));
        result.initial = Some(world.cars.last().unwrap().audio_emitter().id);
    }
    let mut props = PropWorld::default();
    let ticks = (SECONDS / STEP).round() as u32;
    for tick in 0..ticks {
        let (focus, view, camera) = player(tick as f32 * STEP, speed, boost);
        world.player_position = focus.body.map(|body| body.position);
        world.step(focus, &collision, &mut props, &data.surfaces);
        world.collide(None);
        world.finish_step(focus, &camera, &camera, &props, &collision, &data.surfaces);
        // The kinematic target has no player physics body for collide(). Restore
        // the same target position the live world's collision handoff supplies.
        world.player_position = focus.body.map(|body| body.position);
        world.player_speed = focus.speed;
        let cast = crate::scenes::world::drive::sight::sight_cast(&collision);
        world.update_sight(STEP, &cast);
        world.update_police(STEP, &view, &|from, to| cast(from, to).is_none());
        if result.initial.is_none() {
            result.initial = world.cars.iter().find(|car| car.is_cop()).map(|car| car.audio_emitter().id);
        }
        let target = focus.body.unwrap();
        let original = world.cars.iter().find(|car| Some(car.audio_emitter().id) == result.initial);
        if let Some(car) = original {
            let gap = target.position.distance(car.physics_position());
            result.closest = result.closest.min(gap);
            result.final_gap = Some(gap);
            result.original_nos_ticks += usize::from(car.controls().nos);
            result.original_burning_ticks += usize::from(car.nitrous_burning());
            result.original_max_speed = result.original_max_speed.max(car.speed());
            result.peak_gap = result.peak_gap.max(gap);
            result.greatest_recovery = result.greatest_recovery.max(result.peak_gap - gap);
            result.original_late_matched_ticks += usize::from(
                tick >= ticks - 600
                    && !car.is_fleeing()
                    && !car.is_searching()
                    && gap < 120.0
                    && car.speed() >= speed * 0.9,
            );
        }
        if tick >= ticks - 600 {
            result.late_ticks += 1;
            result.matched_ticks += usize::from(world.cars.iter().any(|car| {
                car.is_cop()
                    && !car.is_fleeing()
                    && !car.is_searching()
                    && car.body().position.distance(target.position) < 120.0
                    && car.speed() >= speed * 0.9
            }));
        }
        if tick.is_multiple_of(120) {
            let units: Vec<_> = world
                .cars
                .iter()
                .filter(|car| car.is_cop())
                .map(|car| {
                    let (aim, request) = car.drive_request();
                    format!(
                        "{} id={} gap={:.1} actual={:.1} requested={request:.1} aim_ahead={:.1} search={} flee={} sight_age={:.1} controls={:?} {}; {}",
                        car.name,
                        car.audio_emitter().id,
                        target.position.z - car.physics_position().z,
                        car.speed(),
                        (aim - car.physics_position()).dot(car.physics_forward()),
                        car.is_searching(),
                        car.is_fleeing(),
                        car.sight_age(),
                        car.controls(),
                        car.telemetry_debug(),
                        car.debug()
                    )
                })
                .collect();
            eprintln!(
                "CATCHUP heat={heat} automatic={automatic} entry_ratio={entry_ratio:.2} t={:.1} player={speed:.1} result={result:?} {}; {}",
                tick as f32 * STEP,
                world.pursuit_status(),
                units.join(" | ")
            );
        }
        assert!(world.cars.iter().all(AiCar::is_finite));
    }
    eprintln!(
        "CATCHUP_SUMMARY heat={heat} automatic={automatic} entry_ratio={entry_ratio:.2} player={speed:.1} {result:?}"
    );
    result
}

#[test]
#[ignore = "needs installed actual police fleets, geometry, physics and heat tables"]
fn installed_real_heat_fleets_close_a_deficit_and_sustain_freeway_pursuit() {
    let (mut scene, mut renderer, models) = loaded();
    let mut failures = Vec::new();
    for (heat, speed) in [(1, 40.0), (2, 45.0), (3, 50.0), (4, 55.0), (5, 60.0)] {
        let result = encounter(&scene, &models, heat, speed, 0.7, false);
        // Engineering acceptance: close a 100 m initial deficit and retain a moving
        // pursuer in engagement range. These are not fitted original-game speeds.
        if result.closest > 60.0 || result.matched_ticks * 2 < result.late_ticks {
            failures.push(format!("heat={heat} player={speed} {result:?}"));
        }
    }
    scene.close(&mut renderer);
    assert!(failures.is_empty(), "actual heat fleets must catch and sustain pursuit:\n{}", failures.join("\n"));
}

#[test]
#[ignore = "needs installed actual heat-5 fleet and production automatic reinforcement placement"]
fn installed_heat_five_automatic_fleet_stays_with_a_fast_target_after_entry() {
    let (mut scene, mut renderer, models) = loaded();
    let result = encounter(&scene, &models, 5, 60.0, 0.0, true);
    scene.close(&mut renderer);
    assert!(result.initial.is_some(), "the ordinary manager must deploy its actual fleet");
    assert!(
        result.matched_ticks * 2 >= result.late_ticks,
        "automatic heat-5 reinforcements must remain competitive: {result:?}"
    );
}

#[test]
#[ignore = "needs installed actual heat-5 fleet and sustained physical pursuit"]
fn installed_established_heat_five_unit_closes_and_sustains_a_fast_target() {
    let (mut scene, mut renderer, models) = loaded();
    let result = encounter(&scene, &models, 5, 85.0, 0.9, false);
    scene.close(&mut renderer);
    assert!(result.closest < 60.0, "the established trailing unit must close: {result:?}");
    assert!(
        result.original_late_matched_ticks * 2 >= result.late_ticks,
        "the same established unit must sustain late engagement: {result:?}"
    );
}

#[test]
#[ignore = "needs installed heat-5 physics; adverse entry is a diagnostic, not a guaranteed forty-second capture"]
fn installed_adverse_fast_entry_reaches_physical_nitrous_and_positive_closing() {
    let (mut scene, mut renderer, models) = loaded();
    let result = encounter(&scene, &models, 5, 85.0, 0.7, false);
    scene.close(&mut renderer);
    // This deliberately adverse initial state lost contact in the retained run.
    // Keep that result visible; native rules do not promise a catch within40s.
    assert!(result.original_burning_ticks > 0, "NOS decisions must reach actual combustion: {result:?}");
    assert!(result.original_max_speed > 85.0, "the actual body must exceed target speed: {result:?}");
    assert!(result.greatest_recovery > 20.0, "physical motion must recover a real deficit: {result:?}");
}
