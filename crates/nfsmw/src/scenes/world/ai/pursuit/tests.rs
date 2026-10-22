//! A wave row whose cop model is still on the loader's worker when the chase starts. The row stays in the wave:
//! each tick `update_pursuit` picks from the wave again, and a car that cannot stand yet arms no delay, so the row
//! spawns once its model arrives. (`begin_pursuit` drops only the rows whose models failed in a warm-up,
//! `cop_rows::without_models`.) The support requests (`ai/support`) follow the same loader path: a chase asks for
//! its support cars when it starts, a support car waits for its model, and a model that failed drops its request.

use std::collections::HashMap;

use blackbox_collision::{Article, CollisionPack, Instance, PackedVert, STRIP_UP_FACING, Strip};
use blackbox_gfx::{Capabilities, GraphicsApi};
use blackbox_pursuit::support::{Order, Request, Strategy};
use blackbox_roads::{Body, Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, Zone, zone};
use blackbox_vehicle::VehicleSpec;
use blackbox_vehicle::performance::TABLE_SIZE;
use glam::Vec2;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};
use nfsmw_data::pursuit::{AiVehicle, CopRecord, HeatRow};

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::ai::ModelRequest;
use crate::scenes::world::{keys, sun};
use crate::settings::CarShading;

mod air;
mod arrest;
pub(in crate::scenes::world::ai) mod arrest_fixture;
mod budget;
mod call_911;
mod destruction;
mod entities;
mod fleet;
mod frame_rate;
mod free_roam;
mod heat_progress;
mod installed;
mod installed_arrest;
mod installed_breaker;
mod installed_hiding;
mod installed_route;
mod lockout;
mod placement;
mod recycling;
mod roadblock;
mod sirens;
mod speech_events;
mod speech_feed;
mod state_hash;

/// The frames the model is still loading for, and the length of each frame in simulated seconds.
const LOADING_FRAMES: u32 = 30;
const FRAME: f32 = 1.0 / 30.0;

/// A straight road along the z axis through the origin, `2 * half` m long, with a traffic lane either side.
fn straight_road(half: f32) -> RoadNetwork {
    let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
    let node = |z| RoadNode { position: Vec3::new(0.0, 0.0, z), profile: 0, segments: vec![0] };
    RoadNetwork {
        nodes: vec![node(-half), node(half)],
        segments: vec![RoadSegment {
            nodes: [0, 1],
            length: 2.0 * half,
            road: Some(0),
            flags: 0,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        }],
        profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
}

/// The road of the wave tests: 140 m long.
fn cop_road() -> RoadNetwork {
    straight_road(70.0)
}

/// The floor's half size in vertex units. A vertex is 1/128 m (`VERT_SCALE`), so this is 200 m. The cop road's
/// lanes sit 2 m either side of the origin, so the floor has to reach well past them. The drive tests' floor is
/// only 1 m to each side, which would miss them.
const FLOOR_EDGE: i16 = 25_600;
/// The culling radius of the floor and its instance, metres: a probe farther than this from the origin is not tested.
const FLOOR_RADIUS: f32 = 300.0;

/// A flat floor 200 m to each side of the origin, built as the drive's ground tests build theirs.
fn floor() -> CollisionWorld {
    let edge = FLOOR_EDGE;
    let corners = [(-edge, -edge), (-edge, edge), (edge, -edge), (edge, edge)];
    let strip = Strip {
        center: [0.0; 3],
        radius: FLOOR_RADIUS,
        flags: STRIP_UP_FACING,
        verts: corners.map(|(x, z)| PackedVert { x, y: 0, z, surface: 0, flags: 0 }).to_vec(),
    };
    let mut pack = CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 200.0,
            half_length: 200.0,
            half_height: 1.0,
            radius: FLOOR_RADIUS,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![strip],
            barriers: vec![],
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    };
    pack.apply_group_exclusion();
    let mut world = CollisionWorld::new(None);
    world.insert(pack);
    world
}

fn player() -> Performance {
    Performance { top_speed: 60.0, acceleration: [2.0; TABLE_SIZE], start_grip: 1.0, end_grip: 1.0 }
}

/// A heat row whose wave is one cop car of type `copx`.
fn wave() -> HeatRow {
    HeatRow {
        cops: vec![CopRecord { name: "copx".into(), count: 1, chance: 100 }],
        patrol_cars: 0,
        full_engagement_cops: 1,
        time_between_first_four_spawns: 0.0,
        time_between_cop_spawns: 0.0,
    }
}

/// The player's car as the chase sees it: the target of the wave, standing at the origin.
fn focus() -> Focus {
    Focus {
        position: [0.0, 0.0],
        heading: 0.0,
        speed: 0.0,
        body: Some(Body {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            forward: Vec2::X,
            half_width: 1.0,
            half_length: 2.0,
        }),
    }
}

/// A cop car model of type `name` that stands on the floor: a physics spec and a rig with no parts.
fn cop_model(renderer: &mut NullBackend, name: &str, player: Performance) -> CopModel {
    let spec = VehicleSpec::example();
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO };
    let physics = CarPhysics {
        effect_source: 0,
        tire_integration: Default::default(),
        jump_config: Default::default(),
        spec,
        bounds,
        walls: WallSpec::default(),
        geometry: None,
        events: None,
        reactions: None,
    };
    let model = CarModel {
        driving_lods: Default::default(),
        flare_profiles: Default::default(),
        name: name.into(),
        window_tint_material: None,
        car_type: None,
        solids: HashMap::new(),
        placements: Vec::new(),
        textures: HashMap::new(),
        swaps: Default::default(),
        paint: None,
        paintable: false,
        light_materials: HashMap::new(),
        floor_height: 0.0,
        corners: None,
        wheel_setup: None,
    };
    let rig = Rc::new(CarRig::upload(renderer, keys::next_car(), model, CarShading::default(), sun::to_sun()));
    CopModel {
        model: TrafficModel {
            name: name.into(),
            rig,
            physics,
            tractor: false,
            trailer: None,
            cop_lights: None,
            audio_profile: Default::default(),
            keys: keys::test_keys(),
        },
        car: CopCar { performance: player, ai: AiVehicle::default() },
    }
}

fn cop_count(world: &TrafficWorld) -> usize {
    world.cars.iter().filter(|car| car.is_cop()).count()
}

#[test]
fn a_wave_row_whose_model_is_still_loading_spawns_once_the_model_arrives() {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let player = player();
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut world = TrafficWorld::new(cop_road());
    world.start_pursuit(1, vec![wave()], player);

    // The model is not in yet: the row stays in the wave, nothing is placed and no spawn delay is armed.
    for _ in 0..LOADING_FRAMES {
        world.update_pursuit(FRAME, focus(), &collision, &surfaces);
    }
    assert_eq!(cop_count(&world), 0);
    let pursuit = world.pursuit.as_ref().expect("the chase runs");
    assert_eq!(pursuit.wave().map(|w| w.0).collect::<Vec<_>>(), ["copx"]);
    assert_eq!(pursuit.involved, 0);
    assert!(pursuit.spawn_timer <= 0.0, "a car that did not stand arms no delay");

    // The model arrives: the same row spawns.
    world.add_cop_model(cop_model(&mut renderer, "copx", player));
    let spot = CopSpot { at: Vec3::ZERO, target: Vec3::new(0.0, 0.0, 70.0), speed: START_SPEED, player, band: None };
    assert!(world.spawn_cop_at("copx", spot, &collision, &surfaces));
    assert_eq!(cop_count(&world), 1);
}

#[test]
fn a_cop_model_that_fails_mid_chase_leaves_its_entries_out_of_every_level() {
    let mut world = TrafficWorld::new(cop_road());
    let mut both = wave();
    both.cops.push(CopRecord { name: "copy".into(), count: 2, chance: 50 });
    world.start_pursuit(1, vec![both, wave()], player());

    // The model of `copx` fails on the loader's worker: its entries go from the levels of the chase, `copy` stays.
    world.cop_model_failed("COPX");
    let pursuit = world.pursuit.as_ref().expect("the chase runs");
    assert_eq!(pursuit.wave().map(|w| w.0).collect::<Vec<_>>(), ["copy"]);
    assert!(pursuit.rows[1].cops.is_empty(), "the level that had only the failed car is empty, not removed");
    assert_eq!(pursuit.rows.len(), 2);
}

#[test]
fn a_failed_cop_model_leaves_the_wave_to_the_loaded_cars_only_so_the_chase_does_not_wait() {
    let mut world = TrafficWorld::new(cop_road());
    world.start_pursuit(1, vec![wave()], player());
    world.cop_model_failed("copx");
    // With its only car gone the wave is empty: nothing is placed and the chase neither waits nor arms a delay.
    let collision = floor();
    let surfaces = SurfaceTable::default();
    for _ in 0..LOADING_FRAMES {
        world.update_pursuit(FRAME, focus(), &collision, &surfaces);
    }
    let pursuit = world.pursuit.as_ref().expect("the chase runs");
    assert_eq!(pursuit.wave().count(), 0);
    assert_eq!(cop_count(&world), 0);
    assert_eq!(pursuit.involved, 0);
}

#[test]
fn a_failed_model_with_no_chase_running_changes_nothing() {
    let mut world = TrafficWorld::new(cop_road());
    world.cop_model_failed("copx");
    assert!(world.pursuit.is_none());
}

/// A support request of `strategy` with a 30 s option.
fn support(strategy: Strategy) -> Request {
    Request { strategy, duration: 30.0 }
}

/// The player's car as the support requests see it: standing at `z` on the road, heading along it (+z). A heavy car is
/// placed ahead of it and a leader car behind.
fn support_focus(z: f32) -> Focus {
    Focus {
        position: [0.0, 0.0],
        heading: 0.0,
        speed: 0.0,
        body: Some(Body {
            position: Vec3::new(0.0, 0.0, z),
            velocity: Vec3::ZERO,
            forward: Vec2::Y,
            half_width: 1.0,
            half_length: 2.0,
        }),
    }
}

/// The cars the support requests have on the road.
fn support_count(world: &TrafficWorld) -> usize {
    world.cars.iter().filter(|car| car.support_serial().is_some()).count()
}

/// The chase's pending or active support request, if there is one.
fn support_order(world: &TrafficWorld) -> Option<&Order> {
    world.pursuit.as_ref().and_then(|pursuit| pursuit.support.order())
}

#[test]
fn starting_a_pursuit_asks_for_the_support_cars_that_are_not_loaded() {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(cop_model(&mut renderer, "copcross", player()));
    world.start_pursuit(1, vec![wave()], player());

    let asked = world.take_model_requests();
    for name in ["copsuv", "copsuvl"] {
        assert!(asked.contains(&ModelRequest::Support(name.to_owned())), "{name} is asked for at the chase's start");
    }
    assert!(!asked.contains(&ModelRequest::Support("copcross".to_owned())), "a loaded car is not asked for again");
}

#[test]
fn a_support_car_whose_model_arrives_late_spawns_once_it_is_in() {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut world = TrafficWorld::new(straight_road(300.0));
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.ask_support(support(Strategy::CrossFollow)));

    for _ in 0..LOADING_FRAMES {
        world.update_pursuit(FRAME, support_focus(0.0), &collision, &surfaces);
    }
    // The cross car's model is still loading: the request waits pending, is not denied, and no car stands.
    let order = support_order(&world).expect("the request is pending");
    assert!(order.is_pending());
    assert_eq!(order.denials(), 0, "a model still loading is not a refused placement");
    assert_eq!(support_count(&world), 0);

    world.add_cop_model(cop_model(&mut renderer, "copcross", player()));
    world.update_pursuit(FRAME, support_focus(0.0), &collision, &surfaces);
    let order = support_order(&world).expect("the request is still running");
    assert!(order.is_active(), "the car stands on the next step after its model arrives");
    assert_eq!(support_count(&world), 1);
}

#[test]
fn a_support_model_that_fails_drops_its_request_and_other_support_still_goes() {
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut world = TrafficWorld::new(straight_road(300.0));
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.ask_support(support(Strategy::CrossFollow)));
    world.update_pursuit(FRAME, support_focus(0.0), &collision, &surfaces);
    assert!(support_order(&world).is_some_and(|order| order.is_pending()), "the request waits for its car");

    // The cross car's model fails to load: the request that takes it is dropped, and its strategy is refused.
    world.cop_model_failed("copcross");
    world.update_pursuit(FRAME, support_focus(0.0), &collision, &surfaces);
    assert!(support_order(&world).is_none(), "the request is dropped");
    assert!(!world.ask_support(support(Strategy::CrossFollow)), "its strategy is refused for the chase");

    // Other support is not blocked: the small SUV (`copsuvl`, the EBrake's car) is loaded and stands. The target is
    // moved to z = -150 so the SUV's placement 340 m ahead (z = 190) stays on the floor.
    world.add_cop_model(cop_model(&mut renderer, "copsuvl", player()));
    assert!(world.ask_support(support(Strategy::EBrake)), "other support is taken");
    world.update_pursuit(FRAME, support_focus(-150.0), &collision, &surfaces);
    let order = support_order(&world).expect("the other request is running");
    assert!(order.is_active());
    assert_eq!(support_count(&world), 1);
}
