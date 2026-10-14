//! The full-screen world map (press M in free roam). Pure state: the filter, the camera (zoom and pan), the nearest
//! blip under the cursor and the "Activate GPS?" prompt. Nothing here draws or reads input; the screen binds it.
//! Spec: `docs/specs/world-map.md`.

mod camera;
mod filter;
mod gps;
mod pick;
mod session;
#[cfg(test)]
mod tests;

pub use camera::{Camera, MAX_ZOOM};
pub use filter::Filter;
pub use gps::activate_gps;
pub use pick::{PICK_RADIUS, nearest};
pub use session::{MapInput, Outcome, PlacedBlip, WorldMap, distance_metres};
