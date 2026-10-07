//! Plain packs: info and platform records in the info chunk, pixels in the data array.

use blackbox_chunk::{Chunk, ids};

use crate::TexturePack;
use crate::info::InfoRecord;
use crate::layout::PackLayout;

pub(super) fn read(
    pack: Chunk<'_>,
    info: Chunk<'_>,
    layout: &'static PackLayout,
    out: &mut TexturePack,
) -> Result<(), &'static str> {
    let infos = info.child(ids::TEXTURE_PACK_INFO_TEXTURES).ok_or("no TexturePackInfoTextures")?;
    let plats = info.child(ids::TEXTURE_PACK_INFO_COMPS).ok_or("no TexturePackInfoComps")?;
    let array = pack
        .child(ids::TEXTURE_PACK_DATA)
        .and_then(|d| d.child(ids::TEXTURE_PACK_DATA_ARRAY))
        .ok_or("no TexturePackDataArray")?
        .aligned_payload(layout.data_align);

    for (raw, plat) in infos.payload.chunks_exact(layout.info.len).zip(plats.payload.chunks_exact(layout.plat_len)) {
        let rec = InfoRecord { raw, layout };
        let image = array.get(rec.image_placement()..rec.image_placement() + rec.image_size());
        let palette = if rec.palette_size() > 0 {
            array.get(rec.palette_placement()..rec.palette_placement() + rec.palette_size())
        } else {
            Some(&[][..])
        };
        match (image, palette) {
            (Some(image), Some(palette)) => out.textures.push(rec.build(plat, image.to_vec(), palette.to_vec())),
            _ => out.failed.push((rec.name_hash(), "pixel data lies outside TexturePackDataArray".into())),
        }
    }
    Ok(())
}
