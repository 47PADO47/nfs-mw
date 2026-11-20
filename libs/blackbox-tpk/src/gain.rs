//! Scaling the colour of a picture's pixels without decoding it: a block-compressed picture keeps its colour in two RGB565
//! end points per block and a table of indices, so multiplying both end points by a gain multiplies every pixel of the block by
//! it (the blend between them is linear). The alpha of DXT3 and DXT5 is untouched. Used to bring pictures authored for a
//! brighter lighting model (the 2012 game's, RIDE 6's) to the one the world shader of this game has: it multiplies the picture by
//! 2 times the light, so a picture that is already near white saturates.

use crate::PixelFormat;

/// The end points of a colour block as (red, green, blue) of 5, 6 and 5 bits.
fn unpack(value: u16) -> [u32; 3] {
    [u32::from(value >> 11), u32::from((value >> 5) & 0x3F), u32::from(value & 0x1F)]
}

fn pack([r, g, b]: [u32; 3]) -> u16 {
    ((r as u16) << 11) | ((g as u16) << 5) | b as u16
}

/// An end point with each channel multiplied by `percent / 100` (rounded, never above its own maximum).
fn scaled(value: u16, percent: u32) -> u16 {
    let [r, g, b] = unpack(value);
    let scale = |channel: u32, max: u32| ((channel * percent + 50) / 100).min(max);
    pack([scale(r, 31), scale(g, 63), scale(b, 31)])
}

/// Swaps the end points' roles in the indices: in a four colour block the colours 0 and 1 and 2 and 3 trade places, in a three
/// colour block 0 and 1 do (the middle colour and the transparent one stay).
fn swap_indices(indices: &mut [u8], four_colours: bool) {
    for byte in indices {
        let mut out = 0u8;
        for pixel in 0..4 {
            let index = (*byte >> (2 * pixel)) & 3;
            let swapped = match (four_colours, index) {
                (true, i) => i ^ 1,
                (false, 0) => 1,
                (false, 1) => 0,
                (false, i) => i,
            };
            out |= swapped << (2 * pixel);
        }
        *byte = out;
    }
}

/// Multiplies the colour block `block` (8 bytes: two end points and 4 bytes of indices) by `percent / 100`, keeping its mode: a
/// block whose first end point is above the second has four colours (DXT1 reads the other mode as three colours and a
/// transparent one), and the scaled end points must still say so. When scaling changes the order of the two (a channel that
/// dominates the packed value stops dominating) the end points are swapped and the indices follow.
pub fn scale_colour_block(block: &mut [u8], percent: u32) {
    let first = u16::from_le_bytes([block[0], block[1]]);
    let second = u16::from_le_bytes([block[2], block[3]]);
    let four_colours = first > second;
    let (mut a, mut b) = (scaled(first, percent), scaled(second, percent));
    let mut swap = false;
    if four_colours && a < b {
        swap = true;
        (a, b) = (b, a);
    }
    if four_colours && a == b {
        // Two different colours became one: any index gives it, so only the mode has to be kept.
        match a {
            0 => a = 1,
            _ => b = a - 1,
        }
    }
    if !four_colours && a > b {
        swap = true;
        (a, b) = (b, a);
    }
    if swap {
        swap_indices(&mut block[4..8], four_colours);
    }
    block[0..2].copy_from_slice(&a.to_le_bytes());
    block[2..4].copy_from_slice(&b.to_le_bytes());
}

/// Multiplies the colour of every block of `data` (a DXT1, DXT3 or DXT5 picture, all its levels) by `percent / 100`. Other
/// formats and a gain of 100 leave the data as it is; returns whether anything was changed.
pub fn scale_dxt_colour(format: PixelFormat, data: &mut [u8], percent: u32) -> bool {
    if percent == 100 {
        return false;
    }
    let (block, colour_at) = match format {
        PixelFormat::Dxt1 => (8, 0),
        PixelFormat::Dxt3 | PixelFormat::Dxt5 => (16, 8),
        _ => return false,
    };
    for chunk in data.chunks_exact_mut(block) {
        scale_colour_block(&mut chunk[colour_at..colour_at + 8], percent);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb565(r: u32, g: u32, b: u32) -> u16 {
        pack([r, g, b])
    }

    fn block(first: u16, second: u16) -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0..2].copy_from_slice(&first.to_le_bytes());
        out[2..4].copy_from_slice(&second.to_le_bytes());
        out[4..].copy_from_slice(&[0x1B, 0xE4, 0x55, 0xAA]);
        out
    }

    fn ends(block: &[u8; 8]) -> (u16, u16) {
        (u16::from_le_bytes([block[0], block[1]]), u16::from_le_bytes([block[2], block[3]]))
    }

    #[test]
    fn both_end_points_are_scaled_and_the_indices_stay() {
        let mut b = block(rgb565(30, 60, 30), rgb565(10, 20, 10));
        scale_colour_block(&mut b, 50);
        assert_eq!(ends(&b), (rgb565(15, 30, 15), rgb565(5, 10, 5)));
        assert_eq!(b[4..], [0x1B, 0xE4, 0x55, 0xAA]);
    }

    #[test]
    fn a_four_colour_block_stays_one_and_a_three_colour_block_stays_one() {
        // Neighbouring end points collapse when scaled: the order is kept.
        let mut four = block(rgb565(1, 1, 1), rgb565(0, 1, 1));
        scale_colour_block(&mut four, 40);
        let (a, b) = ends(&four);
        assert!(a > b, "{a} {b}");
        let mut three = block(rgb565(2, 4, 2), rgb565(20, 40, 20));
        scale_colour_block(&mut three, 70);
        let (a, b) = ends(&three);
        assert!(a <= b, "{a} {b}");
        let mut black = block(rgb565(1, 0, 0), rgb565(0, 0, 0));
        scale_colour_block(&mut black, 10);
        let (a, b) = ends(&black);
        assert!(a > b, "{a} {b}");
    }

    #[test]
    fn end_points_that_change_order_are_swapped_and_the_indices_follow() {
        // A red that is above a green as a packed value but under it once scaled: colour 0 and 1 trade places, 2 and 3 too.
        let mut b = block(rgb565(1, 0, 0), rgb565(0, 63, 0));
        let green = scaled(rgb565(0, 63, 0), 40);
        scale_colour_block(&mut b, 40);
        assert_eq!(ends(&b), (green, 0));
        // Indices 0x1B (pixels 3, 2, 1, 0 of one row) were 0b00_01_10_11: now each has its low bit flipped.
        assert_eq!(b[4], 0x1B ^ 0x55);
        let mut three = block(rgb565(0, 63, 0), rgb565(1, 0, 0));
        scale_colour_block(&mut three, 40);
        let (a, c) = ends(&three);
        assert!(a <= c);
    }

    #[test]
    fn a_gain_of_one_hundred_changes_nothing_and_only_dxt_is_scaled() {
        let mut data = block(rgb565(30, 60, 30), rgb565(10, 20, 10)).to_vec();
        let before = data.clone();
        assert!(!scale_dxt_colour(PixelFormat::Dxt1, &mut data, 100));
        assert!(!scale_dxt_colour(PixelFormat::Argb8888, &mut data, 50));
        assert_eq!(data, before);
        assert!(scale_dxt_colour(PixelFormat::Dxt1, &mut data, 50));
        assert_ne!(data, before);
    }

    #[test]
    fn dxt5_keeps_its_alpha_block() {
        let mut data = vec![7u8; 8];
        data.extend(block(rgb565(30, 60, 30), rgb565(10, 20, 10)));
        scale_dxt_colour(PixelFormat::Dxt5, &mut data, 50);
        assert_eq!(data[..8], [7; 8]);
        assert_eq!(ends(&data[8..16].try_into().unwrap()), (rgb565(15, 30, 15), rgb565(5, 10, 5)));
    }
}
