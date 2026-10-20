//! Test fixtures that hit placed scenery with a fast car, for the installed breaker checks.

use blackbox_attrib::vlt_hash;
use blackbox_vehicle::{FlatGround, InputState, Vehicle, VehicleSpec};
use glam::Vec3;

use super::PropWorld;
use crate::scenes::world::drive::{STEP, walls::Impact};

/// Hits the placed body `id` square on at 60 m/s with an example car and checks that the car really touched it.
pub(in crate::scenes::world) fn strike(world: &mut PropWorld, id: u32) {
    let prop = world.colliders[&id].body.as_ref().unwrap();
    let primitive = prop
        .geometry
        .primitives
        .iter()
        .find(|p| p.collides(blackbox_collision::bounds_flags::PRIM_VS_OBJECTS))
        .unwrap();
    let target = prop.obb(primitive);
    let mut car = Vehicle::new(VehicleSpec::example());
    let direction = target.axes.z_axis;
    car.body_mut().place(
        target.centre - direction * (target.half.z + VehicleSpec::example().dimension.z + 0.25),
        prop.rigid.orientation,
    );
    car.body_mut().linear_velocity = direction * 60.0;
    car.step(STEP, &InputState::default(), &FlatGround::new(-1000.0));
    let mut impact = Impact::default();
    world.collide_vehicle(&mut car, None, &mut impact);
    assert!(!impact.contacts.is_empty(), "installed fixture must physically hit {id}");
}

/// The ids of every placed model of event engine `engine`, in id order.
pub(in crate::scenes::world) fn all(world: &PropWorld, engine: &str) -> Vec<u32> {
    let key = vlt_hash(engine);
    world.scenery.models.iter().filter(|(_, m)| m.key == key && m.parent_owner.is_some()).map(|(id, _)| *id).collect()
}

/// The id of the placed model of event engine `engine` nearest to `near` (physics space).
pub(in crate::scenes::world) fn nearest(world: &PropWorld, engine: &str, near: Vec3) -> Option<u32> {
    let key = vlt_hash(engine);
    let away = |id: &u32| world.colliders[id].body.as_ref().unwrap().rigid.position.distance(near);
    world
        .scenery
        .models
        .iter()
        .filter(|(id, m)| {
            m.key == key && m.parent_owner.is_some() && world.colliders.get(id).is_some_and(|c| c.body.is_some())
        })
        .map(|(id, _)| *id)
        .min_by(|a, b| away(a).total_cmp(&away(b)))
}

/// A line about a placed body for a failing check: its detach force, mass, attachment and position.
pub(in crate::scenes::world) fn describe(world: &PropWorld, id: u32) -> String {
    let c = &world.colliders[&id];
    let (d, b) = (c.shape.dynamics.as_ref().unwrap(), c.body.as_ref().unwrap());
    format!(
        "detach {} immobile {} attached {} mass {} at {:?}",
        d.detach_impulse,
        d.immobile,
        b.attached,
        b.rigid.mass(),
        b.rigid.position
    )
}

/// The physics position of a placed body.
pub(in crate::scenes::world) fn position(world: &PropWorld, id: u32) -> Vec3 {
    world.colliders[&id].body.as_ref().unwrap().rigid.position
}
