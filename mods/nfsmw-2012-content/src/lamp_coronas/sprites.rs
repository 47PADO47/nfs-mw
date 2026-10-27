//! The pictures of the corona layers: a layer names a material of the global material dictionary, whose first texture is in
//! the global texture dictionary. Each picture a car uses becomes one texture of the car's own pack, named for the car.

use chameleon_bundle::Bundle;
use chameleon_gfx::{PixelFormat, Texture};

/// Where a material keeps its texture count and the slot of its first texture.
const COUNT_AT: usize = 0x20;
const TABLE_AT: usize = 0x2C;

/// The two dictionaries, and the pictures read from them for one car so far.
pub(super) struct Sprites<'a> {
    materials: Bundle<'a>,
    textures: Bundle<'a>,
    folder: String,
    /// The material of each picture and the name it has in the car's pack.
    names: Vec<(u64, String)>,
    loaded: Vec<(String, Texture)>,
}

impl<'a> Sprites<'a> {
    /// Open the dictionaries' bytes; `None` when either does not read.
    pub(super) fn open(materials: &'a [u8], textures: &'a [u8], folder: &str) -> Option<Self> {
        Some(Self {
            materials: Bundle::parse(materials).ok()?,
            textures: Bundle::parse(textures).ok()?,
            folder: folder.to_owned(),
            names: Vec::new(),
            loaded: Vec::new(),
        })
    }

    /// The name in the car's pack of the picture of `material`, reading it the first time. `None` when the material or its
    /// texture is not in the dictionaries.
    pub(super) fn name(&mut self, material: u64) -> Option<String> {
        if let Some((_, name)) = self.names.iter().find(|(known, _)| *known == material) {
            return Some(name.clone());
        }
        let texture = self.read(material)?;
        let name = format!("{}_CORONA{}", self.folder, self.names.len());
        self.names.push((material, name.clone()));
        self.loaded.push((name.clone(), texture));
        Some(name)
    }

    fn read(&self, material: u64) -> Option<Texture> {
        let resource = self.materials.resource(self.materials.find(material)?).ok()?;
        let main = &resource.blocks[0];
        let word = |at: usize| Some(u32::from_le_bytes(main.get(at..at + 4)?.try_into().ok()?));
        if word(COUNT_AT)? == 0 {
            return None;
        }
        let table = word(TABLE_AT)?;
        let import = resource.imports.iter().find(|import| import.slot() == table)?;
        let texture = self.textures.resource(self.textures.find(import.id)?).ok()?;
        Texture::parse(&texture.blocks[0], &texture.blocks[1]).ok()
    }

    /// The pictures read, with their names, as textures the car's pack can carry: the block formats as they are, a plain
    /// RGBA picture compressed to DXT1 and cut down to [`MAX_SIDE`] texels a side. A picture of another format is left out.
    pub(super) fn finish(self) -> Vec<(String, Texture)> {
        self.loaded.into_iter().filter_map(|(name, texture)| Some((name, carry(texture)?))).collect()
    }
}

/// The largest side of a sprite, texels.
const MAX_SIDE: u32 = 128;

fn carry(texture: Texture) -> Option<Texture> {
    match texture.format {
        PixelFormat::Bc1 | PixelFormat::Bc3 => return Some(texture),
        PixelFormat::Rgba8 => {}
        _ => return None,
    }
    let last = texture.mip_levels.saturating_sub(1);
    let skip =
        (0..=last).find(|&level| (texture.width >> level).max(texture.height >> level) <= MAX_SIDE).unwrap_or(last);
    let mip = texture.mip(skip)?;
    let (width, height) = ((texture.width >> skip).max(1), (texture.height >> skip).max(1));
    let data = compress(mip, width, height)?;
    Some(Texture {
        width,
        height,
        depth: 1,
        array_size: 1,
        mip_levels: 1,
        dxgi_format: BC1_UNORM,
        format: PixelFormat::Bc1,
        data,
    })
}

/// The DXGI format number of BC1.
const BC1_UNORM: u32 = 71;

/// RGBA8 pixels as DXT1 blocks. `None` when the pixels are short.
fn compress(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let (w, h) = (width as usize, height as usize);
    if rgba.len() < w * h * 4 {
        return None;
    }
    let mut out = Vec::with_capacity(w.div_ceil(4) * h.div_ceil(4) * 8);
    for by in 0..h.div_ceil(4) {
        for bx in 0..w.div_ceil(4) {
            let mut pixels = [[0u8; 3]; 16];
            for (i, pixel) in pixels.iter_mut().enumerate() {
                let (x, y) = ((bx * 4 + i % 4).min(w - 1), (by * 4 + i / 4).min(h - 1));
                let at = (y * w + x) * 4;
                *pixel = [rgba[at], rgba[at + 1], rgba[at + 2]];
            }
            out.extend_from_slice(&block(&pixels));
        }
    }
    Some(out)
}

fn luma(p: &[u8; 3]) -> u32 {
    30 * u32::from(p[0]) + 59 * u32::from(p[1]) + 11 * u32::from(p[2])
}

fn rgb565(p: [u8; 3]) -> u16 {
    (u16::from(p[0] >> 3) << 11) | (u16::from(p[1] >> 2) << 5) | u16::from(p[2] >> 3)
}

fn expand565(c: u16) -> [i32; 3] {
    let (r, g, b) = ((c >> 11) & 31, (c >> 5) & 63, c & 31);
    [i32::from((r << 3) | (r >> 2)), i32::from((g << 2) | (g >> 4)), i32::from((b << 3) | (b >> 2))]
}

/// One block: the brightest and the darkest pixel are its end points, every pixel takes the nearest of the four colours.
fn block(pixels: &[[u8; 3]; 16]) -> [u8; 8] {
    let high = pixels.iter().max_by_key(|p| luma(p)).copied().unwrap_or([0; 3]);
    let low = pixels.iter().min_by_key(|p| luma(p)).copied().unwrap_or([0; 3]);
    let (mut c0, mut c1) = (rgb565(high), rgb565(low));
    if c0 < c1 {
        std::mem::swap(&mut c0, &mut c1);
    }
    let mut out = [0u8; 8];
    out[..2].copy_from_slice(&c0.to_le_bytes());
    out[2..4].copy_from_slice(&c1.to_le_bytes());
    if c0 == c1 {
        return out;
    }
    let (p0, p1) = (expand565(c0), expand565(c1));
    let palette = [p0, p1, [0, 1, 2].map(|i| (2 * p0[i] + p1[i]) / 3), [0, 1, 2].map(|i| (p0[i] + 2 * p1[i]) / 3)];
    let mut indices = 0u32;
    for (i, pixel) in pixels.iter().enumerate() {
        let nearest = (0..4usize)
            .min_by_key(|&k| (0..3).map(|c| (palette[k][c] - i32::from(pixel[c])).pow(2)).sum::<i32>())
            .unwrap_or(0);
        indices |= (nearest as u32) << (2 * i);
    }
    out[4..].copy_from_slice(&indices.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uniform_picture_is_one_end_point_and_zero_indices_per_block() {
        let rgba: Vec<u8> = (0..16).flat_map(|_| [200, 100, 50, 7]).collect();
        let data = compress(&rgba, 4, 4).unwrap();
        assert_eq!(data.len(), 8);
        assert_eq!(&data[..2], &data[2..4]);
        assert_eq!(&data[4..], &[0, 0, 0, 0]);
    }

    #[test]
    fn a_ramp_keeps_its_ends_and_orders_the_end_points_for_four_colour_mode() {
        let rgba: Vec<u8> = (0..16u8).flat_map(|n| [n * 16, n * 16, n * 16, 255]).collect();
        let data = compress(&rgba, 4, 4).unwrap();
        let (c0, c1) = (u16::from_le_bytes([data[0], data[1]]), u16::from_le_bytes([data[2], data[3]]));
        assert!(c0 > c1);
        assert_eq!(c1, 0, "black is one end");
        assert_eq!(
            u32::from_le_bytes([data[4], data[5], data[6], data[7]]) & 3,
            1,
            "the first pixel is the darker end"
        );
    }

    #[test]
    fn a_picture_is_cut_to_the_largest_side_and_short_data_is_refused() {
        let mut texture = Texture {
            width: 256,
            height: 256,
            depth: 1,
            array_size: 1,
            mip_levels: 2,
            dxgi_format: 28,
            format: PixelFormat::Rgba8,
            data: vec![255; 256 * 256 * 4 + 128 * 128 * 4],
        };
        let carried = carry(texture.clone()).unwrap();
        assert_eq!((carried.width, carried.height, carried.format), (128, 128, PixelFormat::Bc1));
        assert_eq!(carried.data.len(), 32 * 32 * 8);
        texture.data.truncate(100);
        assert!(carry(texture).is_none());
        assert!(compress(&[0; 10], 4, 4).is_none());
    }
}
