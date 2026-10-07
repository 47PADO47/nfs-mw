//! Pixel formats and alpha modes.

/// Pixel formats of the PC builds (D3D9 `D3DFORMAT` values).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    /// D3DFMT_A8R8G8B8 (21): bytes B, G, R, A.
    Argb8888,
    /// D3DFMT_P8 (41): one palette index per pixel.
    P8,
    /// Anything else, as the raw D3DFORMAT / FourCC value.
    Other(u32),
}

impl PixelFormat {
    pub fn from_d3d(format: u32) -> Self {
        match &format.to_le_bytes() {
            b"DXT1" => Self::Dxt1,
            b"DXT3" => Self::Dxt3,
            b"DXT5" => Self::Dxt5,
            _ => match format {
                21 => Self::Argb8888,
                41 => Self::P8,
                other => Self::Other(other),
            },
        }
    }

    /// Bytes per 4×4 block for block-compressed formats.
    pub fn block_bytes(self) -> Option<usize> {
        match self {
            Self::Dxt1 => Some(8),
            Self::Dxt3 | Self::Dxt5 => Some(16),
            _ => None,
        }
    }
}

/// How the engine treats a texture's alpha channel (`TextureAlphaUsageType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaUsage {
    None,
    PunchThrough,
    Modulated,
    Other(u8),
}

impl From<u8> for AlphaUsage {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::None,
            1 => Self::PunchThrough,
            2 => Self::Modulated,
            other => Self::Other(other),
        }
    }
}
