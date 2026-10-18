//! A kinematic target exercises real-road police physics and real visibility; no sight answers are injected.

use super::*;
use crate::input::ActionState;
use crate::scenes::world::{Options, WorldScene, drive::STEP, props::PropWorld};
use crate::viewer::{Scene, camera::Lens};
use blackbox_roads::{SplitMix, zone};
use game_install::GameDir;

const LOADED_START: [f32; 2] = [2974.2, 3001.9];

fn diagnostic(world: &TrafficWorld) -> String {
    let cars = world
        .cars
        .iter()
        .map(|car| {
            format!(
                "{} id={} position={:?} velocity={:?} airborne={:.2} searching={} controls={:?}; {}",
                car.name,
                car.audio_emitter().id,
                car.physics_position(),
                car.body().velocity,
                car.airborne_time(),
                car.is_searching(),
                car.controls(),
                car.debug(),
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "raw_sight={} tracked_sight={} {}; {cars}",
        world.police.has_contact(),
        world.police.is_perp_in_sight(),
        world.pursuit_status()
    )
}

fn supported_start(
    world: &TrafficWorld,
    model: &CopModel,
    collision: &CollisionWorld,
    surfaces: &SurfaceTable,
) -> (AiCar, RoadNav) {
    let filter = SegmentFilter { traffic: true, cop: true, no_decision: true };
    let mut candidates = Vec::new();
    for (i, segment) in world.network.segments.iter().enumerate() {
        if !filter.allows(segment) || segment.length < 60.0 {
            continue;
        }
        let profile = world.network.profile_at(i as u16, segment.nodes[0]);
        for (lane, zone) in profile.zones.iter().enumerate().filter(|(_, z)| z.kind == zone::TRAFFIC) {
            if zone.width < 2.0 * model.model.physics.spec.dimension.x {
                continue;
            }
            for t in [0.15, 0.35, 0.55, 0.75] {
                let mut nav = RoadNav::traffic();
                nav.init_in_lane(&world.network, i as u16, lane, t);
                let at = space::to_render(nav.position.to_array());
                let gap = (at.x - LOADED_START[0]).hypot(at.y - LOADED_START[1]);
                if gap <= 300.0 {
                    candidates.push((gap, nav));
                }
            }
        }
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut tested = 0;
    let mut supported = 0;
    let mut rng = SplitMix(53_012);
    for (_, mut nav) in candidates {
        tested += 1;
        let mut position = space::to_render(nav.position.to_array());
        let surface = crate::scenes::world::road::probe(collision, position.x, position.y, position.z + 30.0);
        let Some(surface) = surface.filter(|surface| surface.road) else { continue };
        position.z = surface.height;
        nav.half_width = model.model.physics.spec.dimension.x;
        nav.enable_trail(&world.network);
        let facing = space::to_render(nav.forward.to_array());
        let start =
            Start { speed: 10.0, stagger: 0, cruise: Cruise::patrol(35.0 * 0.44704, 55.0 * 0.44704), patrol: true };
        let Some(car) = AiCar::place(
            &model.model,
            Role::Traffic,
            nav.clone(),
            Spawn { position, heading: facing.y.atan2(facing.x) },
            start,
            &WorldGround { collision, surfaces },
        ) else {
            eprintln!(
                "unsupported patrol candidate segment={} lane={} nav={:?} grounded_spawn={position:?}",
                nav.segment, nav.lane, nav.position
            );
            continue;
        };
        supported += 1;
        assert!(
            car.physics_position().distance(car.body().position) < 1e-4,
            "police sensing and navigation must use the same current body origin"
        );
        let mut ahead = nav.clone();
        ahead.advance_with_lookahead(&world.network, 45.0, Vec3::ZERO, 0.0, &mut rng);
        let target = target(&ahead, 18.0);
        let target_render = space::to_render(target.position.to_array());
        let target_ground =
            crate::scenes::world::road::probe(collision, target_render.x, target_render.y, target_render.z + 10.0);
        if ahead.dead_end || target_ground.is_none_or(|surface| !surface.road) {
            continue;
        }
        let cast = crate::scenes::world::drive::sight::sight_cast(collision);
        if cast(car.physics_position() + Vec3::Y * 0.5, target.position + Vec3::Y * 0.5).is_some() {
            continue;
        }
        eprintln!(
            "resident patrol route selected after {tested} samples/{supported} supported: segment={} lane={} nav={:?} spawn={position:?} physical={:?} target45m={:?}",
            nav.segment,
            nav.lane,
            nav.position,
            car.physics_position(),
            target.position
        );
        return (car, nav);
    }
    panic!(
        "no supported resident patrol route near {LOADED_START:?}: {tested} legal samples, {supported} successful physical placements"
    );
}

fn target(nav: &RoadNav, speed: f32) -> Body {
    Body {
        position: nav.position + Vec3::Y * 0.5,
        velocity: nav.forward * speed,
        forward: Vec2::new(nav.forward.x, nav.forward.z).normalize_or_zero(),
        half_width: 1.0,
        half_length: 2.3,
    }
}

fn tick(
    world: &mut TrafficWorld,
    body: Body,
    collision: &CollisionWorld,
    surfaces: &SurfaceTable,
    props: &mut PropWorld,
) {
    let position = space::to_render(body.position.to_array());
    let forward = space::to_render([body.forward.x, 0.0, body.forward.y]);
    let heading = forward.y.atan2(forward.x);
    let focus = Focus { position: [position.x, position.y], heading, speed: body.velocity.length(), body: Some(body) };
    let camera = Lens::look_at(position - forward * 8.0 + Vec3::Z * 3.0, position, 60.0, 0.1, 1.0);
    world.step(focus, collision, props, surfaces);
    world.collide(None);
    world.finish_step(focus, &camera, props, collision, surfaces);
    world.player_position = Some(body.position);
    let cast = crate::scenes::world::drive::sight::sight_cast(collision);
    world.update_sight(STEP, &cast);
    world.update_police(
        STEP,
        &crate::scenes::world::ai::PlayerView {
            position,
            forward,
            speed: body.velocity.length(),
            throttle: f32::from(u8::from(body.velocity.length() > 0.0)),
            nitrous: 0.0,
        },
        &|from, to| cast(from, to).is_none(),
    );
    assert!(world.cars.iter().all(AiCar::is_finite), "physical police state must remain finite: {}", diagnostic(world));
}

#[test]
#[ignore = "needs installed city collision, road network and patrol geometry selected by NFSMW_GAME_DIR"]
fn installed_police_drive_stop_search_and_reacquire_on_real_roads() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: Some(LOADED_START),
            height: 40.0,
            heading: 0.0,
            pitch: -15.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: None,
        },
    )
    .unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    for _ in 0..2000 {
        scene.update(&mut renderer, &ActionState::default(), STEP);
        if scene.ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        scene.ready() && scene.residency.complete(),
        "installed local collision must be fully resident: {:?}",
        scene.residency.counts()
    );
    crate::scenes::world::ai::models::load_cop_models(&mut scene, &mut renderer, &["copmidsize".into()]).unwrap();
    crate::scenes::world::ai::models::load_requested(&mut scene, &mut renderer, false, true);
    let mut world = scene.traffic.take().unwrap();
    world.cars.clear();
    world.configure(0, 0);
    let model = world.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case("copmidsize")).unwrap();
    let performance = blackbox_vehicle::performance::measure_with_integration(
        &model.model.physics.spec,
        model.model.physics.tire_integration,
    );
    let mut rng = SplitMix(53_012);
    let collision = scene.residency.collision();
    let surfaces = &scene.physics.surfaces;
    let (car, mut route) = supported_start(&world, model, collision, surfaces);
    let (segment, lane) = (route.segment, route.lane);
    let identity = car.audio_emitter().id;
    let initial = car.physics_position();
    let initial_forward = car.physics_forward();
    world.cars.push(car);
    route.advance_with_lookahead(&world.network, 45.0, Vec3::ZERO, 0.0, &mut rng);
    let mut body = target(&route, 18.0);
    world.player_position = Some(body.position);
    {
        let cast = crate::scenes::world::drive::sight::sight_cast(collision);
        world.update_sight(0.25, &cast);
        assert!(world.sighting(1).seen, "the real road fixture starts within actual clear sight");
    }
    let mut rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    // This single-unit fixture isolates the physical chase/search transition from wave replacement pressure.
    for row in &mut rows {
        row.cops.clear();
    }
    world.start_pursuit(1, rows, performance);
    let mut props = PropWorld::default();
    for _ in 0..120 {
        route.advance_with_lookahead(&world.network, 18.0 * STEP, Vec3::ZERO, 0.0, &mut rng);
        body = target(&route, 18.0);
        tick(&mut world, body, collision, surfaces, &mut props);
    }
    assert_eq!(world.cars[0].audio_emitter().id, identity);
    let displacement = world.cars[0].physics_position() - initial;
    assert!(
        displacement.dot(initial_forward) > 3.0,
        "the real patrol must advance along its route: displacement={displacement:?} initial={initial:?} forward={initial_forward:?}; {}",
        diagnostic(&world)
    );
    assert!(
        displacement.length() < 100.0,
        "the physical car cannot jump down the route: displacement={displacement:?}; {}",
        diagnostic(&world)
    );
    for _ in 0..30 {
        tick(&mut world, target(&route, 0.0), collision, surfaces, &mut props);
    }
    assert!(world.police.level().is_some(), "a brief stop at distance is not an immediate bust");
    let mut search = false;
    for _ in 0..900 {
        route.advance_with_lookahead(&world.network, 70.0 * STEP, Vec3::ZERO, 0.0, &mut rng);
        tick(&mut world, target(&route, 70.0), collision, surfaces, &mut props);
        search = world.cars.iter().any(AiCar::is_searching);
        if search {
            break;
        }
    }
    assert!(
        search,
        "real sight must be lost long enough for physical ground units to enter lane search: target={:?} route_segment={} dead_end={}; {}",
        target(&route, 70.0).position,
        route.segment,
        route.dead_end,
        diagnostic(&world)
    );
    let cop = world.cars.iter().find(|car| car.audio_emitter().id == identity).expect("the same unit remains");
    let mut reunion = RoadNav::traffic();
    assert!(reunion.init_at_point(&world.network, &world.index, cop.physics_position(), cop.physics_forward(), false));
    reunion.advance_with_lookahead(&world.network, 25.0, Vec3::ZERO, 0.0, &mut rng);
    // Reposition the kinematic target onto this unit's nearby installed lane to isolate actual reacquisition.
    for _ in 0..60 {
        tick(&mut world, target(&reunion, 15.0), collision, surfaces, &mut props);
        if world.police.has_contact() && world.cars.iter().all(|car| !car.is_searching()) {
            break;
        }
    }
    assert!(
        world.police.has_contact(),
        "range and real geometry must reacquire the visible target: target={:?}; {}",
        target(&reunion, 15.0).position,
        diagnostic(&world)
    );
    assert!(world.cars.iter().all(|car| !car.is_searching()));
    assert!(world.cars.iter().any(|car| car.audio_emitter().id == identity), "reacquisition preserves the unit");
    eprintln!(
        "installed police route: segment {segment}, lane {lane}; initial displacement {displacement:?}; chase/stop/search/reacquisition"
    );
    scene.close(&mut renderer);
}
