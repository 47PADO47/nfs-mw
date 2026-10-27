//! Baking a two-colour paint into one picture. Spec: `docs/formats/chameleon-customization.md`.
//!
//! The 2012 game paints a two-colour car with two colours and a rule per surface ([`PaintBlend`]). An older engine that
//! draws a car's paint surfaces with one picture can show the same look when the picture is that rule baked in: a
//! [`Sheet`] gives every vertex of a paint surface a place in the picture, and [`Sheet::bake`] fills the picture for a pair
//! of colours. A surface that blends by a mask keeps its own texture coordinates, so the picture has the mask's size and
//! each texel is the first colour where the mask is 0 and the second where it is 1; the surfaces that take one colour are
//! pointed at a block of the picture that is that colour whatever the pair. A car with no mask gets a picture of two blocks.

use chameleon_gfx::{PixelFormat, Texture};

use crate::paint_surface::PaintBlend;

/// The DXGI number of BC1 (DXT1) with unsigned-normalised texels.
const BC1_UNORM: u32 = 71;
/// DXGI numbers of the sRGB-encoded block formats (BC1, BC2, BC3, BC7): the sampler decodes a texel before it is used.
const SRGB_FORMATS: [u32; 4] = [72, 75, 78, 99];
/// Edge of a block of texels, and of the block a one-colour surface samples.
const BLOCK: usize = 4;
/// A mask value this near 0 or 1 is that colour as far as a block that a one-colour surface samples is concerned.
const PURE: f32 = 0.02;

/// How much of the second colour each texel of a mask takes, 0 to 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    width: usize,
    height: usize,
    amount: Vec<f32>,
}

impl Mask {
    /// A mask of `width` x `height` texels, row by row from the top. `None` when `amount` is not that many values.
    pub fn new(width: usize, height: usize, amount: Vec<f32>) -> Option<Self> {
        let expected = width.checked_mul(height)?;
        (width > 0 && height > 0 && amount.len() == expected).then_some(Self { width, height, amount })
    }

    /// The mask in the top mip of `texture`: its green channel, which is what the two-colour shader reads. `None` when the
    /// texture's format cannot be decoded.
    pub fn read(texture: &Texture) -> Option<Self> {
        let rgba = texture.to_rgba8()?;
        let srgb = SRGB_FORMATS.contains(&texture.dxgi_format);
        let amount = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|texel| {
                let value = f32::from(texel[1]) / 255.0;
                if srgb { srgb_to_linear(value) } else { value }
            })
            .collect();
        Self::new(texture.width as usize, texture.height as usize, amount)
    }
}

/// The layout of the picture of a two-colour car.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    width: usize,
    height: usize,
    /// The amount of the second colour of each texel.
    amount: Vec<f32>,
    /// Where the surfaces that take the first and the second colour sample the picture.
    first: [f32; 2],
    second: [f32; 2],
    /// Whether the picture has the mip levels of its size (a mask is minified, two blocks are only ever sampled at their
    /// centres).
    mips: bool,
}

impl Sheet {
    /// The picture of a car with no mask: two blocks of texels, the first colour on the left and the second on the right.
    pub fn flat() -> Self {
        let amount = (0..BLOCK).flat_map(|_| (0..2 * BLOCK).map(|x| if x < BLOCK { 0.0 } else { 1.0 })).collect();
        Self { width: 2 * BLOCK, height: BLOCK, amount, first: [0.25, 0.5], second: [0.75, 0.5], mips: false }
    }

    /// The picture of a car whose surfaces blend by `mask`: the mask's size. The one-colour surfaces sample the block of
    /// the mask that is nearest to 0, and the one nearest to 1; where the mask has no block that is pure, the nearest is
    /// made so (a few texels). A mask under 8 texels either way is too small to hold the blocks and gets [`Sheet::flat`].
    pub fn with_mask(mask: Mask) -> Self {
        if mask.width < 2 * BLOCK || mask.height < 2 * BLOCK {
            return Self::flat();
        }
        let mut sheet = Self {
            width: mask.width,
            height: mask.height,
            amount: mask.amount,
            first: [0.5; 2],
            second: [0.5; 2],
            mips: true,
        };
        let (first, block) = sheet.anchor(0.0, None);
        let (second, _) = sheet.anchor(1.0, Some(block));
        sheet.first = first;
        sheet.second = second;
        sheet
    }

    /// The place in the picture a vertex of a surface of `blend` takes, given the vertex's own texture coordinates: the
    /// coordinates stay for a surface that blends by the mask (or is not part of the paint scheme), and a surface of one
    /// colour has a fixed place whatever its coordinates.
    pub fn uv_of(&self, blend: PaintBlend, uv: [f32; 2]) -> [f32; 2] {
        match blend {
            PaintBlend::First => self.first,
            PaintBlend::Second => self.second,
            PaintBlend::Mask(_) | PaintBlend::Picture => uv,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// The block of texels (not `avoid`) closest to `target`, made `target` where it is not pure already, and the centre
    /// of the block in texture coordinates.
    fn anchor(&mut self, target: f32, avoid: Option<(usize, usize)>) -> ([f32; 2], (usize, usize)) {
        let mut best: Option<(f32, (usize, usize))> = None;
        for by in 0..self.height / BLOCK {
            for bx in 0..self.width / BLOCK {
                if avoid == Some((bx, by)) {
                    continue;
                }
                let cost = self.worst(bx, by, target);
                match best {
                    Some((least, _)) if cost >= least => {}
                    _ => best = Some((cost, (bx, by))),
                }
            }
        }
        let (cost, (bx, by)) = best.unwrap_or((0.0, (0, 0)));
        if cost > PURE {
            for y in by * BLOCK..(by + 1) * BLOCK {
                for x in bx * BLOCK..(bx + 1) * BLOCK {
                    self.amount[y * self.width + x] = target;
                }
            }
        }
        let centre = |block: usize, size: usize| (block * BLOCK + BLOCK / 2) as f32 / size as f32;
        ([centre(bx, self.width), centre(by, self.height)], (bx, by))
    }

    /// How far the texels of block (`bx`, `by`) are from `target` at most.
    fn worst(&self, bx: usize, by: usize, target: f32) -> f32 {
        let mut worst = 0.0f32;
        for y in by * BLOCK..(by + 1) * BLOCK {
            for x in bx * BLOCK..(bx + 1) * BLOCK {
                worst = worst.max((self.amount[y * self.width + x] - target).abs());
            }
        }
        worst
    }

    /// The picture for a pair of colours (linear RGB): BC1, with the mip levels of the sheet's size.
    pub fn bake(&self, first: [f32; 3], second: [f32; 3]) -> Texture {
        let palette = Palette::of(first, second);
        let (mut level, mut width, mut height) = (self.amount.clone(), self.width, self.height);
        let mut data = Vec::new();
        let mut levels = 0;
        loop {
            encode_level(&level, width, height, (first, second), &palette, &mut data);
            levels += 1;
            if !self.mips || (width == 1 && height == 1) {
                break;
            }
            (level, width, height) = halve(&level, width, height);
        }
        Texture {
            width: self.width as u32,
            height: self.height as u32,
            depth: 1,
            array_size: 1,
            mip_levels: levels,
            dxgi_format: BC1_UNORM,
            format: PixelFormat::Bc1,
            data,
        }
    }
}

/// The four colours of a BC1 block whose end points are the two colours of a pair, in the order of the indices.
struct Palette {
    /// The end points as the block stores them, the larger first (which selects the four-colour mode).
    words: [u16; 2],
    /// The colours in sRGB byte values.
    colours: [[f32; 3]; 4],
}

impl Palette {
    fn of(first: [f32; 3], second: [f32; 3]) -> Self {
        let (a, b) = (pack565(srgb_bytes(first)), pack565(srgb_bytes(second)));
        let words = if a >= b { [a, b] } else { [b, a] };
        let (c0, c1) = (unpack565(words[0]), unpack565(words[1]));
        let mix = |k: f32| [0, 1, 2].map(|i| c0[i] + (c1[i] - c0[i]) * k);
        Self { words, colours: [c0, c1, mix(1.0 / 3.0), mix(2.0 / 3.0)] }
    }

    /// The index of the colour nearest to `target` (the lowest on a tie).
    fn nearest(&self, target: [f32; 3]) -> u8 {
        let distance = |colour: &[f32; 3]| (0..3).map(|i| (colour[i] - target[i]).powi(2)).sum::<f32>();
        let mut best = (f32::MAX, 0u8);
        for (index, colour) in self.colours.iter().enumerate() {
            let away = distance(colour);
            if away < best.0 {
                best = (away, index as u8);
            }
        }
        best.1
    }
}

/// Append the blocks of one mip level, rows of blocks from the top; a block past the edge repeats the last texel.
fn encode_level(
    level: &[f32],
    width: usize,
    height: usize,
    (first, second): ([f32; 3], [f32; 3]),
    palette: &Palette,
    out: &mut Vec<u8>,
) {
    for by in 0..height.div_ceil(BLOCK) {
        for bx in 0..width.div_ceil(BLOCK) {
            out.extend_from_slice(&palette.words[0].to_le_bytes());
            out.extend_from_slice(&palette.words[1].to_le_bytes());
            for row in 0..BLOCK {
                let mut bits = 0u8;
                for col in 0..BLOCK {
                    let (x, y) = ((bx * BLOCK + col).min(width - 1), (by * BLOCK + row).min(height - 1));
                    let amount = level[y * width + x];
                    let colour = [0, 1, 2].map(|i| first[i] + (second[i] - first[i]) * amount);
                    bits |= palette.nearest(srgb_bytes(colour)) << (2 * col);
                }
                out.push(bits);
            }
        }
    }
}

/// The next mip level: each texel the mean of the (up to) four texels above it.
fn halve(level: &[f32], width: usize, height: usize) -> (Vec<f32>, usize, usize) {
    let (next_width, next_height) = ((width / 2).max(1), (height / 2).max(1));
    let mut out = Vec::with_capacity(next_width * next_height);
    for y in 0..next_height {
        for x in 0..next_width {
            let (mut sum, mut count) = (0.0, 0.0);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = (2 * x + dx, 2 * y + dy);
                if sx < width && sy < height {
                    sum += level[sy * width + sx];
                    count += 1.0;
                }
            }
            out.push(sum / count);
        }
    }
    (out, next_width, next_height)
}

/// A linear colour as sRGB byte values (0 to 255, not rounded).
fn srgb_bytes(linear: [f32; 3]) -> [f32; 3] {
    linear.map(|channel| {
        let value = channel.clamp(0.0, 1.0);
        let encoded = if value <= 0.003_130_8 { value * 12.92 } else { 1.055 * value.powf(1.0 / 2.4) - 0.055 };
        encoded * 255.0
    })
}

fn srgb_to_linear(encoded: f32) -> f32 {
    if encoded <= 0.040_45 {
        return encoded / 12.92;
    }
    ((encoded + 0.055) / 1.055).powf(2.4)
}

/// Byte values as the 5:6:5 word of a BC1 end point.
fn pack565([r, g, b]: [f32; 3]) -> u16 {
    let level = |value: f32, max: f32| (value / 255.0 * max).round().clamp(0.0, max) as u16;
    (level(r, 31.0) << 11) | (level(g, 63.0) << 5) | level(b, 31.0)
}

/// The byte values a BC1 decoder makes of a 5:6:5 word.
fn unpack565(word: u16) -> [f32; 3] {
    let (r, g, b) = (u32::from(word >> 11), u32::from((word >> 5) & 0x3F), u32::from(word & 0x1F));
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)].map(|value| value as f32)
}

#[cfg(test)]
mod tests {
    use chameleon_gfx::mip_size;

    use super::*;

    fn rgba(texture: &Texture, x: usize, y: usize) -> [u8; 4] {
        let pixels = texture.to_rgba8().unwrap();
        let at = 4 * (y * texture.width as usize + x);
        [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
    }

    fn near(found: [u8; 4], wanted: [u8; 3]) -> bool {
        (0..3).all(|i| found[i].abs_diff(wanted[i]) <= 8) && found[3] == 255
    }

    /// A mask of `width` x `height` texels that is 0 left of `edge` and 1 from it.
    fn split(width: usize, height: usize, edge: usize) -> Mask {
        let amount = (0..height).flat_map(|_| (0..width).map(|x| if x < edge { 0.0 } else { 1.0 })).collect();
        Mask::new(width, height, amount).unwrap()
    }

    #[test]
    fn the_flat_sheet_is_two_blocks_in_the_two_colours() {
        let sheet = Sheet::flat();
        let picture = sheet.bake([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        assert_eq!((picture.width, picture.height, picture.mip_levels), (8, 4, 1));
        assert_eq!(picture.data.len(), 16);
        assert!(near(rgba(&picture, 0, 0), [255, 0, 0]));
        assert!(near(rgba(&picture, 3, 3), [255, 0, 0]));
        assert!(near(rgba(&picture, 4, 0), [0, 0, 255]));
        assert!(near(rgba(&picture, 7, 3), [0, 0, 255]));
        // The places the one-colour surfaces sample sit in the middle of their blocks.
        assert_eq!(sheet.uv_of(PaintBlend::First, [9.0, 9.0]), [0.25, 0.5]);
        assert_eq!(sheet.uv_of(PaintBlend::Second, [9.0, 9.0]), [0.75, 0.5]);
    }

    #[test]
    fn a_mask_surface_keeps_its_coordinates() {
        let sheet = Sheet::flat();
        assert_eq!(sheet.uv_of(PaintBlend::Mask(7), [1.5, -0.25]), [1.5, -0.25]);
        assert_eq!(sheet.uv_of(PaintBlend::Picture, [0.5, 0.5]), [0.5, 0.5]);
    }

    #[test]
    fn a_masked_sheet_is_the_masks_size_and_the_colours_follow_the_mask() {
        let sheet = Sheet::with_mask(split(16, 8, 8));
        let picture = sheet.bake([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert_eq!((picture.width, picture.height, picture.mip_levels), (16, 8, 5));
        let expected: usize = (0..5).map(|level| mip_size(PixelFormat::Bc1, 16, 8, level).unwrap()).sum();
        assert_eq!(picture.data.len(), expected);
        assert!(near(rgba(&picture, 2, 3), [255, 0, 0]));
        assert!(near(rgba(&picture, 12, 3), [0, 255, 0]));
        // Each one-colour surface samples a block of its own colour.
        let [fu, fv] = sheet.uv_of(PaintBlend::First, [0.0; 2]);
        let [su, sv] = sheet.uv_of(PaintBlend::Second, [0.0; 2]);
        assert!(near(rgba(&picture, (fu * 16.0) as usize, (fv * 8.0) as usize), [255, 0, 0]));
        assert!(near(rgba(&picture, (su * 16.0) as usize, (sv * 8.0) as usize), [0, 255, 0]));
    }

    #[test]
    fn a_mask_with_no_pure_block_gets_one_painted() {
        let sheet = Sheet::with_mask(Mask::new(16, 16, vec![0.0; 256]).unwrap());
        let picture = sheet.bake([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        let [su, sv] = sheet.uv_of(PaintBlend::Second, [0.0; 2]);
        assert!(near(rgba(&picture, (su * 16.0) as usize, (sv * 16.0) as usize), [0, 0, 255]));
        let [fu, fv] = sheet.uv_of(PaintBlend::First, [0.0; 2]);
        assert_ne!([fu, fv], [su, sv], "the two places are different blocks");
        assert!(near(rgba(&picture, (fu * 16.0) as usize, (fv * 16.0) as usize), [255, 0, 0]));
    }

    #[test]
    fn a_mask_too_small_for_the_blocks_is_the_flat_sheet() {
        assert_eq!(Sheet::with_mask(split(4, 4, 2)), Sheet::flat());
        assert_eq!(Sheet::with_mask(split(64, 4, 32)), Sheet::flat());
    }

    #[test]
    fn mixed_texels_take_the_nearest_of_the_blocks_four_colours() {
        let mut amount = vec![0.0; 16 * 16];
        amount[16 * 8..].fill(1.0);
        // One texel half way between the colours, in the top left block.
        amount[1 + 16] = 0.5;
        let sheet = Sheet::with_mask(Mask::new(16, 16, amount).unwrap());
        let picture = sheet.bake([0.0; 3], [1.0; 3]);
        let [r, g, b, _] = rgba(&picture, 1, 1);
        assert!(r == g && g == b, "grey stays grey: {r} {g} {b}");
        assert!((90..=200).contains(&r), "a middle grey, not an end point: {r}");
    }

    #[test]
    fn halving_averages_and_keeps_odd_edges() {
        let (half, width, height) = halve(&[0.0, 1.0, 1.0, 0.0, 0.5, 0.5], 3, 2);
        assert_eq!((width, height), (1, 1));
        assert_eq!(half.len(), 1);
        // The mean of the top left 2 x 2 texels: the third column is dropped.
        assert!((half[0] - 0.375).abs() < 1e-6, "{half:?}");
    }

    #[test]
    fn a_bc1_mask_reads_its_green_channel() {
        // One block, both end points white, so every texel is white.
        let block = vec![0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0];
        let texture = Texture {
            width: 4,
            height: 4,
            depth: 1,
            array_size: 1,
            mip_levels: 1,
            dxgi_format: BC1_UNORM,
            format: PixelFormat::Bc1,
            data: block,
        };
        let mask = Mask::read(&texture).unwrap();
        assert_eq!((mask.width, mask.height), (4, 4));
        assert!(mask.amount.iter().all(|amount| (amount - 1.0).abs() < 0.01), "{:?}", mask.amount);
    }

    #[test]
    fn a_mask_needs_as_many_values_as_texels() {
        assert!(Mask::new(2, 2, vec![0.0; 3]).is_none());
        assert!(Mask::new(0, 2, Vec::new()).is_none());
    }

    #[test]
    fn colour_helpers_round_trip_the_end_points() {
        assert_eq!(unpack565(pack565([255.0, 0.0, 0.0])), [255.0, 0.0, 0.0]);
        assert_eq!(unpack565(pack565([255.0, 255.0, 255.0])), [255.0, 255.0, 255.0]);
        let [dark, bright, clipped] = srgb_bytes([0.0, 1.0, 2.0]);
        assert_eq!(dark, 0.0);
        assert!((bright - 255.0).abs() < 0.01 && (clipped - 255.0).abs() < 0.01, "{bright} {clipped}");
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-5);
        assert!(srgb_to_linear(0.5) < 0.25);
    }
}
