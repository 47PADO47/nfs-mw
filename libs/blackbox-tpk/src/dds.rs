//! Reading a DDS file as a texture of the pack's data model, so a mod can ship pictures in the format its artists use
//! (`docs/modding/api-replace.md`, "Textures").
//!
//! Block-compressed pictures the engine draws as they are (BC1, BC2 and BC3, which are `DXT1`, `DXT3` and `DXT5` here) pass through
//! byte for byte, mips included. Everything else is decoded to RGBA8 and stored as A8R8G8B8: uncompressed BGRA or RGBA, and the
//! block formats the engine has no slot for (BC4, BC5, BC7). A cube map, a volume, an array and a 24-bit or 16-bit file are refused.

use crate::{AlphaUsage, PixelFormat, Texture, TextureSource, mip_level_size};

/// Why a DDS file is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DdsError {
    #[error("not a DDS file (no `DDS ` magic or a short header)")]
    NotDds,
    #[error("the file is a cube map, a volume or an array: only plain 2D pictures are read")]
    NotFlat,
    #[error("the pixel format ({0}) is not read")]
    Unsupported(String),
    #[error("the picture is {0} by {1}: a side is 0 or above {2}")]
    Size(u32, u32, u32),
    #[error("the file holds {have} bytes of pixels, the first level needs {need}")]
    Short { have: usize, need: usize },
}

const MAGIC: &[u8; 4] = b"DDS ";
const HEADER_END: usize = 128;
const DX10_END: usize = 148;
/// The most a side may have: the pack's own limit.
const MAX_SIDE: u32 = crate::MAX_DIMENSION;

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// What the pixels are, from the header.
enum Pixels {
    Block(PixelFormat),
    /// Decode with this function of the block format (BC4, BC5, BC7).
    Decode(&'static str),
    Bgra,
    Rgba,
}

fn pixels(header: &[u8], dx10: Option<u32>) -> Result<Pixels, DdsError> {
    let flags = word(header, 80);
    let four_cc = &header[84..88];
    if let Some(dxgi) = dx10 {
        return match dxgi {
            71 | 72 => Ok(Pixels::Block(PixelFormat::Dxt1)),
            74 | 75 => Ok(Pixels::Block(PixelFormat::Dxt3)),
            77 | 78 => Ok(Pixels::Block(PixelFormat::Dxt5)),
            80 | 81 => Ok(Pixels::Decode("bc4")),
            83 | 84 => Ok(Pixels::Decode("bc5")),
            98 | 99 => Ok(Pixels::Decode("bc7")),
            28 | 29 => Ok(Pixels::Rgba),
            87 | 91 => Ok(Pixels::Bgra),
            other => Err(DdsError::Unsupported(format!("DXGI {other}"))),
        };
    }
    if flags & 0x4 != 0 {
        return match four_cc {
            b"DXT1" => Ok(Pixels::Block(PixelFormat::Dxt1)),
            b"DXT3" => Ok(Pixels::Block(PixelFormat::Dxt3)),
            b"DXT5" => Ok(Pixels::Block(PixelFormat::Dxt5)),
            b"ATI1" | b"BC4U" => Ok(Pixels::Decode("bc4")),
            b"ATI2" | b"BC5U" => Ok(Pixels::Decode("bc5")),
            other => Err(DdsError::Unsupported(String::from_utf8_lossy(other).into_owned())),
        };
    }
    let (bits, red, green, blue) = (word(header, 88), word(header, 92), word(header, 96), word(header, 100));
    match (bits, red, green, blue) {
        (32, 0x00ff_0000, 0x0000_ff00, 0x0000_00ff) => Ok(Pixels::Bgra),
        (32, 0x0000_00ff, 0x0000_ff00, 0x00ff_0000) => Ok(Pixels::Rgba),
        _ => Err(DdsError::Unsupported(format!("{bits} bits per pixel"))),
    }
}

/// The texture of a DDS file, named `name` (its hash is the hash of the name, and the caller sets it again for a replacement).
/// Alpha is read as modulated when the file says it has an alpha channel, else none.
pub fn read_dds(name: &str, bytes: &[u8]) -> Result<Texture, DdsError> {
    if bytes.len() < HEADER_END || &bytes[..4] != MAGIC || word(bytes, 4) != 124 {
        return Err(DdsError::NotDds);
    }
    let header = &bytes[..HEADER_END];
    let (height, width) = (word(header, 12), word(header, 16));
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(DdsError::Size(width, height, MAX_SIDE));
    }
    let caps2 = word(header, 112);
    if caps2 & 0xfe00 != 0 || caps2 & 0x20_0000 != 0 {
        return Err(DdsError::NotFlat);
    }
    let dx10 = (header[84..88] == *b"DX10").then(|| (bytes.len() >= DX10_END).then(|| word(bytes, HEADER_END)));
    let (dx10, start) = match dx10 {
        None => (None, HEADER_END),
        Some(None) => return Err(DdsError::NotDds),
        Some(Some(dxgi)) => {
            if word(bytes, HEADER_END + 12) > 1 || word(bytes, HEADER_END + 8) & 0x4 != 0 {
                return Err(DdsError::NotFlat);
            }
            (Some(dxgi), DX10_END)
        }
    };
    let data = &bytes[start..];
    let max_mips = width.max(height).ilog2() + 1;
    let wanted_mips = word(header, 28).clamp(1, max_mips);
    let has_alpha = word(header, 80) & 0x3 != 0 || dx10.is_some();
    match pixels(header, dx10)? {
        Pixels::Block(format) => {
            let mut mips = 0;
            let mut size = 0;
            while mips < wanted_mips {
                let next = mip_level_size(format, width, height, mips);
                if size + next > data.len() {
                    break;
                }
                size += next;
                mips += 1;
            }
            if mips == 0 {
                return Err(DdsError::Short { have: data.len(), need: mip_level_size(format, width, height, 0) });
            }
            let mut source = TextureSource::new(name, width, height, format, mips, data[..size].to_vec());
            if format != PixelFormat::Dxt1 {
                source.alpha_usage = AlphaUsage::Modulated;
            }
            Ok(source.into_texture())
        }
        Pixels::Bgra | Pixels::Rgba => {
            let need = (width * height * 4) as usize;
            let Some(first) = data.get(..need) else { return Err(DdsError::Short { have: data.len(), need }) };
            let mut out = first.to_vec();
            if matches!(pixels(header, dx10)?, Pixels::Rgba) {
                out.as_chunks_mut::<4>().0.iter_mut().for_each(|px| px.swap(0, 2));
            }
            let mut source = TextureSource::new(name, width, height, PixelFormat::Argb8888, 1, out);
            if has_alpha {
                source.alpha_usage = AlphaUsage::Modulated;
            }
            Ok(source.into_texture())
        }
        Pixels::Decode(which) => {
            let rgba = decode_block(which, data, width, height)?;
            let mut out = Vec::with_capacity(rgba.len());
            out.extend(rgba.as_chunks::<4>().0.iter().flat_map(|&[r, g, b, a]| [b, g, r, a]));
            let mut source = TextureSource::new(name, width, height, PixelFormat::Argb8888, 1, out);
            source.alpha_usage = if which == "bc7" { AlphaUsage::Modulated } else { AlphaUsage::None };
            Ok(source.into_texture())
        }
    }
}

/// The RGBA8 of the first level of a BC4, BC5 or BC7 picture.
fn decode_block(which: &str, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, DdsError> {
    let (w, h) = (width as usize, height as usize);
    let need = (w.div_ceil(4) * h.div_ceil(4)) * if which == "bc4" { 8 } else { 16 };
    let Some(blocks) = data.get(..need) else { return Err(DdsError::Short { have: data.len(), need }) };
    let mut words = vec![0u32; w * h];
    let done = match which {
        "bc4" => texture2ddecoder::decode_bc4(blocks, w, h, &mut words),
        "bc5" => texture2ddecoder::decode_bc5(blocks, w, h, &mut words),
        _ => texture2ddecoder::decode_bc7(blocks, w, h, &mut words),
    };
    if done.is_err() {
        return Err(DdsError::Unsupported(format!("a {which} picture that does not decode")));
    }
    // texture2ddecoder writes 0xAARRGGBB words; BC4 is one channel (red) and BC5 two (red and green).
    Ok(words
        .iter()
        .flat_map(|&px| {
            let [b, g, r, a] = px.to_le_bytes();
            match which {
                "bc4" => [r, r, r, 255],
                "bc5" => [r, g, 0, 255],
                _ => [r, g, b, a],
            }
        })
        .collect())
}

#[cfg(test)]
#[path = "dds_tests.rs"]
mod tests;
