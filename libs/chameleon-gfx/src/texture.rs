use crate::bytes::Block;
use crate::error::{Error, Result};

/// Pixel formats of the texture header (the value is a DXGI format number).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgba8,
    Bgra8,
    R8,
    Bc1,
    Bc2,
    Bc3,
    Bc4,
    Bc5,
    Bc7,
    Other(u32),
}

impl PixelFormat {
    pub const fn from_dxgi(format: u32) -> Self {
        match format {
            28 | 29 => Self::Rgba8,
            87 | 91 => Self::Bgra8,
            61 => Self::R8,
            71 | 72 => Self::Bc1,
            74 | 75 => Self::Bc2,
            77 | 78 => Self::Bc3,
            80 | 81 => Self::Bc4,
            83 | 84 => Self::Bc5,
            98 | 99 => Self::Bc7,
            other => Self::Other(other),
        }
    }

    /// Bytes of one block (4x4 pixels for the block formats, one pixel for the others), or `None` when unknown.
    pub const fn block_bytes(self) -> Option<usize> {
        match self {
            Self::Rgba8 | Self::Bgra8 => Some(4),
            Self::R8 => Some(1),
            Self::Bc1 | Self::Bc4 => Some(8),
            Self::Bc2 | Self::Bc3 | Self::Bc5 | Self::Bc7 => Some(16),
            Self::Other(_) => None,
        }
    }

    pub const fn is_block_compressed(self) -> bool {
        matches!(self, Self::Bc1 | Self::Bc2 | Self::Bc3 | Self::Bc4 | Self::Bc5 | Self::Bc7)
    }
}

/// A texture: its header and the pixels of every mip level, largest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub array_size: u32,
    pub mip_levels: u32,
    /// The raw DXGI format number.
    pub dxgi_format: u32,
    pub format: PixelFormat,
    /// The graphics block: all mips of all array slices.
    pub data: Vec<u8>,
}

/// Bytes of one mip level of a `width` x `height` image.
pub fn mip_size(format: PixelFormat, width: u32, height: u32, level: u32) -> Option<usize> {
    let (w, h) = ((width >> level).max(1) as usize, (height >> level).max(1) as usize);
    let block = format.block_bytes()?;
    if format.is_block_compressed() {
        return Some(w.div_ceil(4) * h.div_ceil(4) * block);
    }
    Some(w * h * block)
}

impl Texture {
    /// Decode the header in the main block and keep the pixels of the graphics block.
    pub fn parse(main: &[u8], graphics: &[u8]) -> Result<Self> {
        let block = Block::new(main, "texture");
        let dxgi_format = block.u32(0x1C)?;
        let (width, height) = (u32::from(block.u16(0x24)?), u32::from(block.u16(0x26)?));
        let (depth, array_size) = (u32::from(block.u16(0x28)?), u32::from(block.u16(0x2A)?));
        let mip_levels = u32::from(block.u8(0x2D)?);
        if width == 0 || height == 0 || mip_levels == 0 {
            return Err(Error::Invalid { what: "texture", detail: format!("{width}x{height} with {mip_levels} mips") });
        }
        let format = PixelFormat::from_dxgi(dxgi_format);
        Ok(Self { width, height, depth, array_size, mip_levels, dxgi_format, format, data: graphics.to_vec() })
    }

    /// The bytes of mip `level` of array slice 0, if the data holds it.
    pub fn mip(&self, level: u32) -> Option<&[u8]> {
        let mut start = 0;
        for earlier in 0..level {
            start += mip_size(self.format, self.width, self.height, earlier)?;
        }
        let len = mip_size(self.format, self.width, self.height, level)?;
        self.data.get(start..start + len)
    }

    /// Bytes of all mips of one array slice.
    pub fn slice_size(&self) -> Option<usize> {
        (0..self.mip_levels)
            .try_fold(0, |sum, level| Some(sum + mip_size(self.format, self.width, self.height, level)?))
    }
}
