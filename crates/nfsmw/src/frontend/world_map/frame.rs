//! What the map's screen draws in one tick: the session (the tab, the camera, the attached blip and the list scroll),
//! the blips placed on the picture and the player's place. The frontend makes one each frame from the overlay and hands
//! it to the screen (`Cx::world_map`).

use super::layout::Picture;
use super::{PlacedBlip, WorldMap};

/// The player's car on the picture: its place on the map, and its bearing in degrees, clockwise from up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerMark {
    pub map: [f32; 2],
    pub bearing: f32,
}

/// How the active theme dresses the map (`ElementId::WorldMapChrome`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Chrome {
    /// The FEng screen as it is.
    #[default]
    Original,
    /// The FEng screen restyled dark in place (`screen/restyle.rs`), with the theme's legend over it.
    Recoloured,
    /// The theme draws the whole map itself, full screen; the FEng screen is not presented.
    Drawn,
}

impl Chrome {
    /// Whether a theme dresses the map at all.
    pub fn themed(self) -> bool {
        self != Chrome::Original
    }

    /// Where the picture sits on the screen: the package's own square, or the full screen's.
    pub fn picture(self) -> Picture {
        match self {
            Chrome::Drawn => Picture::FULL,
            _ => Picture::CLASSIC,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldMapFrame {
    pub session: WorldMap,
    pub blips: Vec<PlacedBlip>,
    /// The player's place in render space, for the distances.
    pub player: [f32; 2],
    /// The player's car on the picture; `None` without a calibration or a car.
    pub player_mark: Option<PlayerMark>,
    /// How the active theme dresses the map. When it does, the FEng screen restyles itself dark in place
    /// (`screen/restyle.rs`), keeping its picture, filter switch, button prompts and mouse behaviour.
    pub chrome: Chrome,
    /// The file name of the picture the career stage's map uses (`MapVariant::world_map_texture`).
    pub texture: &'static str,
    /// Seconds since the map opened, for the themed map's entrance.
    pub clock: f32,
    /// Seconds since the cursor last snapped onto, or off, a blip, for the snap animation.
    pub snap_age: f32,
}
