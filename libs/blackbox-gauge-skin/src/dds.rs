//! A DDS file read to RGBA8: the formats skins use (DXT1, DXT3, DXT5 and 32-bit uncompressed with 8-bit channels). The top
//! mip level only. The layout is Microsoft's public DDS documentation.

use blackbox_tpk::{AlphaUsage, PixelFormat, Texture, decode_rgba8, decode_rgba8_cutout};

use crate::image::Image;

const MAGIC: &[u8; 4] = b"DDS ";
const HEADER_SIZE: u32 = 124;
const DATA_AT: usize = 128;
/// The pictures of a HUD are small; a header that claims more is not a skin picture.
const MAX_SIDE: u32 = 8192;
const PIXEL_FORMAT_RGB: u32 = 0x40;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DdsError {
    #[error("not a DDS file")]
    NotDds,
    #[error("the DDS data is cut short")]
    Short,
    #[error("unsupported DDS format: {0}")]
    Unsupported(String),
    #[error("the {0} picture could not be decoded")]
    Decode(&'static str),
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}

/// The bit position of an 8-bit channel mask, or `None` when the mask is not one byte wide.
fn channel_shift(mask: u32) -> Option<u32> {
    if mask == 0 {
        return None;
    }
    let shift = mask.trailing_zeros();
    (mask >> shift == 0xFF).then_some(shift)
}

/// Reads the top mip level of a DDS file.
pub fn decode(bytes: &[u8]) -> Result<Image, DdsError> {
    if bytes.get(..4) != Some(MAGIC.as_slice()) {
        return Err(DdsError::NotDds);
    }
    if word(bytes, 4) != Some(HEADER_SIZE) {
        return Err(DdsError::NotDds);
    }
    let height = word(bytes, 12).ok_or(DdsError::Short)?;
    let width = word(bytes, 16).ok_or(DdsError::Short)?;
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(DdsError::Unsupported(format!("size {width} x {height}")));
    }
    let flags = word(bytes, 80).ok_or(DdsError::Short)?;
    let four_cc = word(bytes, 84).ok_or(DdsError::Short)?;
    match &four_cc.to_le_bytes() {
        b"DXT1" => block_compressed(bytes, width, height, PixelFormat::Dxt1),
        b"DXT3" => block_compressed(bytes, width, height, PixelFormat::Dxt3),
        b"DXT5" => block_compressed(bytes, width, height, PixelFormat::Dxt5),
        [0, 0, 0, 0] if flags & PIXEL_FORMAT_RGB != 0 => uncompressed(bytes, width, height),
        other => Err(DdsError::Unsupported(format!("pixel format {:?}", String::from_utf8_lossy(other)))),
    }
}

fn block_compressed(bytes: &[u8], width: u32, height: u32, format: PixelFormat) -> Result<Image, DdsError> {
    let block = format.block_bytes().ok_or(DdsError::Decode("block"))?;
    let size = (width as usize).div_ceil(4) * (height as usize).div_ceil(4) * block;
    let data = bytes.get(DATA_AT..DATA_AT + size).ok_or(DdsError::Short)?;
    let texture = Texture {
        name: String::new(),
        name_hash: 0,
        width,
        height,
        mip_levels: 1,
        format,
        compression_type: 0,
        alpha_usage: AlphaUsage::Modulated,
        alpha_sorting: false,
        alpha_blend: 1,
        flags: 0,
        tilable_uv: 0,
        data: data.to_vec(),
        palette: Vec::new(),
    };
    // A one-bit-alpha DXT1 picture keeps its transparent texels, as the D3DX loader gives them.
    let rgba = match format {
        PixelFormat::Dxt1 => decode_rgba8_cutout(&texture),
        _ => decode_rgba8(&texture),
    };
    let rgba = rgba.ok_or(DdsError::Decode("DXT"))?;
    Ok(Image { width, height, rgba })
}

fn uncompressed(bytes: &[u8], width: u32, height: u32) -> Result<Image, DdsError> {
    let bits = word(bytes, 88).ok_or(DdsError::Short)?;
    if bits != 32 {
        return Err(DdsError::Unsupported(format!("{bits}-bit pixels")));
    }
    let masks = [92, 96, 100, 104].map(|at| word(bytes, at));
    let [Some(red), Some(green), Some(blue), Some(alpha)] = masks else {
        return Err(DdsError::Short);
    };
    let shifts = [channel_shift(red), channel_shift(green), channel_shift(blue)];
    let [Some(r), Some(g), Some(b)] = shifts else {
        return Err(DdsError::Unsupported("channel masks".to_owned()));
    };
    let alpha_shift = channel_shift(alpha);
    let size = width as usize * height as usize * 4;
    let data = bytes.get(DATA_AT..DATA_AT + size).ok_or(DdsError::Short)?;
    let mut rgba = Vec::with_capacity(size);
    for pixel in data.as_chunks::<4>().0 {
        let value = u32::from_le_bytes(*pixel);
        let channel = |shift: u32| ((value >> shift) & 0xFF) as u8;
        rgba.extend_from_slice(&[channel(r), channel(g), channel(b), alpha_shift.map_or(255, channel)]);
    }
    Ok(Image { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header for a `width` x `height` picture, with the pixel format words given.
    fn header(width: u32, height: u32, flags: u32, four_cc: &[u8; 4], bits: u32, masks: [u32; 4]) -> Vec<u8> {
        let mut out = vec![0_u8; DATA_AT];
        out[..4].copy_from_slice(MAGIC);
        out[4..8].copy_from_slice(&HEADER_SIZE.to_le_bytes());
        out[12..16].copy_from_slice(&height.to_le_bytes());
        out[16..20].copy_from_slice(&width.to_le_bytes());
        out[76..80].copy_from_slice(&32_u32.to_le_bytes());
        out[80..84].copy_from_slice(&flags.to_le_bytes());
        out[84..88].copy_from_slice(four_cc);
        out[88..92].copy_from_slice(&bits.to_le_bytes());
        for (index, mask) in masks.iter().enumerate() {
            let at = 92 + index * 4;
            out[at..at + 4].copy_from_slice(&mask.to_le_bytes());
        }
        out
    }

    #[test]
    fn an_a8r8g8b8_picture_reads_its_channels() {
        // Bytes are B, G, R, A in memory for the mask layout 0x00FF0000 / 0x0000FF00 / 0x000000FF / 0xFF000000.
        let mut file =
            header(2, 1, PIXEL_FORMAT_RGB | 1, &[0; 4], 32, [0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000]);
        file.extend_from_slice(&[10, 20, 30, 40, 1, 2, 3, 255]);
        let image = decode(&file).expect("a valid picture");
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.rgba, vec![30, 20, 10, 40, 3, 2, 1, 255]);
    }

    #[test]
    fn a_picture_with_no_alpha_mask_is_opaque() {
        let mut file = header(1, 1, PIXEL_FORMAT_RGB, &[0; 4], 32, [0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0]);
        file.extend_from_slice(&[5, 6, 7, 0]);
        assert_eq!(decode(&file).expect("valid").rgba, vec![7, 6, 5, 255]);
    }

    #[test]
    fn a_dxt5_block_decodes_to_a_four_by_four_picture() {
        // One DXT5 block: alpha endpoints 255 and 0 with every index 0 (255), colour endpoints both pure red (0xF800), index 0.
        let mut file = header(4, 4, 4, b"DXT5", 0, [0; 4]);
        file.extend_from_slice(&[255, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF8, 0x00, 0xF8, 0, 0, 0, 0]);
        let image = decode(&file).expect("a valid DXT5 block");
        assert_eq!((image.width, image.height), (4, 4));
        assert_eq!(image.rgba.len(), 4 * 4 * 4);
        assert!(image.rgba[0] >= 248, "red is full");
        assert_eq!(&image.rgba[1..4], &[0, 0, 255], "no green or blue, opaque");
    }

    #[test]
    fn bad_files_are_reported_not_drawn() {
        assert_eq!(decode(b"nope"), Err(DdsError::NotDds));
        assert_eq!(decode(&[]), Err(DdsError::NotDds));
        let truncated = header(4, 4, 4, b"DXT1", 0, [0; 4]);
        assert_eq!(decode(&truncated), Err(DdsError::Short));
        let sixteen = header(1, 1, PIXEL_FORMAT_RGB, &[0; 4], 16, [0xF800, 0x07E0, 0x001F, 0]);
        assert!(matches!(decode(&sixteen), Err(DdsError::Unsupported(_))));
        let dx10 = header(4, 4, 4, b"DX10", 0, [0; 4]);
        assert!(matches!(decode(&dx10), Err(DdsError::Unsupported(_))));
        let empty = header(0, 4, 4, b"DXT1", 0, [0; 4]);
        assert!(matches!(decode(&empty), Err(DdsError::Unsupported(_))));
    }
}
