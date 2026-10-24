//! Complementary city acceptance with a physical player; the recorded-target stress is unchanged.
//! Spec: `docs/specs/ai-pursuit-physical-target-acceptance.md`.

mod route;

use super::installed_city_catchup::{Trace, road_cursor};
use super::*;
use crate::scenes::world::WorldScene;
use crate::scenes::world::ai::PlayerView;
use crate::scenes::world::drive::{CarRig, CarSim, DriveInput, STEP};
use crate::scenes::world::props::PropWorld;
use crate::viewer::{Scene, camera::Lens};
use blackbox_driver::{CarLimits, ControllerKind, DriveFlags, DriveRequest, Driver, potential_speed};

const SECONDS: f32 = 35.0;
const INITIAL_SPEED: f32 = 50.0;

fn stream(scene: &mut WorldScene, renderer: &mut NullBackend, world: &TrafficWorld, player: Vec3) {
    let mut centres: Vec<_> = world.cars.iter().map(|car| [car.position().x, car.position().y]).collect();
    let at = space::to_render(player.to_array());
    centres.push([at.x, at.y]);
    for _ in 0..2000 {
        scene.residency.update_centres(renderer, &centres);
        let collision = scene.residency.collision();
        if scene.residency.complete()
            && collision.resident_near(player.to_array(), 30.0) != Some(false)
            && world
                .cars
                .iter()
                .all(|car| collision.resident_near(car.physics_position().to_array(), 30.0) != Some(false))
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("physical target fixture collision did not become resident");
}

fn body(player: &CarSim, dimensions: Vec3) -> Body {
    let view = player.driver_view();
    Body {
        position: view.position,
        velocity: player.velocity(),
        forward: Vec2::new(view.forward.x, view.forward.z).normalize_or_zero(),
        half_width: dimensions.x,
        half_length: dimensions.z,
    }
}

fn focus_of(body: Body) -> Focus {
    let position = space::to_render(body.position.to_array());
    let forward = space::to_render([body.forward.x, 0.0, body.forward.y]);
    Focus {
        position: [position.x, position.y],
        heading: forward.y.atan2(forward.x),
        speed: body.velocity.length(),
        body: Some(body),
    }
}

#[derive(Debug)]
struct Results {
    initial_gap: f32,
    closest_gap: f32,
    progress: f32,
    speed_sum: f32,
    supported: u32,
    max_route_error: f32,
    off_route_run: u32,
    max_off_route_run: u32,
    stall_run: u32,
    max_stall_run: u32,
    original_present: u32,
    original_max_speed: f32,
    original_nos: u32,
    original_motion: Option<(f32, f32, f32)>,
    late_ticks: u32,
    late_matched: u32,
    late_contact: u32,
}

#[test]
#[ignore = "needs installed city and explicit private NFSMW_PURSUIT_TARGET_TRACE as route geometry only"]
fn installed_physical_bmw_and_heat_five_cop_drive_a_matched_city_route() {
    let trace = Trace::load();
    let route = route::Route::new(trace.frames.iter().map(|frame| frame.body.position).collect());
    let first = trace.frames[0].body;
    let at = space::to_render(first.position.to_array());
    let (mut scene, mut renderer, world) = super::installed_route::open_at([at.x, at.y]);
    let rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    let row = &rows[4];
    let dominant = nfsmw_data::pursuit::ground_cops(row)
        .max_by_key(|record| record.count.saturating_mul(record.chance))
        .unwrap()
        .name
        .clone();
    let names: Vec<_> = nfsmw_data::pursuit::ground_cops(row).map(|record| record.name.clone()).collect();
    scene.traffic = Some(world);
    crate::scenes::world::ai::models::load_cop_models(&mut scene, &mut renderer, &names).unwrap();
    crate::scenes::world::ai::models::load_requested(&mut scene, &mut renderer, false, true);
    let mut world = scene.traffic.take().unwrap();
    assert!(names.iter().all(|name| world.has_cop_model(name)));
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model = nfsmw_data::car::load_driving(
        &dir,
        "BMWM3GTR",
        &nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None },
    )
    .expect("installed production BMW geometry");
    let physics = scene.physics_of(&model).expect("installed stock BMW physics");
    let rig = CarRig::upload(&mut renderer, keys::PLAYER_CAR, model, scene.car_shading, sun::to_sun());
    let dimensions = physics.spec.dimension;
    let performance =
        nfsmw_data::car::physics::measured::ai_performance_of_spec(&physics.spec, physics.tire_integration);
    let limits = CarLimits {
        top_speed: performance.top_speed,
        start_grip: performance.start_grip,
        end_grip: performance.end_grip,
        top_speed_multiplier: 1.0,
        acceleration_multiplier: 1.0,
    };
    let mut player = CarSim::for_rig(physics, &rig);
    player.set_automatic(true);
    let forward = space::to_render((route.point(5.0) - route.point(0.0)).normalize().to_array());
    assert!(player.place_moving(
        &WorldGround { collision: scene.residency.collision(), surfaces: &scene.physics.surfaces },
        Spawn { position: at, heading: forward.y.atan2(forward.x) },
        INITIAL_SPEED,
    ));
    let initial = body(&player, dimensions);
    let mut nav = super::installed_city_placement::trailing(&world, road_cursor(&world, initial), initial);
    super::installed_city_placement::prepare_cursor(&world, &dominant, &mut nav);
    stream(&mut scene, &mut renderer, &world, nav.position);
    let spot =
        CopSpot { at: nav.position, target: initial.position, speed: INITIAL_SPEED, player: performance, band: None };
    assert!(world.spawn_cop_on_road(
        &dominant,
        spot,
        nav.clone(),
        scene.residency.collision(),
        &scene.physics.surfaces
    ));
    let original = world.cars.last().unwrap();
    super::installed_city_placement::assert_handoff(original, &nav);
    let identity = original.audio_emitter().id;
    let initial_gap = original.physics_position().distance(initial.position);
    world.set_career_bin(Some(16));
    world.police.set_forced_pursuit(true);
    world.player_position = Some(initial.position);
    world.note_primary_target(0);
    world.start_pursuit(5, rows, performance);
    let mut props = PropWorld::default();
    let mut driver = Driver::new(ControllerKind::Pid, false);
    let ticks = (SECONDS / STEP).round() as u32;
    let mut result = Results {
        initial_gap,
        closest_gap: initial_gap,
        progress: 0.0,
        speed_sum: 0.0,
        supported: 0,
        max_route_error: 0.0,
        off_route_run: 0,
        max_off_route_run: 0,
        stall_run: 0,
        max_stall_run: 0,
        original_present: 0,
        original_max_speed: 0.0,
        original_nos: 0,
        original_motion: None,
        late_ticks: 0,
        late_matched: 0,
        late_contact: 0,
    };
    eprintln!(
        "PHYSICAL_CITY_SETUP {} route_length={} player_performance={performance:?} cop={dominant} id={identity} initial_gap={initial_gap} initial_speed={INITIAL_SPEED}",
        trace.identity,
        route.length()
    );
    for tick in 0..ticks {
        stream(&mut scene, &mut renderer, &world, player.driver_view().position);
        let view = player.driver_view();
        let (progress, error) = route.project(view.position, result.progress);
        result.progress = progress;
        let look = 30.0 + 70.0 * (view.forward_speed.abs() / 100.0).clamp(0.0, 1.0);
        assert!(progress + look < route.length(), "physical route exhausted before the fixed test duration");
        let curvature = route.curvature(progress, look);
        let requested = potential_speed(&limits, curvature, 1.0, None);
        let (steering_look, chord_deviation) = route.steering_preview(progress, look, dimensions.x * 2.0);
        let controls = driver.step(
            &view,
            &DriveRequest { target: route.point(progress + steering_look), speed: requested, flags: DriveFlags::FULL },
            STEP,
            tick as f32 * STEP,
        );
        assert!(controls.gear.is_none(), "forward physical route follower must not reverse: {controls:?}");
        let input = DriveInput {
            throttle: controls.gas,
            brake: controls.brake,
            steer: controls.steer,
            handbrake: controls.handbrake > 0.5,
            ..Default::default()
        };
        let collision = scene.residency.collision();
        let surfaces = &scene.physics.surfaces;
        player.step(&input, &WorldGround { collision, surfaces }, Some((collision, &mut props)));
        world.step(focus_of(body(&player, dimensions)), collision, &mut props, surfaces);
        world.collide(Some(&mut player));
        let target = body(&player, dimensions);
        let focus = focus_of(target);
        let position = space::to_render(target.position.to_array());
        let forward = space::to_render(player.driver_view().forward.to_array());
        let camera = Lens::look_at(position - forward * 8.0 + Vec3::Z * 3.0, position, 60.0, 0.1, 1.0);
        world.finish_step(focus, &camera, &camera, &props, collision, surfaces);
        let cast = crate::scenes::world::drive::sight::sight_cast(collision);
        world.update_sight(STEP, &cast);
        let nos = player.nitrous_spec();
        world.update_police(
            STEP,
            &PlayerView {
                position,
                velocity: space::to_render(target.velocity.to_array()),
                forward,
                speed: player.driver_view().forward_speed,
                nitrous: player.telemetry().nitrous_burn(),
                nos_input: player.nos_input(),
                nitrous_boost: if nos.present() { nos.torque_boost } else { 0.0 },
                throttle: input.throttle,
            },
            &|from, to| cast(from, to).is_none(),
        );
        assert!(player.is_finite() && world.cars.iter().all(AiCar::is_finite));
        assert_eq!(world.police.level(), Some(5));
        let speed = target.velocity.length();
        result.speed_sum += speed;
        result.supported += u32::from(player.telemetry().wheels_on_ground >= 2);
        result.max_route_error = result.max_route_error.max(error);
        result.off_route_run = if error > 8.0 { result.off_route_run + 1 } else { 0 };
        result.max_off_route_run = result.max_off_route_run.max(result.off_route_run);
        result.stall_run = if speed < 5.0 { result.stall_run + 1 } else { 0 };
        result.max_stall_run = result.max_stall_run.max(result.stall_run);
        let late = tick as f32 * STEP >= SECONDS - 10.0;
        result.late_ticks += u32::from(late);
        result.late_contact += u32::from(late && world.police.has_contact());
        result.original_motion = None;
        if let Some(car) = world.cars.iter().find(|car| car.audio_emitter().id == identity) {
            let gap = car.physics_position().distance(target.position);
            result.original_motion = Some((gap, car.speed(), car.drive_request().1));
            result.closest_gap = result.closest_gap.min(gap);
            result.original_present += 1;
            result.original_max_speed = result.original_max_speed.max(car.speed());
            result.original_nos += u32::from(car.controls().nos);
            result.late_matched += u32::from(
                late && car.is_cop()
                    && !car.is_searching()
                    && !car.is_fleeing()
                    && gap < 150.0
                    && car.speed() >= speed * 0.75
                    && car.sight_age() < 1.0,
            );
        }
        if tick.is_multiple_of(60) {
            let original = world
                .cars
                .iter()
                .find(|car| car.audio_emitter().id == identity)
                .map(AiCar::telemetry_debug)
                .unwrap_or_else(|| "retired".to_owned());
            eprintln!(
                "PHYSICAL_CITY t={:.2} target={target:?} requested={requested:.2} curvature={curvature:.6} speed_preview={look:.3} steering_preview={steering_look:.3} chord_deviation={chord_deviation:.3} route_error={error:.3} input={input:?} telemetry={:?} result={result:?}; original_telemetry={original}; {}",
                tick as f32 * STEP,
                player.telemetry(),
                super::installed_route::diagnostic(&world)
            );
        }
    }
    let (turn, height_range) = route.coverage(result.progress);
    let mean_speed = result.speed_sum / ticks as f32;
    eprintln!(
        "PHYSICAL_CITY_SUMMARY mean_speed={mean_speed:.3} turn_degrees={} height_range={height_range:.3} result={result:?}",
        turn.to_degrees()
    );
    rig.release(&mut renderer);
    scene.close(&mut renderer);
    assert!(
        result.progress >= 1500.0
            && mean_speed >= 50.0
            && result.supported * 10 >= ticks * 9
            && turn >= 30.0_f32.to_radians()
            && height_range >= 10.0
            && result.max_off_route_run < 60
            && result.max_stall_run < 60,
        "physical target fixture must cover a supported fast route with bends and grade before police are scored: {result:?} mean={mean_speed} turn={} height={height_range}",
        turn.to_degrees()
    );
    assert!(
        result.closest_gap <= result.initial_gap - 20.0,
        "same original cop must close a meaningful gap: {result:?}"
    );
    assert!(
        result.late_matched * 3 >= result.late_ticks,
        "same original cop must retain late moving engagement: {result:?}"
    );
    assert!(result.late_contact * 2 >= result.late_ticks, "pursuit must retain real late contact: {result:?}");
}
