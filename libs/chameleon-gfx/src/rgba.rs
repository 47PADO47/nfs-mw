//! Decoding the top mip of a texture to 8-bit RGBA, for tools that write pictures (a logo as a PNG) rather than draw.

use crate::texture::{PixelFormat, Texture, mip_size};

/// A block decoder of `texture2ddecoder`: the data, the size, and the `0xAARRGGBB` words it writes.
type Decoder = fn(&[u8], usize, usize, &mut [u32]) -> Result<(), &'static str>;

impl Texture {
    /// The top mip of array slice 0 as tightly packed RGBA8 (R, G, B, A per pixel, rows top to bottom), or `None` when the
    /// format is not one of the readable ones or the data is shorter than the mip.
    ///
    /// BC1 is read with one-bit alpha: a block in the three-colour mode has transparent texels, which is how the 2012
    /// game's cut-out pictures are stored. A picture with no such block is opaque.
    pub fn to_rgba8(&self) -> Option<Vec<u8>> {
        let (w, h) = (self.width as usize, self.height as usize);
        let size = mip_size(self.format, self.width, self.height, 0)?;
        let data = self.data.get(..size)?;
        let pixels = w.checked_mul(h)?;
        let block_decoded = |decode: Decoder| {
            let mut words = vec![0u32; pixels];
            decode(data, w, h, &mut words).ok()?;
            // The decoder writes 0xAARRGGBB words.
            Some(
                words
                    .iter()
                    .flat_map(|word| {
                        let [b, g, r, a] = word.to_le_bytes();
                        [r, g, b, a]
                    })
                    .collect(),
            )
        };
        match self.format {
            PixelFormat::Bc1 => block_decoded(texture2ddecoder::decode_bc1a),
            PixelFormat::Bc2 => block_decoded(texture2ddecoder::decode_bc2),
            PixelFormat::Bc3 => block_decoded(texture2ddecoder::decode_bc3),
            PixelFormat::Bc4 => block_decoded(texture2ddecoder::decode_bc4),
            PixelFormat::Bc5 => block_decoded(texture2ddecoder::decode_bc5),
            PixelFormat::Bc7 => block_decoded(texture2ddecoder::decode_bc7),
            PixelFormat::Rgba8 => Some(data.to_vec()),
            PixelFormat::Bgra8 => Some(data.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], p[3]]).collect()),
            PixelFormat::R8 => Some(data.iter().flat_map(|&v| [v, v, v, 255]).collect()),
            PixelFormat::Other(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texture(dxgi_format: u32, width: u32, height: u32, data: Vec<u8>) -> Texture {
        let format = PixelFormat::from_dxgi(dxgi_format);
        Texture { width, height, depth: 1, array_size: 1, mip_levels: 1, dxgi_format, format, data }
    }

    #[test]
    fn a_bc3_block_with_a_flat_colour_and_alpha_decodes_to_that_colour() {
        // Alpha endpoints 255 and 0 with every index 0 (255), colour endpoints pure red (RGB565 0xF800) for both.
        let mut block = vec![255, 0, 0, 0, 0, 0, 0, 0];
        block.extend_from_slice(&[0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0]);
        let rgba = texture(77, 4, 4, block).to_rgba8().unwrap();
        assert_eq!(rgba.len(), 4 * 4 * 4);
        assert!(rgba.chunks(4).all(|p| p == [255, 0, 0, 255]));
    }

    #[test]
    fn a_three_colour_bc1_block_is_transparent_where_it_uses_index_three() {
        let block = vec![0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let rgba = texture(71, 4, 4, block).to_rgba8().unwrap();
        assert!(rgba.chunks(4).all(|p| p[3] == 0));
    }

    #[test]
    fn plain_formats_are_reordered_and_short_data_is_refused() {
        assert_eq!(texture(87, 1, 1, vec![1, 2, 3, 4]).to_rgba8().unwrap(), [3, 2, 1, 4]);
        assert_eq!(texture(61, 2, 1, vec![9, 7]).to_rgba8().unwrap(), [9, 9, 9, 255, 7, 7, 7, 255]);
        assert!(texture(77, 8, 8, vec![0; 16]).to_rgba8().is_none());
        assert!(texture(1, 4, 4, vec![0; 64]).to_rgba8().is_none());
    }
}
