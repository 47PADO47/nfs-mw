use super::*;
use crate::{bounds_flags as flags, find_bounds, read_bounds_sets};

const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// `v` rounded to the millimetre, as stored.
fn q(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

fn node(flags: u16, position: [f32; 3], half: [f32; 3]) -> Bounds {
    Bounds {
        orientation: IDENTITY,
        position,
        flags,
        half_dimensions: half,
        child_count: 0,
        point_cloud: None,
        pivot: [q(position[0] + 0.25), q(position[1] + 0.5), q(position[2] - 0.125)],
        first_child: -1,
        radius: half[0].max(half[1]).max(half[2]),
        surface: 0x99DA_6531,
        name_hash: 0,
    }
}

/// Every length is a whole number of millimetres, so reading the written bytes gives these exact values back.
fn sample(name_hash: u32) -> BoundsSet {
    let mut root = node(flags::BOX, [0.0; 3], [0.934, 0.621, 2.271]);
    root.child_count = 3;
    root.first_child = 1;
    let mut mesh = node(flags::MESH_VS_GROUND | flags::SPHERE, [0.002, 0.013, -0.032], [0.574, 0.879, 2.235]);
    mesh.point_cloud = Some(0);
    mesh.orientation = [0.0, 0.0, 0.0, -1.0];
    BoundsSet {
        name_hash,
        nodes: vec![
            root,
            node(flags::BOX | flags::PRIM_VS_WORLD, [-0.004, -0.244, -0.002], [0.934, 0.376, 2.275]),
            mesh,
            node(flags::SPHERE | flags::PRIM_VS_WORLD, [0.747, -0.444, 2.091], [0.328, 0.328, 0.328]),
        ],
        point_clouds: vec![vec![[-0.732, 0.064, -2.268], [0.723, 0.064, -2.268], [0.0, 0.5, 1.0]]],
    }
}

#[test]
fn a_written_set_reads_back_equal() {
    let set = sample(0x20D3_071F);
    let bytes = write_bounds_pack(std::slice::from_ref(&set)).unwrap();
    let read = read_bounds_sets(&bytes).unwrap();
    assert_eq!(read, vec![set]);
}

#[test]
fn writing_what_was_read_gives_the_same_bytes() {
    let mut second = sample(7);
    second.nodes.truncate(1);
    second.nodes[0].child_count = 0;
    second.nodes[0].first_child = -1;
    second.point_clouds.clear();
    let first = write_bounds_pack(&[sample(1), second]).unwrap();
    let again = write_bounds_pack(&read_bounds_sets(&first).unwrap()).unwrap();
    assert_eq!(first, again);
}

#[test]
fn a_pack_holds_several_sets_found_by_hash() {
    let bytes = write_bounds_pack(&[sample(10), sample(20), sample(30)]).unwrap();
    let sets = read_bounds_sets(&bytes).unwrap();
    assert_eq!(sets.len(), 3);
    assert_eq!(find_bounds(&sets, 20).map(|s| s.name_hash), Some(20));
    assert!(find_bounds(&sets, 21).is_none());
}

#[test]
fn the_payload_is_exactly_the_size_the_parser_walks() {
    // Header 0x10, 4 nodes of 0x30, cloud count 0x10, one descriptor 0x10 and 3 points of 0x10.
    let payload = sample(1).to_payload().unwrap();
    assert_eq!(payload.len(), 0x10 + 4 * 0x30 + 0x10 + 0x10 + 3 * 0x10);
    assert!(BoundsSet::parse(&payload).unwrap().is_tree());
}

#[test]
fn the_pack_payload_starts_on_a_16_byte_boundary() {
    let bytes = write_bounds_pack(&[sample(1), sample(2)]).unwrap();
    let pack = blackbox_chunk::find(&bytes, ids::BOUNDS_PACK).unwrap();
    let mut seen = 0;
    for child in pack.children().map(|c| c.unwrap()) {
        assert_eq!(child.id, ids::BOUNDS);
        let start = child.payload_offset() + (child.payload.len() - child.aligned_payload(16).len());
        assert_eq!(start % 16, 0);
        seen += 1;
    }
    assert_eq!(seen, 2);
}

#[test]
fn lengths_beyond_the_16_bit_millimetre_range_are_refused() {
    let mut set = sample(1);
    set.nodes[0].half_dimensions[2] = 40.0;
    assert!(set.to_payload().is_err());
    let mut set = sample(1);
    set.nodes[0].position[0] = f32::NAN;
    assert!(set.to_payload().is_err());
}

#[test]
fn a_set_the_parser_would_refuse_is_not_written() {
    let mut set = sample(1);
    set.nodes[0].first_child = 3;
    set.nodes[0].child_count = 2;
    assert!(set.to_payload().is_err(), "children past the end");
    let mut set = sample(1);
    set.nodes[2].point_cloud = Some(4);
    assert!(set.to_payload().is_err(), "no such cloud");
    let mut set = sample(1);
    set.nodes.clear();
    assert!(set.to_payload().is_err(), "no nodes");
    let mut set = sample(1);
    set.point_clouds[0][1][1] = f32::INFINITY;
    assert!(set.to_payload().is_err(), "a point that is not finite");
}

#[test]
fn an_orientation_is_stored_over_32767() {
    let mut set = sample(1);
    set.nodes[1].orientation = [0.707_1, -0.707_1, 0.0, 0.0];
    let read = BoundsSet::parse(&set.to_payload().unwrap()).unwrap();
    for (a, b) in read.nodes[1].orientation.iter().zip(set.nodes[1].orientation) {
        assert!((a - b).abs() < 1e-4, "{a} vs {b}");
    }
}
