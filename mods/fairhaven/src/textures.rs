//! A 2012 texture as a texture of the game's texture pack: the same block-compressed pixels with the top mip levels dropped to
//! fit `max_texture` (no decoding, no resampling), or decoded and stored as DXT for the formats the game has no name for
//! (BC7, BC4, BC5, plain RGBA).

use anyhow::{Result, bail};
use blackbox_tpk::{AlphaUsage, Level, PixelFormat, TextureSource, full_mip_count};
use chameleon_gfx::{PixelFormat as Source, Texture, mip_size};

/// The flag of a texture the game draws on both sides (`TextureInfo.Flags`, bit 1).
const TWO_SIDED: u8 = 2;
/// Tiles across and down: the world's pictures repeat.
const TILES_BOTH: u8 = 3;

/// How the surface that uses the picture draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Look {
    pub cut_out: bool,
    pub two_sided: bool,
}

impl Look {
    /// The suffix of the picture's name: one picture can be used by surfaces that draw it differently, and the game keeps
    /// those facts on the texture, so each look is a texture of its own.
    pub fn suffix(self) -> &'static str {
        match (self.cut_out, self.two_sided) {
            (false, false) => "",
            (true, false) => "C",
            (false, true) => "D",
            (true, true) => "E",
        }
    }
}

/// The game's name of 2012 texture `id` drawn with `look`: at most 23 bytes, so the pack keeps the whole name.
pub fn name_of(id: u64, look: Look) -> String {
    format!("F{id:016X}{}", look.suffix())
}

/// How many of the top levels of `texture` to drop so that its longer side is at most `limit`; never all of them.
pub fn levels_to_drop(texture: &Texture, limit: u32) -> u32 {
    let longest = texture.width.max(texture.height);
    let mut drop = 0;
    while (longest >> drop) > limit.max(1) && drop + 1 < texture.mip_levels {
        drop += 1;
    }
    drop
}

/// The pixels of the levels from `drop` down, back to back, and the number of levels. `None` when the data is shorter than
/// the levels it should hold.
fn levels_from(texture: &Texture, drop: u32) -> Option<(&[u8], u32)> {
    let skipped: usize =
        (0..drop).map(|level| mip_size(texture.format, texture.width, texture.height, level)).sum::<Option<usize>>()?;
    let kept: usize = (drop..texture.mip_levels)
        .map(|level| mip_size(texture.format, texture.width, texture.height, level))
        .sum::<Option<usize>>()?;
    Some((texture.data.get(skipped..skipped + kept)?, texture.mip_levels - drop))
}

/// The texture of the game for `texture`, named `name`, at most `limit` pixels on a side.
pub fn texture_source(name: &str, texture: &Texture, limit: u32, look: Look) -> Result<TextureSource> {
    let drop = levels_to_drop(texture, limit);
    let (width, height) = ((texture.width >> drop).max(1), (texture.height >> drop).max(1));
    let Some((pixels, levels)) = levels_from(texture, drop) else {
        bail!(
            "{name}: the pixels are shorter than the {} levels of {}x{}",
            texture.mip_levels,
            texture.width,
            texture.height
        );
    };
    let kept = match texture.format {
        Source::Bc1 => Some(PixelFormat::Dxt1),
        Source::Bc2 => Some(PixelFormat::Dxt3),
        Source::Bc3 => Some(PixelFormat::Dxt5),
        _ => None,
    };
    let mut source = match kept {
        Some(format) => {
            TextureSource::new(name, width, height, format, levels.min(full_mip_count(width, height)), pixels.to_vec())
        }
        None => reencode(name, texture, drop, (width, height), pixels, look)?,
    };
    source.alpha_usage = if look.cut_out { AlphaUsage::PunchThrough } else { AlphaUsage::None };
    source.flags = if look.two_sided { TWO_SIDED } else { 0 };
    source.tilable_uv = TILES_BOTH;
    Ok(source)
}

/// A texture the game has no format for: decode level `drop` and store it as DXT1 (DXT5 when it has alpha it uses) with a
/// full chain of levels made here.
fn reencode(
    name: &str,
    texture: &Texture,
    drop: u32,
    (width, height): (u32, u32),
    pixels: &[u8],
    look: Look,
) -> Result<TextureSource> {
    let level_bytes = mip_size(texture.format, texture.width, texture.height, drop).unwrap_or(0);
    let level = Texture {
        width,
        height,
        depth: 1,
        array_size: 1,
        mip_levels: 1,
        dxgi_format: texture.dxgi_format,
        format: texture.format,
        data: pixels.get(..level_bytes).unwrap_or_default().to_vec(),
    };
    let Some(rgba) = level.to_rgba8() else { bail!("{name}: the pixel format {:?} cannot be read", texture.format) };
    let has_alpha = !look.cut_out && rgba.as_chunks::<4>().0.iter().any(|pixel| pixel[3] < 255);
    let format = if has_alpha { PixelFormat::Dxt5 } else { PixelFormat::Dxt1 };
    let picture = Level { width, height, rgba };
    let mips = full_mip_count(width, height);
    TextureSource::from_rgba(name, picture, format, mips)
        .ok_or_else(|| anyhow::anyhow!("{name}: {width}x{height} cannot be stored"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BC1 texture of `size` pixels square with `levels` levels, each block filled with its level number.
    fn bc1(size: u32, levels: u32) -> Texture {
        let mut data = Vec::new();
        for level in 0..levels {
            let blocks = ((size >> level).max(1).div_ceil(4)).pow(2) as usize;
            data.extend(std::iter::repeat_n(level as u8, blocks * 8));
        }
        Texture {
            width: size,
            height: size,
            depth: 1,
            array_size: 1,
            mip_levels: levels,
            dxgi_format: 71,
            format: Source::Bc1,
            data,
        }
    }

    const PLAIN: Look = Look { cut_out: false, two_sided: false };

    #[test]
    fn the_top_levels_are_dropped_until_the_picture_fits() {
        assert_eq!(levels_to_drop(&bc1(2048, 12), 512), 2);
        assert_eq!(levels_to_drop(&bc1(256, 9), 512), 0);
        assert_eq!(levels_to_drop(&bc1(2048, 2), 64), 1, "the last level is never dropped");
    }

    #[test]
    fn a_block_compressed_picture_keeps_its_pixels_and_loses_its_top_levels() {
        let texture = bc1(64, 7);
        let source = texture_source("F1", &texture, 16, PLAIN).unwrap();
        assert_eq!((source.width, source.height, source.mip_count), (16, 16, 5));
        assert_eq!(source.format, PixelFormat::Dxt1);
        // The first stored level is the third (level 2) of the original: its blocks are filled with 2.
        assert!(source.data[..8].iter().all(|&byte| byte == 2));
        assert_eq!(source.data.len(), source.expected_data_len());
    }

    #[test]
    fn the_look_sets_the_alpha_use_the_sides_and_the_name() {
        let texture = bc1(8, 4);
        let cut = texture_source("F1C", &texture, 512, Look { cut_out: true, two_sided: true }).unwrap();
        assert_eq!(cut.alpha_usage, AlphaUsage::PunchThrough);
        assert_eq!(cut.flags, TWO_SIDED);
        assert_eq!(name_of(0x0123_4567_89AB_CDEF, Look { cut_out: true, two_sided: true }), "F0123456789ABCDEFE");
        assert!(name_of(u64::MAX, Look { cut_out: true, two_sided: true }).len() <= 23);
        assert_eq!(texture_source("F1", &texture, 512, PLAIN).unwrap().alpha_usage, AlphaUsage::None);
    }

    #[test]
    fn short_pixel_data_is_refused() {
        let mut texture = bc1(16, 5);
        texture.data.truncate(100);
        assert!(texture_source("F1", &texture, 512, PLAIN).is_err());
    }

    #[test]
    fn a_plain_rgba_picture_is_stored_as_dxt1_with_a_full_chain() {
        let texture = Texture {
            width: 8,
            height: 8,
            depth: 1,
            array_size: 1,
            mip_levels: 1,
            dxgi_format: 28,
            format: Source::Rgba8,
            data: [10u8, 120, 30, 255].repeat(64),
        };
        let source = texture_source("F2", &texture, 512, PLAIN).unwrap();
        assert_eq!((source.format, source.mip_count), (PixelFormat::Dxt1, 4));
        assert_eq!(source.data.len(), source.expected_data_len());
        let alpha = Texture { data: [10u8, 120, 30, 100].repeat(64), ..texture };
        assert_eq!(texture_source("F3", &alpha, 512, PLAIN).unwrap().format, PixelFormat::Dxt5);
    }
}
