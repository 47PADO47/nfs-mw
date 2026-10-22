//! Packs read through a seekable source: the chunk headers are walked with seeks, the pack's info chunks are read, and
//! only the wanted textures' byte ranges follow. For big uncompressed files (a pack of hundreds of megabytes of which a
//! few textures are wanted); a wrapped or compressed file must be inflated whole and read with
//! [`read_texture_packs_filtered`](crate::read_texture_packs_filtered).

use std::io::{Read, Seek, SeekFrom};

use blackbox_chunk::{BARE_JDLZ_ID, CONTAINER_BIT, Chunk, HEADER_LEN, chunks, ids};

use super::compressed::{entry_fields, inflate_texture, outside_error};
use super::parse_header;
use crate::info::InfoRecord;
use crate::layout::PackLayout;
use crate::{Error, Result, TexturePack};

const HEADER: u64 = HEADER_LEN as u64;

/// Like [`read_texture_packs_filtered`](crate::read_texture_packs_filtered), but over a `Read + Seek` source holding an
/// unwrapped file from its first byte. Only the chunk headers, each pack's info chunk and the byte ranges of the
/// textures `keep` accepts are read; the rest of the file is skipped by seeking. The result equals the in-memory read of
/// the same bytes, except that an I/O error fails the whole read with [`Error::Io`].
///
/// A chunk that overruns its parent ends the walk of that level quietly, as the in-memory walk does.
pub fn read_texture_packs_ranged<R: Read + Seek>(src: &mut R, keep: impl Fn(u32) -> bool) -> Result<Vec<TexturePack>> {
    let len = src.seek(SeekFrom::End(0))?;
    let mut packs = Vec::new();
    walk(src, 0, len, len, &keep, &mut packs)?;
    Ok(packs)
}

/// Reads `len` bytes at `at`.
fn read_range<R: Read + Seek>(src: &mut R, at: u64, len: u64) -> Result<Vec<u8>> {
    src.seek(SeekFrom::Start(at))?;
    let too_big = || Error::Pack { offset: at as usize, detail: format!("a range of {len} bytes is too big to read") };
    let mut buf = vec![0u8; usize::try_from(len).map_err(|_| too_big())?];
    src.read_exact(&mut buf)?;
    Ok(buf)
}

/// A chunk header at `at`: `(id, size)`.
fn read_header<R: Read + Seek>(src: &mut R, at: u64) -> Result<(u32, u64)> {
    let h = read_range(src, at, HEADER)?;
    let id = u32::from_le_bytes(h[0..4].try_into().unwrap());
    let size = u64::from(u32::from_le_bytes(h[4..8].try_into().unwrap()));
    Ok((id, size))
}

/// Walks the sibling chunks in `at..end`, descending into containers and reading every texture pack.
fn walk<R: Read + Seek>(
    src: &mut R,
    mut at: u64,
    end: u64,
    file_len: u64,
    keep: &impl Fn(u32) -> bool,
    out: &mut Vec<TexturePack>,
) -> Result<()> {
    while at + HEADER <= end {
        let (id, size) = read_header(src, at)?;
        if id == BARE_JDLZ_ID && at + 16 <= end {
            // A bare blob: its 16-byte header stands in for the chunk header, the compressed size sits at byte 12.
            let tail = read_range(src, at + HEADER, HEADER)?;
            let blob = u64::from(u32::from_le_bytes(tail[4..8].try_into().unwrap()));
            if blob < 16 || blob > end - at {
                return Ok(());
            }
            at += blob;
            continue;
        }
        let payload = at + HEADER;
        if size > end - payload {
            return Ok(());
        }
        if id == ids::TEXTURE_PACK {
            read_pack(src, at, payload + size, file_len, keep, out)?;
        } else if id & CONTAINER_BIT != 0 {
            walk(src, payload, payload + size, file_len, keep, out)?;
        }
        at = payload + size;
    }
    Ok(())
}

/// The child of a chunk payload with `id`.
fn child(payload: &[u8], id: u32) -> Option<Chunk<'_>> {
    chunks(payload).filter_map(|c| c.ok()).find(|c| c.id == id)
}

/// Reads the pack chunk at `at` (header) ending at `end`.
fn read_pack<R: Read + Seek>(
    src: &mut R,
    at: u64,
    end: u64,
    file_len: u64,
    keep: &impl Fn(u32) -> bool,
    out: &mut Vec<TexturePack>,
) -> Result<()> {
    let offset = at as usize;
    let err = |detail: &str| Error::Pack { offset, detail: detail.to_owned() };

    // The info chunk is read whole; the data array is only located (its absolute payload start and length).
    let mut info: Option<Vec<u8>> = None;
    let mut array: Option<(u64, u64)> = None;
    let mut data_seen = false;
    let mut pos = at + HEADER;
    while pos + HEADER <= end {
        let (id, size) = read_header(src, pos)?;
        let payload = pos + HEADER;
        if size > end - payload {
            break;
        }
        if id == ids::TEXTURE_PACK_INFO && info.is_none() {
            info = Some(read_range(src, payload, size)?);
        }
        if id == ids::TEXTURE_PACK_DATA && !data_seen {
            data_seen = true;
            array = find_array(src, payload, payload + size)?;
        }
        pos = payload + size;
    }

    let info = info.ok_or_else(|| err("no TexturePackInfo"))?;
    let header = child(&info, ids::TEXTURE_PACK_INFO_HEADER).ok_or_else(|| err("no TexturePackInfoHeader"))?;
    let (mut pack, layout) = parse_header(offset, header.payload)?;
    match child(&info, ids::TEXTURE_PACK_INFO_ENTRIES) {
        Some(entries) => read_compressed(src, entries.payload, layout, file_len, keep, &mut pack)?,
        None => read_plain(src, offset, &info, array, layout, keep, &mut pack)?,
    }
    out.push(pack);
    Ok(())
}

/// The `TexturePackDataArray` child of the data container in `start..end`: `(payload offset, payload length)`.
fn find_array<R: Read + Seek>(src: &mut R, start: u64, end: u64) -> Result<Option<(u64, u64)>> {
    let mut pos = start;
    while pos + HEADER <= end {
        let (id, size) = read_header(src, pos)?;
        let payload = pos + HEADER;
        if size > end - payload {
            return Ok(None);
        }
        if id == ids::TEXTURE_PACK_DATA_ARRAY {
            return Ok(Some((payload, size)));
        }
        pos = payload + size;
    }
    Ok(None)
}

/// Plain pack: the records are in `info`, the pixels at their placements in the data array.
fn read_plain<R: Read + Seek>(
    src: &mut R,
    offset: usize,
    info: &[u8],
    array: Option<(u64, u64)>,
    layout: &'static PackLayout,
    keep: &impl Fn(u32) -> bool,
    out: &mut TexturePack,
) -> Result<()> {
    let err = |detail: &str| Error::Pack { offset, detail: detail.to_owned() };
    let infos = child(info, ids::TEXTURE_PACK_INFO_TEXTURES).ok_or_else(|| err("no TexturePackInfoTextures"))?;
    let plats = child(info, ids::TEXTURE_PACK_INFO_COMPS).ok_or_else(|| err("no TexturePackInfoComps"))?;
    let (payload, len) = array.ok_or_else(|| err("no TexturePackDataArray"))?;
    // The array's payload starts at its address rounded up to the alignment.
    let skip = (payload.next_multiple_of(layout.data_align as u64) - payload).min(len);
    let (base, len) = (payload + skip, len - skip);

    for (raw, plat) in infos.payload.chunks_exact(layout.info.len).zip(plats.payload.chunks_exact(layout.plat_len)) {
        let rec = InfoRecord { raw, layout };
        if !keep(rec.name_hash()) {
            continue;
        }
        let image = (rec.image_placement() as u64, rec.image_size() as u64);
        let palette = (rec.palette_placement() as u64, rec.palette_size() as u64);
        let image_fits = image.0 + image.1 <= len;
        let palette_fits = palette.1 == 0 || palette.0 + palette.1 <= len;
        if !image_fits || !palette_fits {
            out.failed.push((rec.name_hash(), "pixel data lies outside TexturePackDataArray".into()));
            continue;
        }
        let pixels = read_range(src, base + image.0, image.1)?;
        let colours = match palette.1 {
            0 => Vec::new(),
            n => read_range(src, base + palette.0, n)?,
        };
        out.textures.push(rec.build(plat, pixels, colours));
    }
    Ok(())
}

/// Compressed pack: one blob per wanted entry, read by its file offset and inflated.
fn read_compressed<R: Read + Seek>(
    src: &mut R,
    entries: &[u8],
    layout: &'static PackLayout,
    file_len: u64,
    keep: &impl Fn(u32) -> bool,
    out: &mut TexturePack,
) -> Result<()> {
    for e in entries.chunks_exact(layout.entry_len) {
        let (hash, offset, csize, usize_) = entry_fields(e);
        if !keep(hash) {
            continue;
        }
        if offset as u64 + csize as u64 > file_len {
            out.failed.push((hash, outside_error(hash, offset, csize).to_string()));
            continue;
        }
        let blob = read_range(src, offset as u64, csize as u64)?;
        match inflate_texture(hash, &blob, usize_, layout) {
            Ok(t) => out.textures.push(t),
            Err(why) => out.failed.push((hash, why.to_string())),
        }
    }
    Ok(())
}
