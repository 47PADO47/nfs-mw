//! A kinematic target exercises real-road police physics and real visibility; no sight answers are injected.

use super::*;
use crate::input::ActionState;
use crate::scenes::world::{Options, WorldScene, drive::STEP, props::PropWorld};
use crate::viewer::{Scene, camera::Lens};
use blackbox_roads::{LaneType, NavKind, SplitMix, zone};
use game_install::GameDir;

pub(super) const LOADED_START: [f32; 2] = [2974.2, 3001.9];

pub(super) fn diagnostic(world: &TrafficWorld) -> String {
    let cars = world
        .cars
        .iter()
        .map(|car| {
            format!(
                "{} id={} position={:?} velocity={:?} airborne={:.2} searching={} sight_active={} sight_age={:.3} controls={:?}; {}; {}",
                car.name,
                car.audio_emitter().id,
                car.physics_position(),
                car.body().velocity,
                car.airborne_time(),
                car.is_searching(),
                car.sight_active(),
                car.sight_age(),
                car.controls(),
                car.debug(),
                search_navigation_diagnostic(world, car),
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

fn search_navigation_diagnostic(world: &TrafficWorld, car: &AiCar) -> String {
    let mut nav =
        RoadNav::new(NavKind::Traffic, LaneType::Traffic, SegmentFilter { traffic: true, ..Default::default() });
    let initialized =
        nav.init_at_point(&world.network, &world.index, car.physics_position(), car.physics_forward(), false);
    if !initialized {
        return "search_nav_init=false".into();
    }
    let segment = world.network.segment(nav.segment);
    let (selected_segment, selected_lane, selected_node) = (nav.segment, nav.lane, nav.node_ind);
    // A private probe stream keeps failure diagnostics from changing the real driver's random state.
    let spawn_allowed = nav.can_traffic_spawn(&world.network, &mut SplitMix(53_012));
    format!(
        "search_nav_init=true segment={selected_segment} flags={:#06x} decision={} lane={selected_lane} node={selected_node} traffic_spawn_allowed={spawn_allowed}",
        segment.flags,
        segment.is_decision(),
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

pub(super) fn target(nav: &RoadNav, speed: f32) -> Body {
    Body {
        position: nav.position + Vec3::Y * 0.5,
        velocity: nav.forward * speed,
        forward: Vec2::new(nav.forward.x, nav.forward.z).normalize_or_zero(),
        half_width: 1.0,
        half_length: 2.3,
    }
}

pub(super) fn tick(
    world: &mut TrafficWorld,
    body: Body,
    scene: &mut WorldScene,
    renderer: &mut NullBackend,
    props: &mut PropWorld,
) {
    tick_with_boost(world, body, scene, renderer, props, 0.0);
}

pub(super) fn tick_with_boost(
    world: &mut TrafficWorld,
    body: Body,
    scene: &mut WorldScene,
    renderer: &mut NullBackend,
    props: &mut PropWorld,
    nitrous_boost: f32,
) {
    // This fixture isolates physical chase/search, so both the cop and target must
    // retain collision residency. A fixed opening camera would suspend the cop.
    let mut centres: Vec<_> = world
        .cars
        .iter()
        .map(|car| {
            let at = car.position();
            [at.x, at.y]
        })
        .collect();
    let target_at = space::to_render(body.position.to_array());
    centres.push([target_at.x, target_at.y]);
    for _ in 0..2000 {
        scene.residency.update_centres(renderer, &centres);
        let collision = scene.residency.collision();
        let supported = world
            .cars
            .iter()
            .all(|car| collision.resident_near(car.physics_position().to_array(), 30.0) != Some(false));
        if scene.residency.complete() && supported {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(scene.residency.complete(), "route fixture collision must finish streaming");
    let collision = scene.residency.collision();
    assert!(
        world.cars.iter().all(|car| collision.resident_near(car.physics_position().to_array(), 30.0) != Some(false)),
        "the route fixture must keep the physical cop active"
    );
    let surfaces = &scene.physics.surfaces;
    let position = space::to_render(body.position.to_array());
    let forward = space::to_render([body.forward.x, 0.0, body.forward.y]);
    let heading = forward.y.atan2(forward.x);
    let focus = Focus { position: [position.x, position.y], heading, speed: body.velocity.length(), body: Some(body) };
    let camera = Lens::look_at(position - forward * 8.0 + Vec3::Z * 3.0, position, 60.0, 0.1, 1.0);
    world.step(focus, collision, props, surfaces);
    world.collide(None);
    world.finish_step(focus, &camera, &camera, props, collision, surfaces);
    world.player_position = Some(body.position);
    world.player_speed = body.velocity.length();
    let cast = crate::scenes::world::drive::sight::sight_cast(collision);
    world.update_sight(STEP, &cast);
    world.update_police(
        STEP,
        &crate::scenes::world::ai::PlayerView {
            nos_input: false,
            nitrous_boost,
            position,
            velocity: space::to_render(body.velocity.to_array()),
            forward,
            speed: body.velocity.length(),
            throttle: f32::from(u8::from(body.velocity.length() > 0.0)),
            nitrous: 0.0,
        },
        &|from, to| cast(from, to).is_none(),
    );
    assert!(world.cars.iter().all(AiCar::is_finite), "physical police state must remain finite: {}", diagnostic(world));
}

pub(super) fn open() -> (WorldScene, NullBackend, TrafficWorld) {
    open_at(LOADED_START)
}

pub(super) fn open_at(at: [f32; 2]) -> (WorldScene, NullBackend, TrafficWorld) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: Some(at),
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
    (scene, renderer, world)
}

#[test]
#[ignore = "needs installed city collision, road network and patrol geometry selected by NFSMW_GAME_DIR"]
fn installed_police_drive_stop_search_and_reacquire_on_real_roads() {
    let (mut scene, mut renderer, mut world) = open();
    let model = world.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case("copmidsize")).unwrap();
    let performance = blackbox_vehicle::performance::measure_with_integration(
        &model.model.physics.spec,
        model.model.physics.tire_integration,
    );
    let mut rng = SplitMix(53_012);
    let (car, mut route) = supported_start(&world, model, scene.residency.collision(), &scene.physics.surfaces);
    let (segment, lane) = (route.segment, route.lane);
    let identity = car.audio_emitter().id;
    let initial = car.physics_position();
    let initial_forward = car.physics_forward();
    world.cars.push(car);
    route.advance_with_lookahead(&world.network, 45.0, Vec3::ZERO, 0.0, &mut rng);
    let mut body = target(&route, 18.0);
    world.player_position = Some(body.position);
    {
        let cast = crate::scenes::world::drive::sight::sight_cast(scene.residency.collision());
        world.update_sight(0.25, &cast);
        assert!(world.sighting(1).seen, "the real road fixture starts within actual clear sight");
    }
    let mut rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    // This single-unit fixture isolates the physical chase/search transition from wave replacement pressure.
    // The wave is the one unit: a cop of a type the wave does not want would be sent away as surplus.
    for row in &mut rows {
        row.cops = vec![nfsmw_data::pursuit::CopRecord { name: "copmidsize".into(), count: 1, chance: 100 }];
    }
    // Keep the single-unit heat-1 fixture in the stock starting career range.
    world.set_career_bin(Some(16));
    world.start_pursuit(1, rows, performance);
    assert_eq!(world.police.level(), Some(1));
    let mut props = PropWorld::default();
    for _ in 0..120 {
        route.advance_with_lookahead(&world.network, 18.0 * STEP, Vec3::ZERO, 0.0, &mut rng);
        body = target(&route, 18.0);
        tick(&mut world, body, &mut scene, &mut renderer, &mut props);
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
        tick(&mut world, target(&route, 0.0), &mut scene, &mut renderer, &mut props);
    }
    assert!(world.police.level().is_some(), "a brief stop at distance is not an immediate bust");
    let mut search = false;
    for _ in 0..900 {
        route.advance_with_lookahead(&world.network, 70.0 * STEP, Vec3::ZERO, 0.0, &mut rng);
        tick(&mut world, target(&route, 70.0), &mut scene, &mut renderer, &mut props);
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
    // Cooldown refuses ground sightings for its first 2.5 seconds. Leave room
    // for that display gate and this unit's next staggered visibility query.
    for _ in 0..240 {
        tick(&mut world, target(&reunion, 15.0), &mut scene, &mut renderer, &mut props);
        if world.police.has_contact() && world.cars.iter().all(|car| !car.is_searching()) {
            break;
        }
    }
    let collision = scene.residency.collision();
    let reunion_body = target(&reunion, 15.0);
    let reunion_render = space::to_render(reunion_body.position.to_array());
    let reunion_ground =
        crate::scenes::world::road::probe(collision, reunion_render.x, reunion_render.y, reunion_render.z + 10.0);
    let cast = crate::scenes::world::drive::sight::sight_cast(collision);
    let sight_diagnostic: Vec<_> = world
        .cars
        .iter()
        .map(|car| {
            let from = car.physics_position() + Vec3::Y * 0.5;
            let to = reunion_body.position + Vec3::Y * 0.5;
            let hit = cast(from, to).map(|hit| (hit.point, hit.normal, hit.info));
            (car.sight_active(), car.sight_age(), hit)
        })
        .collect();
    drop(cast);
    assert!(
        world.police.has_contact(),
        "range and real geometry must reacquire the visible target: target={:?} ground={reunion_ground:?} allowed={} hidden={} ground_age={:?} unit_sight={sight_diagnostic:?}; {}",
        target(&reunion, 15.0).position,
        world.police.sight_allowed(),
        world.hidden_from_cars(),
        world.police.observed_ground_age(),
        diagnostic(&world)
    );
    assert!(world.cars.iter().all(|car| !car.is_searching()));
    assert_eq!(world.police.level(), Some(1));
    assert!(world.cars.iter().any(|car| car.audio_emitter().id == identity), "reacquisition preserves the unit");
    eprintln!(
        "installed police route: segment {segment}, lane {lane}; initial displacement {displacement:?}; chase/stop/search/reacquisition"
    );
    scene.close(&mut renderer);
}
