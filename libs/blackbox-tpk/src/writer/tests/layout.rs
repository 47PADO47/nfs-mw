//! The chunk frame and alignment of written packs, checked with the chunk parser.

use blackbox_chunk::{chunks, ids};

use super::source;
use crate::{PackForm, PixelFormat, WriteOptions, write_tpk_with};

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

fn sample() -> Vec<crate::TextureSource> {
    (0..9u8).map(|i| source(&format!("LAYOUT_{i}"), 16 + 4 * u32::from(i), 8, PixelFormat::Dxt3, 2, i)).collect()
}

#[test]
fn the_frame_is_one_pack_with_info_and_data() {
    for form in [PackForm::Streamed, PackForm::Plain] {
        let file = write_tpk_with("FRAME", &sample(), &WriteOptions { form, filename: None }).unwrap();
        let top: Vec<_> = chunks(&file).map(Result::unwrap).collect();
        assert_eq!(top.len(), 1);
        let pack = top[0];
        assert_eq!(pack.id, ids::TEXTURE_PACK);
        assert_eq!(pack.end(), file.len());

        let info = pack.child(ids::TEXTURE_PACK_INFO).unwrap();
        assert_eq!(info.offset, 0x40);
        let header = info.child(ids::TEXTURE_PACK_INFO_HEADER).unwrap();
        assert_eq!((header.payload.len(), u32_at(header.payload, 0)), (0x7C, 5));

        let data = pack.child(ids::TEXTURE_PACK_DATA).unwrap();
        assert_eq!(data.offset % 0x80, 0);
        assert_eq!(data.child(ids::TEXTURE_PACK_DATA_HEADER).unwrap().payload.len(), 24);
        let array = data.child(ids::TEXTURE_PACK_DATA_ARRAY).unwrap();
        assert_eq!(array.offset % 0x80, 0);
        // The reader aligns the payload by address; the padding it skips is 0x11.
        let skip = array.payload.len() - array.aligned_payload(0x80).len();
        assert!(array.payload[..skip].iter().all(|&b| b == blackbox_chunk::PAD_BYTE));
    }
}

#[test]
fn keys_are_sorted_hashes_and_match_the_tables() {
    for form in [PackForm::Streamed, PackForm::Plain] {
        let file = write_tpk_with("KEYS", &sample(), &WriteOptions { form, filename: None }).unwrap();
        let pack = chunks(&file).next().unwrap().unwrap();
        let info = pack.child(ids::TEXTURE_PACK_INFO).unwrap();
        let keys = info.child(ids::TEXTURE_PACK_INFO_KEYS).unwrap().payload;
        let hashes: Vec<u32> = keys.as_chunks::<8>().0.iter().map(|k| u32_at(k, 0)).collect();
        assert_eq!(hashes.len(), 9);
        assert!(hashes.windows(2).all(|w| w[0] < w[1]));

        let table = match form {
            PackForm::Streamed => info.child(ids::TEXTURE_PACK_INFO_ENTRIES).unwrap().payload.chunks_exact(24),
            PackForm::Plain => info.child(ids::TEXTURE_PACK_INFO_TEXTURES).unwrap().payload.chunks_exact(0x7C),
        };
        let table_hashes: Vec<u32> =
            table.map(|r| u32_at(r, if form == PackForm::Streamed { 0 } else { 0x24 })).collect();
        assert_eq!(table_hashes, hashes);
    }
}

#[test]
fn streamed_blobs_sit_in_the_array_at_aligned_file_offsets() {
    let file = write_tpk_with("BLOBS", &sample(), &WriteOptions::default()).unwrap();
    let pack = chunks(&file).next().unwrap().unwrap();
    let info = pack.child(ids::TEXTURE_PACK_INFO).unwrap();
    let array = pack.child(ids::TEXTURE_PACK_DATA).unwrap().child(ids::TEXTURE_PACK_DATA_ARRAY).unwrap();
    let (start, end) = (array.payload_offset(), array.end());
    let mut previous_end = start;
    for e in info.child(ids::TEXTURE_PACK_INFO_ENTRIES).unwrap().payload.as_chunks::<24>().0 {
        let (offset, size) = (u32_at(e, 4) as usize, u32_at(e, 8) as usize);
        assert_eq!(offset % 0x40, 0);
        assert!(offset >= previous_end && offset + size <= end);
        assert_eq!(&file[offset..offset + 4], b"JDLZ");
        assert_eq!(u32_at(&file[offset..], 12) as usize, size);
        assert_eq!(&e[16..20], &[0, 1, 0, 0]);
        previous_end = offset + size;
    }
    assert_eq!(end % 0x40, 0);
}
