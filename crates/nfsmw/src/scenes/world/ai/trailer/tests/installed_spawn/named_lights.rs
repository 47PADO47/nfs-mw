//! Installed top contact flows through cargo, lamp admission and the world siren adapter.
use super::*;
use crate::scenes::world::ai::{
    TrafficWorld,
    car::{AiCar, Ctx, Role, Start},
    traffic::Cruise,
};
use crate::scenes::world::car_light_flares::FlareView;
use crate::scenes::world::events::{DamageHit, OtherBody};
use blackbox_pursuit::siren::State;
use blackbox_roads::{RoadNav, SegmentIndex, SplitMix};

#[test]
#[ignore = "needs installed police geometry, event sequences, lamp profiles and highway ground"]
fn installed_named_lightbar_collision_reaches_cargo_flares_and_world_siren_without_wrecking() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let network = index.road_network.as_ref().unwrap();
    let (mut spawn, _, _) = highway(network);
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let model = model(&dir, &data, "copmidsize", &mut renderer);
    let spec = model.physics.events.as_ref().unwrap();
    assert_eq!(spec.sequence.as_ref().unwrap().package.engines[spec.sequence.as_ref().unwrap().engine].name, "CopCar");
    let segments = SegmentIndex::build(network);
    let mut nav = RoadNav::traffic();
    let heading = Vec3::new(spawn.heading.cos(), spawn.heading.sin(), 0.0);
    assert!(nav.init_at_point(
        network,
        &segments,
        Vec3::from(space::to_physics(spawn.position)),
        Vec3::from(space::to_physics(heading)),
        false
    ));
    let mut car = AiCar::place(
        &model,
        Role::Traffic,
        nav,
        spawn,
        Start { speed: 0.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &ground,
    )
    .unwrap();
    car.set_roadblock();
    let mut world = TrafficWorld::new(network.clone());
    world.cars.push(car);
    assert_eq!(world.siren_frame(0.1).unwrap().sources[0].state, State::Wail);
    let vertices = |effects: &[blackbox_gfx::TexturedEffect]| effects.iter().map(|e| e.vertices.len()).sum::<usize>();
    let mut props = PropWorld::default();
    let mut rng = SplitMix(0x6c61_6d70);
    let mut witness = None;
    for _ in 0..120 {
        let pose = world.cars[0].pose(1.0);
        let forward = pose.rotation * Vec3::X;
        let view = FlareView {
            camera: pose.position + forward * 20.0 + Vec3::Z,
            forward: -forward,
            fov_degrees: 70.0,
            height: 1080,
        };
        let mut ordinary = Vec::new();
        let state = world.cars[0].lamp_state();
        model.rig.flares().draw(
            model.rig.lights(),
            pose.transform(),
            crate::scenes::world::car_lights::LampState { broken: 0x7000, ..state },
            view,
            &mut ordinary,
        );
        let mut enabled = Vec::new();
        world.cars[0].flares(1.0, view, &mut enabled);
        if vertices(&enabled) > vertices(&ordinary) {
            witness = Some((pose, view, ordinary));
            break;
        }
        // The actor's normal fixed-step clock advances its per-car phase; draw cadence cannot.
        let bodies = [(world.cars[0].avoidable(), false)];
        let mut ctx = Ctx {
            net: network,
            index: &segments,
            rng: &mut rng,
            bodies: &bodies,
            me: 0,
            player: None,
            aims: &[],
            lights: None,
        };
        world.cars[0].step(&mut ctx, &collision, &mut props, &data.surfaces);
    }
    let (pose, view, ordinary) =
        witness.expect("the real actor must expose emergency output within two simulation seconds");
    assert_eq!(world.cars[0].lamp_state().broken, 0);
    assert!(spec.strength > 0.0 && spec.visual_scale[8] > 0.0);
    world.cars[0].sims_mut().0.events.as_mut().unwrap().hit(DamageHit {
        arm: Vec3::Y * 2.0,
        normal: Vec3::NEG_Y,
        half: Vec3::new(1.0, 1.0, 4.0),
        closing_speed: 10.0,
        force: 1.1 * spec.strength / spec.visual_scale[8],
        shock_force: 0.0,
        before_velocity: Vec3::ZERO,
        other: OtherBody::default(),
    });
    world.cars[0].update_events(&mut props, &data, pose.position);
    assert!(!world.cars[0].is_wrecked(), "authored light-bar damage does not destroy the police car");
    assert_eq!(world.cars[0].lamp_state().broken, 0x7000);
    let mut damaged = Vec::new();
    world.cars[0].flares(1.0, view, &mut damaged);
    assert_eq!(damaged.len(), ordinary.len());
    for (actual, expected) in damaged.iter().zip(&ordinary) {
        assert_eq!(actual.texture, expected.texture);
        assert_eq!(actual.vertices, expected.vertices, "only authored emergency lamps are removed");
    }
    assert_eq!(world.siren_frame(0.1).unwrap().sources[0].state, State::Die);
    world.cars[0].sims_mut().0.events.as_mut().unwrap().repair_lights();
    assert_eq!(world.cars[0].lamp_state().broken, 0);
    assert_eq!(world.siren_frame(0.1).unwrap().sources[0].state, State::Wail);
}
