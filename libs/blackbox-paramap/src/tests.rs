use super::*;

fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_le_bytes().to_vec();
    bytes.extend((payload.len() as u32).to_le_bytes());
    bytes.extend(payload);
    bytes
}

fn fixture(width: usize, bounds: [f32; 4], nodes: &[[u16; 4]]) -> Vec<u8> {
    let mut header = 123u32.to_le_bytes().to_vec();
    for bound in bounds {
        header.extend(bound.to_le_bytes());
    }
    for count in [nodes.len() as i32, 4, 8, 2] {
        header.extend(count.to_le_bytes());
    }
    let mut body = chunk(0x3B602, &header);
    body.extend(chunk(0x3B603, &[1i32.to_le_bytes(), 0i32.to_le_bytes()].concat()));
    body.extend(chunk(0x3B604, &[0i32.to_le_bytes(), 4i32.to_le_bytes()].concat()));
    let mut data = Vec::new();
    for i in 0i32..4 {
        data.extend((i * 10).to_le_bytes());
        data.extend((i as f32 + 0.5).to_le_bytes());
    }
    body.extend(chunk(0x3B605, &data));
    let mut quad = Vec::new();
    for node in nodes {
        for &word in node {
            if width == 1 {
                quad.push(word as u8);
                continue;
            }
            quad.extend(word.to_le_bytes());
        }
    }
    body.extend(chunk(if width == 1 { 0x3B607 } else { 0x3B608 }, &quad));
    chunk(0x8003_B600, &chunk(0x8003_B601, &body))
}

const NODES: [[u16; 4]; 5] = [[1, 2, 3, 4], [0, 0, 0, 0], [0, 1, 0, 0], [0, 2, 0, 0], [0, 3, 0, 0]];

#[test]
fn both_widths_match_quadrants_midpoints_and_typed_fields() {
    for width in [1, 2] {
        let layers = Layer::read_all(&fixture(width, [-2.0, -2.0, 2.0, 2.0], &NODES)).unwrap();
        let layer = &layers[0];
        assert_eq!(layer.node_count(), 5);
        for (x, y, expected) in
            [(-1.0, -1.0, 0), (-1.0, 1.0, 1), (1.0, -1.0, 2), (1.0, 1.0, 3), (0.0, 0.0, 3), (100.0, 100.0, 3)]
        {
            assert_eq!(layer.set_at(x, y), Some(expected));
            assert_eq!(layer.integer(expected, 0), Some(expected as i32 * 10));
            assert_eq!(layer.float(expected, 1), Some(expected as f32 + 0.5));
        }
        assert_eq!(layer.float(0, 0), None);
        assert_eq!(layer.integer(0, 1), None);
        assert_eq!(layer.integer(4, 0), None);
        assert_eq!(layer.set_at(f32::NAN, 0.0), None);
    }
}

#[test]
fn reversed_bounds_preserve_original_conjunction_and_no_nodes_select_set_zero() {
    let layer = Layer::read_all(&fixture(1, [2.0, 2.0, -2.0, -2.0], &NODES)).unwrap().remove(0);
    assert_eq!(layer.set_at(0.0, 0.0), Some(0));
    let constant = Layer::read_all(&fixture(2, [0.0; 4], &[])).unwrap().remove(0);
    assert_eq!(constant.set_at(100.0, -100.0), Some(0));
}

#[test]
fn invalid_children_sets_and_cycles_are_rejected_before_query() {
    for nodes in [vec![[1, 1, 1, 1]], vec![[0, 4, 0, 0]], vec![[1, 1, 1, 1], [1, 1, 1, 1]]] {
        assert!(Layer::read_all(&fixture(2, [-2.0, -2.0, 2.0, 2.0], &nodes)).is_err());
    }
    let mut bad = fixture(1, [-2.0, -2.0, 2.0, 2.0], &NODES);
    bad.pop();
    assert!(Layer::read_all(&bad).is_err());
}
