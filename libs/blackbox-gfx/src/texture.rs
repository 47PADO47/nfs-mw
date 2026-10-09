//! Texture uploads.

/// Pixel formats a renderer accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// DXT1
    Bc1,
    /// DXT3
    Bc2,
    /// DXT5
    Bc3,
    Rgba8,
}

/// A texture to upload. `mips` holds each level, largest first. Block-compressed
/// textures need a width and height that are multiples of 4.
#[derive(Debug, Clone)]
pub struct TextureDesc<'a> {
    pub label: &'a str,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub mips: Vec<&'a [u8]>,
}
