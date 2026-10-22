//! Compressed packs: one JDLZ / HUFF blob per texture, addressed by file offset.

use blackbox_chunk::Chunk;

use crate::info::InfoRecord;
use crate::layout::PackLayout;
use crate::{Error, Result, Texture, TexturePack};

pub(super) fn read(
    file: &[u8],
    entries: Chunk<'_>,
    layout: &'static PackLayout,
    keep: &impl Fn(u32) -> bool,
    out: &mut TexturePack,
) {
    for e in entries.payload.chunks_exact(layout.entry_len) {
        let hash = u32::from_le_bytes(e[0..4].try_into().unwrap());
        if !keep(hash) {
            continue;
        }
        match read_compressed_texture(file, e, layout) {
            Ok(t) => out.textures.push(t),
            Err(why) => out.failed.push((hash, why.to_string())),
        }
    }
}

/// The fields of one streaming entry: `(hash, file offset, blob size, inflated size)`.
pub(crate) fn entry_fields(entry: &[u8]) -> (u32, usize, usize, usize) {
    let u32_at = |o: usize| u32::from_le_bytes(entry[o..o + 4].try_into().unwrap());
    (u32_at(0), u32_at(4) as usize, u32_at(8) as usize, u32_at(12) as usize)
}

/// The error for a blob that does not lie inside the file.
pub(crate) fn outside_error(hash: u32, offset: usize, csize: usize) -> Error {
    Error::Texture { hash, detail: format!("blob 0x{offset:X}+{csize} lies outside the file") }
}

/// One streaming entry: `{hash, file offset, blob size, inflated size, flags…}`.
pub(crate) fn read_compressed_texture(file: &[u8], entry: &[u8], layout: &'static PackLayout) -> Result<Texture> {
    let (hash, offset, csize, usize_) = entry_fields(entry);
    let blob = offset
        .checked_add(csize)
        .and_then(|end| file.get(offset..end))
        .ok_or_else(|| outside_error(hash, offset, csize))?;
    inflate_texture(hash, blob, usize_, layout)
}

/// Inflates one texture's blob (`usize_` bytes once inflated) and builds the texture from its pixels and trailer.
pub(crate) fn inflate_texture(hash: u32, blob: &[u8], usize_: usize, layout: &'static PackLayout) -> Result<Texture> {
    let err = |detail: String| Error::Texture { hash, detail };
    let block = ea_compress::unwrap(blob)?;
    let trailer = layout.info.len + layout.plat_len;
    if block.len() != usize_ || block.len() < trailer {
        return Err(err(format!("inflated to {} bytes, expected {usize_}", block.len())));
    }
    let split = block.len() - trailer;
    // The trailer is small: copy it, so the inflated block itself becomes the image without a second copy.
    let tail = block[split..].to_vec();
    let rec = InfoRecord { raw: &tail[..layout.info.len], layout };
    let plat = &tail[layout.info.len..];
    if rec.name_hash() != hash {
        return Err(err(format!("trailer names 0x{:08X}", rec.name_hash())));
    }
    let image_end = rec.image_size().min(split);
    // Unconfirmed: a palette, if any, follows the image inside the block.
    let palette = block.get(image_end..(image_end + rec.palette_size()).min(split)).unwrap_or_default().to_vec();
    let mut image = block.into_owned();
    image.truncate(image_end);
    Ok(rec.build(plat, image, palette))
}
