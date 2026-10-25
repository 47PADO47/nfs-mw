//! The `GeometryPack` chunk: the mesh container's tables, then the solid packs.

use blackbox_chunk::{ChunkWriter, ids};

use super::error::WriteError;
use super::put::{hash_pairs, put_u32};

/// `SolidPack`s start on a multiple of this (stock files: every one of the 14,843 car solids).
const PACK_ALIGN: usize = 0x80;
/// `MeshContainerInfo` starts on this, after an empty padding chunk.
const INFO_ALIGN: usize = 0x10;

/// `MeshContainerHeader` is 144 bytes, aligned to 0x10. The reader does not use it; the layout is from the stock files.
const HEADER_LEN: usize = 144;
/// 29 in every car file, including the community add-on ones.
const KIND_AT: usize = 0x08;
const KIND: u32 = 29;
const SOLID_COUNT_AT: usize = 0x0C;
/// Source file name, NUL-terminated (`GEOMETRY.BIN`).
const NAME_AT: usize = 0x10;
const NAME: &str = "GEOMETRY.BIN";
/// A second string (`DEFAULT` in the stock files).
const TAG_AT: usize = 0x48;
const TAG: &str = "DEFAULT";
/// Length of the whole file (stock files: including the zero padding after the `GeometryPack`).
const FILE_SIZE_AT: usize = 0x68;
const ALIGN_AT: usize = 0x70;
const ALIGN: u32 = 0x80;

/// One entry of `MeshContainerOffsets` is `(hash, file offset, size, size, 0, 0)`.
const OFFSET_ENTRY_LEN: usize = 24;

/// A built solid pack, with the name hash it is listed under.
pub(super) struct BuiltPack {
    pub hash: u32,
    pub bytes: Vec<u8>,
}

/// The `GeometryPack` file for these packs, which are laid out in the given order. The tables list them sorted by
/// hash.
pub(super) fn assemble(packs: &[BuiltPack]) -> Result<Vec<u8>, WriteError> {
    let mut order: Vec<usize> = (0..packs.len()).collect();
    order.sort_by_key(|&i| packs[i].hash);
    let keys = hash_pairs(order.iter().map(|&i| packs[i].hash));

    let (mut header_at, mut offsets_at) = (0, 0);
    let mut starts = vec![0; packs.len()];
    let mut w = ChunkWriter::new();
    w.container(ids::GEOMETRY_PACK, |w| {
        w.pad_to(INFO_ALIGN);
        w.container(ids::MESH_CONTAINER_INFO, |w| {
            header_at = (w.position() + 8).next_multiple_of(INFO_ALIGN);
            w.leaf_aligned(ids::MESH_CONTAINER_HEADER, INFO_ALIGN, &header(packs.len()));
            w.leaf(ids::MESH_CONTAINER_KEYS, &keys);
            offsets_at = w.position() + 8;
            w.leaf(ids::MESH_CONTAINER_OFFSETS, &vec![0; packs.len() * OFFSET_ENTRY_LEN]);
            w.container(ids::MESH_CONTAINER_EMPTY, |_| {});
        });
        for (start, pack) in starts.iter_mut().zip(packs) {
            w.pad_to(PACK_ALIGN);
            *start = w.position();
            w.append(&pack.bytes);
        }
    });
    let mut file = w.finish()?;

    // The tables hold file offsets, which are only known now; they do not change any size.
    for (n, &i) in order.iter().enumerate() {
        let entry = offsets_at + n * OFFSET_ENTRY_LEN;
        let (start, size) = (to_u32(starts[i])?, to_u32(packs[i].bytes.len())?);
        put_u32(&mut file, entry, packs[i].hash);
        put_u32(&mut file, entry + 4, start);
        put_u32(&mut file, entry + 8, size);
        put_u32(&mut file, entry + 12, size);
    }
    let size = to_u32(file.len())?;
    put_u32(&mut file, header_at + FILE_SIZE_AT, size);
    Ok(file)
}

fn header(solids: usize) -> Vec<u8> {
    let mut b = vec![0u8; HEADER_LEN];
    put_u32(&mut b, KIND_AT, KIND);
    put_u32(&mut b, SOLID_COUNT_AT, solids as u32);
    b[NAME_AT..NAME_AT + NAME.len()].copy_from_slice(NAME.as_bytes());
    b[TAG_AT..TAG_AT + TAG.len()].copy_from_slice(TAG.as_bytes());
    put_u32(&mut b, ALIGN_AT, ALIGN);
    b
}

fn to_u32(n: usize) -> Result<u32, WriteError> {
    u32::try_from(n).map_err(|_| WriteError::PackTooLarge)
}
