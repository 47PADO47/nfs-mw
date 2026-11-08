use super::*;

/// A DDS header for a `width` by `height` picture with the given pixel-format fields and mip count.
fn header(width: u32, height: u32, mips: u32, flags: u32, four_cc: &[u8; 4], bits: u32, masks: [u32; 4]) -> Vec<u8> {
    let mut out = vec![0u8; 128];
    out[..4].copy_from_slice(b"DDS ");
    let mut put = |at: usize, value: u32| out[at..at + 4].copy_from_slice(&value.to_le_bytes());
    put(4, 124);
    put(8, 0x1007 | if mips > 1 { 0x2_0000 } else { 0 });
    put(12, height);
    put(16, width);
    put(28, mips);
    put(76, 32);
    put(80, flags);
    put(88, bits);
    for (at, mask) in [92, 96, 100, 104].into_iter().zip(masks) {
        put(at, mask);
    }
    out[84..88].copy_from_slice(four_cc);
    out
}

fn block_file(four_cc: &[u8; 4], width: u32, height: u32, mips: u32, bytes: usize) -> Vec<u8> {
    let mut out = header(width, height, mips, 0x4, four_cc, 0, [0; 4]);
    out.extend((0..bytes).map(|n| (n % 251) as u8));
    out
}

#[test]
fn dxt_pictures_pass_through_with_their_mips() {
    // 16 by 16 DXT1: 128 bytes, then 8 by 8: 32, 4 by 4: 8, 2 by 2: 8, 1 by 1: 8 (blocks are whole).
    let file = block_file(b"DXT1", 16, 16, 3, 128 + 32 + 8);
    let texture = read_dds("MY_TEX", &file).unwrap();
    assert_eq!((texture.width, texture.height, texture.mip_levels), (16, 16, 3));
    assert_eq!(texture.format, PixelFormat::Dxt1);
    assert_eq!(texture.data, file[128..]);
    assert_eq!(texture.name_hash, blackbox_hash::bstring_hash("MY_TEX"));
    assert_eq!(texture.alpha_usage, AlphaUsage::None);
    let dxt5 = read_dds("T", &block_file(b"DXT5", 8, 8, 1, 64)).unwrap();
    assert_eq!((dxt5.format, dxt5.alpha_usage), (PixelFormat::Dxt5, AlphaUsage::Modulated));
}

#[test]
fn a_file_with_fewer_mips_than_it_says_keeps_the_levels_it_has() {
    let file = block_file(b"DXT1", 16, 16, 5, 128 + 32);
    assert_eq!(read_dds("T", &file).unwrap().mip_levels, 2);
    assert!(matches!(read_dds("T", &block_file(b"DXT1", 16, 16, 1, 100)), Err(DdsError::Short { need: 128, .. })));
}

#[test]
fn uncompressed_bgra_and_rgba_become_a8r8g8b8() {
    let mut bgra = header(2, 1, 1, 0x41, b"\0\0\0\0", 32, [0x00ff_0000, 0xff00, 0xff, 0xff00_0000]);
    bgra.extend([10, 20, 30, 255, 1, 2, 3, 4]);
    let texture = read_dds("T", &bgra).unwrap();
    assert_eq!((texture.format, texture.data.as_slice()), (PixelFormat::Argb8888, &[10, 20, 30, 255, 1, 2, 3, 4][..]));
    assert_eq!(texture.alpha_usage, AlphaUsage::Modulated);
    let mut rgba = header(1, 1, 1, 0x41, b"\0\0\0\0", 32, [0xff, 0xff00, 0x00ff_0000, 0xff00_0000]);
    rgba.extend([30, 20, 10, 255]);
    assert_eq!(read_dds("T", &rgba).unwrap().data, [10, 20, 30, 255], "red and blue swapped into BGRA");
}

#[test]
fn bc7_and_bc5_are_decoded_and_the_rest_is_refused() {
    let mut dx10 = header(4, 4, 1, 0x4, b"DX10", 0, [0; 4]);
    dx10.extend([98, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
    dx10.extend([0u8; 16]);
    let texture = read_dds("T", &dx10).unwrap();
    assert_eq!((texture.format, texture.width, texture.data.len()), (PixelFormat::Argb8888, 4, 64));
    let mut bc5 = block_file(b"ATI2", 4, 4, 1, 16);
    bc5[128..].fill(0);
    assert_eq!(read_dds("T", &bc5).unwrap().data.len(), 64);
    assert!(matches!(read_dds("T", &block_file(b"ABCD", 4, 4, 1, 16)), Err(DdsError::Unsupported(_))));
    let rgb24 = header(4, 4, 1, 0x40, b"\0\0\0\0", 24, [0xff0000, 0xff00, 0xff, 0]);
    assert!(matches!(read_dds("T", &rgb24), Err(DdsError::Unsupported(_))));
}

#[test]
fn bad_files_are_refused() {
    assert_eq!(read_dds("T", b"nothing"), Err(DdsError::NotDds));
    let mut cube = block_file(b"DXT1", 4, 4, 1, 8);
    cube[112..116].copy_from_slice(&0xfe00u32.to_le_bytes());
    assert_eq!(read_dds("T", &cube), Err(DdsError::NotFlat));
    assert!(matches!(read_dds("T", &block_file(b"DXT1", 0, 4, 1, 8)), Err(DdsError::Size(..))));
}
