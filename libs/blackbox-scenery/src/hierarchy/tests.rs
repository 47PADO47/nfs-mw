use super::*;

const TREE: u32 = 0x8000_0040;
const RECORD: u32 = 0x41;

fn record(nodes: &[[u8; 16]]) -> Vec<u8> {
    let mut payload = vec![0; 8];
    payload[0..4].copy_from_slice(&77_u32.to_le_bytes());
    payload[4] = nodes.len() as u8;
    for node in nodes {
        payload.extend(node);
    }
    let mut child = RECORD.to_le_bytes().to_vec();
    child.extend((payload.len() as u32).to_le_bytes());
    child.extend(payload);
    let mut tree = TREE.to_le_bytes().to_vec();
    tree.extend((child.len() as u32).to_le_bytes());
    tree.extend(child);
    tree
}

fn node(name: u32, model: u32, parent: u8, first: u8, count: u8, internal: bool) -> [u8; 16] {
    let mut n = [0; 16];
    n[0..4].copy_from_slice(&name.to_le_bytes());
    n[4..8].copy_from_slice(&model.to_le_bytes());
    n[8..12].copy_from_slice(&u32::MAX.to_le_bytes()); // Runtime pointer is deliberately unusable.
    n[12] = u8::from(internal);
    n[13] = parent;
    n[14] = count;
    n[15] = first;
    n
}

#[test]
fn internal_descendants_are_reserved_but_a_selected_internal_part_can_draw() {
    let bytes = record(&[
        node(1, 11, 0, 1, 2, false),
        node(2, 12, 0, 0, 0, false),
        node(3, 13, 0, 3, 1, true),
        node(4, 14, 2, 0, 0, false),
    ]);
    let hierarchies = read_model_hierarchies(&bytes, TREE, RECORD).unwrap();
    let hierarchy = &hierarchies[0];
    assert_eq!(hierarchy.name_hash, 77);
    assert_eq!(hierarchy.visible_nodes(0), [0, 1]);
    assert_eq!(hierarchy.visible_nodes(2), [2, 3]);
    assert_eq!(hierarchy.find(4), Some(3));
    assert_eq!(hierarchy.find(55), None);
}

#[test]
fn malformed_ranges_cycles_and_truncated_records_are_rejected() {
    for nodes in [
        vec![node(1, 1, 0, 1, 1, false)],
        vec![node(1, 1, 0, 0, 1, false)],
        vec![node(1, 1, 0, 1, 1, false), node(2, 2, 0, 0, 1, false)],
        vec![],
    ] {
        assert!(read_model_hierarchies(&record(&nodes), TREE, RECORD).is_err());
    }
    let mut bytes = record(&[node(1, 1, 0, 0, 0, false)]);
    bytes.truncate(bytes.len() - 1);
    let tree_len = bytes.len() as u32 - 8;
    let record_len = bytes.len() as u32 - 16;
    bytes[4..8].copy_from_slice(&tree_len.to_le_bytes());
    bytes[12..16].copy_from_slice(&record_len.to_le_bytes());
    assert!(read_model_hierarchies(&bytes, TREE, RECORD).is_err());
}
