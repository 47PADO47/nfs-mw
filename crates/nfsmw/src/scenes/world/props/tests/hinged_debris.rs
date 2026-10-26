//! A hinged part that left a mod car is an ordinary released fragment: it falls, lands, rests and goes with its car.

use std::rc::Rc;

use blackbox_gfx::{Instance, MeshHandle};
use nfsmw_data::car::mods::hinged::{Parent, debris_bounds, debris_dynamics, debris_motion, part_frame};

use super::*;

fn spawn(world: &mut PropWorld, owner: &Rc<()>, parent: &Parent, model: Mat4) -> u32 {
    // A bonnet: 1.4 m long, 1.0 m wide and 0.06 m thick in the model's space, its centre 1.5 m ahead of the car's
    // origin and 1 m up, turned 15 degrees about the lateral axis when it comes off.
    let centre = Vec3::new(1.5, 0.0, 1.0);
    let (position, orientation) = part_frame(model, centre);
    let motion = debris_motion(parent, position, Vec3::X, -15.0);
    world
        .spawn_fragment(FragmentSpawn {
            bounds: Arc::new(debris_bounds(Vec3::new(0.7, 0.5, 0.03))),
            root: 0,
            spec: Arc::new(debris_dynamics(40.0, 0)),
            pose: (position, orientation),
            model,
            velocity: (motion.velocity, motion.spin),
            instances: vec![Instance::new(MeshHandle::from_raw(1), Mat4::IDENTITY)],
            owner: Rc::downgrade(owner),
        })
        .expect("a part is a body the solver accepts")
}

#[test]
fn a_released_part_falls_to_the_road_keeps_the_cars_speed_and_comes_to_rest() {
    let owner = Rc::new(());
    let mut world = PropWorld::default();
    let parent = Parent { position: Vec3::new(0.0, 0.5, 0.0), velocity: Vec3::Z * 10.0, angular_velocity: Vec3::ZERO };
    let id = spawn(&mut world, &owner, &parent, Mat4::IDENTITY);
    let start = world.colliders[&id].body.as_ref().unwrap().rigid.clone();
    assert!((start.position - Vec3::new(0.0, 1.0, 1.5)).length() < 1e-4);
    assert!(
        start.linear_velocity.z > 10.0 && start.linear_velocity.y > 0.0,
        "it leaves with the car's speed and a lift"
    );

    let mut highest = start.position.y;
    for _ in 0..120 {
        world.step(&FlatGround::new(0.0), 1.0 / 60.0);
        highest = highest.max(world.colliders[&id].body.as_ref().unwrap().rigid.position.y);
    }
    assert!(highest > start.position.y, "the kick throws it up before gravity wins");
    for _ in 0..480 {
        world.step(&FlatGround::new(0.0), 1.0 / 60.0);
    }
    let body = &world.colliders[&id].body.as_ref().unwrap().rigid;
    assert!(body.position.is_finite() && body.orientation.is_finite());
    assert!(body.position.y > -0.05 && body.position.y < 0.8, "it lies on the road, not in it or above it: {body:?}");
    assert!(body.position.z > start.position.z + 10.0, "it carried the car's speed down the road");
    assert!(body.linear_velocity.length() < 1.0, "it has come to rest: {:?}", body.linear_velocity);
    assert!(body.orientation.angle_between(start.orientation) > 0.01, "it tumbled on the way");

    world.set_alpha(1.0);
    let mut drawn = Vec::new();
    world.fragment_instances(&mut drawn);
    assert_eq!(drawn.len(), 1, "the part is drawn where it lies");
    assert_eq!(world.fragment_count(), 1);
    drop(owner);
    world.step(&FlatGround::new(0.0), 1.0 / 60.0);
    assert!(world.colliders.is_empty(), "the part goes with its car");
}
