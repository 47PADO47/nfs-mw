//! What the map's screen draws in one tick: the session (the tab, the camera, the attached blip and the list scroll),
//! the blips placed on the picture and the player's place. The frontend makes one each frame from the overlay and hands
//! it to the screen (`Cx::world_map`).

use super::{PlacedBlip, WorldMap};

/// The player's car on the picture: its place on the map, and its bearing in degrees, clockwise from up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerMark {
    pub map: [f32; 2],
    pub bearing: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldMapFrame {
    pub session: WorldMap,
    pub blips: Vec<PlacedBlip>,
    /// The player's place in render space, for the distances.
    pub player: [f32; 2],
    /// The player's car on the picture; `None` without a calibration or a car.
    pub player_mark: Option<PlayerMark>,
    /// Whether the active theme recolours the map. When set, the FEng screen restyles itself dark in place
    /// (`screen/restyle.rs`), keeping its picture, filter switch, button prompts and mouse behaviour.
    pub chrome_themed: bool,
}
