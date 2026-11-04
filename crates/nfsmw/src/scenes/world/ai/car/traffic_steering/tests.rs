//! Synthetic physical-step checks, independent of an installed game or a GPU.

mod queue;

use std::collections::HashMap;
use std::rc::Rc;

use blackbox_collision::{Article, CollisionPack, CollisionWorld, Instance, PackedVert, STRIP_UP_FACING, Strip};
use blackbox_gfx::{Capabilities, GraphicsApi};
use blackbox_roads::{
    Body, Road, RoadNav, RoadNetwork, RoadNode, RoadProfile, RoadSegment, SegmentIndex, SplitMix, Zone, zone,
};
use blackbox_vehicle::VehicleSpec;
use glam::Vec3;
use nfsmw_data::car::CarModel;
use nfsmw_data::car::physics::{CarBounds, CarPhysics, SurfaceTable, WallSpec};

use super::super::{STEP, Start};
use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::ai::TrafficModel;
use crate::scenes::world::ai::traffic::Cruise;
use crate::scenes::world::drive::{CarRig, WorldGround};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::{keys, space, sun};
use crate::settings::CarShading;

fn fixture(patrol: bool, speed: f32) -> (AiCar, RoadNetwork, CollisionWorld, SurfaceTable) {
    fixture_with_tires(patrol, speed, Default::default())
}

fn fixture_with_tires(
    patrol: bool,
    speed: f32,
    tire_integration: blackbox_vehicle::tires::TireIntegration,
) -> (AiCar, RoadNetwork, CollisionWorld, SurfaceTable) {
    let zone = |offset| Zone { kind: zone::TRAFFIC, width: 4.0, offset };
    let node = |z| RoadNode { position: Vec3::new(0.0, 0.0, z), profile: 0, segments: vec![0] };
    let net = RoadNetwork {
        nodes: vec![node(-100.0), node(300.0)],
        segments: vec![RoadSegment {
            nodes: [0, 1],
            length: 400.0,
            road: Some(0),
            flags: 0,
            start_handle: Vec3::ZERO,
            end_handle: Vec3::ZERO,
        }],
        profiles: vec![RoadProfile { middle: 1, zones: vec![zone(2.0), zone(2.0)] }],
        roads: vec![Road { scale: 1.0, speech_id: 0 }],
    };
    let mut collision = CollisionWorld::new(None);
    collision.insert(CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 100.0,
            half_length: 100.0,
            half_height: 1.0,
            radius: 150.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![Strip {
                center: [0.0; 3],
                radius: 150.0,
                flags: STRIP_UP_FACING,
                verts: [(-12800, -12800), (-12800, 12800), (12800, -12800), (12800, 12800)]
                    .map(|(x, z)| PackedVert { x, y: 0, z, surface: 0, flags: 0 })
                    .to_vec(),
            }],
            barriers: vec![],
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    });
    let surfaces = SurfaceTable::default();
    let spec = VehicleSpec::example();
    let physics = CarPhysics {
        effect_source: 0,
        tire_integration,
        jump_config: Default::default(),
        bounds: CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO },
        spec,
        walls: WallSpec::default(),
        geometry: None,
        events: None,
        reactions: None,
        bike: None,
    };
    let model = CarModel {
        driving_lods: Default::default(),
        flare_profiles: Default::default(),
        name: "trafx".into(),
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
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let rig = Rc::new(CarRig::upload(&mut renderer, keys::next_car(), model, CarShading::default(), sun::to_sun()));
    let model = TrafficModel {
        name: "trafx".into(),
        rig,
        physics,
        tunings: Default::default(),
        tractor: false,
        trailer: None,
        cop_lights: None,
        audio_profile: Default::default(),
        keys: keys::test_keys(),
    };
    let mut nav = RoadNav::traffic();
    nav.init_in_lane(&net, 0, 1, 0.25);
    nav.enable_trail(&net);
    nav.advance(&net, 30.0, Vec3::ZERO, &mut SplitMix(1));
    let spawn = Spawn { position: space::to_render([2.0, 0.0, 0.0]), heading: 0.0 };
    let mut car = AiCar::place(
        &model,
        Role::Traffic,
        nav,
        spawn,
        Start { stagger: 0, speed, cruise: Cruise::default(), patrol },
        &WorldGround { collision: &collision, surfaces: &surfaces },
    )
    .expect("traffic fixture stands on the synthetic road");
    car.steps = 1;
    car.has_thought = true;
    (car, net, collision, surfaces)
}

fn blocker(car: &AiCar) -> Body {
    Body { position: car.body().position + Vec3::new(0.5, 0.0, 14.0), velocity: Vec3::ZERO, ..car.body() }
}

#[test]
fn traffic_parity_refreshes_steering_between_decisions_without_advancing_navigation() {
    let (mut car, net, collision, _surfaces) = fixture(false, 10.0);
    let index = SegmentIndex::build(&net);
    let mut rng = SplitMix(42);
    let bodies = [(car.body(), false), (blocker(&car), false)];
    let mut ctx =
        Ctx { net: &net, index: &index, rng: &mut rng, bodies: &bodies, me: 0, player: None, aims: &[], lights: None };
    let before = car.target;
    let mut state = Vec::new();
    car.nav.visit_state(&mut |word| state.push(word));
    let clock = car.think_delta;
    let mut props = PropWorld::default();
    let _ = car.begin_step(&mut ctx, &mut props, &collision).expect("physical step");
    assert!(car.blocked, "a new obstacle must be seen before the next decision");
    assert_ne!(car.target, before, "the physical controller must not steer at the stale target");
    assert_eq!(car.speed, 10.0, "the previous decision's target speed is retained");
    assert_eq!(car.think_delta, clock, "no decision was run");
    assert!((car.think_elapsed - STEP).abs() < 1e-6);
    let mut after = Vec::new();
    car.nav.visit_state(&mut |word| after.push(word));
    assert_eq!(state, after, "steering cannot advance the cursor or select a junction exit");
    assert_eq!(ctx.rng.0, 42, "steering cannot draw a navigation random number");
    let clear = Ctx { bodies: &bodies[..1], ..ctx };
    car.refresh_traffic_target(&clear);
    assert!(!car.blocked, "the current blockage must clear when the obstacle leaves");
    assert_eq!(car.target, car.nav.position);
}

#[test]
fn traffic_parity_keeps_patrol_control_and_the_native_stopped_guard() {
    for (patrol, speed) in [(true, 10.0), (false, 0.0)] {
        let (mut car, net, _, _) = fixture(patrol, speed);
        let index = SegmentIndex::build(&net);
        let bodies = [(car.body(), false), (blocker(&car), false)];
        let mut rng = SplitMix(3);
        let ctx = Ctx {
            net: &net,
            index: &index,
            rng: &mut rng,
            bodies: &bodies,
            me: 0,
            player: None,
            aims: &[],
            lights: None,
        };
        car.target = Vec3::new(1.0, 2.0, 3.0);
        car.refresh_traffic_target(&ctx);
        assert_eq!(car.target, Vec3::new(1.0, 2.0, 3.0), "patrol={patrol}, speed={speed}");
        assert!(!car.blocked);
    }
}

#[test]
fn traffic_parity_retains_the_decision_radius_between_physical_steps() {
    let (mut car, net, _, _) = fixture(false, 10.0);
    let mut motion = car.sim.motion();
    motion.linear = Vec3::ZERO;
    car.sim.set_motion(&motion);
    car.avoidable_radius = 12.5;
    car.blocked = true;
    let index = SegmentIndex::build(&net);
    // Both stationary cars must also pass the trail's independent close-obstacle filter.
    let obstacle = blocker(&car);
    let bodies = [(car.body(), false), (obstacle, false)];
    let mut rng = SplitMix(42);
    let ctx =
        Ctx { net: &net, index: &index, rng: &mut rng, bodies: &bodies, me: 0, player: None, aims: &[], lights: None };
    car.refresh_traffic_target(&ctx);
    assert!(!car.blocked, "a new blockage cannot enlarge the last decision's search radius");
    assert_eq!(car.avoidable_radius, 12.5);
    assert_eq!(car.target, car.nav.position);

    car.avoidable_radius = 20.0;
    car.refresh_traffic_target(&ctx);
    assert!(car.blocked, "a retained larger radius must survive the speed falling to zero");
    assert_eq!(car.avoidable_radius, 20.0);

    car.avoidable_radius = 12.5;
    car.blocked = false;
    motion.linear = space::to_render([12.0, 0.0, 0.0]);
    car.sim.set_motion(&motion);
    assert!(car.sim.driver_view().forward_speed.abs() < 1e-5);
    car.refresh_traffic_target(&ctx);
    assert!(car.blocked, "twice the full current speed can extend the retained radius during a sideways slide");
    assert_eq!(car.avoidable_radius, 12.5, "the speed extension does not replace the decision's radius");
}
