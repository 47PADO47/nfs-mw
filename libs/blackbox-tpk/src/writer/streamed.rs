//! The streamed form of the car `TEXTURES.BIN`: one JDLZ blob per texture inside the data array, addressed by
//! 24-byte entries holding **file offsets**. The pack must therefore start at offset 0 of the file it is saved to.

use blackbox_chunk::ids;

use super::chunk_buf::ChunkBuf;
use super::error::{WriteError, WriteResult};
use super::source::TextureSource;
use super::{jdlz, records, skeleton};
use crate::layout::PackLayout;

/// Each blob starts on this boundary (the game's files use 0x40, sometimes 0x80).
const BLOB_ALIGN: usize = 0x40;

/// `UserFlags, Flags, RefCount` of an entry: the game's entries all carry `Flags = 1`.
const ENTRY_FLAGS: [u8; 4] = [0, 1, 0, 0];

/// One blob's contents, inflated: `pixels | palette | TextureInfo | platform record`.
fn block(layout: &PackLayout, t: &TextureSource) -> Vec<u8> {
    let mut block = Vec::with_capacity(t.data.len() + t.palette.len() + layout.info.len + layout.plat_len);
    block.extend_from_slice(&t.data);
    block.extend_from_slice(&t.palette);
    block.extend(records::info_record(layout, t, 0));
    block.extend(records::plat_record(layout, t));
    block
}

fn entry(hash: u32, offset: u32, blob_len: u32, inflated: u32) -> [u8; 24] {
    let mut e = [0u8; 24];
    e[0..4].copy_from_slice(&hash.to_le_bytes());
    e[4..8].copy_from_slice(&offset.to_le_bytes());
    e[8..12].copy_from_slice(&blob_len.to_le_bytes());
    e[12..16].copy_from_slice(&inflated.to_le_bytes());
    e[16..20].copy_from_slice(&ENTRY_FLAGS);
    e
}

pub(super) fn build(
    layout: &PackLayout,
    header: &[u8],
    filename: &str,
    textures: &[&TextureSource],
) -> WriteResult<Vec<u8>> {
    let keys = records::keys_payload(textures.iter().map(|t| t.hash()));
    let blocks: Vec<Vec<u8>> = textures.iter().map(|t| block(layout, t)).collect();
    let blobs: Vec<Vec<u8>> = blocks.iter().map(|b| jdlz::compress(b)).collect();

    let mut buf = ChunkBuf::default();
    let (pack, info) = skeleton::open_info(&mut buf, header, &keys);
    // The entries need file offsets that depend on this chunk's own size: write zeros now, patch below.
    let entries_len = layout.entry_len * textures.len();
    buf.leaf(ids::TEXTURE_PACK_INFO_ENTRIES, &vec![0u8; entries_len]);
    let entries_at = buf.bytes.len() - entries_len;
    let open = skeleton::open_data(&mut buf, pack, info, filename);

    let mut entries = Vec::with_capacity(entries_len);
    for ((t, block), blob) in textures.iter().zip(&blocks).zip(&blobs) {
        buf.align_zero(BLOB_ALIGN);
        let offset = u32::try_from(buf.bytes.len()).map_err(|_| too_large())?;
        entries.extend(entry(t.hash(), offset, blob.len() as u32, block.len() as u32));
        buf.bytes.extend_from_slice(blob);
    }
    buf.align_zero(BLOB_ALIGN);
    skeleton::close(&mut buf, open);
    if buf.bytes.len() > i32::MAX as usize {
        return Err(too_large());
    }
    buf.bytes[entries_at..entries_at + entries_len].copy_from_slice(&entries);
    Ok(buf.bytes)
}

fn too_large() -> WriteError {
    WriteError::TooLarge("file offsets exceed 2 GiB".to_owned())
}
