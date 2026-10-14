//! The full-screen world map (press M in free roam). Pure state: the filter, the camera (zoom and pan), the nearest
//! blip under the cursor and the "Activate GPS?" prompt. The frontend's stage runs it (`control.rs`), the overlay
//! holds it (`overlay.rs`) and the map's FEng screen draws it (`screen/`, the package `WorldMapMain.fng`). Spec:
//! `docs/specs/world-map.md`.

mod camera;
pub mod control;
mod filter;
mod frame;
mod gps;
pub mod layout;
pub mod overlay;
mod pick;
mod screen;
mod session;
#[cfg(test)]
mod tests;

pub use camera::Camera;
pub use filter::Filter;
pub use frame::WorldMapFrame;
pub use gps::activate_gps;
pub use overlay::WorldMapOverlay;
pub use pick::{PICK_RADIUS, nearest};
pub use session::{MapInput, Outcome, PlacedBlip, WorldMap, distance_metres};
