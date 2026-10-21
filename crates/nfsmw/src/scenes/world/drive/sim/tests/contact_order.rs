//! An authored wall must not consume a prop's incoming breakaway hit first.

use std::sync::Arc;

use blackbox_collision::{Article, Barrier, Bounds, BoundsSet, CollisionPack, Instance, Shape, bounds_flags};
use blackbox_scenery::ObjectSpawner;
use glam::Mat4;
use nfsmw_data::world::{PropDynamics, PropKind, PropShape};

use super::*;
use crate::scenes::world::resident::Placed;

fn attached_pole() -> Placed {
    let half = Vec3::new(0.2, 4.0, 0.2);
    let bounds = Arc::new(BoundsSet {
        name_hash: 1,
        nodes: vec![Bounds {
            orientation: Quat::IDENTITY.to_array(),
            position: [0.0; 3],
            flags: bounds_flags::BOX | bounds_flags::PRIM_VS_OBJECTS,
            half_dimensions: half.to_array(),
            child_count: 0,
            point_cloud: None,
            pivot: [0.0; 3],
            first_child: -1,
            radius: half.length(),
            surface: 0,
            name_hash: 2,
        }],
        point_clouds: vec![],
    });
    let dynamics = Arc::new(PropDynamics {
        sound_profile: 0,
        camera_avoidable: false,
        mass: 200.0,
        tensor_scale: Vec3::ONE,
        body: Default::default(),
        walls: Default::default(),
        no_world_collisions: false,
        no_object_collisions: false,
        fallback_shape: Shape::Box,
        base_surface: 0,
        detach_impulse: 7500.0,
        no_car_effect: true,
        immobile: false,
        respawn_time: 120.0,
        kill_off_screen: 8.0,
        dropout: [0.0; 2],
        allow_off_world: false,
        event_sequencer: 0,
        start_sequencer: false,
        no_trigger: false,
        explosion_effect: 0.0,
        cost_to_state: 0,
    });
    let shape = Arc::new(PropShape {
        name: "authored-pole".into(),
        boxes: vec![],
        kind: PropKind::Movable { mass: 200.0 },
        bounds: Some(bounds),
        dynamics: Some(dynamics),
    });
    let mut placed = Placed::prop_only(Mat4::IDENTITY, shape);
    placed.spawner = Some(ObjectSpawner {
        orientation: Quat::IDENTITY.to_array(),
        position: [0.0, 0.0, 2.4],
        model: 1,
        collision: 1,
        attributes: 1,
        override_index: 0,
        unique_id: 15,
        exclude_flags: 0,
    });
    placed
}

fn nearby_wall() -> CollisionWorld {
    let mut world = CollisionWorld::new(None);
    world.insert(CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 10.0,
            half_height: 10.0,
            half_length: 10.0,
            radius: 15.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![],
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
            barriers: vec![Barrier {
                p0: [-10.0, -10.0, 1.9],
                p1: [10.0, 10.0, 1.9],
                surface: 0,
                flags: blackbox_collision::BARRIER_TWO_SIDED,
                inv_xz_len: 0.05,
            }],
        }],
        objects: vec![],
    });
    world
}

#[test]
fn a_pole_receives_the_incoming_impulse_before_an_authored_wall() {
    let collision = nearby_wall();
    let mut props = PropWorld::default();
    props.add_tile(0, &mut [attached_pole()]);
    let mut car = sim();
    car.vehicle.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
    car.vehicle.body_mut().linear_velocity = Vec3::Z * 30.0;
    let impact = car.step(&DriveInput::default(), &NoGround, Some((&collision, &mut props)));
    assert!(
        impact.contacts.iter().any(|c| c.kind == walls::ContactKind::PropMovable),
        "an earlier wall response must not discard the approaching breakaway hit"
    );
    assert_eq!(props.knocked_count(), 1, "the authored prop must detach at this speed");
    assert!(impact.rigid > 0, "the authored wall must still constrain the car afterwards");
    assert!(car.is_finite());
}
