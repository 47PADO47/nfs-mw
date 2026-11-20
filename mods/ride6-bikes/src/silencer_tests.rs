use super::*;
use crate::glb::Node;

fn node(name: &str) -> Node {
    Node { name: name.to_owned(), parent: None }
}

/// Two cans of two triangles each over twelve vertices: vertices 0 to 5 on joint 0 (`BONE_MUFFLER_001`) and 1 (`BONE_MOUNT_001`),
/// 6 to 11 on joints 2 (`BONE_MUFFLER_002`) and 3 (`BONE_MOUNT_002`).
fn twin() -> Glb {
    let on = [0u16, 0, 1, 0, 1, 1, 2, 3, 2, 2, 3, 3];
    let count = on.len();
    let primitive = Primitive {
        positions: vec![[0.0; 3]; count],
        normals: vec![[0.0, 1.0, 0.0]; count],
        uvs: vec![[0.0; 2]; count],
        tangents: Vec::new(),
        indices: (0..count as u32).collect(),
        material: 0,
        joints: on.iter().map(|&j| [j, 0, 0, 0]).collect(),
        weights: vec![[1.0, 0.0, 0.0, 0.0]; count],
    };
    let nodes = ["BONE_MUFFLER_001", "BONE_MOUNT_001", "BONE_MUFFLER_002", "BONE_MOUNT_002"].map(node).to_vec();
    Glb { primitives: vec![primitive], materials: Vec::new(), nodes, joints: vec![0, 1, 2, 3] }
}

#[test]
fn a_bone_belongs_to_the_silencer_its_number_names() {
    assert_eq!(bone_group("BONE_MUFFLER_001"), 1);
    assert_eq!(bone_group("BONE_MOUNT_001"), 1);
    assert_eq!(bone_group("BONE_MUFFLER_002"), 2);
    assert_eq!(bone_group("BONE_MOUNT_002"), 2);
    for other in ["BONE_CHASSIS", "VEHICLE_BASE", "BONE_MUFFLER_", "BONE_MUFFLER_x", "BONE_MOUNT_000"] {
        assert_eq!(bone_group(other), 1, "{other}");
    }
}

#[test]
fn a_twin_has_two_groups_and_a_single_silencer_one() {
    assert_eq!(groups(&twin()), [1, 2]);
    let mut single = twin();
    single.joints = vec![0, 1];
    single.nodes.truncate(2);
    for joints in &mut single.primitives[0].joints {
        joints[0] %= 2;
    }
    assert_eq!(groups(&single), [1]);
    let unskinned = Glb { primitives: Vec::new(), materials: Vec::new(), nodes: Vec::new(), joints: Vec::new() };
    assert_eq!(groups(&unskinned), [1], "a mesh with no skin is the first silencer");
}

#[test]
fn the_triangles_of_a_twin_split_between_its_cans_and_none_is_lost() {
    let glb = twin();
    let joints = joint_groups(&glb);
    assert_eq!(joints, [1, 1, 2, 2]);
    let first = triangles(&glb.primitives[0], &joints, 1);
    let second = triangles(&glb.primitives[0], &joints, 2);
    assert_eq!(first, vec![[0, 1, 2], [3, 4, 5]]);
    assert_eq!(second, vec![[6, 7, 8], [9, 10, 11]]);
}

#[test]
fn a_triangle_goes_where_most_of_its_corners_are() {
    assert_eq!(majority([1, 2, 2]), 2);
    assert_eq!(majority([2, 1, 2]), 2);
    assert_eq!(majority([2, 2, 1]), 2);
    assert_eq!(majority([3, 2, 1]), 1, "all different: the lowest");
}

#[test]
fn the_first_silencer_keeps_the_parts_own_solid_name() {
    assert_eq!(solid_name("MUFFLER974_001", 1), "MUFFLER974_001");
    assert_eq!(solid_name("MUFFLER974_001", 2), "MUFFLER974_001_G2");
}
