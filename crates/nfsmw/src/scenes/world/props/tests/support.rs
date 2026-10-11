use blackbox_collision::BodyMesh;

use super::*;

fn world(disabled: bool) -> (PropWorld, u32) {
    let mut world = PropWorld::default();
    let mut p = placement(90, Vec3::Y * 3.0, 1000.0, false, 0.0);
    let bounds = Arc::make_mut(Arc::make_mut(p.prop.as_mut().unwrap()).bounds.as_mut().unwrap());
    bounds.nodes[0].flags = flags::BOX | flags::PRIM_VS_OBJECTS;
    bounds.nodes[0].half_dimensions = [0.5, 1.0, 0.5];
    world.add_tile(1, std::slice::from_mut(&mut p));
    let body = world.colliders.get_mut(&p.prop_id).unwrap().body.as_mut().unwrap();
    body.geometry.ground_meshes.push(BodyMesh {
        name: 1,
        points: vec![
            Vec3::new(-0.4, -1.0, -0.4),
            Vec3::new(0.4, -1.0, -0.4),
            Vec3::new(0.4, -1.0, 0.4),
            Vec3::new(-0.4, -1.0, 0.4),
            Vec3::new(-0.4, 1.0, -0.4),
            Vec3::new(0.4, 1.0, -0.4),
            Vec3::new(0.4, 1.0, 0.4),
            Vec3::new(-0.4, 1.0, 0.4),
        ],
        surface: 0,
        flags: flags::MESH_VS_GROUND
            | match disabled {
                true => flags::DISABLED,
                false => 0,
            },
    });
    body.rigid.wake();
    (world, p.prop_id)
}

fn trace(fps: usize, disabled: bool) -> (Vec<u32>, f32) {
    let (mut world, id) = world(disabled);
    let mut clock = crate::scenes::world::drive::FixedClock::default();
    let mut trace = Vec::new();
    for _ in 0..fps * 4 {
        for _ in 0..clock.advance(1.0 / fps as f32) {
            world.step(&FlatGround::new(0.0), 1.0 / 60.0);
            let body = &world.colliders[&id].body.as_ref().unwrap().rigid;
            assert!(
                body.position.is_finite()
                    && body.orientation.is_finite()
                    && body.linear_velocity.is_finite()
                    && body.angular_velocity.is_finite()
            );
            trace.extend(
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
    (trace, world.colliders[&id].body.as_ref().unwrap().rigid.position.y)
}

#[test]
fn ground_clouds_support_the_body_without_box_corners_and_follow_the_fixed_clock() {
    let (reference, y) = trace(60, false);
    assert!((0.39..1.05).contains(&y), "cloud support should hold the body above the ground after falling, got {y}");
    for fps in [5, 10, 30, 120, 144, 240] {
        assert_eq!(trace(fps, false).0, reference, "{fps} FPS");
    }
    assert!(trace(60, true).1 < -1.0, "disabled ground clouds cannot support a body");
}
