//! The full-screen world map (press M in free roam). Pure state: the filter, the camera (zoom and pan) and the blip
//! the cursor is focused on. The frontend's stage runs it (`control.rs`), the overlay holds it (`overlay.rs`) and the
//! map's FEng screen draws it (`screen/`, the package `WorldMapMain.fng`). Confirming GPS raises the game's own
//! `Dialog.fng` Yes/No. Spec: `docs/specs/world-map.md`.

mod camera;
pub mod control;
mod filter;
mod frame;
pub mod layout;
pub mod legend;
pub mod overlay;
mod pick;
mod screen;
mod session;
#[cfg(test)]
mod tests;

pub use camera::Camera;
pub use filter::Filter;
pub use frame::WorldMapFrame;
pub use overlay::WorldMapOverlay;
pub use pick::{PICK_RADIUS, nearest};
pub use session::{MapInput, Outcome, PlacedBlip, WorldMap, distance_metres};
