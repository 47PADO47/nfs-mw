//! Building textures from RGBA8 pixels: mip chains and a small, deterministic DXT encoder.
//!
//! The encoder is a range fit: a block's endpoints are the extreme pixels along the principal axis of its colours
//! (found with a few power iterations), pulled in by a sixteenth of their distance, and each texel takes the nearest of
//! the four palette colours. It is not as exact as an exhaustive search; it is fast, needs no tables and gives the
//! same bytes on every machine.

use crate::{PixelFormat, TextureSource, mip_level_size};

/// One level of a mip chain: tightly packed RGBA8 rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The next smaller level: 2 by 2 blocks averaged (an odd edge repeats its last pixel). Sizes stop at 1.
pub fn halve(level: &Level) -> Level {
    let (width, height) = ((level.width / 2).max(1), (level.height / 2).max(1));
    let at = |x: u32, y: u32, channel: usize| {
        let (x, y) = (x.min(level.width - 1), y.min(level.height - 1));
        u32::from(level.rgba[((y * level.width + x) * 4) as usize + channel])
    };
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            for channel in 0..4 {
                let sum = at(2 * x, 2 * y, channel)
                    + at(2 * x + 1, 2 * y, channel)
                    + at(2 * x, 2 * y + 1, channel)
                    + at(2 * x + 1, 2 * y + 1, channel);
                rgba.push(((sum + 2) / 4) as u8);
            }
        }
    }
    Level { width, height, rgba }
}

/// The levels of a mip chain of `count` levels (at least 1), the picture first.
pub fn mip_chain(first: Level, count: u32) -> Vec<Level> {
    let mut levels = vec![first];
    while (levels.len() as u32) < count {
        let next = halve(levels.last().expect("the chain is never empty"));
        levels.push(next);
    }
    levels
}

/// The number of levels down to 1 by 1.
pub fn full_mip_count(width: u32, height: u32) -> u32 {
    width.max(height).max(1).ilog2() + 1
}

/// The largest difference between the texels of a block, in levels of one channel, at which the block is taken as flat.
const NEAR_FLAT_SPREAD: f32 = 8.0;

/// The 565 packing of an 8-bit colour, rounding to nearest.
fn pack565([r, g, b]: [u8; 3]) -> u16 {
    let q = |v: u8, bits: u32| {
        let max = (1u32 << bits) - 1;
        (u32::from(v) * max + 127) / 255
    };
    ((q(r, 5) << 11) | (q(g, 6) << 5) | q(b, 5)) as u16
}

/// The 8-bit colour a 565 value decodes to (the high bits repeat into the low ones).
fn unpack565(c: u16) -> [u8; 3] {
    let (r, g, b) = (u32::from(c >> 11) & 31, u32::from(c >> 5) & 63, u32::from(c) & 31);
    [((r << 3) | (r >> 2)) as u8, ((g << 2) | (g >> 4)) as u8, ((b << 3) | (b >> 2)) as u8]
}

/// The 16 texels of the block at (`bx`, `by`), edges clamped so a picture that is not a whole number of blocks pads
/// with its own border.
fn block(level: &Level, bx: u32, by: u32) -> [[u8; 4]; 16] {
    let mut out = [[0u8; 4]; 16];
    for (n, texel) in out.iter_mut().enumerate() {
        let x = (bx * 4 + (n as u32 % 4)).min(level.width - 1);
        let y = (by * 4 + (n as u32 / 4)).min(level.height - 1);
        let at = ((y * level.width + x) * 4) as usize;
        texel.copy_from_slice(&level.rgba[at..at + 4]);
    }
    out
}

/// The four colours a pair of 565 end points gives (the end points and the two thirds between them).
fn palette_of(c0: u16, c1: u16) -> [[i32; 3]; 4] {
    let (p0, p1) = (unpack565(c0), unpack565(c1));
    [
        p0.map(i32::from),
        p1.map(i32::from),
        std::array::from_fn(|c| (2 * i32::from(p0[c]) + i32::from(p1[c])) / 3),
        std::array::from_fn(|c| (i32::from(p0[c]) + 2 * i32::from(p1[c])) / 3),
    ]
}

/// The two 565 end points whose palette comes nearest to `target`, trying the 565 value below and above each channel of both
/// end points. A flat block rounded to the nearest 565 value alone is off by up to 4 levels in red and blue and 2 in green,
/// which turns a dark neutral grey green; the thirds of two end points reach it much closer.
fn nearest_end_points(target: [f32; 3]) -> (u16, u16) {
    let candidates = |value: f32, bits: u32| -> [u32; 2] {
        let max = ((1u32 << bits) - 1) as f32;
        let at = (value.clamp(0.0, 255.0) * max / 255.0).floor() as u32;
        [at, (at + 1).min(max as u32)]
    };
    let pack = |channels: [u32; 3]| ((channels[0] << 11) | (channels[1] << 5) | channels[2]) as u16;
    let ends: Vec<u16> = (0..8)
        .map(|n| {
            pack([
                candidates(target[0], 5)[n & 1],
                candidates(target[1], 6)[(n >> 1) & 1],
                candidates(target[2], 5)[(n >> 2) & 1],
            ])
        })
        .collect();
    let error = |entry: &[i32; 3]| (0..3).map(|c| (entry[c] as f32 - target[c]).powi(2)).sum::<f32>();
    let mut best = (f32::MAX, ends[0], ends[0]);
    for &a in &ends {
        for &b in &ends {
            let nearest = palette_of(a, b).iter().map(error).fold(f32::MAX, f32::min);
            if nearest < best.0 {
                best = (nearest, a, b);
            }
        }
    }
    (best.1, best.2)
}

/// Colour half of a block whose texels all lie within a few levels of their mean: end points chosen for the mean, every texel
/// on the palette entry nearest to it.
fn near_flat_block(texels: &[[u8; 4]; 16], mean: [f32; 3]) -> [u8; 8] {
    let (mut c0, mut c1) = nearest_end_points(mean);
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let palette = palette_of(c0, c1);
    let mut indices = 0u32;
    if c0 != c1 {
        for (n, t) in texels.iter().enumerate() {
            let distance = |p: &[i32; 3]| (0..3).map(|c| (p[c] - i32::from(t[c])).pow(2)).sum::<i32>();
            let best = (0..4).min_by_key(|&k| (distance(&palette[k]), k)).unwrap_or(0);
            indices |= (best as u32) << (2 * n);
        }
    }
    let mut out = [0u8; 8];
    out[..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    out[4..].copy_from_slice(&indices.to_le_bytes());
    out
}

/// The 8 bytes of a DXT1 colour block (also the colour half of a DXT5 block) for the colours of `texels`.
fn color_block(texels: &[[u8; 4]; 16]) -> [u8; 8] {
    let colors: Vec<[f32; 3]> = texels.iter().map(|t| [f32::from(t[0]), f32::from(t[1]), f32::from(t[2])]).collect();
    let mean = std::array::from_fn::<f32, 3, _>(|c| colors.iter().map(|p| p[c]).sum::<f32>() / 16.0);
    let spread = (0..3)
        .map(|c| {
            let high = colors.iter().map(|p| p[c]).fold(f32::MIN, f32::max);
            high - colors.iter().map(|p| p[c]).fold(f32::MAX, f32::min)
        })
        .fold(0.0, f32::max);
    if spread <= NEAR_FLAT_SPREAD {
        return near_flat_block(texels, mean);
    }
    let mut cov = [[0.0f32; 3]; 3];
    for p in &colors {
        let d = [p[0] - mean[0], p[1] - mean[1], p[2] - mean[2]];
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += d[i] * d[j];
            }
        }
    }
    // Power iteration. The start is the strongest column of the covariance: a start that happens to be at right angles to
    // the principal axis (the diagonal, for a red and blue block) would never find it. A flat block keeps the diagonal.
    let mut axis = [1.0f32, 1.0, 1.0];
    let strongest = (0..3)
        .map(|j| (j, cov[0][j] * cov[0][j] + cov[1][j] * cov[1][j] + cov[2][j] * cov[2][j]))
        .max_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((column, norm)) = strongest.filter(|(_, norm)| *norm > 1e-6) {
        let length = norm.sqrt();
        axis = std::array::from_fn(|i| cov[i][column] / length);
    }
    for _ in 0..6 {
        let next: [f32; 3] = std::array::from_fn(|i| cov[i][0] * axis[0] + cov[i][1] * axis[1] + cov[i][2] * axis[2]);
        let length = (next[0] * next[0] + next[1] * next[1] + next[2] * next[2]).sqrt();
        if length < 1e-6 {
            break;
        }
        axis = next.map(|v| v / length);
    }
    let along = |p: &[f32; 3]| (p[0] - mean[0]) * axis[0] + (p[1] - mean[1]) * axis[1] + (p[2] - mean[2]) * axis[2];
    let (mut low, mut high) = (0, 0);
    for (i, p) in colors.iter().enumerate() {
        if along(p) < along(&colors[low]) {
            low = i;
        }
        if along(p) > along(&colors[high]) {
            high = i;
        }
    }
    // Pull the end points in a sixteenth towards each other: the extremes of a block are usually outliers.
    let inset = |from: usize, to: usize| -> [u8; 3] {
        std::array::from_fn(|c| {
            (colors[from][c] + (colors[to][c] - colors[from][c]) / 16.0).round().clamp(0.0, 255.0) as u8
        })
    };
    let (mut c0, mut c1) = (pack565(inset(high, low)), pack565(inset(low, high)));
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let (p0, p1) = (unpack565(c0), unpack565(c1));
    let palette: [[i32; 3]; 4] = [
        p0.map(i32::from),
        p1.map(i32::from),
        std::array::from_fn(|c| (2 * i32::from(p0[c]) + i32::from(p1[c])) / 3),
        std::array::from_fn(|c| (i32::from(p0[c]) + 2 * i32::from(p1[c])) / 3),
    ];
    let mut indices = 0u32;
    if c0 != c1 {
        for (n, t) in texels.iter().enumerate() {
            let distance = |p: &[i32; 3]| (0..3).map(|c| (p[c] - i32::from(t[c])).pow(2)).sum::<i32>();
            let best = (0..4).min_by_key(|&k| (distance(&palette[k]), k)).unwrap_or(0);
            indices |= (best as u32) << (2 * n);
        }
    }
    let mut out = [0u8; 8];
    out[..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    out[4..].copy_from_slice(&indices.to_le_bytes());
    out
}

/// The 8 bytes of a DXT5 alpha block: two end points and sixteen 3-bit indices.
fn alpha_block(texels: &[[u8; 4]; 16]) -> [u8; 8] {
    let high = texels.iter().map(|t| t[3]).max().unwrap_or(255);
    let low = texels.iter().map(|t| t[3]).min().unwrap_or(0);
    let mut out = [0u8; 8];
    out[0] = high;
    out[1] = low;
    if high == low {
        return out;
    }
    // high > low: the eight-value mode, six values interpolated between the end points.
    let (h, l) = (i32::from(high), i32::from(low));
    let palette: [i32; 8] = [
        h,
        l,
        (6 * h + l) / 7,
        (5 * h + 2 * l) / 7,
        (4 * h + 3 * l) / 7,
        (3 * h + 4 * l) / 7,
        (2 * h + 5 * l) / 7,
        (h + 6 * l) / 7,
    ];
    let mut bits = 0u64;
    for (n, t) in texels.iter().enumerate() {
        let a = i32::from(t[3]);
        let best = (0..8).min_by_key(|&k| ((palette[k] - a).abs(), k)).unwrap_or(0);
        bits |= (best as u64) << (3 * n);
    }
    out[2..].copy_from_slice(&bits.to_le_bytes()[..6]);
    out
}

/// One level as DXT1 (opaque: alpha is ignored) or DXT5 blocks.
pub fn encode_level(format: PixelFormat, level: &Level) -> Option<Vec<u8>> {
    let (blocks_x, blocks_y) = (level.width.div_ceil(4), level.height.div_ceil(4));
    let mut out = Vec::with_capacity((blocks_x * blocks_y) as usize * format.block_bytes()?);
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let texels = block(level, bx, by);
            match format {
                PixelFormat::Dxt1 => out.extend_from_slice(&color_block(&texels)),
                PixelFormat::Dxt5 => {
                    out.extend_from_slice(&alpha_block(&texels));
                    out.extend_from_slice(&color_block(&texels));
                }
                _ => return None,
            }
        }
    }
    Some(out)
}

impl TextureSource {
    /// A texture of `mips` levels built from the RGBA8 rows of `picture`, stored as `format`: DXT1 (alpha dropped),
    /// DXT5, or A8R8G8B8. `None` for another format, a picture whose pixels are not `width * height * 4` bytes, or a
    /// level count outside `1..=full_mip_count`.
    pub fn from_rgba(name: impl Into<String>, picture: Level, format: PixelFormat, mips: u32) -> Option<Self> {
        if picture.rgba.len() != (picture.width * picture.height * 4) as usize
            || picture.width == 0
            || picture.height == 0
            || mips == 0
            || mips > full_mip_count(picture.width, picture.height)
        {
            return None;
        }
        let (width, height) = (picture.width, picture.height);
        let mut data = Vec::new();
        for level in mip_chain(picture, mips) {
            match format {
                PixelFormat::Argb8888 => {
                    data.extend(level.rgba.as_chunks::<4>().0.iter().flat_map(|&[r, g, b, a]| [b, g, r, a]));
                }
                _ => data.extend(encode_level(format, &level)?),
            }
        }
        debug_assert_eq!(data.len(), (0..mips).map(|l| mip_level_size(format, width, height, l)).sum::<usize>());
        Some(Self::new(name, width, height, format, mips, data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Texture, decode_mip_rgba8};

    fn level(width: u32, height: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Level {
        let rgba = (0..height).flat_map(|y| (0..width).map(move |x| (x, y))).flat_map(|(x, y)| f(x, y)).collect();
        Level { width, height, rgba }
    }

    fn decoded(source: &TextureSource, mip: u32) -> Vec<u8> {
        let texture = Texture {
            name: source.name.clone(),
            name_hash: source.hash(),
            width: source.width,
            height: source.height,
            mip_levels: source.mip_count,
            format: source.format,
            compression_type: source.compression_type,
            alpha_usage: source.alpha_usage,
            alpha_sorting: source.alpha_sorting,
            alpha_blend: source.alpha_blend,
            flags: source.flags,
            data: source.data.clone(),
            palette: Vec::new(),
        };
        decode_mip_rgba8(&texture, mip).expect("the level decodes")
    }

    #[test]
    fn a_flat_colour_survives_dxt1_within_quantisation() {
        let source =
            TextureSource::from_rgba("T", level(8, 8, |_, _| [200, 100, 50, 255]), PixelFormat::Dxt1, 1).unwrap();
        for px in decoded(&source, 0).chunks(4) {
            assert!(px[0].abs_diff(200) <= 4 && px[1].abs_diff(100) <= 2 && px[2].abs_diff(50) <= 4, "{px:?}");
        }
    }

    #[test]
    fn a_flat_block_of_dark_grey_stays_grey_and_keeps_its_level() {
        for grey in [20u8, 27, 33, 40, 55, 77, 100, 140] {
            let source =
                TextureSource::from_rgba("T", level(8, 8, |_, _| [grey, grey, grey, 255]), PixelFormat::Dxt1, 1)
                    .unwrap();
            for px in decoded(&source, 0).chunks(4) {
                let spread = px[0].abs_diff(px[1]).max(px[1].abs_diff(px[2]));
                assert!(spread <= 2, "grey {grey} became {px:?}");
                assert!(px[1].abs_diff(grey) <= 2, "grey {grey} became {px:?}");
            }
        }
    }

    #[test]
    fn a_two_colour_block_keeps_both_colours() {
        let picture = level(4, 4, |x, _| if x < 2 { [250, 20, 20, 255] } else { [20, 20, 250, 255] });
        let source = TextureSource::from_rgba("T", picture, PixelFormat::Dxt1, 1).unwrap();
        let out = decoded(&source, 0);
        assert!(out[0] > 200 && out[2] < 60, "left is red: {:?}", &out[..4]);
        assert!(out[12] < 60 && out[14] > 200, "right is blue: {:?}", &out[12..16]);
    }

    #[test]
    fn dxt5_keeps_a_cut_out_alpha() {
        let picture = level(8, 8, |x, y| [90, 90, 90, if (x + y) % 2 == 0 { 255 } else { 0 }]);
        let source = TextureSource::from_rgba("T", picture, PixelFormat::Dxt5, 1).unwrap();
        for (n, px) in decoded(&source, 0).chunks(4).enumerate() {
            let (x, y) = (n % 8, n / 8);
            assert_eq!(px[3], if (x + y) % 2 == 0 { 255 } else { 0 });
        }
    }

    #[test]
    fn the_mip_chain_reaches_one_pixel_and_sizes_add_up() {
        let source = TextureSource::from_rgba(
            "T",
            level(16, 4, |x, y| [x as u8 * 8, y as u8 * 8, 0, 255]),
            PixelFormat::Dxt5,
            5,
        )
        .unwrap();
        assert_eq!(source.data.len(), source.expected_data_len());
        assert_eq!(full_mip_count(16, 4), 5);
        assert_eq!(decoded(&source, 4).len(), 4);
    }

    #[test]
    fn the_encoder_is_deterministic_and_refuses_bad_input() {
        let picture = level(8, 8, |x, y| [(x * 31) as u8, (y * 17) as u8, (x * y) as u8, 255]);
        let a = TextureSource::from_rgba("T", picture.clone(), PixelFormat::Dxt1, 2).unwrap();
        let b = TextureSource::from_rgba("T", picture.clone(), PixelFormat::Dxt1, 2).unwrap();
        assert_eq!(a.data, b.data);
        assert!(TextureSource::from_rgba("T", picture.clone(), PixelFormat::Dxt1, 9).is_none());
        assert!(TextureSource::from_rgba("T", picture, PixelFormat::P8, 1).is_none());
        assert!(
            TextureSource::from_rgba("T", Level { width: 2, height: 2, rgba: vec![0; 3] }, PixelFormat::Dxt1, 1)
                .is_none()
        );
    }

    #[test]
    fn halving_averages_blocks() {
        let one = Level {
            width: 2,
            height: 2,
            rgba: vec![0, 0, 0, 0, 100, 100, 100, 100, 100, 100, 100, 100, 200, 200, 200, 200],
        };
        assert_eq!(halve(&one).rgba, vec![100, 100, 100, 100]);
    }
}
