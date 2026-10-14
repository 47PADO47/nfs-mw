use blackbox_collision::{Bounds, BoundsSet, bounds_flags as flags};
use blackbox_scenery::ObjectSpawner;
use blackbox_vehicle::{FlatGround, VehicleSpec, rigid_body::RigidBodySpec};
use glam::Quat;
use nfsmw_data::world::{LocalBox, PropDynamics};

use super::*;

mod compound;
mod fragments;
mod installed;
mod lifetime;
mod placement;
mod support;

pub(super) fn placement(id: u32, at: Vec3, mass: f32, no_car_effect: bool, detach: f32) -> Placed {
    let node = Bounds {
        orientation: [0.0, 0.0, 0.0, 1.0],
        position: [0.0; 3],
        flags: flags::BOX | flags::PRIM_VS_OBJECTS | flags::PRIM_VS_GROUND,
        half_dimensions: [0.5; 3],
        child_count: 0,
        point_cloud: None,
        pivot: [0.0; 3],
        first_child: 0,
        radius: 0.5,
        surface: 0,
        name_hash: 0,
    };
    let dynamics = PropDynamics {
        sound_profile: 0,
        mass,
        tensor_scale: Vec3::splat(3.0),
        body: RigidBodySpec::default(),
        fallback_shape: blackbox_collision::Shape::Box,
        base_surface: 0,
        detach_impulse: detach,
        no_car_effect,
        immobile: false,
        respawn_time: 120.0,
        kill_off_screen: 8.0,
        dropout: [0.0; 2],
        allow_off_world: false,
        event_sequencer: 0,
        start_sequencer: false,
        no_trigger: false,
        explosion_effect: 0.0,
    };
    let shape = Arc::new(PropShape {
        name: format!("test-{id}"),
        kind: PropKind::Movable { mass },
        boxes: vec![LocalBox { centre: Vec3::ZERO, rotation: Quat::IDENTITY, half: Vec3::splat(0.5) }],
        bounds: Some(Arc::new(BoundsSet { name_hash: id, nodes: vec![node], point_clouds: vec![] })),
        dynamics: Some(Arc::new(dynamics)),
    });
    let render = space::to_render(at.to_array());
    let mut p = Placed::prop_only(Mat4::from_translation(render), shape);
    p.spawner = Some(ObjectSpawner {
        orientation: [0.0, 0.0, 0.0, 1.0],
        position: at.to_array(),
        model: 0,
        collision: 0,
        attributes: 0,
        override_index: 0,
        unique_id: id,
        exclude_flags: 0,
    });
    p
}

fn car(speed: f32) -> Vehicle {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.body_mut().place(Vec3::new(0.0, 0.6, -2.5), Quat::IDENTITY);
    v.body_mut().linear_velocity = Vec3::Z * speed;
    v
}

fn hit(no_effect: bool, detach: f32, speed: f32) -> (PropWorld, Vehicle, Impact, u32) {
    let mut world = PropWorld::default();
    let mut p = [placement(3, Vec3::new(0.0, 0.6, 0.0), 100.0, no_effect, detach)];
    world.add_tile(1, &mut p);
    let mut v = car(speed);
    let mut impact = Impact::default();
    world.collide_vehicle(&mut v, None, &mut impact);
    (world, v, impact, p[0].prop_id)
}

#[test]
fn movable_prop_transfers_momentum_and_remains_drawn() {
    let (mut world, v, impact, id) = hit(false, 0.0, 20.0);
    assert!(impact.impulse > 0.0);
    assert!(v.linear_velocity().z < 20.0);
    let before = world.colliders[&id].body.as_ref().unwrap().rigid.position;
    assert!(world.colliders[&id].body.as_ref().unwrap().rigid.linear_velocity.z > 0.0);
    for _ in 0..60 {
        world.step(&FlatGround::new(0.0), 1.0 / 60.0);
    }
    world.set_alpha(1.0);
    let transform = world.pose(id).unwrap().0;
    assert!(transform.is_finite());
    assert!((transform.w_axis.x - before.z).abs() > 0.1);
    assert_eq!(world.knocked_count(), 1);
    assert_eq!(world.len(), 1);
}

#[test]
fn no_car_effect_leaves_car_motion_unchanged_and_launches_prop() {
    let (world, v, impact, id) = hit(true, 0.0, 20.0);
    assert_eq!(v.linear_velocity(), Vec3::Z * 20.0);
    assert_eq!(v.position(), car(20.0).position());
    assert_eq!(impact.impulse, 0.0);
    assert!(!impact.contacts.is_empty());
    assert_eq!(impact.contacts[0].kind, ContactKind::PropMovable);
    assert!(world.colliders[&id].body.as_ref().unwrap().rigid.linear_velocity.length() > 1.0);
}

#[test]
fn attachment_threshold_is_respected_before_no_car_effect() {
    let (world, _, _, id) = hit(true, 500.0, 0.01);
    assert!(world.colliders[&id].body.as_ref().unwrap().attached);
    let (world, _, impact, id) = hit(true, 500.0, 20.0);
    assert!(!world.colliders[&id].body.as_ref().unwrap().attached);
    assert!(impact.impulse > 0.0, "breaking an attached object can affect the car");
    let (world, _, impact, id) = hit(true, -1.0, 20.0);
    assert!(world.colliders[&id].body.as_ref().unwrap().attached, "negative force means permanent attachment");
    assert_eq!(
        impact.contacts[0].kind,
        ContactKind::PropRigid,
        "a pinned object is a rigid contact regardless of mass"
    );
}

#[test]
fn moving_collision_grid_and_unload_follow_the_prop() {
    let (mut world, _, _, id) = hit(true, 0.0, 20.0);
    let prop = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    prop.rigid.position.z = 50.0;
    world.regrid(id);
    let around = |x: f32| Aabb::new([x - 2.0, -2.0, -2.0], [x + 2.0, 2.0, 2.0]);
    assert!(world.candidates(&around(50.0)).contains(&id));
    assert!(world.candidates(&around(0.0)).contains(&id), "the movement path remains in broad phase for this tick");
    world.step(&FlatGround::new(0.0), 1.0 / 60.0);
    assert!(!world.candidates(&around(0.0)).contains(&id));
    world.remove_tile(1);
    assert!(world.colliders.is_empty() && world.grid.is_empty());
}

fn swept_trace(fps: usize, shape_flag: u16) -> Vec<u32> {
    let mut world = PropWorld::default();
    let mut p = placement(40, Vec3::new(0.0, 5.0, 14.0), 100.0, false, 0.0);
    let shape = Arc::make_mut(p.prop.as_mut().unwrap());
    let bounds = Arc::make_mut(shape.bounds.as_mut().unwrap());
    bounds.nodes[0].flags = shape_flag | flags::PRIM_VS_OBJECTS;
    let mut placed = [p, placement(41, Vec3::new(0.0, 5.0, 15.0), 100.0, false, 0.0)];
    world.add_tile(1, &mut placed);
    let id = placed[0].prop_id;
    let prop = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    prop.rigid.wake();
    prop.rigid.linear_velocity = Vec3::Z * 300.0;
    let mut trace = Vec::new();
    let mut clock = crate::scenes::world::drive::FixedClock::default();
    for _ in 0..fps {
        for _ in 0..clock.advance(1.0 / fps as f32) {
            world.step(&FlatGround::new(0.0), 1.0 / 60.0);
            for p in &placed {
                let body = &world.colliders[&p.prop_id].body.as_ref().unwrap().rigid;
                trace.extend(
                    body.position.to_array().into_iter().chain(body.linear_velocity.to_array()).map(f32::to_bits),
                );
            }
        }
    }
    assert!(
        world.colliders[&placed[1].prop_id].body.as_ref().unwrap().moved,
        "the object must hit the box it crossed in another grid cell"
    );
    trace
}

#[test]
fn sphere_sweeps_cross_grid_cells_and_transfer_momentum_at_every_render_rate() {
    for shape in [flags::SPHERE, flags::BOX] {
        let expected = swept_trace(60, shape);
        assert_eq!(expected.len(), 60 * 2 * 6);
        for fps in [5, 10, 30, 120, 144, 240] {
            assert_eq!(swept_trace(fps, shape), expected, "shape {shape} at {fps} FPS");
        }
    }
}

#[test]
fn loose_prop_can_hit_another_prop_and_be_hit_again() {
    let (mut world, _, _, id) = hit(true, 0.0, 20.0);
    let p = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    p.rigid.place(Vec3::new(0.0, 0.6, 0.0), Quat::IDENTITY);
    p.rigid.linear_velocity = Vec3::Z * 10.0;
    let mut second = [placement(10, Vec3::new(0.0, 0.6, 0.9), 100.0, true, 0.0)];
    world.add_tile(2, &mut second);
    world.regrid(id);
    world.step(&FlatGround::new(0.0), 1.0 / 60.0);
    let other = world.colliders[&second[0].prop_id].body.as_ref().unwrap();
    assert!(other.rigid.linear_velocity.length() > 0.0);
    let mut v = car(20.0);
    v.body_mut().position = world.colliders[&id].body.as_ref().unwrap().rigid.position - Vec3::Z * 2.5;
    let mut impact = Impact::default();
    world.collide_vehicle(&mut v, None, &mut impact);
    assert!(!impact.contacts.is_empty(), "fallen prop remains collidable");
}

#[test]
fn visual_interpolation_never_changes_simulation() {
    let (mut world, _, _, id) = hit(true, 0.0, 20.0);
    world.step(&FlatGround::new(0.0), 1.0 / 60.0);
    let body = world.colliders[&id].body.as_ref().unwrap();
    let before = body.rigid.position;
    world.set_alpha(0.0);
    let a = world.pose(id).unwrap();
    world.set_alpha(1.0);
    let b = world.pose(id).unwrap();
    world.set_alpha(0.5);
    let m = world.pose(id).unwrap();
    assert!((m.0.w_axis.truncate() - a.0.w_axis.truncate().lerp(b.0.w_axis.truncate(), 0.5)).length() < 1e-4);
    assert_eq!(world.colliders[&id].body.as_ref().unwrap().rigid.position, before);
}

fn trace(fps: usize) -> Vec<u32> {
    let (mut world, _, _, id) = hit(true, 0.0, 20.0);
    let mut clock = crate::scenes::world::drive::FixedClock::default();
    let mut out = Vec::new();
    for _ in 0..fps * 4 {
        for _ in 0..clock.advance(1.0 / fps as f32) {
            world.step(&FlatGround::new(0.0), 1.0 / 60.0);
            let b = &world.colliders[&id].body.as_ref().unwrap().rigid;
            out.extend(
                b.position
                    .to_array()
                    .into_iter()
                    .chain(b.orientation.to_array())
                    .chain(b.linear_velocity.to_array())
                    .chain(b.angular_velocity.to_array())
                    .map(f32::to_bits),
            );
        }
        world.set_alpha(clock.alpha());
        assert!(world.pose(id).unwrap().0.is_finite());
    }
    out
}

#[test]
fn prop_motion_and_ground_contacts_are_identical_at_every_render_rate() {
    let expected = trace(60);
    assert_eq!(expected.len(), 240 * 13);
    for fps in [5, 10, 30, 120, 144, 240] {
        assert_eq!(trace(fps), expected, "{fps} FPS");
    }
}

#[test]
fn flagless_cone_bounds_make_a_body_at_the_root_pivot_without_a_visual_jump() {
    let at = Vec3::new(5.0, 0.0, 7.0);
    let mut p = placement(12, at, 100.0, true, 0.0);
    let shape = Arc::make_mut(p.prop.as_mut().unwrap());
    let bounds = Arc::make_mut(shape.bounds.as_mut().unwrap());
    let root = &mut bounds.nodes[0];
    root.flags = flags::BOX;
    root.pivot = [0.0, 0.5, 0.0];
    let mut world = PropWorld::default();
    world.add_tile(1, std::slice::from_mut(&mut p));
    let body = world.colliders[&p.prop_id].body.as_ref().unwrap();
    assert_eq!(body.rigid.position, at + Vec3::Y * 0.5);
    assert!(body.geometry.used_fallback);
    assert_eq!(world.pose(p.prop_id).unwrap().0, p.transform);
    let mut v = car(20.0);
    v.body_mut().position += at;
    let mut impact = Impact::default();
    world.collide_vehicle(&mut v, None, &mut impact);
    assert!(!impact.contacts.is_empty(), "a root without collision flags still needs a collision body");
    assert!(world.colliders[&p.prop_id].body.as_ref().unwrap().rigid.linear_velocity.length() > 1.0);
}

#[test]
fn knocked_props_are_hidden_for_a_while_and_come_back() {
    let mut world = PropWorld::default();
    let mut p = placement(7, Vec3::ZERO, 100.0, false, 0.0);
    world.add_tile(0, std::slice::from_mut(&mut p));
    let ground = FlatGround::new(0.0);
    assert!(!world.hidden(p.prop_id));
    world.knock(p.prop_id);
    assert!(world.hidden(p.prop_id));
    world.step(&ground, REGENERATE_AFTER - 1.0);
    assert!(world.hidden(p.prop_id), "still gone just before the time is up");
    world.step(&ground, 1.5);
    assert!(!world.hidden(p.prop_id), "back after REGENERATE_AFTER seconds");
}
