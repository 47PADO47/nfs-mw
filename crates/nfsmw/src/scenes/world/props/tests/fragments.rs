//! Released fragments integrate inherited motion without acquiring placement attachment.

use std::rc::Rc;

use blackbox_gfx::{Instance, MeshHandle};

use super::*;

#[test]
fn released_fragments_move_and_draw_even_with_breakaway_attributes() {
    for detach in [0.0, 7500.0, -1.0] {
        let placed = placement(60, Vec3::Y * 10.0, 200.0, false, detach);
        let shape = placed.prop.as_ref().unwrap();
        let bounds = shape.bounds.as_ref().unwrap();
        let spec = shape.dynamics.as_ref().unwrap();
        let mut virgin = PropBody::new(placed.spawner.as_ref().unwrap(), bounds, spec, placed.transform).unwrap();
        assert_eq!(virgin.attached, detach != 0.0, "authored placement retains its breakaway rule");
        virgin.attached = false;
        virgin.restore(spec);
        assert_eq!(virgin.attached, detach != 0.0, "restored placement retains its breakaway rule");

        let owner = Rc::new(());
        let mut world = PropWorld::default();
        let mesh = MeshHandle::from_raw(1);
        let id = world
            .spawn_fragment(FragmentSpawn {
                bounds: bounds.clone(),
                root: 0,
                spec: spec.clone(),
                pose: (Vec3::Y * 10.0, Quat::IDENTITY),
                model: placed.transform,
                velocity: (Vec3::X, Vec3::new(0.1, 0.0, 0.3)),
                instances: vec![Instance::new(mesh, Mat4::IDENTITY)],
                owner: Rc::downgrade(&owner),
            })
            .unwrap();
        let initial = world.colliders[&id].body.as_ref().unwrap().rigid.clone();
        for _ in 0..30 {
            world.step(&FlatGround::new(-1000.0), 1.0 / 60.0);
        }
        world.set_alpha(1.0);
        let body = &world.colliders[&id].body.as_ref().unwrap().rigid;
        assert!(body.position.y < initial.position.y - 0.5, "fragment with threshold {detach} must fall");
        assert!(body.position.x > initial.position.x + 0.1, "fragment inherits linear motion");
        assert!(body.orientation.angle_between(initial.orientation) > 0.01, "fragment inherits angular motion");
        let expected = world.pose(id).unwrap().0;
        assert!(expected.is_finite());
        assert_ne!(expected, placed.transform, "released mesh follows the moving body");
        let mut drawn = Vec::new();
        world.fragment_instances(&mut drawn);
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].mesh, mesh);
        assert_eq!(drawn[0].transform, expected);
        drop(owner);
        world.step(&FlatGround::new(-1000.0), 1.0 / 60.0);
        assert!(world.colliders.is_empty(), "expired owner cleans up released physics and art");
    }
}
