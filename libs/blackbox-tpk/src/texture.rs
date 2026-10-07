//! The decoded data model.

use crate::{AlphaUsage, PixelFormat, mip_level_size};

#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    /// Name as stored (truncated to 23 characters by the format).
    pub name: String,
    /// `bStringHash` of the full name.
    pub name_hash: u32,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub format: PixelFormat,
    /// `TextureCompressionType` byte from the info record (e.g. 0x22 = DXT1).
    pub compression_type: u8,
    pub alpha_usage: AlphaUsage,
    /// `ApplyAlphaSorting`: the engine sorts and blends this texture (true transparency).
    pub alpha_sorting: bool,
    pub alpha_blend: u8,
    /// All mip levels, largest first, as stored.
    pub data: Vec<u8>,
    /// Palette for [`PixelFormat::P8`], as stored.
    pub palette: Vec<u8>,
}

impl Texture {
    /// The bytes of one mip level (0 = largest), if present.
    pub fn mip(&self, level: u32) -> Option<&[u8]> {
        if level >= self.mip_levels {
            return None;
        }
        let start: usize = (0..level).map(|l| mip_level_size(self.format, self.width, self.height, l)).sum();
        let size = mip_level_size(self.format, self.width, self.height, level);
        self.data.get(start..start + size)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TexturePack {
    pub name: String,
    pub filename: String,
    /// `Version` from `TexturePackInfoHeader`.
    pub version: u32,
    pub textures: Vec<Texture>,
    /// Textures that could not be read (e.g. an unsupported compressor), with the reason.
    pub failed: Vec<(u32, String)>,
}
