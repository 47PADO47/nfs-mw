//! The input of the writer: one texture, its pixels already encoded and stored as the file holds them.

use blackbox_hash::bstring_hash;

use super::error::{WriteError, WriteResult};
use crate::{AlphaUsage, PixelFormat, Texture, mip_level_size};

/// Longest name the `TextureInfo` record holds (24 bytes, NUL-terminated). Longer names are cut in the record
/// but hashed whole, exactly as the game's own packs do.
pub const MAX_STORED_NAME: usize = 23;

/// Largest width or height: `TextureInfo` stores them as `i16`.
pub const MAX_DIMENSION: u32 = 0x7FFF;

/// `ApplyAlphaSorting` and friends default to "opaque, drawn in the normal pass"; the rendering order the game's
/// own car textures carry is 5 (`TextureInfo.RenderingOrder`).
pub const DEFAULT_RENDERING_ORDER: u8 = 5;

/// One texture to put into a pack.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureSource {
    /// Name as the artists wrote it. The record keeps the first 23 bytes; the hash covers the whole name.
    pub name: String,
    /// Overrides `bStringHash(name)`. Set when re-writing a parsed [`Texture`], whose name may be truncated.
    pub name_hash: Option<u32>,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    /// Number of mip levels stored in `data` (at least 1).
    pub mip_count: u32,
    /// All mip levels back to back, largest first, exactly as stored in the file.
    pub data: Vec<u8>,
    /// Palette for [`PixelFormat::P8`] (typically 256 BGRA entries), empty otherwise.
    pub palette: Vec<u8>,
    /// `ImageCompressionType`; [`TextureSource::new`] picks the usual byte for the format.
    pub compression_type: u8,
    pub alpha_usage: AlphaUsage,
    /// `ApplyAlphaSorting`.
    pub alpha_sorting: bool,
    /// `AlphaBlendType` (0 copy, 1 blend, 2 additive).
    pub alpha_blend: u8,
    /// The info record's `Flags` byte (bit 1 = two-sided).
    pub flags: u8,
    /// `RenderingOrder`.
    pub rendering_order: u8,
}

/// The `TEXCOMP_*` byte the game's packs use for `format`.
pub fn default_compression_type(format: PixelFormat) -> u8 {
    match format {
        PixelFormat::Dxt1 => 0x22,
        PixelFormat::Dxt3 => 0x24,
        PixelFormat::Dxt5 => 0x26,
        PixelFormat::Argb8888 => 0x20,
        PixelFormat::P8 => 0x08,
        PixelFormat::Other(_) => 0,
    }
}

impl TextureSource {
    /// A texture with every optional field at its default: opaque, no flags, no palette.
    pub fn new(
        name: impl Into<String>,
        width: u32,
        height: u32,
        format: PixelFormat,
        mip_count: u32,
        data: Vec<u8>,
    ) -> Self {
        Self {
            name: name.into(),
            name_hash: None,
            width,
            height,
            format,
            mip_count,
            data,
            palette: Vec::new(),
            compression_type: default_compression_type(format),
            alpha_usage: AlphaUsage::None,
            alpha_sorting: false,
            alpha_blend: 0,
            flags: 0,
            rendering_order: DEFAULT_RENDERING_ORDER,
        }
    }

    /// The hash the pack files this texture under.
    pub fn hash(&self) -> u32 {
        self.name_hash.unwrap_or_else(|| bstring_hash(&self.name))
    }

    /// The name bytes the record stores (cut to [`MAX_STORED_NAME`]).
    pub(super) fn stored_name(&self) -> &[u8] {
        let bytes = self.name.as_bytes();
        &bytes[..bytes.len().min(MAX_STORED_NAME)]
    }

    fn fail(&self, detail: impl Into<String>) -> WriteError {
        WriteError::Texture { name: self.name.clone(), detail: detail.into() }
    }

    /// Checks everything the reader and the engine rely on.
    pub(super) fn validate(&self) -> WriteResult<()> {
        if self.name.is_empty() {
            return Err(self.fail("empty name"));
        }
        if !self.name.is_ascii() || self.name.bytes().any(|b| b == 0) {
            return Err(self.fail("name must be ASCII without NUL"));
        }
        if self.width == 0 || self.height == 0 || self.width > MAX_DIMENSION || self.height > MAX_DIMENSION {
            return Err(self.fail(format!("size {}x{} is outside 1..={MAX_DIMENSION}", self.width, self.height)));
        }
        if matches!(self.format, PixelFormat::Other(_)) {
            return Err(self.fail("pixel format has no known size"));
        }
        let max_mips = self.width.max(self.height).ilog2() + 1;
        if self.mip_count == 0 || self.mip_count > max_mips {
            return Err(self.fail(format!("mip count {} is outside 1..={max_mips}", self.mip_count)));
        }
        let expected = self.expected_data_len();
        if self.data.len() != expected {
            return Err(self.fail(format!(
                "{} bytes of pixels for {} mip(s) of {:?}, expected {expected}",
                self.data.len(),
                self.mip_count,
                self.format
            )));
        }
        let has_palette = !self.palette.is_empty();
        if (self.format == PixelFormat::P8) != has_palette {
            return Err(self.fail("a palette is required for P8 and not allowed for other formats"));
        }
        if self.palette.len() % 4 != 0 || self.palette.len() / 4 > 256 {
            return Err(self.fail("palette must be 1 to 256 four-byte entries"));
        }
        Ok(())
    }

    /// Bytes `data` must hold: the sum of the mip levels' sizes.
    pub fn expected_data_len(&self) -> usize {
        (0..self.mip_count).map(|l| mip_level_size(self.format, self.width, self.height, l)).sum()
    }
}

/// Re-write a parsed texture: keeps its hash (the parsed name may be truncated) and every field the reader exposes.
impl From<&Texture> for TextureSource {
    fn from(t: &Texture) -> Self {
        Self {
            name: t.name.clone(),
            name_hash: Some(t.name_hash),
            width: t.width,
            height: t.height,
            format: t.format,
            mip_count: t.mip_levels,
            data: t.data.clone(),
            palette: t.palette.clone(),
            compression_type: t.compression_type,
            alpha_usage: t.alpha_usage,
            alpha_sorting: t.alpha_sorting,
            alpha_blend: t.alpha_blend,
            flags: t.flags,
            rendering_order: DEFAULT_RENDERING_ORDER,
        }
    }
}
