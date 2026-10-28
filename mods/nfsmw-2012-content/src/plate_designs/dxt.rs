//! A small DXT1 (BC1) encoder for the plate design pictures: the colour range of each 4 x 4 block is the box of its colours,
//! pulled in a little at the ends, and each texel takes the nearest of the four colours between them. Good enough for the
//! smooth, photographic pictures of the designs; it is not a general encoder.

/// Bytes of one encoded block.
pub const BLOCK_BYTES: usize = 8;

/// A colour as RGB565.
fn pack565([r, g, b]: [u8; 3]) -> u16 {
    (u16::from(r >> 3) << 11) | (u16::from(g >> 2) << 5) | u16::from(b >> 3)
}

/// An RGB565 colour back to 8 bits a channel, as a decoder expands it.
fn unpack565(color: u16) -> [i32; 3] {
    let (r, g, b) = (i32::from(color >> 11) & 31, i32::from(color >> 5) & 63, i32::from(color) & 31);
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

/// The direction in which the colours of `texels` vary most (a unit vector), by a few rounds of power iteration on their
/// covariance. A block of one colour has no direction: the grey diagonal.
fn principal_axis(texels: &[[u8; 4]; 16], mean: [f32; 3]) -> [f32; 3] {
    let mut covariance = [[0.0f32; 3]; 3];
    for texel in texels {
        let d = [0, 1, 2].map(|k| f32::from(texel[k]) - mean[k]);
        for (row, line) in covariance.iter_mut().enumerate() {
            for (column, cell) in line.iter_mut().enumerate() {
                *cell += d[row] * d[column];
            }
        }
    }
    let mut axis = [1.0f32, 1.0, 1.0];
    for _ in 0..4 {
        let next = [0, 1, 2].map(|row| (0..3).map(|column| covariance[row][column] * axis[column]).sum::<f32>());
        let length = next.iter().map(|v| v * v).sum::<f32>().sqrt();
        if length < 1e-3 {
            break;
        }
        axis = next.map(|v| v / length);
    }
    let length = axis.iter().map(|v| v * v).sum::<f32>().sqrt();
    axis.map(|v| v / length)
}

/// The block for 16 texels (RGBA, row by row, the alpha ignored).
pub fn encode_block(texels: &[[u8; 4]; 16]) -> [u8; BLOCK_BYTES] {
    let mean = [0, 1, 2].map(|k| texels.iter().map(|texel| f32::from(texel[k])).sum::<f32>() / 16.0);
    let axis = principal_axis(texels, mean);
    // Each texel's place along the axis; the ends of the block are the colours at the extremes, pulled in by a sixteenth of the
    // span: the extreme texels are rare, the ones between them are not.
    let places: Vec<f32> =
        texels.iter().map(|texel| (0..3).map(|k| (f32::from(texel[k]) - mean[k]) * axis[k]).sum()).collect();
    let low = places.iter().copied().fold(f32::MAX, f32::min);
    let high = places.iter().copied().fold(f32::MIN, f32::max);
    let inset = (high - low) / 16.0;
    let at = |place: f32| -> [u8; 3] { [0, 1, 2].map(|k| (mean[k] + axis[k] * place).round().clamp(0.0, 255.0) as u8) };
    let (mut end0, mut end1) = (pack565(at(high - inset)), pack565(at(low + inset)));
    let mut block = [0u8; BLOCK_BYTES];
    if end0 == end1 {
        // One colour: every texel takes the first.
        block[..2].copy_from_slice(&end0.to_le_bytes());
        block[2..4].copy_from_slice(&end1.to_le_bytes());
        return block;
    }
    if end0 < end1 {
        std::mem::swap(&mut end0, &mut end1);
    }
    // end0 > end1: four colours, the two ends and the thirds between them.
    let (c0, c1) = (unpack565(end0), unpack565(end1));
    let palette = [c0, c1, [0, 1, 2].map(|k| (2 * c0[k] + c1[k]) / 3), [0, 1, 2].map(|k| (c0[k] + 2 * c1[k]) / 3)];
    block[..2].copy_from_slice(&end0.to_le_bytes());
    block[2..4].copy_from_slice(&end1.to_le_bytes());
    for (place, texel) in texels.iter().enumerate() {
        let distance = |colour: &[i32; 3]| -> i32 { (0..3).map(|k| (i32::from(texel[k]) - colour[k]).pow(2)).sum() };
        let best = (0..4).min_by_key(|&index| distance(&palette[index])).unwrap_or(0);
        block[4 + place / 4] |= (best as u8) << (2 * (place % 4));
    }
    block
}

/// The DXT1 data of an RGBA image (`width` and `height` multiples of 4), blocks row by row.
pub fn encode(rgba: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(width / 4 * (height / 4) * BLOCK_BYTES);
    for block_y in 0..height / 4 {
        for block_x in 0..width / 4 {
            let mut texels = [[0u8; 4]; 16];
            for (at, texel) in texels.iter_mut().enumerate() {
                let (x, y) = (block_x * 4 + at % 4, block_y * 4 + at / 4);
                let from = (y * width + x) * 4;
                texel.copy_from_slice(&rgba[from..from + 4]);
            }
            data.extend(encode_block(&texels));
        }
    }
    data
}

#[cfg(test)]
#[path = "dxt_tests.rs"]
mod tests;
