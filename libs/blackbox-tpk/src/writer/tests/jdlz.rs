//! The JDLZ encoder against the decoder in `ea-compress`.

use super::bytes;
use crate::writer::jdlz::compress;

fn assert_round_trip(input: &[u8]) -> usize {
    let blob = compress(input);
    assert_eq!(&blob[0..4], b"JDLZ");
    assert_eq!(u32::from_le_bytes(blob[12..16].try_into().unwrap()) as usize, blob.len());
    let out = ea_compress::unwrap(&blob).unwrap();
    assert_eq!(&*out, input);
    blob.len()
}

#[test]
fn empty_and_tiny_inputs() {
    assert_round_trip(&[]);
    assert_round_trip(&[7]);
    assert_round_trip(&[1, 2]);
    assert_round_trip(&[5, 5, 5]);
    assert_round_trip(&[1, 2, 3, 1, 2, 3]);
}

#[test]
fn runs_use_overlapping_short_copies() {
    let size = assert_round_trip(&vec![0xAB; 100_000]);
    assert!(size < 200, "{size}");
    assert_round_trip(&[vec![1; 5000], vec![2; 3], vec![1; 70]].concat());
}

#[test]
fn incompressible_data_grows_only_by_the_flag_bytes() {
    // A xorshift stream has no repeats worth a copy.
    let mut x = 0x2545_F491_4F6C_DD1Du64;
    let noise: Vec<u8> = (0..20_000)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect();
    let size = assert_round_trip(&noise);
    assert!(size <= 16 + noise.len() + noise.len() / 8 + 8, "{size}");
}

#[test]
fn long_distance_copies() {
    let block = bytes(3, 600);
    let input = [block.clone(), bytes(9, 1500), block.clone(), bytes(1, 40), block].concat();
    let size = assert_round_trip(&input);
    assert!(size < input.len(), "{size}");
}

#[test]
fn copies_beyond_the_window_fall_back_to_literals() {
    let block = bytes(5, 64);
    let gap = bytes(200, 5000);
    assert_round_trip(&[block.clone(), gap, block].concat());
}

#[test]
fn mixed_texture_like_data() {
    // Block-compressed-looking data: repeated 8-byte blocks with small variations.
    let mut input = Vec::new();
    for i in 0..4000usize {
        let block = [0x10, 0x84, 0xEF, 0x7B, (i % 7) as u8, (i % 3) as u8, 0xAA, 0x55];
        input.extend_from_slice(&block);
    }
    let size = assert_round_trip(&input);
    assert!(size < input.len() / 2, "{size}");
}
