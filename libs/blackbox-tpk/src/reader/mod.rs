//! Finding packs, reading their header and dispatching on the pack form.

mod compressed;
mod plain;

use blackbox_chunk::{Chunk, ids};

use crate::{Error, Result, TexturePack, layout};

#[cfg(test)]
pub(crate) use compressed::read_compressed_texture;

fn cstr(b: &[u8]) -> String {
    let len = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..len]).into_owned()
}

/// Every texture pack in `file` (a whole file with any wrapper removed).
///
/// Compressed packs address their blobs by offset into `file`, so pass the whole
/// file, not just the pack chunk.
pub fn read_texture_packs(file: &[u8]) -> Result<Vec<TexturePack>> {
    blackbox_chunk::find_all(file, ids::TEXTURE_PACK).into_iter().map(|pack| read_pack(file, pack)).collect()
}

fn read_pack(file: &[u8], pack: Chunk<'_>) -> Result<TexturePack> {
    let err = |detail: &str| Error::Pack { offset: pack.offset, detail: detail.to_owned() };
    let info = pack.child(ids::TEXTURE_PACK_INFO).ok_or_else(|| err("no TexturePackInfo"))?;
    let header = info.child(ids::TEXTURE_PACK_INFO_HEADER).ok_or_else(|| err("no TexturePackInfoHeader"))?;
    let header = header.payload;
    if header.len() < 0x60 {
        return Err(err("TexturePackInfoHeader too short"));
    }
    let version = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let layout = layout::for_version(version).ok_or(Error::UnsupportedVersion { offset: pack.offset, version })?;
    let mut out = TexturePack {
        name: cstr(&header[0x04..0x20]),
        filename: cstr(&header[0x20..0x60]),
        version,
        textures: Vec::new(),
        failed: Vec::new(),
    };

    if let Some(entries) = info.child(ids::TEXTURE_PACK_INFO_ENTRIES) {
        compressed::read(file, entries, layout, &mut out);
    } else {
        plain::read(pack, info, layout, &mut out).map_err(err)?;
    }
    Ok(out)
}
