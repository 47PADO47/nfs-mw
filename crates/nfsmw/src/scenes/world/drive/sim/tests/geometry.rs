use blackbox_collision::{Bounds, BoundsSet, bounds_flags as flags};

use super::*;

fn node(at: Vec3, half: Vec3, flags: u16) -> Bounds {
    Bounds {
        orientation: Quat::IDENTITY.to_array(),
        position: at.to_array(),
        flags,
        half_dimensions: half.to_array(),
        child_count: 0,
        point_cloud: None,
        pivot: [50.0; 3],
        first_child: -1,
        radius: half.length(),
        surface: 0,
        name_hash: 0,
    }
}

fn install(sim: &mut CarSim, children: Vec<Bounds>) {
    let mut root = node(Vec3::ZERO, sim.half_dimensions(), flags::BOX);
    root.first_child = 1;
    root.child_count = children.len() as u8;
    let set = BoundsSet { name_hash: 1, nodes: std::iter::once(root).chain(children).collect(), point_clouds: vec![] };
    sim.geometry = CarGeometry::read(Arc::new(set), None).map(Arc::new);
}

pub(super) fn compound(sim: &mut CarSim) {
    let half = sim.half_dimensions();
    let end = Vec3::Z * half.z * 0.5;
    let part = half * Vec3::new(1.0, 1.0, 0.5);
    let collision = flags::BOX | flags::PRIM_VS_OBJECTS;
    install(sim, vec![node(end, part, collision), node(-end, part, collision)]);
}

#[test]
fn cars_collide_with_body_primitives_and_leave_gaps_between_them() {
    let (mut a, mut b) = (sim(), sim());
    let collision = flags::BOX | flags::PRIM_VS_OBJECTS;
    let half = Vec3::new(0.2, 0.4, 1.5);
    install(&mut a, vec![node(Vec3::NEG_X * 0.75, half, collision)]);
    install(&mut b, vec![node(Vec3::X * 0.75, half, collision)]);
    a.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
    b.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
    b.body_mut().linear_velocity = Vec3::NEG_X * 10.0;
    assert!(blackbox_vehicle::rigid_body::obb_contact(&a.collision_box(), &b.collision_box()).is_some());
    assert!(a.collide_with(&mut b).is_none(), "empty parts of the root box are not solid");
    b.body_mut().position.x = -1.4;
    assert!(a.collide_with(&mut b).is_some(), "moving into the actual bodywork produces a hit");
}

#[test]
fn disabled_vehicle_parts_do_not_hit_and_protruding_shapes_expand_broad_phase() {
    let (mut a, mut b) = (sim(), sim());
    let collision = flags::BOX | flags::PRIM_VS_OBJECTS;
    install(&mut a, vec![node(Vec3::ZERO, Vec3::ONE, collision | flags::DISABLED)]);
    b.body_mut().linear_velocity = Vec3::X * 10.0;
    assert!(a.collide_with(&mut b).is_none());
    install(&mut a, vec![node(Vec3::Z * 12.0, Vec3::ONE, collision)]);
    assert!(a.collision_radius() > 13.0, "root dimensions alone miss protruding contact shapes");
}
