use crate::info::InfoRecord;
use crate::reader::read_compressed_texture;
use crate::{AlphaUsage, PixelFormat, layout};

const V5: &layout::PackLayout = layout::LAYOUTS[0];

#[test]
fn pixel_formats() {
    assert_eq!(PixelFormat::from_d3d(u32::from_le_bytes(*b"DXT1")), PixelFormat::Dxt1);
    assert_eq!(PixelFormat::from_d3d(u32::from_le_bytes(*b"DXT5")), PixelFormat::Dxt5);
    assert_eq!(PixelFormat::from_d3d(21), PixelFormat::Argb8888);
    assert_eq!(PixelFormat::from_d3d(41), PixelFormat::P8);
    assert_eq!(PixelFormat::from_d3d(7), PixelFormat::Other(7));
}

#[test]
fn layouts_by_version() {
    assert_eq!(layout::for_version(5).unwrap().info.len, 0x7C);
    assert!(layout::for_version(8).is_none());
}

fn info_record(name: &str, hash: u32, w: u16, h: u16, image_size: u32, mips: u8) -> Vec<u8> {
    let mut r = vec![0u8; V5.info.len];
    r[0x0C..0x0C + name.len()].copy_from_slice(name.as_bytes());
    r[0x24..0x28].copy_from_slice(&hash.to_le_bytes());
    r[0x38..0x3C].copy_from_slice(&image_size.to_le_bytes());
    r[0x44..0x46].copy_from_slice(&w.to_le_bytes());
    r[0x46..0x48].copy_from_slice(&h.to_le_bytes());
    r[0x4A] = 0x22;
    r[0x4E] = mips;
    r[0x55] = 1;
    r
}

fn plat_record(fourcc: &[u8; 4]) -> Vec<u8> {
    let mut p = vec![0u8; V5.plat_len];
    p[0x14..0x18].copy_from_slice(fourcc);
    p
}

#[test]
fn compressed_entry_with_raww_blob() {
    // 8x8 DXT1 with 2 mips: 32 + 8 bytes of pixels, then the trailer.
    let mut block = vec![0xAB; 40];
    block.extend(info_record("TEST", 0x1234_5678, 8, 8, 40, 2));
    block.extend(plat_record(b"DXT1"));
    let mut blob = b"RAWW".to_vec();
    blob.extend_from_slice(&[1, 0x10, 0, 0]);
    blob.extend_from_slice(&(block.len() as u32).to_le_bytes());
    blob.extend_from_slice(&((block.len() + 16) as u32).to_le_bytes());
    blob.extend_from_slice(&block);

    let mut file = vec![0u8; 32];
    let offset = file.len() as u32;
    file.extend_from_slice(&blob);
    let mut entry = Vec::new();
    entry.extend_from_slice(&0x1234_5678u32.to_le_bytes());
    entry.extend_from_slice(&offset.to_le_bytes());
    entry.extend_from_slice(&(blob.len() as u32).to_le_bytes());
    entry.extend_from_slice(&(block.len() as u32).to_le_bytes());
    entry.extend_from_slice(&[0; 8]);

    let t = read_compressed_texture(&file, &entry, V5).unwrap();
    assert_eq!(t.name, "TEST");
    assert_eq!((t.width, t.height, t.mip_levels), (8, 8, 2));
    assert_eq!(t.format, PixelFormat::Dxt1);
    assert_eq!(t.alpha_usage, AlphaUsage::PunchThrough);
    assert_eq!(t.data.len(), 40);
    assert_eq!(t.mip(1).map(<[u8]>::len), Some(8));
    assert_eq!(t.mip(2), None);
}

#[test]
fn mip_count_is_clamped_to_data() {
    let info = info_record("X", 1, 8, 8, 32, 4);
    let t = InfoRecord { raw: &info, layout: V5 }.build(&plat_record(b"DXT1"), vec![0; 32], Vec::new());
    assert_eq!(t.mip_levels, 1);
}
