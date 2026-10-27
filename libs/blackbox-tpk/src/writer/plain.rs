//! The plain form: `TextureInfo` and platform records in the info chunk, pixels in the data array.

use blackbox_chunk::ids;

use super::chunk_buf::ChunkBuf;
use super::error::{WriteError, WriteResult};
use super::source::TextureSource;
use super::{records, skeleton};
use crate::layout::PackLayout;

/// Each image starts on this boundary inside the array.
const IMAGE_ALIGN: usize = 0x80;

pub(super) fn build(
    layout: &PackLayout,
    header: &[u8],
    filename: &str,
    textures: &[&TextureSource],
) -> WriteResult<Vec<u8>> {
    let (mut infos, mut plats, mut array) = (Vec::new(), Vec::new(), Vec::new());
    for t in textures {
        array.resize(array.len().next_multiple_of(IMAGE_ALIGN), 0);
        let placement = u32::try_from(array.len()).map_err(|_| too_large())?;
        infos.extend(records::info_record(layout, t, placement));
        plats.extend(records::plat_record(layout, t));
        array.extend_from_slice(&t.data);
        array.extend_from_slice(&t.palette);
    }
    if array.len() > i32::MAX as usize {
        return Err(too_large());
    }
    let keys = records::keys_payload(textures.iter().map(|t| t.hash()));

    let mut buf = ChunkBuf::default();
    let (pack, info) = skeleton::open_info(&mut buf, header, &keys);
    buf.leaf(ids::TEXTURE_PACK_INFO_TEXTURES, &infos);
    buf.leaf(ids::TEXTURE_PACK_INFO_COMPS, &plats);
    let open = skeleton::open_data(&mut buf, pack, info, filename);
    buf.bytes.extend_from_slice(&array);
    skeleton::close(&mut buf, open);
    Ok(buf.bytes)
}

fn too_large() -> WriteError {
    WriteError::TooLarge("pixel data exceeds 2 GiB".to_owned())
}
