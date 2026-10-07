//! Compressed packs: one JDLZ / HUFF blob per texture, addressed by file offset.

use blackbox_chunk::Chunk;

use crate::info::InfoRecord;
use crate::layout::PackLayout;
use crate::{Error, Result, Texture, TexturePack};

pub(super) fn read(file: &[u8], entries: Chunk<'_>, layout: &'static PackLayout, out: &mut TexturePack) {
    for e in entries.payload.chunks_exact(layout.entry_len) {
        let hash = u32::from_le_bytes(e[0..4].try_into().unwrap());
        match read_compressed_texture(file, e, layout) {
            Ok(t) => out.textures.push(t),
            Err(why) => out.failed.push((hash, why.to_string())),
        }
    }
}

/// One streaming entry: `{hash, file offset, blob size, inflated size, flags…}`.
pub(crate) fn read_compressed_texture(file: &[u8], entry: &[u8], layout: &'static PackLayout) -> Result<Texture> {
    let u32_at = |o: usize| u32::from_le_bytes(entry[o..o + 4].try_into().unwrap());
    let (hash, offset, csize, usize_) = (u32_at(0), u32_at(4) as usize, u32_at(8) as usize, u32_at(12) as usize);
    let err = |detail: String| Error::Texture { hash, detail };

    let blob = file
        .get(offset..offset + csize)
        .ok_or_else(|| err(format!("blob 0x{offset:X}+{csize} lies outside the file")))?;
    let block = ea_compress::unwrap(blob)?;
    let trailer = layout.info.len + layout.plat_len;
    if block.len() != usize_ || block.len() < trailer {
        return Err(err(format!("inflated to {} bytes, expected {usize_}", block.len())));
    }
    let split = block.len() - trailer;
    let rec = InfoRecord { raw: &block[split..split + layout.info.len], layout };
    let plat = &block[split + layout.info.len..];
    if rec.name_hash() != hash {
        return Err(err(format!("trailer names 0x{:08X}", rec.name_hash())));
    }
    let image_end = rec.image_size().min(split);
    // Unconfirmed: a palette, if any, follows the image inside the block.
    let palette = block.get(image_end..(image_end + rec.palette_size()).min(split)).unwrap_or_default().to_vec();
    Ok(rec.build(plat, block[..image_end].to_vec(), palette))
}
