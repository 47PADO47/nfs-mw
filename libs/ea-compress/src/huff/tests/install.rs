//! Against a real NFS: Most Wanted install (set `NFSMW_GAME_DIR`).

use super::decompress;
use crate::HEADER_LEN;

fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

fn find_chunk(data: &[u8], id: u32) -> Option<&[u8]> {
    let mut pos = 0;
    while let Some(header) = data.get(pos..pos + 8) {
        let cid = u32_at(header, 0);
        let size = u32_at(header, 4) as usize;
        let payload = data.get(pos + 8..pos + 8 + size)?;
        if cid == id {
            return Some(payload);
        }
        if cid & 0x8000_0000 != 0
            && let Some(found) = find_chunk(payload, id)
        {
            return Some(found);
        }
        pos += 8 + size;
    }
    None
}

/// Decompresses every HUFF texture in `CARS/BMWM3GTR/TEXTURES.BIN` and checks
/// its size and the name hash in the 0x9C-byte texture trailer.
#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_bmw_m3_gtr_textures() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else {
        return;
    };
    let path = std::path::Path::new(&dir).join("CARS/BMWM3GTR/TEXTURES.BIN");
    let file = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));

    // The file is a single TexturePack chunk, so entry offsets are file offsets.
    let entries = find_chunk(&file, 0x3331_0003).expect("TexturePackInfoEntries chunk");
    let mut huff = 0;
    for e in entries.as_chunks::<24>().0 {
        let (hash, offset) = (u32_at(e, 0), u32_at(e, 4) as usize);
        let (packed, size) = (u32_at(e, 8) as usize, u32_at(e, 12) as usize);
        let blob = &file[offset..];
        if !blob.starts_with(b"HUFF") {
            continue;
        }
        // The entry's size counts the wrapper header; the wrapper's own field does not.
        assert_eq!(u32_at(blob, 12) as usize + HEADER_LEN, packed, "texture {hash:08X}");
        let out = decompress(blob).unwrap_or_else(|e| panic!("texture {hash:08X}: {e}"));
        assert_eq!(out.len(), size, "texture {hash:08X}");
        assert_eq!(u32_at(&out, size - 0x9C + 0x24), hash, "texture {hash:08X}");
        huff += 1;
    }
    assert!(huff > 0, "no HUFF textures found");
}
