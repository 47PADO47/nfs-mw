//! Encoding the fixed-size records of a version 5 pack: header, keys, `TextureInfo` and platform records.
//!
//! Field offsets the reader uses come from [`PackLayout`]; the few the reader ignores are constants here.
//! Their default values were read off the game's own `CARS/*/TEXTURES.BIN` (see `docs/formats/textures.md`).

use blackbox_hash::bstring_hash;

use super::error::{WriteError, WriteResult};
use super::source::TextureSource;
use crate::layout::PackLayout;
use crate::{AlphaUsage, mip_level_size};

/// Bytes of the `TexturePackInfoHeader` payload.
pub const HEADER_LEN: usize = 0x7C;
/// Room for the pack name (NUL-terminated) and the file name in the header.
pub const MAX_PACK_NAME: usize = 27;
pub const MAX_FILE_NAME: usize = 63;
/// Bytes of the `TexturePackDataHeader` payload.
pub const DATA_HEADER_LEN: usize = 24;

const NAME_AT: usize = 0x04;
const FILE_AT: usize = 0x20;
const FILE_HASH_AT: usize = 0x60;

// `TextureInfo` fields the reader does not use.
const CLASS_NAME_HASH: usize = 0x28;
const BASE_IMAGE_SIZE: usize = 0x40;
const SHIFT_WIDTH: usize = 0x48;
const SHIFT_HEIGHT: usize = 0x49;
const NUM_PALETTE_ENTRIES: usize = 0x4C;
const RENDERING_ORDER: usize = 0x51;
const SCROLL_TIME_STEP: usize = 0x58;
const SCALE_S: usize = 0x62;
const SCALE_T: usize = 0x64;

/// `ClassNameHash` of every car texture in the install (a constant of the texture class, not of the texture).
const CLASS_HASH: u32 = 0x001A_93CF;
/// The `i16` the game's records hold for scroll time step and for both scale fields (1.0 in 8.8 fixed point).
const FIXED_ONE: u16 = 0x100;

fn put_u32(rec: &mut [u8], at: usize, v: u32) {
    rec[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_u16(rec: &mut [u8], at: usize, v: u16) {
    rec[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn check_len(what: &str, text: &str, max: usize) -> WriteResult<()> {
    if text.len() > max || !text.is_ascii() || text.bytes().any(|b| b == 0) {
        return Err(WriteError::Header(format!("{what} {text:?} must be ASCII, without NUL, at most {max} bytes")));
    }
    Ok(())
}

/// The header payload: version, pack name, file name, and the file name's hash.
pub fn header_record(layout: &PackLayout, name: &str, filename: &str) -> WriteResult<Vec<u8>> {
    check_len("pack name", name, MAX_PACK_NAME)?;
    check_len("file name", filename, MAX_FILE_NAME)?;
    let mut h = vec![0u8; HEADER_LEN];
    put_u32(&mut h, 0, layout.version);
    h[NAME_AT..NAME_AT + name.len()].copy_from_slice(name.as_bytes());
    h[FILE_AT..FILE_AT + filename.len()].copy_from_slice(filename.as_bytes());
    put_u32(&mut h, FILE_HASH_AT, bstring_hash(filename));
    Ok(h)
}

/// The data header payload: `0, 0, 1, hash(file name), 0, 0` as in the game's packs.
pub fn data_header_record(filename: &str) -> Vec<u8> {
    let mut h = vec![0u8; DATA_HEADER_LEN];
    put_u32(&mut h, 8, 1);
    put_u32(&mut h, 12, bstring_hash(filename));
    h
}

/// One `TextureInfo` record. `placement` is where the image sits in the data array (0 inside a streamed blob).
pub fn info_record(layout: &PackLayout, t: &TextureSource, placement: u32) -> Vec<u8> {
    let l = &layout.info;
    let mut r = vec![0u8; l.len];
    let name = t.stored_name();
    r[l.name.start..l.name.start + name.len()].copy_from_slice(name);
    put_u32(&mut r, l.name_hash, t.hash());
    put_u32(&mut r, CLASS_NAME_HASH, CLASS_HASH);
    let image_size = t.data.len() as u32;
    put_u32(&mut r, l.image_placement, placement);
    put_u32(&mut r, l.palette_placement, placement + image_size);
    put_u32(&mut r, l.image_size, image_size);
    put_u32(&mut r, l.palette_size, t.palette.len() as u32);
    put_u32(&mut r, BASE_IMAGE_SIZE, mip_level_size(t.format, t.width, t.height, 0) as u32);
    put_u16(&mut r, l.width, t.width as u16);
    put_u16(&mut r, l.height, t.height as u16);
    r[SHIFT_WIDTH] = t.width.ilog2() as u8;
    r[SHIFT_HEIGHT] = t.height.ilog2() as u8;
    r[l.compression_type] = t.compression_type;
    put_u16(&mut r, NUM_PALETTE_ENTRIES, (t.palette.len() / 4) as u16);
    r[l.mip_levels] = t.mip_count as u8;
    r[RENDERING_ORDER] = t.rendering_order;
    r[l.alpha_sorting] = u8::from(t.alpha_sorting);
    r[l.alpha_usage] = alpha_usage_byte(t.alpha_usage);
    r[l.alpha_blend] = t.alpha_blend;
    r[l.flags] = t.flags;
    put_u16(&mut r, SCROLL_TIME_STEP, FIXED_ONE);
    put_u16(&mut r, SCALE_S, FIXED_ONE);
    put_u16(&mut r, SCALE_T, FIXED_ONE);
    r
}

fn alpha_usage_byte(usage: AlphaUsage) -> u8 {
    match usage {
        AlphaUsage::None => 0,
        AlphaUsage::PunchThrough => 1,
        AlphaUsage::Modulated => 2,
        AlphaUsage::Other(v) => v,
    }
}

/// One platform record: `0, 0, 1, 5, 6`, the D3DFORMAT, then zeros.
pub fn plat_record(layout: &PackLayout, t: &TextureSource) -> Vec<u8> {
    let mut p = vec![0u8; layout.plat_len];
    put_u32(&mut p, 8, 1);
    put_u32(&mut p, 12, 5);
    put_u32(&mut p, 16, 6);
    put_u32(&mut p, layout.plat_format, t.format.to_d3d());
    p
}

/// The keys payload: `{hash, 0}` per texture, in the order given (callers sort by hash).
pub fn keys_payload(hashes: impl Iterator<Item = u32>) -> Vec<u8> {
    hashes.flat_map(|h| [h.to_le_bytes(), [0; 4]]).flatten().collect()
}
