//! The minimap of EA Black Box games (Need for Speed: Most Wanted): the map tiles, the projection from world
//! metres to map units, and the placement of the view around the player (which four tiles, the scroll, the turn).
//! No rendering and no file access: a host feeds it bytes and draws the result.
//!
//! Format: `docs/formats/minimap.md`. Behaviour: `docs/specs/hud-minimap.md`.
//!
//! ```
//! use blackbox_minimap::{Calibration, Grid, Orientation, View};
//!
//! let grid = Grid { tiles_per_side: 8, tile_size: 128.0 };
//! let map = Calibration { origin: [0.0, 0.0], width: 8000.0 };
//! // A car 1250 m east and 1250 m north of the bottom left corner, heading east.
//! let view = View::new(grid, &map, [1250.0, 1250.0], [1.0, 0.0], 1.0);
//! assert_eq!(view.tiles, [48, 49, 56, 57]);
//! assert_eq!(view.arrow_rotation(Orientation::North), 90.0);
//! ```

mod blip;
mod chops;
mod projection;
mod view;

pub use blip::{Blip, place as place_blip};
pub use chops::{COMP_TPK_BLOCK, Error, Result, TileSet, tile_name};
pub use projection::{Calibration, bearing_degrees};
pub use view::{Grid, Orientation, View, speed_zoom};
