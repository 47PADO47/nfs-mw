//! The map pictures: a grid of equal square tiles ("chops"), each stored as a JDLZ-compressed one-texture TPK
//! in a `CompTPKBlock` chunk. Format: `docs/formats/minimap.md`.

use blackbox_chunk::chunks;
use blackbox_tpk::{Texture, read_texture_packs};

/// The chunk id of one compressed tile.
pub const COMP_TPK_BLOCK: u32 = 0x0003_A100;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the file is not a chunk sequence: {0}")]
    Chunks(#[from] blackbox_chunk::Error),
    #[error("tile {0}: the compressed block does not inflate: {1}")]
    Inflate(usize, ea_compress::Error),
    #[error("tile {0}: the texture pack does not read: {1}")]
    Pack(usize, blackbox_tpk::Error),
    #[error("tile {0}: the texture pack holds {1} textures, not one")]
    NotOneTexture(usize, usize),
}

pub type Result<T> = std::result::Result<T, Error>;

/// All the tiles of one map file, in file order: the tile at position `n` is tile number `n`, counted along the
/// rows from the top left of the picture. Each is a texture; its name hash is the key the HUD package refers to
/// (`<header>_CHOP<n>`, see [`tile_name`]).
#[derive(Debug, Clone, Default)]
pub struct TileSet {
    pub tiles: Vec<Texture>,
}

impl TileSet {
    /// Reads a map file (a sequence of `CompTPKBlock` chunks). Chunks of other ids are skipped.
    pub fn parse(file: &[u8]) -> Result<Self> {
        let mut tiles = Vec::new();
        for chunk in chunks(file) {
            let chunk = chunk?;
            if chunk.id != COMP_TPK_BLOCK {
                continue;
            }
            let n = tiles.len();
            let pack = ea_compress::jdlz_decompress(chunk.payload).map_err(|e| Error::Inflate(n, e))?;
            let packs = read_texture_packs(&pack).map_err(|e| Error::Pack(n, e))?;
            let mut textures: Vec<Texture> = packs.into_iter().flat_map(|p| p.textures).collect();
            if textures.len() != 1 {
                return Err(Error::NotOneTexture(n, textures.len()));
            }
            tiles.extend(textures.pop());
        }
        Ok(Self { tiles })
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

/// The name the game gives tile `number` of the map whose file stem (upper case) is `header`: `MINI_MAP_CHOP12`.
pub fn tile_name(header: &str, number: i32) -> String {
    format!("{header}_CHOP{number}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut out = id.to_le_bytes().to_vec();
        out.extend((payload.len() as u32).to_le_bytes());
        out.extend(payload);
        out
    }

    #[test]
    fn a_file_without_tiles_is_empty() {
        assert!(TileSet::parse(&[]).unwrap().is_empty());
        let other = chunk(0x0003_4201, &[1, 2, 3, 4]);
        assert!(TileSet::parse(&other).unwrap().is_empty(), "other chunks are skipped");
    }

    #[test]
    fn a_tile_that_does_not_inflate_names_its_place() {
        let mut file = chunk(0x0003_4201, &[0; 4]);
        file.extend(chunk(COMP_TPK_BLOCK, &[0xAA; 32]));
        let Err(Error::Inflate(0, _)) = TileSet::parse(&file) else { panic!("expected an inflate error") };
    }

    #[test]
    fn tiles_are_named_after_the_header() {
        assert_eq!(tile_name("MINI_MAP", 0), "MINI_MAP_CHOP0");
        assert_eq!(tile_name("MINI_MAP_UNLOCK_1", 63), "MINI_MAP_UNLOCK_1_CHOP63");
    }
}
