use blackbox_collision::{Bounds, Shape, bounds_flags};
use blackbox_vehicle::rigid_body::RigidBodySpec;

use super::*;

mod duplicates;

#[test]
fn models_sharing_original_physics_keys_keep_their_own_identity() {
    let bounds = Arc::new(BoundsSet {
        name_hash: 1,
        nodes: vec![Bounds {
            orientation: Quat::IDENTITY.to_array(),
            position: [0.0; 3],
            flags: bounds_flags::BOX | bounds_flags::PRIM_VS_WORLD | bounds_flags::PRIM_VS_OBJECTS,
            half_dimensions: [1.0, 4.0, 2.0],
            child_count: 0,
            point_cloud: None,
            pivot: [0.0; 3],
            first_child: -1,
            radius: 5.0,
            surface: 0,
            name_hash: 3,
        }],
        point_clouds: vec![],
    });
    let dynamics = Arc::new(PropDynamics {
        mass: 10000.0,
        tensor_scale: Vec3::splat(2.0),
        body: RigidBodySpec::default(),
        fallback_shape: Shape::Box,
        base_surface: 0,
        detach_impulse: -1.0,
        no_car_effect: false,
        immobile: false,
        respawn_time: 120.0,
        kill_off_screen: 8.0,
        dropout: [0.0; 2],
        allow_off_world: false,
        event_sequencer: 0,
        start_sequencer: false,
        no_trigger: false,
        explosion_effect: 0.0,
    });
    let catalog = PropCatalog {
        hierarchies: HashMap::new(),
        sets: HashMap::from([(1, bounds)]),
        dynamics: HashMap::from([(2, dynamics)]),
        cache: RefCell::default(),
        placed_cache: RefCell::default(),
    };
    let spawn = ObjectSpawner {
        orientation: Quat::IDENTITY.to_array(),
        position: [0.0; 3],
        model: 3,
        collision: 1,
        attributes: 2,
        override_index: 0,
        unique_id: 0,
        exclude_flags: 0,
    };
    let a = catalog.for_spawner("large-a", &spawn).unwrap();
    let b = catalog.for_spawner("large-b", &spawn).unwrap();
    assert_eq!((&a.name, &b.name), (&"large-a".to_owned(), &"large-b".to_owned()));
    assert!(Arc::ptr_eq(a.bounds.as_ref().unwrap(), b.bounds.as_ref().unwrap()));
    assert!(Arc::ptr_eq(a.dynamics.as_ref().unwrap(), b.dynamics.as_ref().unwrap()));
    assert!(Arc::ptr_eq(&a, &catalog.for_spawner("large-a", &spawn).unwrap()));
    assert_eq!(b.kind, PropKind::Movable { mass: 10000.0 });
}
