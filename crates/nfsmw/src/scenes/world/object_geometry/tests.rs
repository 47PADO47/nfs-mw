use blackbox_vehicle::rigid_body::RigidBodySpec;

use super::*;

#[test]
fn primitive_travel_includes_body_rotation_and_reverse_dispatch_flips_the_normal() {
    let mut body = RigidBody::new(100.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    body.place(Vec3::ZERO, Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
    let geometry = BodyGeometry {
        primitives: vec![BodyPrimitive {
            name: 0,
            centre: Vec3::X,
            rotation: Quat::IDENTITY,
            half: Vec3::splat(0.5),
            shape: Shape::Sphere,
            flags: bounds_flags::PRIM_VS_OBJECTS,
            surface: 0,
        }],
        ..Default::default()
    };
    let shapes =
        object_shapes(&body, Some(BodyGeometryFrame { geometry: &geometry, previous: (Vec3::ZERO, Quat::IDENTITY) }));
    assert!(shapes[0].travel.distance(Vec3::new(-1.0, 0.0, -1.0)) < 1e-6);
    let sphere = ObjectGeometry {
        bounds: Obb { centre: Vec3::X * 1.4, half: Vec3::splat(0.5), axes: Mat3::IDENTITY },
        shape: Shape::Sphere,
        travel: Vec3::ZERO,
        surface: 0,
    };
    let b = ObjectGeometry {
        bounds: Obb { centre: Vec3::ZERO, half: Vec3::ONE, axes: Mat3::IDENTITY },
        shape: Shape::Box,
        travel: Vec3::ZERO,
        surface: 0,
    };
    let ab = sphere.contact(&b).unwrap();
    let ba = b.contact(&sphere).unwrap();
    assert_eq!(ab.normal, -ba.normal);
    assert_eq!(ab.point, ba.point);
    assert_eq!(ab.overlap, ba.overlap);
}
