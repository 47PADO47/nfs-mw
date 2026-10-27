//! The chunk frame shared by both pack forms: pack, info and data containers with the alignment the game's files use.

use blackbox_chunk::ids;

use super::chunk_buf::ChunkBuf;
use super::records;

/// Alignment of the info container's header (the game's packs start it at 0x40).
const INFO_ALIGN: usize = 0x40;
/// Alignment of the data container, its array chunk and the array payload (`PackLayout::data_align`).
pub const DATA_ALIGN: usize = 0x80;

/// Handles of the chunks `open_data` leaves unfinished.
pub struct Open {
    pack: usize,
    data: usize,
    array: usize,
}

/// Start the pack: the pack container and the info container with its header and key chunks, ready for the
/// form's own info chunks. Returns the `(pack, info)` handles.
pub fn open_info(buf: &mut ChunkBuf, header: &[u8], keys: &[u8]) -> (usize, usize) {
    let pack = buf.begin(ids::TEXTURE_PACK);
    buf.align_header(INFO_ALIGN);
    let info = buf.begin(ids::TEXTURE_PACK_INFO);
    buf.leaf(ids::TEXTURE_PACK_INFO_HEADER, header);
    buf.leaf(ids::TEXTURE_PACK_INFO_KEYS, keys);
    (pack, info)
}

/// Close the info container and open the data container up to the start of the array payload.
pub fn open_data(buf: &mut ChunkBuf, pack: usize, info: usize, filename: &str) -> Open {
    buf.end(info);
    buf.align_header(DATA_ALIGN);
    let data = buf.begin(ids::TEXTURE_PACK_DATA);
    buf.leaf(ids::TEXTURE_PACK_DATA_HEADER, &records::data_header_record(filename));
    buf.align_header(DATA_ALIGN);
    let array = buf.begin(ids::TEXTURE_PACK_DATA_ARRAY);
    buf.align_payload(DATA_ALIGN);
    Open { pack, data, array }
}

/// Close the array, data and pack chunks.
pub fn close(buf: &mut ChunkBuf, open: Open) {
    buf.end(open.array);
    buf.end(open.data);
    buf.end(open.pack);
}
