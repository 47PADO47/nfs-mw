use super::*;

fn node(name: u32, flags: u16, position: [f32; 3]) -> Bounds {
    Bounds {
        orientation: Quat::IDENTITY.to_array(),
        position,
        flags,
        half_dimensions: [1.0, 2.0, 3.0],
        child_count: 0,
        point_cloud: None,
        pivot: (Vec3::from_array(position) + Vec3::splat(10.0)).to_array(),
        first_child: -1,
        radius: 0.75,
        surface: 0,
        name_hash: name,
    }
}

fn set() -> BoundsSet {
    let mut root = node(0, flags::BOX, [0.0; 3]);
    root.child_count = 2;
    root.first_child = 1;
    let mut fragment = node(22, flags::BOX | flags::INTERNAL, [5.0, 6.0, 7.0]);
    fragment.child_count = 1;
    fragment.first_child = 3;
    let mut part = node(33, flags::BOX | PRIMITIVE, [0.5, 0.6, 0.7]);
    part.orientation = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2).to_array();
    part.half_dimensions = [0.3, 0.4, 0.5];
    BoundsSet {
        name_hash: 1,
        nodes: vec![root, node(11, flags::BOX | PRIMITIVE, [2.0, 3.0, 4.0]), fragment, part],
        point_clouds: vec![],
    }
}

fn options() -> GeometryOptions {
    GeometryOptions {
        fallback_half: Vec3::new(1.0, 2.0, 3.0),
        fallback_shape: Shape::Box,
        box_padding: Vec3::ZERO,
        surface: 99,
    }
}

#[test]
fn pc_bounds_vectors_select_one_body_and_keep_disabled_geometry() {
    // Independent synthetic inputs also executed through the PC construction routines.
    let mut s = set();
    for disabled in [false, true] {
        s.nodes[1].flags |= u16::from(disabled);
        let g = BodyGeometry::from_bounds(&s, 0, options()).unwrap();
        assert!(!g.used_fallback);
        assert_eq!(g.primitives.len(), 1);
        let p = &g.primitives[0];
        assert_eq!(p.name, 11);
        assert_eq!(p.centre, Vec3::new(2.0, 3.0, 4.0));
        assert_eq!(p.half, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(p.rotation, Quat::IDENTITY);
        assert_eq!(p.flags, 0x4e | u16::from(disabled));
        assert_eq!(p.collides(flags::PRIM_VS_OBJECTS), !disabled);
        assert_eq!(g.centre_of_gravity(), p.centre);
    }
    let g = BodyGeometry::from_bounds(&s, 2, options()).unwrap();
    assert_eq!(g.primitives.len(), 1);
    assert_eq!(g.primitives[0].name, 33);
    assert_eq!(g.primitives[0].centre, Vec3::new(0.5, 0.6, 0.7));
    assert!((g.primitives[0].rotation.x - 0.70711386).abs() < 0.00001);
    let g = BodyGeometry::from_bounds(&s, 1, options()).unwrap();
    assert_eq!(g.primitives[0].centre, Vec3::ZERO);
    assert_eq!(g.primitives[0].rotation, Quat::IDENTITY);
}

#[test]
fn missing_primitive_flags_still_make_a_collidable_fallback() {
    let mut s = set();
    s.nodes.truncate(1);
    s.nodes[0].child_count = 0;
    let mut opts = options();
    opts.box_padding = Vec3::splat(0.1);
    let g = BodyGeometry::from_bounds(&s, 0, opts).unwrap();
    assert!(g.used_fallback);
    assert_eq!(g.primitives[0].half, opts.fallback_half + opts.box_padding);
    assert_eq!(g.primitives[0].surface, 99);
    assert!(g.primitives[0].collides(flags::PRIM_VS_GROUND));
    opts.fallback_shape = Shape::Sphere;
    let g = BodyGeometry::from_bounds(&s, 0, opts).unwrap();
    assert_eq!(g.primitives[0].half, Vec3::splat(3.0));
    assert!(g.primitives[0].collides(flags::PRIM_VS_OBJECTS));
    assert!(!g.primitives[0].collides(flags::PRIM_VS_GROUND));
}

#[test]
fn sphere_radius_order_padding_and_default_material_are_preserved() {
    let mut s = set();
    s.nodes.truncate(1);
    s.nodes[0].child_count = 3;
    for (name, shape) in [(1, flags::SPHERE), (2, flags::BOX), (3, flags::SPHERE)] {
        s.nodes.push(node(name, shape | PRIMITIVE, [name as f32, 0.0, 0.0]));
    }
    s.nodes[3].surface = 88;
    let mut opts = options();
    opts.box_padding = Vec3::splat(0.1);
    let g = BodyGeometry::from_bounds(&s, 0, opts).unwrap();
    assert_eq!(g.primitives.iter().map(|p| p.name).collect::<Vec<_>>(), [3, 1, 2]);
    assert_eq!(g.primitives[0].half, Vec3::splat(0.75));
    assert_eq!(g.primitives[0].surface, 88);
    assert_eq!(g.primitives[1].surface, 99);
    assert_eq!(g.primitives[2].half, Vec3::new(1.1, 2.1, 3.1));
}

#[test]
fn mesh_only_geometry_prevents_fallback_and_root_mesh_uses_its_inverse_frame() {
    let mut s = set();
    s.nodes.truncate(2);
    s.nodes[0].child_count = 1;
    s.nodes[1].flags = flags::MESH_VS_GROUND;
    s.nodes[1].point_cloud = Some(0);
    s.point_clouds.push(vec![[2.0, 3.0, 5.0]]);
    let g = BodyGeometry::from_bounds(&s, 0, options()).unwrap();
    assert!(!g.used_fallback);
    assert!(g.primitives.is_empty());
    assert_eq!(g.ground_meshes[0].points, [Vec3::new(2.0, 3.0, 5.0)]);
    let g = BodyGeometry::from_bounds(&s, 1, options()).unwrap();
    assert_eq!(g.ground_meshes[0].points, [Vec3::Z]);
    s.point_clouds[0].clear();
    assert!(BodyGeometry::from_bounds(&s, 1, options()).unwrap().used_fallback);
}
