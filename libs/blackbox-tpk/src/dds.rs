//! Reading a DDS file's pixels, to stand in for a pack texture: a replacement picture a player supplies keeps the
//! texture's identity and render state and brings its own size, format and mip chain.
//!
//! Supported: DXT1, DXT3 and DXT5 (FourCC, or the matching DXGI BC1 to BC3 formats of a DX10 header) and 32-bit
//! A8R8G8B8. Cube maps and volumes are refused.

use crate::{Error, PixelFormat, Result, Texture, mip_level_size};

const HEADER: usize = 128;
const DX10_HEADER: usize = 20;
const FOURCC: u32 = 0x4;
const CUBE_OR_VOLUME: u32 = 0x0020_0000 | 0x0000_0200;

/// The pixels of a DDS file: every stored mip level, largest first.
#[derive(Debug, Clone, PartialEq)]
pub struct DdsImage {
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub format: PixelFormat,
    pub data: Vec<u8>,
}

fn refuse(detail: &str) -> Error {
    Error::Pack { offset: 0, detail: format!("DDS: {detail}") }
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn dx10_format(dxgi: u32) -> Option<PixelFormat> {
    match dxgi {
        70..=72 => Some(PixelFormat::Dxt1),
        73..=75 => Some(PixelFormat::Dxt3),
        76..=78 => Some(PixelFormat::Dxt5),
        _ => None,
    }
}

/// Parse a DDS file. Mip levels the file declares but does not hold are dropped.
pub fn read_dds(bytes: &[u8]) -> Result<DdsImage> {
    if bytes.len() < HEADER || &bytes[..4] != b"DDS " {
        return Err(refuse("not a DDS file"));
    }
    let (height, width) = (word(bytes, 12), word(bytes, 16));
    if width == 0 || height == 0 || width > 16384 || height > 16384 {
        return Err(refuse("the size is out of range"));
    }
    if word(bytes, 112) & CUBE_OR_VOLUME != 0 {
        return Err(refuse("cube maps and volumes are not supported"));
    }
    let fourcc = word(bytes, 84);
    let compressed = word(bytes, 80) & FOURCC != 0;
    let dx10 = compressed && fourcc == u32::from_le_bytes(*b"DX10");
    let mut start = HEADER;
    let format = if dx10 {
        if bytes.len() < HEADER + DX10_HEADER {
            return Err(refuse("the DX10 header is cut short"));
        }
        start += DX10_HEADER;
        dx10_format(word(bytes, HEADER)).ok_or_else(|| refuse("this DXGI format is not supported"))?
    } else if compressed {
        PixelFormat::from_d3d(fourcc)
    } else if word(bytes, 88) == 32 && word(bytes, 92) == 0x00FF_0000 && word(bytes, 104) == 0xFF00_0000 {
        PixelFormat::Argb8888
    } else {
        return Err(refuse("only DXT1, DXT3, DXT5 and A8R8G8B8 are supported"));
    };
    if matches!(format, PixelFormat::Other(_) | PixelFormat::P8) {
        return Err(refuse("only DXT1, DXT3, DXT5 and A8R8G8B8 are supported"));
    }
    let declared = word(bytes, 28).max(1);
    let available = bytes.len() - start;
    let mut used = 0;
    let mut mip_levels = 0;
    for level in 0..declared.min(15) {
        let size = mip_level_size(format, width, height, level);
        if used + size > available {
            break;
        }
        used += size;
        mip_levels += 1;
    }
    if mip_levels == 0 {
        return Err(refuse("the first mip level is cut short"));
    }
    Ok(DdsImage { width, height, mip_levels, format, data: bytes[start..start + used].to_vec() })
}

impl DdsImage {
    /// Drop the largest mip levels while a side is over `max` pixels, as long as a smaller stored level remains: a size
    /// cap that costs no re-encoding. `0` is no cap. A file without the smaller levels keeps its size. Returns how many
    /// levels were dropped.
    pub fn limit_size(&mut self, max: u32) -> u32 {
        let mut dropped = 0;
        while max > 0 && self.width.max(self.height) > max && self.mip_levels > 1 {
            let size = mip_level_size(self.format, self.width, self.height, 0);
            self.data.drain(..size);
            self.width = (self.width / 2).max(1);
            self.height = (self.height / 2).max(1);
            self.mip_levels -= 1;
            dropped += 1;
        }
        dropped
    }
}

impl Texture {
    /// Take the pixels of `image` and keep the name, the alpha rules and the flags.
    pub fn replace_image(&mut self, image: DdsImage) {
        self.width = image.width;
        self.height = image.height;
        self.mip_levels = image.mip_levels;
        self.format = image.format;
        self.data = image.data;
        self.palette.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dds(fourcc: &[u8; 4], width: u32, height: u32, mips: u32, body: usize) -> Vec<u8> {
        let mut out = vec![0u8; HEADER];
        out[..4].copy_from_slice(b"DDS ");
        out[12..16].copy_from_slice(&height.to_le_bytes());
        out[16..20].copy_from_slice(&width.to_le_bytes());
        out[28..32].copy_from_slice(&mips.to_le_bytes());
        out[80..84].copy_from_slice(&FOURCC.to_le_bytes());
        out[84..88].copy_from_slice(fourcc);
        out.extend((0..body).map(|i| i as u8));
        out
    }

    #[test]
    fn a_dxt1_file_gives_its_whole_mip_chain() {
        // 8x8 DXT1: 4 blocks, then 1, 1 and 1.
        let image = read_dds(&dds(b"DXT1", 8, 8, 4, 32 + 8 + 8 + 8)).unwrap();
        assert_eq!((image.width, image.height, image.mip_levels, image.format), (8, 8, 4, PixelFormat::Dxt1));
        assert_eq!(image.data.len(), 56);
        assert_eq!(image.data[0], 0);
    }

    #[test]
    fn mip_levels_the_file_does_not_hold_are_dropped() {
        let image = read_dds(&dds(b"DXT5", 8, 8, 4, 64 + 16)).unwrap();
        assert_eq!(image.mip_levels, 2);
        assert_eq!(image.data.len(), 80);
    }

    #[test]
    fn a_size_cap_drops_the_largest_stored_levels() {
        // 16x8 DXT1: 8 blocks, then 2, 1, 1 and 1.
        let mut image = read_dds(&dds(b"DXT1", 16, 8, 5, 64 + 16 + 8 + 8 + 8)).unwrap();
        assert_eq!(image.limit_size(4), 2);
        assert_eq!((image.width, image.height, image.mip_levels, image.data.len()), (4, 2, 3, 24));
        assert_eq!(image.data[0], 80);
        assert_eq!(image.limit_size(0), 0);
        let mut single = read_dds(&dds(b"DXT1", 16, 8, 1, 64)).unwrap();
        assert_eq!(single.limit_size(4), 0);
        assert_eq!((single.width, single.height), (16, 8));
    }

    #[test]
    fn other_files_are_refused() {
        assert!(read_dds(b"not a dds").is_err());
        assert!(read_dds(&dds(b"ATI2", 8, 8, 1, 64)).is_err());
        assert!(read_dds(&dds(b"DXT1", 8, 8, 1, 4)).is_err());
    }
}
