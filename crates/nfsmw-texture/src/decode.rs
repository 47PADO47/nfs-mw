//! CPU decoding to RGBA8, for previews and for GPUs/backends without BC support.

use crate::{PixelFormat, Texture};

/// Byte size of one mip level (0 = largest).
pub fn mip_level_size(format: PixelFormat, width: u32, height: u32, level: u32) -> usize {
    let w = (width >> level).max(1) as usize;
    let h = (height >> level).max(1) as usize;
    match format {
        PixelFormat::Dxt1 | PixelFormat::Dxt3 | PixelFormat::Dxt5 => {
            w.div_ceil(4) * h.div_ceil(4) * format.block_bytes().unwrap_or(0)
        }
        PixelFormat::Argb8888 => w * h * 4,
        PixelFormat::P8 => w * h,
        PixelFormat::Other(_) => 0,
    }
}

/// Decode mip level 0 to tightly packed RGBA8. `None` for unsupported formats or
/// short data.
pub fn decode_rgba8(t: &Texture) -> Option<Vec<u8>> {
    let (w, h) = (t.width as usize, t.height as usize);
    let size = mip_level_size(t.format, t.width, t.height, 0);
    let data = t.data.get(..size)?;
    match t.format {
        PixelFormat::Dxt1 | PixelFormat::Dxt3 | PixelFormat::Dxt5 => {
            let mut px = vec![0u32; w * h];
            let ok = match t.format {
                PixelFormat::Dxt1 => texture2ddecoder::decode_bc1(data, w, h, &mut px),
                PixelFormat::Dxt3 => texture2ddecoder::decode_bc2(data, w, h, &mut px),
                _ => texture2ddecoder::decode_bc3(data, w, h, &mut px),
            };
            ok.ok()?;
            // texture2ddecoder writes 0xAARRGGBB words.
            Some(
                px.iter()
                    .flat_map(|p| {
                        let [b, g, r, a] = p.to_le_bytes();
                        [r, g, b, a]
                    })
                    .collect(),
            )
        }
        PixelFormat::Argb8888 => Some(data.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0], p[3]]).collect()),
        PixelFormat::P8 => {
            // Unconfirmed: palette entries assumed to be 4 bytes, B G R A like D3DCOLOR.
            let pal = &t.palette;
            Some(
                data.iter()
                    .flat_map(|&i| {
                        let o = usize::from(i) * 4;
                        match pal.get(o..o + 4) {
                            Some(p) => [p[2], p[1], p[0], p[3]],
                            None => [255, 0, 255, 255],
                        }
                    })
                    .collect(),
            )
        }
        PixelFormat::Other(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(mip_level_size(PixelFormat::Dxt1, 256, 256, 0), 32768);
        assert_eq!(mip_level_size(PixelFormat::Dxt3, 256, 256, 0), 65536);
        assert_eq!(mip_level_size(PixelFormat::Dxt1, 8, 8, 3), 8); // 1x1 still one block
        assert_eq!(mip_level_size(PixelFormat::Argb8888, 4, 2, 0), 32);
    }

    #[test]
    fn argb_is_swizzled() {
        let t = Texture {
            name: String::new(),
            name_hash: 0,
            width: 1,
            height: 1,
            mip_levels: 1,
            format: PixelFormat::Argb8888,
            compression_type: 32,
            alpha_usage: crate::AlphaUsage::None,
            alpha_blend: 0,
            data: vec![1, 2, 3, 4],
            palette: Vec::new(),
        };
        assert_eq!(decode_rgba8(&t).unwrap(), [3, 2, 1, 4]);
    }
}
