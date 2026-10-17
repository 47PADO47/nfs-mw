use super::*;
use blackbox_gfx::{InstanceKey, MeshHandle};
use blackbox_scene::Aabb;
use glam::{Quat, Vec3};

#[test]
fn source_road_slots_and_thresholds_do_not_fall_back_to_main_view_lod() {
    assert_eq!(slot(0, 31), None);
    assert_eq!(slot(0, 32), Some(3));
    assert_eq!(slot(0x80, 32), Some(2));
    assert_eq!(slot(0x100, 32), Some(0));
    assert_eq!(slot(0x0100_0000, 32), Some(0));
    assert_eq!(slot(0x180, 32), Some(2), "slot2 flag wins");
    assert_eq!(slot(0x0200_0000, 22), Some(3));
    assert_eq!(slot(0x0200_0000, 21), None);
    assert_eq!(slot(0x0200_0000, 1), None, "initial two-pixel rejection precedes bias");
    let mut p = placed();
    p.lods[3] = None;
    let mut out = Vec::new();
    scenery(std::iter::once(&p), &camera(), &PropWorld::default(), 240.0, &mut out);
    assert!(out.is_empty(), "missing authored reflection mesh must stay missing");
    p.flags = 0x80;
    scenery(std::iter::once(&p), &camera(), &PropWorld::default(), 240.0, &mut out);
    assert_eq!(out[0].mesh, p.lods[2].unwrap());
}

#[test]
fn scenery_flips_one_source_matrix_element_and_keeps_the_source_origin() {
    let pose = Mat4::from_scale_rotation_translation(
        Vec3::new(2.0, 3.0, 4.0),
        Quat::from_rotation_x(0.7),
        Vec3::new(100.0, 200.0, 60.0),
    );
    let reflected = scenery_transform(pose, 0);
    let mut expected = pose.to_cols_array();
    expected[10] = -expected[10];
    assert_eq!(reflected.to_cols_array(), expected);
    assert_eq!(reflected.w_axis, pose.w_axis);
    assert_eq!(scenery_transform(pose, 0x200), Mat4::IDENTITY, "source null matrix uses authored world geometry");
    let body = vehicle_root(pose);
    assert_eq!(body.z_axis, -pose.z_axis);
    assert_eq!(body.w_axis, pose.w_axis);
    assert_ne!(body, reflected, "body callback and scenery callback have different source contracts");
}

#[test]
fn source_scenery_collection_is_stable_and_uses_authored_reflection_mesh() {
    let p = placed();
    let mut out = vec![];
    scenery(std::iter::once(&p), &camera(), &PropWorld::default(), 240.0, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].mesh, MeshHandle::from_raw(4));
    assert_eq!(out[0].transform.w_axis, p.transform.w_axis);
    assert_eq!(out[0].transform.z_axis.z, -p.transform.z_axis.z);
    assert_eq!(out[0].key, p.key);
}

fn placed() -> ReflectionPlaced {
    ReflectionPlaced {
        lods: [1, 2, 3, 4].map(|i| Some(MeshHandle::from_raw(i))),
        transform: Mat4::from_translation(Vec3::Z * 0.5),
        bounds: Aabb::new([-0.1, -0.1, 0.4], [0.1, 0.1, 0.6]),
        radius: 1.0,
        flags: 0x40,
        prop_id: 0,
        key: InstanceKey::new(3, 9),
        player_index: None,
    }
}
fn camera() -> Camera {
    Camera {
        view_proj: Mat4::IDENTITY,
        position: [0.0; 3],
        forward: [0.0, 0.0, 1.0],
        fov_y_radians: 1.0,
        light_dir: [0.0, 0.0, -1.0],
    }
}
