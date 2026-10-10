//! A tiny BC1 (DXT1) codec for the testkit's procedural textures.
//!
//! The encoder is a plain range fit (the two extreme colours of a block, then the nearest of four palette
//! entries per texel): fast, deterministic and good enough for synthetic art. The decoder exists so a
//! backend without BC support can be given the same pixels as RGBA8.

/// Bytes of one 4x4 block.
pub const BLOCK_BYTES: usize = 8;

fn to_565(rgb: [u8; 3]) -> u16 {
    (u16::from(rgb[0] >> 3) << 11) | (u16::from(rgb[1] >> 2) << 5) | u16::from(rgb[2] >> 3)
}

fn from_565(value: u16) -> [u8; 3] {
    let (r, g, b) = ((value >> 11) as u8 & 31, (value >> 5) as u8 & 63, value as u8 & 31);
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

fn luma(rgb: [u8; 3]) -> u32 {
    u32::from(rgb[0]) * 77 + u32::from(rgb[1]) * 150 + u32::from(rgb[2]) * 29
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u32 {
    (0..3).map(|i| (i32::from(a[i]) - i32::from(b[i])).pow(2) as u32).sum()
}

/// The four-colour palette of a block with `c0 > c1`.
fn palette(c0: u16, c1: u16) -> [[u8; 3]; 4] {
    let (a, b) = (from_565(c0), from_565(c1));
    let mix = |wa: u32, wb: u32| [0, 1, 2].map(|i| ((wa * u32::from(a[i]) + wb * u32::from(b[i])) / 3) as u8);
    [a, b, mix(2, 1), mix(1, 2)]
}

/// Encode one 4x4 block of RGBA texels (row-major, alpha ignored).
pub fn encode_block(texels: &[[u8; 4]; 16]) -> [u8; BLOCK_BYTES] {
    let rgb = texels.map(|t| [t[0], t[1], t[2]]);
    let brightest = rgb.iter().copied().max_by_key(|&c| luma(c)).unwrap_or_default();
    let darkest = rgb.iter().copied().min_by_key(|&c| luma(c)).unwrap_or_default();
    let (mut c0, mut c1) = (to_565(brightest), to_565(darkest));
    let mut indices = 0u32;
    if c0 != c1 {
        if c0 < c1 {
            std::mem::swap(&mut c0, &mut c1);
        }
        let colours = palette(c0, c1);
        for (i, texel) in rgb.iter().enumerate() {
            let best = (0..4u32).min_by_key(|&k| distance(*texel, colours[k as usize])).unwrap_or(0);
            indices |= best << (2 * i);
        }
    }
    let mut block = [0u8; BLOCK_BYTES];
    block[0..2].copy_from_slice(&c0.to_le_bytes());
    block[2..4].copy_from_slice(&c1.to_le_bytes());
    block[4..8].copy_from_slice(&indices.to_le_bytes());
    block
}

/// Decode one block to 16 RGBA texels. A block with `c0 <= c1` has three colours and a transparent fourth.
pub fn decode_block(block: &[u8]) -> [[u8; 4]; 16] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    let (a, b) = (from_565(c0), from_565(c1));
    let colours: [[u8; 4]; 4] = match c0 > c1 {
        true => palette(c0, c1).map(|c| [c[0], c[1], c[2], 255]),
        false => {
            let half = [0, 1, 2].map(|i| ((u16::from(a[i]) + u16::from(b[i])) / 2) as u8);
            [[a[0], a[1], a[2], 255], [b[0], b[1], b[2], 255], [half[0], half[1], half[2], 255], [0; 4]]
        }
    };
    std::array::from_fn(|i| colours[((indices >> (2 * i)) & 3) as usize])
}

/// Encode a `width` x `height` RGBA8 image (both multiples of 4) to BC1.
pub fn encode(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    assert!(width.is_multiple_of(4) && height.is_multiple_of(4), "BC1 needs whole blocks");
    assert_eq!(rgba.len(), (width * height * 4) as usize);
    let mut out = Vec::with_capacity((width / 4 * height / 4) as usize * BLOCK_BYTES);
    for by in 0..height / 4 {
        for bx in 0..width / 4 {
            let texels = std::array::from_fn(|i| {
                let (x, y) = (bx * 4 + (i as u32 % 4), by * 4 + (i as u32 / 4));
                let at = ((y * width + x) * 4) as usize;
                [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
            });
            out.extend_from_slice(&encode_block(&texels));
        }
    }
    out
}

/// Decode BC1 data of a `width` x `height` image (both multiples of 4) to RGBA8.
pub fn decode(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    assert!(width.is_multiple_of(4) && height.is_multiple_of(4), "BC1 needs whole blocks");
    let blocks_per_row = (width / 4) as usize;
    assert_eq!(data.len(), blocks_per_row * (height / 4) as usize * BLOCK_BYTES);
    let mut out = vec![0u8; (width * height * 4) as usize];
    for (n, block) in data.as_chunks::<BLOCK_BYTES>().0.iter().enumerate() {
        let (bx, by) = ((n % blocks_per_row) as u32, (n / blocks_per_row) as u32);
        for (i, texel) in decode_block(block).iter().enumerate() {
            let (x, y) = (bx * 4 + (i as u32 % 4), by * 4 + (i as u32 / 4));
            let at = ((y * width + x) * 4) as usize;
            out[at..at + 4].copy_from_slice(texel);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_block_round_trips_to_its_565_colour() {
        let block = encode_block(&[[200, 100, 50, 255]; 16]);
        let texels = decode_block(&block);
        let expected = from_565(to_565([200, 100, 50]));
        assert!(texels.iter().all(|t| t[..3] == expected && t[3] == 255));
    }

    #[test]
    fn two_colour_blocks_keep_both_colours() {
        let mut texels = [[255, 255, 255, 255]; 16];
        for t in texels.iter_mut().step_by(2) {
            *t = [0, 0, 0, 255];
        }
        let decoded = decode_block(&encode_block(&texels));
        for (a, b) in texels.iter().zip(decoded) {
            assert_eq!(a[..3], b[..3]);
        }
    }

    #[test]
    fn a_gradient_along_one_axis_stays_close_to_the_source() {
        let (w, h) = (16u32, 8u32);
        let rgba: Vec<u8> = (0..w * h).flat_map(|i| [(i % w * 16) as u8, 60, 90, 255]).collect();
        let back = decode(&encode(&rgba, w, h), w, h);
        let worst = rgba.iter().zip(&back).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
        assert!(worst <= 24, "range fit error {worst}");
        assert_eq!(back.len(), rgba.len());
    }

    #[test]
    fn the_data_size_is_eight_bytes_per_block() {
        assert_eq!(encode(&[255; 8 * 12 * 4], 8, 12).len(), 2 * 3 * BLOCK_BYTES);
    }

    #[test]
    fn a_three_colour_block_has_a_transparent_fourth_entry() {
        let mut block = [0u8; 8];
        block[0..2].copy_from_slice(&0u16.to_le_bytes());
        block[2..4].copy_from_slice(&0xFFFFu16.to_le_bytes());
        block[4..8].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        assert!(decode_block(&block).iter().all(|t| *t == [0, 0, 0, 0]));
    }
}
