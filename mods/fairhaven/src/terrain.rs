//! The mask of a terrain: where each of its three layers is. A `Terrain*` material blends three tiling pictures by the red, green
//! and blue of a mask picture laid over the whole mesh (`docs/formats/chameleon-world.md`); here each triangle takes the layer
//! that is strongest at its middle, so a terrain is three kinds of ground with hard edges between them instead of one smeared
//! picture.

use chameleon_gfx::Texture;

/// The longest side the mask is read at: a terrain tile is kilometres wide, so a few hundred pixels place the layers to within
/// the size of a triangle.
pub const MAX_MASK: u32 = 256;

/// A mask: its pixels as RGBA, rows top to bottom.
#[derive(Debug, Clone, PartialEq)]
pub struct Mask {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// The mask of a picture, read at the level of detail that fits [`MAX_MASK`]. `None` when its format cannot be read.
pub fn mask_of(texture: &Texture) -> Option<Mask> {
    let mut level = 0;
    while (texture.width.max(texture.height) >> level) > MAX_MASK && level + 1 < texture.mip_levels {
        level += 1;
    }
    let (width, height) = ((texture.width >> level).max(1), (texture.height >> level).max(1));
    let start: usize = (0..level)
        .map(|earlier| chameleon_gfx::mip_size(texture.format, texture.width, texture.height, earlier))
        .sum::<Option<usize>>()?;
    let size = chameleon_gfx::mip_size(texture.format, texture.width, texture.height, level)?;
    let level_texture = Texture {
        width,
        height,
        depth: 1,
        array_size: 1,
        mip_levels: 1,
        dxgi_format: texture.dxgi_format,
        format: texture.format,
        data: texture.data.get(start..start + size)?.to_vec(),
    };
    Some(Mask { width: width as usize, height: height as usize, rgba: level_texture.to_rgba8()? })
}

/// The layer (0, 1 or 2) whose weight is the largest in the mask at texture coordinates `uv` (repeated outside 0 to 1). A tie, and
/// a mask with no weight, give layer 0.
pub fn layer_of(mask: &Mask, uv: [f32; 2]) -> usize {
    let wrap = |v: f32| v - v.floor();
    let x = ((wrap(uv[0]) * mask.width as f32) as usize).min(mask.width - 1);
    let y = ((wrap(uv[1]) * mask.height as f32) as usize).min(mask.height - 1);
    let at = (y * mask.width + x) * 4;
    let weights = &mask.rgba[at..at + 3];
    let mut best = 0;
    for layer in 1..3 {
        if weights[layer] > weights[best] {
            best = layer;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask() -> Mask {
        // 2 by 2: red, green / blue, a tie of green and blue.
        let rgba = vec![200, 10, 10, 255, 10, 200, 10, 255, 10, 10, 200, 255, 0, 90, 90, 255];
        Mask { width: 2, height: 2, rgba }
    }

    #[test]
    fn the_strongest_channel_is_the_layer_and_a_tie_takes_the_first_of_them() {
        let mask = mask();
        assert_eq!(layer_of(&mask, [0.25, 0.25]), 0);
        assert_eq!(layer_of(&mask, [0.75, 0.25]), 1);
        assert_eq!(layer_of(&mask, [0.25, 0.75]), 2);
        assert_eq!(layer_of(&mask, [0.75, 0.75]), 1, "green and blue tie");
    }

    #[test]
    fn coordinates_outside_the_picture_repeat_it() {
        let mask = mask();
        assert_eq!(layer_of(&mask, [1.25, 0.25]), 0);
        assert_eq!(layer_of(&mask, [-0.25, 0.25]), 1);
        assert_eq!(layer_of(&mask, [0.25, 2.75]), 2);
    }

    #[test]
    fn a_picture_is_read_at_the_level_that_fits() {
        // A BC1 picture of 512 by 512 with one level beyond: 256 by 256 is read, from the second level.
        let block = [0u8, 0xF8, 0, 0xF8, 0, 0, 0, 0];
        let mut data = block.repeat(128 * 128);
        data.extend_from_slice(&[0xE0u8, 0x07, 0xE0, 0x07, 0, 0, 0, 0].repeat(64 * 64));
        let texture = Texture {
            width: 512,
            height: 512,
            depth: 1,
            array_size: 1,
            mip_levels: 2,
            dxgi_format: 71,
            format: chameleon_gfx::PixelFormat::Bc1,
            data,
        };
        // 512 over 2 levels is 256 at level 1, so level 1 is read.
        let mask = mask_of(&texture).unwrap();
        assert_eq!((mask.width, mask.height), (256, 256));
        // Level 1 is the pure green blocks (0x07E0).
        assert_eq!(&mask.rgba[..4], [0, 255, 0, 255]);
    }
}
