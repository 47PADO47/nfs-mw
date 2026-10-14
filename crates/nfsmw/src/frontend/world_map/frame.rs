//! What the map's screen draws in one tick: the session (the tab, the camera, the attached blip and the prompt), the
//! blips placed on the picture and the player's place. The frontend makes one each frame from the overlay and hands it
//! to the screen (`Cx::world_map`).

use super::{PlacedBlip, WorldMap};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldMapFrame {
    pub session: WorldMap,
    pub blips: Vec<PlacedBlip>,
    /// The player's place in render space, for the distances.
    pub player: [f32; 2],
}
