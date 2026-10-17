//! A wave row whose cop model is still on the loader's worker when the chase starts. The row stays in the wave:
//! each tick `update_pursuit` picks from the wave again, and a car that cannot stand yet arms no delay, so the row
//! spawns once its model arrives. (`begin_pursuit` drops only the rows whose models failed in a warm-up,
//! `cop_rows::without_models`.)

use std::collections::HashMap;

use blackbox_collision::{Article, CollisionPack, Instance, PackedVert, STRIP_UP_FACING, Strip};
use blackbox_gfx::{Capabilities, GraphicsApi};
use blackbox_roads::{Body, Road, RoadNetwork, RoadNode, RoadProfile, RoadSegment, Zone, zone};
use blackbox_vehicle::VehicleSpec;
use blackbox_vehicle::performance::TABLE_SIZE;
use glam::Vec2;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};
use nfsmw_data::pursuit::{AiVehicle, CopRecord, HeatRow};

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::{keys, sun};
use crate::settings::CarShading;

/// The frames the model is still loading for, and the length of each frame in simulated seconds.
const LOADING_FRAMES: u32 = 30;
const FRAME: f32 = 1.0 / 30.0;

/// A straight road along the z axis through the origin, 140 m long, with a traffic lane either side.
fn cop_road() -> RoadNetwork {
    let lane = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
    let node = |z| RoadNode { position: Vec3::new(0.0, 0.0, z), profile: 0, segments: vec![0] };
    RoadNetwork {
        nodes: vec![node(-70.0), node(70.0)],
        segments: vec![RoadSegment {
            nodes: [0, 1],
            length: 140.0,
            road: Some(0),
            flags: 0,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        }],
        profiles: vec![RoadProfile { middle: 1, zones: vec![lane(2.0), lane(2.0)] }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    }
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

/// A cop car model of type `copx` that stands on the floor: a physics spec and a rig with no parts.
fn cop_model(renderer: &mut NullBackend, player: Performance) -> CopModel {
    let spec = VehicleSpec::example();
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO };
    let physics = CarPhysics {
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
        flare_profiles: Default::default(),
        name: "copx".into(),
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
            name: "copx".into(),
            rig,
            physics,
            tractor: false,
            trailer: None,
            cop_lights: None,
            audio_profile: Default::default(),
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
    world.add_cop_model(cop_model(&mut renderer, player));
    let spot = CopSpot { at: Vec3::ZERO, target: Vec3::new(0.0, 0.0, 70.0), speed: START_SPEED, player };
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
