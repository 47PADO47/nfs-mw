use super::*;
use crate::scenes::world::{
    drive::WorldGround,
    props::{PropWorld, tests::placement},
};
use blackbox_collision::{Article, Barrier, CollisionPack, Instance};
use blackbox_vehicle::rigid_body::BodyState;
use glam::Quat;

fn wall(flags: u8) -> CollisionWorld {
    let mut world = CollisionWorld::default();
    world.insert(CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 10.0,
            half_height: 10.0,
            half_length: 10.0,
            radius: 20.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![],
            barriers: vec![Barrier { p0: [-5.0, 0.0, 0.0], p1: [5.0, 10.0, 0.0], surface: 0, flags, inv_xz_len: 0.1 }],
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    });
    world
}

fn body(z: f32, previous_z: f32, speed: f32) -> (PropBody, PropDynamics) {
    let placed = placement(1, Vec3::new(0.0, 5.25, z), 500.0, false, 0.0);
    let spec = placed.prop.as_ref().unwrap().dynamics.as_ref().unwrap().as_ref().clone();
    let mut world = PropWorld::default();
    world.add_tile(0, &mut [placed]);
    let mut body = world.colliders.remove(&2).unwrap().body.unwrap();
    body.geometry.primitives[0].flags |= bounds_flags::PRIM_VS_WORLD;
    body.rigid.state = BodyState::Awake;
    body.moved = true;
    body.previous = (Vec3::new(0.0, 5.25, previous_z), Quat::IDENTITY);
    body.rigid.linear_velocity = Vec3::Z * speed;
    (body, spec)
}

#[test]
fn loose_parked_body_hits_full_height_barrier_and_separates_before_reaction() {
    let (mut body, mut spec) = body(-0.2, -0.7, 20.0);
    spec.sound_profile = 42;
    let mut sounds = Vec::new();
    step(&mut body, &spec, &wall(0), &SurfaceTable::default(), &mut vec![], &mut sounds, &mut vec![]);
    assert!((body.rigid.position.z + 0.5).abs() < 1e-5);
    assert!(body.rigid.linear_velocity.z < 1.0, "barrier must remove incoming motion");
    assert_eq!(body.rigid.mass(), 500.0);
    assert!(body.rigid.angular_velocity.is_finite());
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].source, ContactSource::World);
    assert_eq!(sounds[0].sound_profile, 42);
    assert!(sounds[0].velocity.z > 19.0 && sounds[0].delta_v > 0.0);
}

#[test]
fn separating_body_is_still_corrected_without_extra_impulse() {
    let (mut body, spec) = body(-0.2, -0.2, -8.0);
    let velocity = body.rigid.linear_velocity;
    step(&mut body, &spec, &wall(0), &SurfaceTable::default(), &mut vec![], &mut vec![], &mut vec![]);
    assert!((body.rigid.position.z + 0.5).abs() < 1e-5);
    assert_eq!(body.rigid.linear_velocity, velocity);
}

#[test]
fn source_world_and_primitive_gates_remain_independent() {
    for disabled in [true, false] {
        let (mut body, mut spec) = body(-0.2, -0.7, 20.0);
        if disabled {
            spec.no_world_collisions = true;
        } else {
            body.geometry.primitives[0].flags &= !bounds_flags::PRIM_VS_WORLD;
        }
        let position = body.rigid.position;
        step(&mut body, &spec, &wall(0), &SurfaceTable::default(), &mut vec![], &mut vec![], &mut vec![]);
        assert_eq!(body.rigid.position, position);
        assert_eq!(body.rigid.linear_velocity.z, 20.0);
    }
}

#[test]
fn one_sided_primitive_rejects_rear_even_on_two_sided_wall() {
    for (barrier_flags, shape, responds) in
        [(0, Shape::Box, false), (BARRIER_TWO_SIDED, Shape::Box, true), (BARRIER_TWO_SIDED, Shape::Sphere, false)]
    {
        let (mut body, spec) = body(0.2, 0.7, -20.0);
        body.geometry.primitives[0].shape = shape;
        step(&mut body, &spec, &wall(barrier_flags), &SurfaceTable::default(), &mut vec![], &mut vec![], &mut vec![]);
        assert_eq!(body.rigid.position.z > 0.49, responds);
    }
}

#[test]
fn world_finish_applies_barrier_before_sleep_and_does_not_integrate_twice() {
    let (body, spec) = body(-0.2, -0.7, 20.0);
    let mut placed = placement(1, Vec3::new(0.0, 5.25, -0.2), 500.0, false, 0.0);
    std::sync::Arc::make_mut(placed.prop.as_mut().unwrap()).dynamics = Some(std::sync::Arc::new(spec));
    let mut props = PropWorld::default();
    props.add_tile(0, &mut [placed]);
    props.colliders.get_mut(&2).unwrap().body = Some(body);
    let collision = wall(0);
    let surfaces = SurfaceTable::default();
    props.finish_native_world(&WorldGround { collision: &collision, surfaces: &surfaces }, 1.0 / 60.0);
    let body = props.colliders[&2].body.as_ref().unwrap();
    assert!((body.rigid.position.z + 0.5).abs() < 1e-5);
    assert_eq!(body.rigid.position.y, 5.25, "end frame only responds; no second integration");
    assert_eq!(body.previous.0.z, -0.7);
}

#[test]
fn world_visual_callbacks_do_not_depend_on_audio_profile_or_audio_drain() {
    let (mut body, mut spec) = body(-0.2, -0.7, 20.0);
    spec.sound_profile = 0;
    let mut sounds = Vec::new();
    let mut visuals = Vec::new();
    step(&mut body, &spec, &wall(0), &SurfaceTable::default(), &mut vec![], &mut sounds, &mut visuals);
    assert!(sounds.is_empty());
    assert_eq!(visuals.len(), 1);
    assert_eq!(visuals[0].kind, crate::scenes::world::impact_effects::Kind::World);
    assert!(visuals[0].closing.length() > 19.0 && visuals[0].delta_v > 0.0);
    assert_eq!(visuals[0].owner_velocity, body.rigid.linear_velocity);
}
