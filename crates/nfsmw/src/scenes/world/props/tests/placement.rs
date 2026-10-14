//! Authored pivot placement is distinct from the unit rotation used by rigid physics.

use super::*;

#[test]
fn stored_spawner_quaternion_places_root_without_normalizing_its_offset() {
    let at = Vec3::new(5.0, 0.0, 7.0);
    // Hand-computed matrix expansions, covering subunit, superunit and three-axis rotations.
    let cases = [
        ([0.0, 0.0, 0.0, 1.0], Vec3::new(1.0, 2.0, 3.0)),
        ([0.0, 0.5, 0.0, 0.5], Vec3::new(2.0, 2.0, 1.0)),
        ([0.0, 1.0, 0.0, 1.0], Vec3::new(5.0, 2.0, -5.0)),
        ([0.25, -0.25, 0.25, 0.25], Vec3::new(0.25, 0.75, 2.5)),
        ([-0.25, 0.25, -0.25, -0.25], Vec3::new(0.25, 0.75, 2.5)),
    ];
    for (orientation, offset) in cases {
        let mut p = placement(1, at, 100.0, false, 0.0);
        p.spawner.as_mut().unwrap().orientation = orientation;
        p.transform = Mat4::from_scale_rotation_translation(
            Vec3::new(2.0, 1.5, 0.8),
            Quat::from_rotation_z(0.3),
            space::to_render(at.to_array()),
        );
        let shape = Arc::make_mut(p.prop.as_mut().unwrap());
        Arc::make_mut(shape.bounds.as_mut().unwrap()).nodes[0].pivot = [1.0, 2.0, 3.0];
        let spec = shape.dynamics.as_ref().unwrap();
        let mut body =
            PropBody::new(p.spawner.as_ref().unwrap(), shape.bounds.as_ref().unwrap(), spec, p.transform).unwrap();
        assert!(body.rigid.position.distance(at + offset) < 1e-5, "stored quaternion {orientation:?}");
        assert!((body.rigid.orientation.length_squared() - 1.0).abs() < 1e-5);
        let primitive = body.obb(&body.geometry.primitives[0]);
        assert!(primitive.centre.distance(at + offset) < 1e-5);
        let identity = primitive.axes.transpose() * primitive.axes;
        assert!(identity.abs_diff_eq(glam::Mat3::IDENTITY, 1e-5));
        for alpha in [0.0, 0.5, 1.0] {
            assert!(body.transform(alpha).abs_diff_eq(p.transform, 1e-5), "the initial mesh must not move");
        }
        body.rigid.position += Vec3::splat(10.0);
        body.restore(spec);
        assert!(body.rigid.position.distance(at + offset) < 1e-5, "restoration retains the authored centre");
    }
}
