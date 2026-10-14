use super::*;

fn node(opcode: u8, output_selector: u8, parameters: [i16; 3]) -> Node {
    Node { opcode, output_selector, parameters }
}

fn patch(nodes: Vec<Node>, connections: Vec<(usize, usize, u8)>, channels: usize) -> Patch {
    let mut layouts: [Layout; 6] = std::array::from_fn(|_| Layout::default());
    layouts[channels - 1] = Layout {
        nodes,
        connections: connections
            .into_iter()
            .map(|(source, destination, destination_port)| Connection {
                source,
                destination,
                source_port: 1,
                destination_port,
            })
            .collect(),
    };
    Patch { layouts }
}

fn graph() -> Patch {
    patch(
        vec![node(0, 0, [0; 3]), node(9, 0, [0; 3]), node(6, 1, [128, 0, 0]), node(6, 2, [64, 0, 0])],
        vec![(0, 1, 1), (1, 2, 1), (1, 3, 1)],
        2,
    )
}

#[test]
fn cached_shared_input_reaches_both_speakers_once_and_advances_next_block() {
    let mut processor = Processor::new(&graph(), 48000, 2).unwrap();
    let mut left = [0.0; 16];
    let mut right = [0.0; 16];
    processor.process_mono(&[1.0; 16], &mut left, &mut right).unwrap();
    assert_eq!(left, [0.5; 16]);
    assert_eq!(right, [0.25; 16]);
    processor.process_mono(&[0.0; 16], &mut left, &mut right).unwrap();
    assert_eq!(left, [0.0; 16]);
    assert_eq!(right, [0.0; 16]);
}

#[test]
fn native_delay_lengths_are_prime_multiples_of_sixteen_and_reset_erases_tail() {
    assert_eq!(math::delay_length(44100, 50).unwrap(), 2192);
    assert_eq!(math::delay_length(48000, 50).unwrap(), 2384);
    let patch = patch(vec![node(0, 0, [0; 3]), node(7, 1, [0, 50, 0])], vec![(0, 1, 1)], 1);
    let mut processor = Processor::new(&patch, 48000, 2).unwrap();
    assert_eq!(processor.layout_channels(), 1);
    let mut values = Vec::new();
    for block in 0..150 {
        let mut input = [0.0; 16];
        if block == 0 {
            input[0] = 1.0;
        }
        let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
        processor.process_mono(&input, &mut left, &mut right).unwrap();
        assert_eq!(left, right);
        values.extend(left);
    }
    assert_eq!(values.iter().position(|sample| sample.abs() > 0.5), Some(2384));
    processor.reset();
    let (mut left, mut right) = ([0.0; 16], [0.0; 16]);
    processor.process_mono(&[0.0; 16], &mut left, &mut right).unwrap();
    assert!(left.iter().all(|sample| sample.abs() < 1e-10));
}

#[test]
fn malformed_graphs_and_unsupported_processors_fail_explicitly() {
    let cycle = patch(vec![node(6, 1, [256, 0, 0]), node(6, 0, [256, 0, 0])], vec![(0, 1, 1), (1, 0, 1)], 1);
    assert!(Processor::new(&cycle, 48000, 1).is_err());
    let unsupported = patch(vec![node(0, 0, [0; 3]), node(2, 1, [1000, 0, 0])], vec![(0, 1, 1)], 1);
    assert!(matches!(Processor::new(&unsupported, 48000, 1), Err(Error::Unsupported(2))));
    assert!(Processor::new(&graph(), 48000, 6).is_err());
    let mut processor = Processor::new(&graph(), 48000, 2).unwrap();
    assert!(processor.process_mono(&[0.0; 15], &mut [0.0; 15], &mut [0.0; 15]).is_err());
}

#[test]
fn reader_rejects_truncation_version_and_oversized_counts() {
    let mut bytes = b"FX\x0b\0".to_vec();
    bytes.extend(56i32.to_be_bytes());
    bytes.extend([0; 48]);
    let parsed = Patch::parse(&bytes).unwrap();
    assert!(parsed.layouts.iter().all(|layout| layout.nodes.is_empty()));
    let mut wrong = bytes.clone();
    wrong[2] = 12;
    assert!(Patch::parse(&wrong).is_err());
    bytes[8..12].copy_from_slice(&i32::MAX.to_be_bytes());
    assert!(Patch::parse(&bytes).is_err());
    assert!(Patch::parse(b"FX").is_err());
}
