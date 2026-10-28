use super::*;

/// The 16 texels of a block as a decoder reads them (RGB, the four-colour mode).
fn decode_block(block: &[u8]) -> [[i32; 3]; 16] {
    let end0 = u16::from_le_bytes([block[0], block[1]]);
    let end1 = u16::from_le_bytes([block[2], block[3]]);
    let (c0, c1) = (unpack565(end0), unpack565(end1));
    let palette = [c0, c1, [0, 1, 2].map(|k| (2 * c0[k] + c1[k]) / 3), [0, 1, 2].map(|k| (c0[k] + 2 * c1[k]) / 3)];
    let mut out = [[0; 3]; 16];
    for (at, texel) in out.iter_mut().enumerate() {
        *texel = palette[usize::from(block[4 + at / 4] >> (2 * (at % 4))) & 3];
    }
    out
}

fn flat(color: [u8; 3]) -> [[u8; 4]; 16] {
    [[color[0], color[1], color[2], 255]; 16]
}

#[test]
fn a_flat_block_decodes_to_its_colour_within_the_565_rounding() {
    for color in [[255, 0, 0], [12, 200, 90], [128, 128, 128], [0, 0, 0], [255, 255, 255]] {
        let decoded = decode_block(&encode_block(&flat(color)));
        for texel in decoded {
            for channel in 0..3 {
                assert!((texel[channel] - i32::from(color[channel])).abs() <= 8, "{color:?} became {texel:?}");
            }
        }
    }
}

#[test]
fn a_flat_block_has_equal_ends_and_all_indices_zero() {
    let block = encode_block(&flat([40, 80, 120]));
    assert_eq!(block[..2], block[2..4]);
    assert_eq!(block[4..], [0, 0, 0, 0]);
}

#[test]
fn a_gradient_block_stays_close_to_the_source() {
    let mut texels = [[0u8; 4]; 16];
    for (at, texel) in texels.iter_mut().enumerate() {
        let value = ((at % 4) * 85) as u8; // 0, 85, 170, 255 across each row
        *texel = [value, value / 2, 255 - value, 255];
    }
    let decoded = decode_block(&encode_block(&texels));
    for (texel, found) in texels.iter().zip(decoded) {
        for channel in 0..3 {
            assert!((found[channel] - i32::from(texel[channel])).abs() <= 32, "{texel:?} became {found:?}");
        }
    }
}

#[test]
fn the_first_end_is_the_larger_so_a_decoder_reads_four_colours() {
    let mut texels = flat([0, 0, 0]);
    texels[5] = [255, 255, 255, 255];
    let block = encode_block(&texels);
    assert!(u16::from_le_bytes([block[0], block[1]]) > u16::from_le_bytes([block[2], block[3]]));
}

#[test]
fn an_image_is_a_row_of_blocks_per_four_rows() {
    // 8 x 8: the left half red, the right half blue.
    let mut rgba = Vec::new();
    for _ in 0..8 {
        for x in 0..8 {
            rgba.extend(if x < 4 { [255, 0, 0, 255] } else { [0, 0, 255, 255] });
        }
    }
    let data = encode(&rgba, 8, 8);
    assert_eq!(data.len(), 4 * BLOCK_BYTES);
    let colour_of = |block: usize| decode_block(&data[block * BLOCK_BYTES..(block + 1) * BLOCK_BYTES])[0];
    assert!(colour_of(0)[0] > 200 && colour_of(0)[2] < 50, "top left is red");
    assert!(colour_of(1)[2] > 200 && colour_of(1)[0] < 50, "top right is blue");
    assert_eq!(colour_of(2), colour_of(0), "the second row of blocks repeats the first");
}
