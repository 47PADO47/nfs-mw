//! A legacy `.dds` file around block-compressed pixels: the 128-byte header, then the mips largest first. It is the file the game's
//! replacement API reads (`docs/modding/api-replace.md`, "Textures"): `DXT1` and `DXT5` pass through with their mips.

/// The header of a `width` by `height` picture of `mips` levels in the block format `four_cc` (`DXT1` or `DXT5`), whose first
/// level takes `top_size` bytes.
pub fn header(width: u32, height: u32, mips: u32, four_cc: [u8; 4], top_size: usize) -> [u8; 128] {
    let mut out = [0u8; 128];
    let mut put = |at: usize, value: u32| out[at..at + 4].copy_from_slice(&value.to_le_bytes());
    put(4, 124);
    // CAPS | HEIGHT | WIDTH | PIXELFORMAT | MIPMAPCOUNT | LINEARSIZE
    put(8, 0x1007 | 0x2_0000 | 0x8_0000);
    put(12, height);
    put(16, width);
    put(20, top_size as u32);
    put(28, mips);
    put(76, 32);
    put(80, 0x4);
    // TEXTURE | COMPLEX | MIPMAP
    put(108, 0x1000 | 0x8 | 0x40_0000);
    out[0..4].copy_from_slice(b"DDS ");
    out[84..88].copy_from_slice(&four_cc);
    out
}

/// The whole file: the header and the mips (`data`, all levels back to back).
pub fn file(width: u32, height: u32, mips: u32, four_cc: [u8; 4], top_size: usize, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(128 + data.len());
    out.extend_from_slice(&header(width, height, mips, four_cc, top_size));
    out.extend_from_slice(data);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_is_128_bytes_and_names_the_size_and_the_format() {
        let file = file(8, 4, 2, *b"DXT1", 16, &[0; 24]);
        assert_eq!(file.len(), 128 + 24);
        assert_eq!(&file[0..4], b"DDS ");
        assert_eq!(u32::from_le_bytes(file[12..16].try_into().unwrap()), 4, "height");
        assert_eq!(u32::from_le_bytes(file[16..20].try_into().unwrap()), 8, "width");
        assert_eq!(u32::from_le_bytes(file[28..32].try_into().unwrap()), 2, "mips");
        assert_eq!(&file[84..88], b"DXT1");
    }

    #[test]
    fn the_game_reads_what_is_written() {
        use blackbox_tpk::{Level, PixelFormat, TextureSource, full_mip_count, read_dds};
        let level = Level { width: 16, height: 8, rgba: vec![200; 16 * 8 * 4] };
        let mips = full_mip_count(16, 8);
        let source = TextureSource::from_rgba("T", level, PixelFormat::Dxt1, mips).unwrap();
        let top = 16 * 8 / 2;
        let bytes = file(16, 8, mips, *b"DXT1", top, &source.data);
        let texture = read_dds("T", &bytes).unwrap();
        assert_eq!((texture.width, texture.height, texture.mip_levels), (16, 8, mips));
        assert_eq!(texture.format, PixelFormat::Dxt1);
    }
}
