//! Twin silencers: an aftermarket silencer mesh that is two rigid pieces, told apart by the numbers of its bones.
//!
//! A silencer is skinned to `BONE_MUFFLER_001` (and `BONE_MOUNT_001`). A twin (a pair of cans, 122 of the 765 exhausts of the
//! catalogue) also has `BONE_MUFFLER_002` and `BONE_MOUNT_002`, and the two sets of bones sit at different places on the header
//! pipes it is fitted to, so one transform cannot place both: the mesh is split by the number, each group is its own solid in the
//! pack and has its own anchor in the catalogue (`tools/ride6-exporter/option_place.py`, `docs/formats/ride6-bike-parts.md`).
//! **[choice]** the number is read from the bone's name (`BONE_MUFFLER_<n>`, `BONE_MOUNT_<n>`); every other bone is the first's.

use crate::glb::{Glb, Primitive};

/// The group of the bone named `name`: the number of `BONE_MUFFLER_<n>` and `BONE_MOUNT_<n>`, else 1.
pub fn bone_group(name: &str) -> u8 {
    let number = name.strip_prefix("BONE_MUFFLER_").or_else(|| name.strip_prefix("BONE_MOUNT_"));
    number.and_then(|digits| digits.parse::<u8>().ok()).filter(|group| *group >= 1).unwrap_or(1)
}

/// The group of each joint of `glb`.
pub fn joint_groups(glb: &Glb) -> Vec<u8> {
    (0..glb.joints.len()).map(|joint| glb.joint_name(joint).map_or(1, bone_group)).collect()
}

/// The group of the joint that moves the vertex `vertex` of `primitive` most (1 for a vertex with no skin).
fn dominant(primitive: &Primitive, joints: &[u8], vertex: usize) -> u8 {
    let (Some(on), Some(weights)) = (primitive.joints.get(vertex), primitive.weights.get(vertex)) else { return 1 };
    let mut best = (0.0f32, 1u8);
    for (&joint, &weight) in on.iter().zip(weights) {
        if weight > best.0 {
            best = (weight, joints.get(usize::from(joint)).copied().unwrap_or(1));
        }
    }
    best.1
}

/// The groups the vertices of `glb` belong to, in order: `[1]` for a single silencer, `[1, 2]` for a twin.
pub fn groups(glb: &Glb) -> Vec<u8> {
    let joints = joint_groups(glb);
    let mut found: Vec<u8> = glb
        .primitives
        .iter()
        .flat_map(|primitive| (0..primitive.positions.len()).map(|vertex| dominant(primitive, &joints, vertex)))
        .collect();
    found.sort_unstable();
    found.dedup();
    match found.is_empty() {
        true => vec![1],
        false => found,
    }
}

/// The group two or three corners agree on; when all three differ, the lowest.
fn majority([a, b, c]: [u8; 3]) -> u8 {
    if a == b || a == c {
        return a;
    }
    if b == c {
        return b;
    }
    a.min(b).min(c)
}

/// The triangles of `primitive` that belong to `group`: those whose corners mostly have their most weight on its joints.
pub fn triangles(primitive: &Primitive, joints: &[u8], group: u8) -> Vec<[u32; 3]> {
    let of = |corner: u32| dominant(primitive, joints, corner as usize);
    let own = |triangle: &&[u32; 3]| majority(triangle.map(of)) == group;
    primitive.indices.as_chunks::<3>().0.iter().filter(own).copied().collect()
}

/// The name of the solid of silencer `group` (1 based) of the part whose solid is `base`: the first has the part's own name.
pub fn solid_name(base: &str, group: u8) -> String {
    match group {
        0 | 1 => base.to_owned(),
        n => format!("{base}_G{n}"),
    }
}

#[cfg(test)]
#[path = "silencer_tests.rs"]
mod tests;
