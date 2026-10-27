//! Writing a [`Texture`] as a `.dds` file. Spec: `docs/formats/chameleon-gfx.md` (the DDS section); the file layout is
//! Microsoft's public DDS documentation.
//!
//! The texture's graphics block is already in DDS order (every mip of a slice, largest first, slices one after the
//! other, tightly packed), so the pixels are copied as they are and only a header is added.

use crate::error::{Error, Result};
use crate::texture::Texture;

const MAGIC: &[u8; 4] = b"DDS ";
const HEADER_SIZE: u32 = 124;
const PIXEL_FORMAT_SIZE: u32 = 32;

const FLAGS_CAPS: u32 = 0x1;
const FLAGS_HEIGHT: u32 = 0x2;
const FLAGS_WIDTH: u32 = 0x4;
const FLAGS_PITCH: u32 = 0x8;
const FLAGS_PIXEL_FORMAT: u32 = 0x1000;
const FLAGS_MIP_COUNT: u32 = 0x2_0000;
const FLAGS_LINEAR_SIZE: u32 = 0x8_0000;

const PIXEL_FORMAT_FOUR_CC: u32 = 0x4;

const CAPS_COMPLEX: u32 = 0x8;
const CAPS_TEXTURE: u32 = 0x1000;
const CAPS_MIP_MAP: u32 = 0x40_0000;

/// DXGI numbers that have a classic four-character code, so a viewer without DX10 support still opens them.
const DXGI_BC1_UNORM: u32 = 71;
const DXGI_BC2_UNORM: u32 = 74;
const DXGI_BC3_UNORM: u32 = 77;

const DIMENSION_TEXTURE_2D: u32 = 3;

fn four_cc(code: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*code)
}

/// The classic four-character code of a format a one-slice texture can use it for.
fn legacy_four_cc(dxgi_format: u32) -> Option<u32> {
    match dxgi_format {
        DXGI_BC1_UNORM => Some(four_cc(b"DXT1")),
        DXGI_BC2_UNORM => Some(four_cc(b"DXT3")),
        DXGI_BC3_UNORM => Some(four_cc(b"DXT5")),
        _ => None,
    }
}

impl Texture {
    /// The texture as the bytes of a `.dds` file. BC1, BC2 and BC3 single textures get the classic header; everything
    /// else (arrays, sRGB and other formats) gets the `DX10` extension with the texture's own DXGI number.
    ///
    /// Fails for volume textures (depth above 1) and when the graphics block holds fewer bytes than the header says.
    pub fn to_dds(&self) -> Result<Vec<u8>> {
        if self.depth > 1 {
            return Err(Error::Invalid { what: "dds", detail: format!("volume texture of depth {}", self.depth) });
        }
        let slices = self.array_size.max(1) as usize;
        let known_size = self.slice_size().map(|slice| slice * slices);
        let pixels = match known_size {
            Some(size) if size <= self.data.len() => &self.data[..size],
            Some(size) => {
                return Err(Error::Invalid {
                    what: "dds",
                    detail: format!("{size} bytes of pixels expected, the graphics block has {}", self.data.len()),
                });
            }
            None => &self.data[..],
        };

        let classic = legacy_four_cc(self.dxgi_format).filter(|_| slices == 1);
        let mut flags = FLAGS_CAPS | FLAGS_HEIGHT | FLAGS_WIDTH | FLAGS_PIXEL_FORMAT;
        let mut caps = CAPS_TEXTURE;
        if self.mip_levels > 1 {
            flags |= FLAGS_MIP_COUNT;
            caps |= CAPS_COMPLEX | CAPS_MIP_MAP;
        }
        // The top mip's size for block formats, the row pitch for the others, nothing for an unknown format.
        let pitch_or_size = match (self.format.block_bytes(), self.format.is_block_compressed()) {
            (Some(block), true) => {
                flags |= FLAGS_LINEAR_SIZE;
                (self.width.div_ceil(4) * self.height.div_ceil(4)) as usize * block
            }
            (Some(block), false) => {
                flags |= FLAGS_PITCH;
                self.width as usize * block
            }
            (None, _) => 0,
        };

        let mut out = Vec::with_capacity(4 + HEADER_SIZE as usize + 20 + pixels.len());
        out.extend_from_slice(MAGIC);
        let mut put = |value: u32| out.extend_from_slice(&value.to_le_bytes());
        put(HEADER_SIZE);
        put(flags);
        put(self.height);
        put(self.width);
        put(pitch_or_size as u32);
        put(0);
        put(self.mip_levels);
        (0..11).for_each(|_| put(0));
        put(PIXEL_FORMAT_SIZE);
        put(PIXEL_FORMAT_FOUR_CC);
        put(classic.unwrap_or_else(|| four_cc(b"DX10")));
        (0..5).for_each(|_| put(0));
        put(caps);
        (0..4).for_each(|_| put(0));
        if classic.is_none() {
            put(self.dxgi_format);
            put(DIMENSION_TEXTURE_2D);
            put(0);
            put(slices as u32);
            put(0);
        }
        out.extend_from_slice(pixels);
        Ok(out)
    }
}
