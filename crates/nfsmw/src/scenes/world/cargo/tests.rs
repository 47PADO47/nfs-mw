//! Installed meshes and collision data are read locally. CPU tests need no fabricated GPU handles.

use super::*;
use crate::scenes::world::drive::{DriveInput, FixedClock};
use blackbox_vehicle::{FlatGround, VehicleSpec};
use game_install::GameDir;
use nfsmw_data::car::{
    CarModel, LoadOptions,
    physics::{CarBounds, CarPhysics, WallSpec},
};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

mod visuals;

fn installed() -> Option<(GameDir, PhysicsData)> {
    let _ = env_logger::try_init();
    let path = std::env::var_os("NFSMW_GAME_DIR")?;
    let dir = GameDir::open(path).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(WorldIndex::open(&dir, DEFAULT_TRACK).unwrap().sequences);
    Some((dir, data))
}

fn striker() -> CarSim {
    let spec = VehicleSpec::example();
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO };
    CarSim::new(
        CarPhysics { spec, bounds, walls: WallSpec::default(), geometry: None, events: None, reactions: None },
        [0.2; 4],
    )
}

fn rear_hit(trailer: &mut CarSim) {
    let ground = FlatGround::new(-100.0);
    // Run real fixed ticks so previous contact poses describe motion, not fixture teleports.
    trailer.step(&DriveInput::default(), &ground, None);
    let mut car = striker();
    let geometry = trailer.geometry.as_ref().unwrap();
    let rear = geometry
        .body
        .primitives
        .iter()
        .filter(|p| p.collides(blackbox_collision::bounds_flags::PRIM_VS_OBJECTS))
        .map(|p| {
            crate::scenes::world::object_geometry::primitive_at(
                p,
                (trailer.body().position, trailer.body().orientation),
            )
        })
        .min_by(|a, b| (a.centre.z - a.half.z).total_cmp(&(b.centre.z - b.half.z)))
        .unwrap();
    let distance = rear.half.z + car.half_dimensions().z + 0.5;
    car.body_mut().place(rear.centre - Vec3::Z * distance, Quat::IDENTITY);
    car.body_mut().linear_velocity = Vec3::Z * 50.0;
    car.step(&DriveInput::default(), &ground, None);
    assert!(car.collide_with(trailer).is_some(), "fixture must physically strike the installed rear bounds");
}

fn parts(model: &CarModel, slot: usize) -> Option<Vec<blackbox_render::Instance>> {
    model.placements.iter().any(|p| p.slot == slot && model.solids.contains_key(&p.solid)).then_some(Vec::new())
}

#[derive(Debug, PartialEq)]
struct Trace {
    counts: Vec<usize>,
    states: Vec<u32>,
    hidden: Vec<usize>,
}

fn run(data: &PhysicsData, model: &CarModel, name: &str, frames: impl Iterator<Item = f32>) -> Trace {
    let mut sim = CarSim::new(data.car(name).unwrap(), [0.4; 4]);
    sim.configure_trailer();
    sim.body_mut().place(Vec3::Y * 4.0, Quat::IDENTITY);
    rear_hit(&mut sim);
    let mut props = PropWorld::default();
    let ground = FlatGround::new(0.0);
    let mut clock = FixedClock::default();
    let mut trace = Trace { counts: Vec::new(), states: Vec::new(), hidden: Vec::new() };
    for dt in frames {
        for _ in 0..clock.advance(dt) {
            props.step(&ground, 1.0 / 60.0);
            sim.step(&DriveInput::default(), &ground, None);
            update_with(&mut sim, &mut props, data, Vec3::ZERO, |s| parts(model, s));
            trace.counts.push(props.fragment_count());
            for (_, body, pose) in props.fragment_bodies() {
                assert!(body.position.is_finite() && pose.is_finite());
                trace.states.extend(
                    body.position
                        .to_array()
                        .into_iter()
                        .chain(body.orientation.to_array())
                        .chain(body.linear_velocity.to_array())
                        .chain(body.angular_velocity.to_array())
                        .map(f32::to_bits),
                );
            }
        }
    }
    trace.hidden.extend(&sim.events.as_ref().unwrap().hidden_slots);
    let before = props.fragment_count();
    // A later real collision cannot create a second load after the Cargo_Empty transition.
    sim.body_mut().place(Vec3::Y * 4.0, Quat::IDENTITY);
    rear_hit(&mut sim);
    for _ in 0..240 {
        update_with(&mut sim, &mut props, data, Vec3::ZERO, |s| parts(model, s));
    }
    assert_eq!(props.fragment_count(), before);
    // Parent destruction removes child bodies from drawing immediately and from collision at the next tick.
    drop(sim);
    assert_eq!(props.fragment_count(), 0);
    props.step(&ground, 1.0 / 60.0);
    assert_eq!(props.len(), 0);
    trace
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn rear_end_collision_releases_installed_cargo_at_identical_ticks_and_states_at_every_frame_rate() {
    let Some((dir, data)) = installed() else { return };
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    for (name, expected) in [("TRAILERLOG", 7), ("TRAILERB", 3)] {
        let model = nfsmw_data::car::load(&dir, name, &options).unwrap();
        let baseline = run(&data, &model, name, std::iter::repeat_n(1.0 / 60.0, 240));
        assert_eq!(baseline.counts.last(), Some(&expected), "{name} counts {:?}", baseline.counts);
        assert_eq!(baseline.hidden.len(), expected, "{name}");
        for fps in [5, 10, 30, 120, 144, 240] {
            assert_eq!(
                run(&data, &model, name, std::iter::repeat_n(1.0 / fps as f32, fps * 4)),
                baseline,
                "{name} at {fps} FPS"
            );
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn missing_meshes_do_not_hide_cargo_and_original_side_scaling_does_not_drop_it() {
    let Some((_, data)) = installed() else { return };
    let mut sim = CarSim::new(data.car("trailerlog").unwrap(), [0.4; 4]);
    sim.body_mut().place(Vec3::Y * 4.0, Quat::IDENTITY);
    let mut props = PropWorld::default();
    let mut hit = super::super::events::DamageHit::capture(
        sim.body(),
        sim.body().position + Vec3::X * 2.0,
        Vec3::NEG_X,
        30.0,
        super::super::events::OtherBody::default(),
    );
    hit.force = 1_000_000.0;
    sim.events.as_mut().unwrap().hit(hit);
    update_with(&mut sim, &mut props, &data, Vec3::ZERO, |_| Some(Vec::new()));
    assert_eq!(props.fragment_count(), 0);
    rear_hit(&mut sim);
    for _ in 0..240 {
        update_with(&mut sim, &mut props, &data, Vec3::ZERO, |_| None);
    }
    assert_eq!(props.fragment_count(), 0);
    assert!(sim.events.as_ref().unwrap().hidden_slots.is_empty());
}
