use blackbox_chunk::{PAD_BYTE, ids};

use crate::info::InfoRecord;
use crate::reader::read_compressed_texture;
use crate::{AlphaUsage, PixelFormat, TexturePack, layout, read_texture_packs, read_texture_packs_filtered};

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
    r[0x57] = 3;
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
    assert_eq!(t.flags, 3);
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

const TRAILER_LEN: usize = V5.info.len + V5.plat_len;

fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

/// One 8x8 DXT1 mip: 32 bytes, distinct per seed.
fn pixels(seed: u8) -> Vec<u8> {
    (0..32).map(|i| seed.wrapping_add(i)).collect()
}

fn sample_textures() -> Vec<(u32, Vec<u8>)> {
    vec![(0xA1, pixels(1)), (0xB2, pixels(2)), (0xC3, pixels(3)), (0xD4, pixels(4))]
}

fn pack_header(name: &str) -> Vec<u8> {
    let mut h = vec![0u8; 0x60];
    h[0..4].copy_from_slice(&5u32.to_le_bytes());
    h[0x04..0x04 + name.len()].copy_from_slice(name.as_bytes());
    h[0x20..0x28].copy_from_slice(b"PACK.TPK");
    h
}

fn plain_info(name: &str, hash: u32, placement: u32, size: u32) -> Vec<u8> {
    let mut r = info_record(name, hash, 8, 8, size, 1);
    r[0x30..0x34].copy_from_slice(&placement.to_le_bytes());
    r
}

/// A plain pack. Each `(hash, pixels)` pair is one texture, its pixels packed in order
/// into the data array. `bad` adds a record whose pixels lie outside the array.
fn plain_pack(textures: &[(u32, Vec<u8>)], bad: Option<u32>) -> Vec<u8> {
    let (mut infos, mut plats, mut array) = (Vec::new(), Vec::new(), Vec::new());
    for (hash, px) in textures {
        infos.extend(plain_info(&format!("TEX{hash:X}"), *hash, array.len() as u32, px.len() as u32));
        plats.extend(plat_record(b"DXT1"));
        array.extend_from_slice(px);
    }
    if let Some(hash) = bad {
        infos.extend(plain_info("BAD", hash, 0xFFFF_0000, 32));
        plats.extend(plat_record(b"DXT1"));
    }
    let header = chunk(ids::TEXTURE_PACK_INFO_HEADER, &pack_header("PLAIN"));
    let textures = chunk(ids::TEXTURE_PACK_INFO_TEXTURES, &infos);
    let comps = chunk(ids::TEXTURE_PACK_INFO_COMPS, &plats);
    let info = chunk(ids::TEXTURE_PACK_INFO, &[header, textures, comps].concat());
    // The array payload is aligned by address: pack header, info chunk, data header and
    // array header come first.
    let array_start = 8 + info.len() + 8 + 8;
    let skip = array_start.next_multiple_of(V5.data_align) - array_start;
    let mut payload = vec![PAD_BYTE; skip];
    payload.extend_from_slice(&array);
    let data = chunk(ids::TEXTURE_PACK_DATA, &chunk(ids::TEXTURE_PACK_DATA_ARRAY, &payload));
    chunk(ids::TEXTURE_PACK, &[info, data].concat())
}

fn stream_entry(hash: u32, offset: u32, csize: u32, usize_: u32) -> Vec<u8> {
    [hash, offset, csize, usize_, 0, 0].iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A stored (`RAWW`) blob: the pixels, then the trailer the entry names.
fn raww_blob(hash: u32, px: &[u8]) -> Vec<u8> {
    let mut block = px.to_vec();
    block.extend(info_record(&format!("TEX{hash:X}"), hash, 8, 8, px.len() as u32, 1));
    block.extend(plat_record(b"DXT1"));
    let mut blob = b"RAWW".to_vec();
    blob.extend_from_slice(&[1, 0x10, 0, 0]);
    blob.extend_from_slice(&(block.len() as u32).to_le_bytes());
    blob.extend_from_slice(&((block.len() + 16) as u32).to_le_bytes());
    blob.extend_from_slice(&block);
    blob
}

/// A compressed pack: one blob per texture, stored after the pack chunk and addressed by
/// file offset. `bad` adds an entry whose blob lies outside the file.
fn compressed_pack(textures: &[(u32, Vec<u8>)], bad: Option<u32>) -> Vec<u8> {
    let n = textures.len() + usize::from(bad.is_some());
    let blobs_start = 8 + 8 + (8 + 0x60) + 8 + 24 * n;
    let (mut entries, mut blobs) = (Vec::new(), Vec::new());
    let mut offset = blobs_start;
    for (hash, px) in textures {
        let blob = raww_blob(*hash, px);
        let inflated = (px.len() + TRAILER_LEN) as u32;
        entries.extend(stream_entry(*hash, offset as u32, blob.len() as u32, inflated));
        offset += blob.len();
        blobs.extend(blob);
    }
    if let Some(hash) = bad {
        entries.extend(stream_entry(hash, 0xFFFF_0000, 64, 0));
    }
    let header = chunk(ids::TEXTURE_PACK_INFO_HEADER, &pack_header("COMP"));
    let entries = chunk(ids::TEXTURE_PACK_INFO_ENTRIES, &entries);
    let info = chunk(ids::TEXTURE_PACK_INFO, &[header, entries].concat());
    let pack = chunk(ids::TEXTURE_PACK, &info);
    assert_eq!(pack.len(), blobs_start);
    [pack, blobs].concat()
}

/// The full read, reduced by `keep` the way the filtered read must reduce it.
fn expected(file: &[u8], keep: impl Fn(u32) -> bool) -> Vec<TexturePack> {
    let mut packs = read_texture_packs(file).unwrap();
    for p in &mut packs {
        p.textures.retain(|t| keep(t.name_hash));
        p.failed.retain(|(hash, _)| keep(*hash));
    }
    packs
}

fn assert_filtered_matches(file: &[u8], keep: impl Fn(u32) -> bool + Copy) {
    let filtered = read_texture_packs_filtered(file, keep).unwrap();
    assert_eq!(filtered, expected(file, keep));
}

#[test]
fn plain_filtered_read_matches_full_read() {
    let file = plain_pack(&sample_textures(), Some(0xE5));
    assert_eq!(read_texture_packs(&file).unwrap()[0].textures.len(), 4);
    assert_filtered_matches(&file, |h| h == 0xB2 || h == 0xD4);
    assert_filtered_matches(&file, |h| h != 0xE5);
    assert_filtered_matches(&file, |_| true);

    let packs = read_texture_packs_filtered(&file, |h| h == 0xB2 || h == 0xD4).unwrap();
    assert_eq!(packs[0].name, "PLAIN");
    assert_eq!(packs[0].filename, "PACK.TPK");
    let hashes: Vec<u32> = packs[0].textures.iter().map(|t| t.name_hash).collect();
    assert_eq!(hashes, [0xB2, 0xD4]);
    assert_eq!(packs[0].textures[0].data, pixels(2));
    assert_eq!(packs[0].textures[1].data, pixels(4));
}

#[test]
fn compressed_filtered_read_matches_full_read() {
    let file = compressed_pack(&sample_textures(), Some(0xE5));
    assert_eq!(read_texture_packs(&file).unwrap()[0].failed.len(), 1);
    assert_filtered_matches(&file, |h| h == 0xA1 || h == 0xC3);
    assert_filtered_matches(&file, |h| h != 0xE5);
    assert_filtered_matches(&file, |_| true);

    let packs = read_texture_packs_filtered(&file, |h| h != 0xE5).unwrap();
    assert_eq!(packs[0].name, "COMP");
    assert!(packs[0].failed.is_empty());
    let hashes: Vec<u32> = packs[0].textures.iter().map(|t| t.name_hash).collect();
    assert_eq!(hashes, [0xA1, 0xB2, 0xC3, 0xD4]);
}

#[test]
fn rejecting_everything_yields_no_textures_and_no_error() {
    let files = [plain_pack(&sample_textures(), Some(0xE5)), compressed_pack(&sample_textures(), Some(0xE5))];
    for file in files {
        let packs = read_texture_packs_filtered(&file, |_| false).unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].version, 5);
        assert!(packs[0].textures.is_empty());
        assert!(packs[0].failed.is_empty());
    }
}
