//! The map picture the Custom map draws: the 64 tiles of the minimap file the HUD shows now, and where the picture lies
//! in the world (`docs/formats/minimap.md`). The Original and Remastered looks move the game's own pieces over these
//! tiles; the Custom look cuts them up itself, so it needs their texture keys and the calibration, not the pieces.

use blackbox_feng::fe_hash_upper;
use blackbox_minimap::{Calibration, tile_name};
use nfsmw_data::minimap::GRID;

/// The tiles of one map file and its calibration. The HUD plugin builds one when the file changes and shares it.
#[derive(Debug, Clone, PartialEq)]
pub struct MapPicture {
    stem: String,
    calibration: Calibration,
    /// The texture key of each tile, numbered along the rows from the top left.
    tiles: Vec<u32>,
}

impl MapPicture {
    /// The picture of the map file `stem` (upper case, `MINI_MAP`) placed by `calibration`.
    pub fn new(stem: &str, calibration: Calibration) -> Self {
        let count = GRID.tiles_per_side * GRID.tiles_per_side;
        let tiles = (0..count).map(|number| fe_hash_upper(&tile_name(stem, number))).collect();
        Self { stem: stem.to_owned(), calibration, tiles }
    }

    /// The map file's stem.
    pub fn stem(&self) -> &str {
        &self.stem
    }

    /// Where the picture lies in the world.
    pub fn calibration(&self) -> Calibration {
        self.calibration
    }

    /// How many tiles the picture is wide and high.
    pub fn tiles_per_side(&self) -> i32 {
        GRID.tiles_per_side
    }

    /// The texture key of the tile in `column` and `row` (from the top left), or `None` outside the picture.
    pub fn tile(&self, column: i32, row: i32) -> Option<u32> {
        let side = GRID.tiles_per_side;
        if !(0..side).contains(&column) || !(0..side).contains(&row) {
            return None;
        }
        self.tiles.get((row * side + column) as usize).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CITY: Calibration = Calibration { origin: [-1224.8894, -1591.1449], width: 6659.332 };

    #[test]
    fn tiles_are_named_after_the_file_and_counted_along_the_rows() {
        let picture = MapPicture::new("MINI_MAP", CITY);
        assert_eq!(picture.tile(0, 0), Some(fe_hash_upper("MINI_MAP_CHOP0")));
        assert_eq!(picture.tile(7, 0), Some(fe_hash_upper("MINI_MAP_CHOP7")));
        assert_eq!(picture.tile(0, 1), Some(fe_hash_upper("MINI_MAP_CHOP8")));
        assert_eq!(picture.tile(7, 7), Some(fe_hash_upper("MINI_MAP_CHOP63")));
    }

    #[test]
    fn a_tile_outside_the_picture_has_no_key() {
        let picture = MapPicture::new("MINI_MAP", CITY);
        for (column, row) in [(-1, 0), (0, -1), (8, 0), (0, 8)] {
            assert_eq!(picture.tile(column, row), None);
        }
        assert_eq!((picture.stem(), picture.calibration()), ("MINI_MAP", CITY));
    }
}
