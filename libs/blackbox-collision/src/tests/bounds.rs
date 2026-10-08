use super::synth::*;
use crate::{Error, Shape, bounds_flags as f, find_bounds, read_bounds_sets};

/// A 0x30-byte node, with positions in millimetres as stored.
#[allow(clippy::too_many_arguments)]
fn node(
    flags: u16,
    half: [i16; 3],
    pos: [i16; 3],
    children: u8,
    first: i16,
    cloud: i8,
    radius: f32,
    surface: u32,
) -> Vec<u8> {
    let mut v = Vec::new();
    for q in [0i16, 0, 0, 32767] {
        v.extend_from_slice(&q.to_le_bytes());
    }
    for p in pos {
        v.extend_from_slice(&p.to_le_bytes());
    }
    push_u16(&mut v, flags);
    for h in half {
        v.extend_from_slice(&h.to_le_bytes());
    }
    v.push(children);
    v.push(cloud as u8);
    for p in pos {
        v.extend_from_slice(&(p + 100).to_le_bytes()); // pivot
    }
    v.extend_from_slice(&first.to_le_bytes());
    push_f32(&mut v, radius);
    push_u32(&mut v, surface);
    push_u32(&mut v, 0xABCD);
    push_u32(&mut v, 0);
    v
}

/// Payload of a `0x3B901` chunk: root box with two children (a sphere and a mesh node).
fn bounds_payload(name: u32, child_first: i16) -> Vec<u8> {
    let mut v = Vec::new();
    for w in [name, 3, 0, 0] {
        push_u32(&mut v, w);
    }
    v.extend(node(f::BOX | f::PRIM_VS_WORLD, [900, 600, 2200], [0, 0, 0], 2, child_first, -1, 2.2, 0));
    v.extend(node(f::SPHERE | f::PRIM_VS_GROUND, [300, 300, 300], [500, -400, 2000], 0, -1, -1, 0.3, 0x99));
    v.extend(node(f::BOX | f::MESH_VS_GROUND, [800, 400, 2000], [0, 10, 0], 0, -1, 0, 2.0, 0));
    // One point cloud of two points.
    for w in [1, 0, 0, 0] {
        push_u32(&mut v, w);
    }
    for w in [2, 0, 0, 0] {
        push_u32(&mut v, w);
    }
    for p in [[1.0f32, 0.5, 2.0, 0.0], [-1.0, 0.25, -2.0, 0.0]] {
        for c in p {
            push_f32(&mut v, c);
        }
    }
    v
}

#[test]
fn parses_a_bounds_set() {
    let set = crate::BoundsSet::parse(&bounds_payload(0x1234, 1)).unwrap();
    assert_eq!(set.name_hash, 0x1234);
    assert_eq!(set.nodes.len(), 3);
    let root = set.root();
    assert_eq!(root.shape(), Shape::Box);
    assert_eq!(root.half_dimensions, [0.9, 0.6, 2.2]);
    assert_eq!(root.orientation, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!((root.child_count, root.first_child, root.point_cloud), (2, 1, None));
    let kids = set.children(root);
    assert_eq!(kids.len(), 2);
    assert_eq!(kids[0].shape(), Shape::Sphere);
    assert_eq!(kids[0].position, [0.5, -0.4, 2.0]);
    assert_eq!(kids[0].pivot, [0.6, -0.3, 2.1]);
    assert_eq!((kids[0].radius, kids[0].surface), (0.3, 0x99));
    assert_eq!(kids[1].point_cloud, Some(0));
    assert_eq!(kids[1].flags & f::MESH_VS_GROUND, f::MESH_VS_GROUND);
    assert_eq!(set.point_clouds, [vec![[1.0, 0.5, 2.0], [-1.0, 0.25, -2.0]]]);
    assert!(set.is_tree());
    assert!(set.children(&kids[0]).is_empty());
}

#[test]
fn finds_sets_in_a_chunk_stream() {
    // The container header takes 8 bytes, so children start at file offset 8; each payload is
    // padded with 0x11 so it starts 16-byte aligned, as in the game's files.
    let mut inner = Vec::new();
    for payload in [bounds_payload(7, 1), bounds_payload(9, 1)] {
        let pad = (16 - (8 + inner.len() + 8) % 16) % 16;
        let mut p = vec![0x11; pad];
        p.extend_from_slice(&payload);
        inner.extend(chunk(0x0003_B901, &p));
    }
    let file = chunk(0x8003_B900, &inner);
    let sets = read_bounds_sets(&file).unwrap();
    assert_eq!(sets.len(), 2);
    assert_eq!(find_bounds(&sets, 9).unwrap().name_hash, 9);
    assert!(find_bounds(&sets, 10).is_none());
}

#[test]
fn rejects_bad_trees() {
    // Children pointing past the node list.
    assert!(matches!(crate::BoundsSet::parse(&bounds_payload(1, 2)), Err(Error::Malformed { .. })));
    // A tree whose root lists itself as a child is not a tree.
    let set = crate::BoundsSet::parse(&bounds_payload(1, 0)).unwrap();
    assert!(!set.is_tree());
    assert!(matches!(crate::BoundsSet::parse(&bounds_payload(1, 1)[..100]), Err(Error::Truncated { .. })));
}
