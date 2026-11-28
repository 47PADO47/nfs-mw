//! Private regression against the mounted concrete-pipe cargo fixture.
use std::collections::BTreeSet;

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::sun;
use crate::settings::CarShading;
use blackbox_gfx::{Capabilities, GraphicsApi, Instance};

fn pipe_sim(data: &PhysicsData, rig: &CarRig) -> CarSim {
    let physics = data.car("TRAILERCMT").unwrap();
    assert_eq!(physics.events.as_ref().unwrap().sequence.as_ref().unwrap().package.engines[0].name, "TRAILERCMT");
    let mut sim = CarSim::new(physics, rig.rest_heights());
    sim.configure_trailer();
    sim.body_mut().place(Vec3::Y * 4.0, Quat::IDENTITY);
    sim
}

fn gentle_bump(trailer: &mut CarSim) {
    let air = FlatGround::new(-100.);
    trailer.step(&DriveInput::default(), &air, None);
    let mut car = striker();
    let body = trailer.body();
    let rear = trailer
        .geometry
        .as_ref()
        .unwrap()
        .body
        .primitives
        .iter()
        .filter(|p| p.collides(blackbox_collision::bounds_flags::PRIM_VS_OBJECTS))
        .map(|p| crate::scenes::world::object_geometry::primitive_at(p, (body.position, body.orientation)))
        .min_by(|a, b| (a.centre.z - a.half.z).total_cmp(&(b.centre.z - b.half.z)))
        .unwrap();
    let gap = rear.half.z + car.half_dimensions().z + 0.008;
    car.body_mut().place(rear.centre - Vec3::Z * gap, Quat::IDENTITY);
    car.body_mut().linear_velocity = Vec3::Z * 2.0;
    car.step(&DriveInput::default(), &air, None);
    assert!(car.collide_with(trailer).is_some(), "weak hit must actually contact the rear");
}

#[test]
#[ignore = "requires the concrete-release mod fixture in NFSMW_GAME_DIR"]
fn concrete_rear_impact_releases_four_rolling_pipes_and_four_straps() {
    let (dir, data) = installed().expect("set NFSMW_GAME_DIR to the pipe fixture");
    let model =
        nfsmw_data::car::load(&dir, "TRAILERCMT", &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan))
        .with_mesh_tracking()
        .with_glossy_tracking();
    let rig = CarRig::upload(&mut renderer, 123, model, CarShading::default(), sun::to_sun());
    let mut sim = pipe_sim(&data, &rig);
    let mut props = PropWorld::default();
    gentle_bump(&mut sim);
    for _ in 0..240 {
        update(&mut sim, &rig, &mut props, &data, Vec3::ZERO);
    }
    assert_eq!(props.fragment_count(), 0, "a parking-speed bump must keep the cargo strapped");
    let mut sim = pipe_sim(&data, &rig);
    sim.body_mut().linear_velocity.z = 7.;
    rear_hit(&mut sim);
    let parent = sim.pose();
    for _ in 0..240 {
        update(&mut sim, &rig, &mut props, &data, parent.position);
    }
    let events = sim.events.as_ref().unwrap();
    let expected: BTreeSet<_> = (1..=8).map(|i| vlt_hash(&format!("ATTACHMENT{i}"))).collect();
    assert_eq!(events.spawned, expected);
    assert_eq!(events.hidden_slots.len(), 8);
    assert_eq!(props.fragment_count(), 8);
    let mut actual = Vec::new();
    props.fragment_instances(&mut actual);
    let mut expected_visuals: Vec<_> = events
        .hidden_slots
        .iter()
        .flat_map(|s| rig.fragment_instances(*s, None))
        .map(|i| Instance::new(i.mesh, parent.transform() * i.transform))
        .collect();
    assert_eq!(actual.len(), expected_visuals.len());
    for i in actual {
        let at = expected_visuals
            .iter()
            .position(|j| j.mesh == i.mesh && i.transform.abs_diff_eq(j.transform, 0.0001))
            .expect("released mesh starts at its attached pose");
        expected_visuals.swap_remove(at);
    }
    let ground = FlatGround::new(0.);
    let mut rolling = BTreeSet::new();
    let mut grounded = BTreeSet::new();
    let pipe = data.fragment_dynamics(vlt_hash("heavyconcreteobject")).unwrap();
    let webbing = data.fragment_dynamics(vlt_hash("traffic_cable")).unwrap();
    assert!(!pipe.no_car_effect && !pipe.no_object_collisions && !pipe.immobile);
    let masses: BTreeSet<_> = props.fragment_bodies().iter().map(|(_, b, _)| b.mass().round() as i32).collect();
    assert_eq!(masses, [pipe.mass.round() as i32, webbing.mass.round() as i32].into());
    for _ in 0..240 {
        props.step(&ground, 1. / 60.);
        for (id, body, _) in props.fragment_bodies() {
            assert!(body.position.is_finite() && body.angular_velocity.is_finite());
            if body.mass() == pipe.mass && body.position.y < 1.52 {
                grounded.insert(id);
            }
            if body.mass() == pipe.mass && body.angular_velocity.x.abs() > 0.1 {
                rolling.insert(id);
            }
        }
    }
    assert_eq!(grounded.len(), 4, "all four concrete pipes should fall to the road");
    assert_eq!(rolling.len(), 4, "all four should rotate about their cylinder axes");
    sim.body_mut().place(Vec3::Y * 4., Quat::IDENTITY);
    rear_hit(&mut sim);
    for _ in 0..240 {
        update(&mut sim, &rig, &mut props, &data, parent.position);
    }
    assert_eq!(props.fragment_count(), 8, "repeated contact must not duplicate the load");
    eprintln!(
        "Parking-speed hit: 0 releases. Hard rear hit: 4 concrete pipes and 4 webbing assemblies left their native slots. All four pipes touched ground and rolled. Second hit: no additional cargo."
    );
    rig.release(&mut renderer);
}
