//! The view around the player: which four tiles, how far the group of pieces scrolls, how the mask and the
//! pieces' texture rectangles follow, how everything turns. Spec: `docs/specs/hud-minimap.md` section 4.

use crate::projection::{Calibration, bearing_degrees};

/// The tile grid of a map picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// Tiles along one side (the picture is square).
    pub tiles_per_side: i32,
    /// The width of one tile on the HUD, in HUD units (the width of one of the four pieces).
    pub tile_size: f32,
}

impl Grid {
    /// The array index of a tile number, if the picture has such a tile. Numbers run along the rows from the top
    /// left; the game does not wrap them by column (a column one past the edge is the next row's first tile).
    pub fn tile_index(&self, number: i32) -> Option<usize> {
        let count = self.tiles_per_side * self.tiles_per_side;
        (0..count).contains(&number).then_some(number as usize)
    }
}

/// How the picture is turned on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// North (+y) is up the screen and the arrow turns ("fixed", the free roam default).
    North,
    /// The heading is up the screen: the picture turns under a fixed arrow ("rotating").
    Heading,
}

/// The zoom of the picture for a speed in m/s: `1 - speed / max_speed` with the speed clamped to
/// `0..=max_speed`, raised to 1 when below. That is never anything but 1: the retail game has no speed zoom.
pub fn speed_zoom(speed: f32, max_speed: f32) -> f32 {
    let speed = speed.clamp(0.0, max_speed);
    (1.0 - speed / max_speed).max(1.0)
}

/// What one frame of the minimap needs, worked out from where the player is and where the car points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    grid: Grid,
    /// The numbers of the four tiles around the player: top left, top right, bottom left, bottom right. They can
    /// lie outside the picture's `0..n*n` (see [`Grid::tile_index`]).
    pub tiles: [i32; 4],
    /// The player relative to the centre of the 2 x 2 block of tiles, in tiles, each `-0.5 .. 0.5` (y down).
    pub offset: [f32; 2],
    pub zoom: f32,
    /// The car's bearing in degrees, `0..360`, clockwise from up the picture.
    pub bearing: f32,
    /// The player's place on the picture, in units of its width.
    pub player: [f32; 2],
}

impl View {
    /// The view for a car at `position` (world x, y) pointing along `heading` (world x, y).
    pub fn new(grid: Grid, calibration: &Calibration, position: [f32; 2], heading: [f32; 2], zoom: f32) -> Self {
        let player = calibration.to_map(position);
        let n = grid.tiles_per_side;
        // The game truncates towards zero.
        let (a, b) = (player[0] * n as f32, player[1] * n as f32);
        let (column, row) = (a as i32, b as i32);
        let (fx, fy) = (a - column as f32, b - row as f32);
        let (first_column, offset_x) = if fx < 0.5 { (column - 1, fx) } else { (column, fx - 1.0) };
        let (first_row, offset_y) = if fy < 0.5 { (row - 1, fy) } else { (row, fy - 1.0) };
        let tile = |r: i32, c: i32| r * n + c;
        Self {
            grid,
            tiles: [
                tile(first_row, first_column),
                tile(first_row, first_column + 1),
                tile(first_row + 1, first_column),
                tile(first_row + 1, first_column + 1),
            ],
            offset: [offset_x, offset_y],
            zoom,
            bearing: bearing_degrees(heading),
            player,
        }
    }

    /// How far the group of pieces is moved against its place and where it turns about, in HUD units: the
    /// player's offset in the block times the zoom times the tile width.
    pub fn scroll(&self) -> [f32; 2] {
        [self.offset[0] * self.zoom * self.grid.tile_size, self.offset[1] * self.zoom * self.grid.tile_size]
    }

    /// What is added to the mask rectangle of every piece, so the mask stays where it is on the screen while the
    /// group scrolls: the scroll in tiles, negated.
    pub fn mask_shift(&self) -> [f32; 2] {
        [-self.offset[0] * self.zoom, -self.offset[1] * self.zoom]
    }

    /// The part of its tile each piece shows, `[u0, v0, u1, v1]`, in the order of [`View::tiles`]. The whole tile
    /// at zoom 1.
    pub fn piece_rectangles(&self) -> [[f32; 4]; 4] {
        let s = self.zoom - 1.0;
        [[s, s, 1.0, 1.0], [0.0, s, 1.0 - s, 1.0], [s, 0.0, 1.0, 1.0 - s], [0.0, 0.0, 1.0 - s, 1.0 - s]]
    }

    /// The rotation of the group of pieces, degrees, positive clockwise on the screen (FEng's z rotation).
    pub fn group_rotation(&self, orientation: Orientation) -> f32 {
        match orientation {
            Orientation::North => 0.0,
            Orientation::Heading => -self.bearing,
        }
    }

    /// The rotation of the player's arrow (which points up unrotated), degrees.
    pub fn arrow_rotation(&self, orientation: Orientation) -> f32 {
        match orientation {
            Orientation::North => self.bearing,
            Orientation::Heading => 0.0,
        }
    }

    /// The turn of the picture on the screen that other objects (the blips) are placed with: the bearing when the
    /// picture turns, else 0.
    pub fn picture_turn(&self, orientation: Orientation) -> f32 {
        match orientation {
            Orientation::North => 0.0,
            Orientation::Heading => self.bearing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRID: Grid = Grid { tiles_per_side: 8, tile_size: 128.0 };
    /// A tile is 1000 m wide: the picture's bottom-left corner is the world origin.
    const MAP: Calibration = Calibration { origin: [0.0, 0.0], width: 8000.0 };

    fn view(x: f32, y: f32) -> View {
        View::new(GRID, &MAP, [x, y], [0.0, 1.0], 1.0)
    }

    fn close(a: [f32; 2], b: [f32; 2]) {
        assert!((a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4, "{a:?} vs {b:?}");
    }

    #[test]
    fn the_four_tiles_are_the_nearest_two_columns_and_rows() {
        // Tile column 1, row 1 (world y counted up from the bottom: row r spans y from 8000-1000(r+1) to 8000-1000r).
        // Upper left quarter of that tile: the block is columns 0..1, rows 0..1.
        let v = view(1250.0, 6750.0);
        assert_eq!(v.tiles, [0, 1, 8, 9]);
        close(v.offset, [0.25, 0.25]);
        // Lower right quarter: columns 1..2, rows 1..2, the player is left of and above the block's centre.
        let v = view(1750.0, 6250.0);
        assert_eq!(v.tiles, [9, 10, 17, 18]);
        close(v.offset, [-0.25, -0.25]);
        // Upper right quarter and lower left quarter.
        let v = view(1750.0, 6750.0);
        assert_eq!(v.tiles, [1, 2, 9, 10]);
        close(v.offset, [-0.25, 0.25]);
        let v = view(1250.0, 6250.0);
        assert_eq!(v.tiles, [8, 9, 16, 17]);
        close(v.offset, [0.25, -0.25]);
    }

    #[test]
    fn the_offset_is_the_distance_to_the_blocks_centre() {
        // Half way through the tile column: on the line where the choice flips, the offset is -0.5.
        let v = view(1500.0, 6500.0);
        assert_eq!(v.tiles, [9, 10, 17, 18]);
        close(v.offset, [-0.5, -0.5]);
        // Just before it: +0.5 less a little.
        let v = view(1499.0, 6501.0);
        assert_eq!(v.tiles, [0, 1, 8, 9]);
        assert!(v.offset[0] > 0.49 && v.offset[1] > 0.49);
    }

    #[test]
    fn the_edges_of_the_picture_give_numbers_outside_it() {
        let v = view(100.0, 7900.0);
        assert_eq!(v.tiles, [-9, -8, -1, 0]);
        assert_eq!(GRID.tile_index(-1), None);
        assert_eq!(GRID.tile_index(0), Some(0));
        assert_eq!(GRID.tile_index(63), Some(63));
        assert_eq!(GRID.tile_index(64), None);
        // A column past the left edge is the previous row's last tile, as in the game.
        let v = view(100.0, 6900.0);
        assert_eq!(v.tiles, [-1, 0, 7, 8]);
    }

    #[test]
    fn the_group_scrolls_by_the_offset_in_tile_widths() {
        close(view(1250.0, 6750.0).scroll(), [32.0, 32.0]);
        close(view(1750.0, 6250.0).scroll(), [-32.0, -32.0]);
        close(view(1250.0, 6750.0).mask_shift(), [-0.25, -0.25]);
        let zoomed = View::new(GRID, &MAP, [1250.0, 6750.0], [0.0, 1.0], 1.5);
        close(zoomed.scroll(), [48.0, 48.0]);
        close(zoomed.mask_shift(), [-0.375, -0.375]);
    }

    #[test]
    fn the_pieces_show_whole_tiles_at_zoom_one() {
        let whole = [0.0, 0.0, 1.0, 1.0];
        assert_eq!(view(1250.0, 6750.0).piece_rectangles(), [whole; 4]);
        let zoomed = View::new(GRID, &MAP, [1250.0, 6750.0], [0.0, 1.0], 1.5);
        assert_eq!(
            zoomed.piece_rectangles(),
            [[0.5, 0.5, 1.0, 1.0], [0.0, 0.5, 0.5, 1.0], [0.5, 0.0, 1.0, 0.5], [0.0, 0.0, 0.5, 0.5]]
        );
    }

    #[test]
    fn there_is_no_speed_zoom() {
        for speed in [-5.0, 0.0, 1.0, 30.0, 99.9, 100.0, 250.0] {
            assert_eq!(speed_zoom(speed, 100.0), 1.0, "{speed}");
        }
    }

    #[test]
    fn the_arrow_or_the_picture_turns_by_the_bearing() {
        // Heading right (+x): bearing 90.
        let v = View::new(GRID, &MAP, [1250.0, 6750.0], [1.0, 0.0], 1.0);
        assert!((v.bearing - 90.0).abs() < 1e-3);
        assert!((v.arrow_rotation(Orientation::North) - 90.0).abs() < 1e-3);
        assert_eq!(v.group_rotation(Orientation::North), 0.0);
        assert_eq!(v.arrow_rotation(Orientation::Heading), 0.0);
        assert!((v.group_rotation(Orientation::Heading) + 90.0).abs() < 1e-3);
        assert_eq!(v.picture_turn(Orientation::North), 0.0);
        assert!((v.picture_turn(Orientation::Heading) - 90.0).abs() < 1e-3);
    }
}
