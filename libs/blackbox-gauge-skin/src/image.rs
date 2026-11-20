//! A decoded picture: straight (not premultiplied) RGBA8, rows top to bottom.

use blackbox_tpk::{AlphaUsage, PixelFormat, Texture};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes, `r g b a` per pixel.
    pub rgba: Vec<u8>,
}

impl Image {
    /// The picture as a texture a UI can draw: uncompressed, one mip level, blended additively or by its alpha.
    pub fn to_texture(&self, name: &str, name_hash: u32, additive: bool) -> Texture {
        let mut data = Vec::with_capacity(self.rgba.len());
        for pixel in self.rgba.as_chunks::<4>().0 {
            data.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
        Texture {
            name: name.to_owned(),
            name_hash,
            width: self.width,
            height: self.height,
            mip_levels: 1,
            format: PixelFormat::Argb8888,
            compression_type: 0,
            alpha_usage: AlphaUsage::Modulated,
            alpha_sorting: false,
            alpha_blend: match additive {
                true => 2,
                false => 1,
            },
            flags: 0,
            tilable_uv: 0,
            data,
            palette: Vec::new(),
        }
    }

    /// The picture shrunk by a whole factor (a box average) until neither side is above `max_side`. A picture that is small
    /// enough is returned as it is.
    pub fn limited(&self, max_side: u32) -> Image {
        let largest = self.width.max(self.height);
        if max_side == 0 || largest <= max_side || self.width == 0 || self.height == 0 {
            return self.clone();
        }
        let factor = largest.div_ceil(max_side);
        let (width, height) = ((self.width / factor).max(1), (self.height / factor).max(1));
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0_u32; 4];
                let mut count = 0_u32;
                for dy in 0..factor {
                    for dx in 0..factor {
                        let (sx, sy) = (x * factor + dx, y * factor + dy);
                        if sx >= self.width || sy >= self.height {
                            continue;
                        }
                        let at = ((sy * self.width + sx) * 4) as usize;
                        for (channel, total) in sum.iter_mut().enumerate() {
                            *total += u32::from(self.rgba[at + channel]);
                        }
                        count += 1;
                    }
                }
                let count = count.max(1);
                rgba.extend(sum.iter().map(|total| (total / count) as u8));
            }
        }
        Image { width, height, rgba }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(width: u32, height: u32) -> Image {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&[x as u8, y as u8, 7, 255]);
            }
        }
        Image { width, height, rgba }
    }

    #[test]
    fn a_texture_holds_bgra_bytes_and_the_blend_mode() {
        let image = Image { width: 1, height: 2, rgba: vec![1, 2, 3, 4, 5, 6, 7, 8] };
        let texture = image.to_texture("needle", 0xABCD, true);
        assert_eq!(texture.data, vec![3, 2, 1, 4, 7, 6, 5, 8]);
        assert_eq!((texture.width, texture.height, texture.mip_levels), (1, 2, 1));
        assert_eq!(texture.format, PixelFormat::Argb8888);
        assert_eq!(texture.alpha_blend, 2);
        assert_eq!(image.to_texture("n", 1, false).alpha_blend, 1);
        assert_eq!(texture.name_hash, 0xABCD);
    }

    #[test]
    fn a_small_picture_is_left_alone_and_a_large_one_is_averaged_down() {
        let small = gradient(8, 8);
        assert_eq!(small.limited(16), small);
        let big = gradient(8, 8);
        let half = big.limited(4);
        assert_eq!((half.width, half.height), (4, 4));
        // Each output pixel averages a 2 x 2 block: x 0 and 1 average to 0 (integer), y 0 and 1 to 0.
        assert_eq!(&half.rgba[..4], &[0, 0, 7, 255]);
        // The last output pixel covers x 6, 7 and y 6, 7.
        let last = (3 * 4 + 3) * 4;
        assert_eq!(&half.rgba[last..last + 4], &[6, 6, 7, 255]);
    }

    #[test]
    fn a_zero_limit_changes_nothing() {
        let image = gradient(4, 4);
        assert_eq!(image.limited(0), image);
    }
}
