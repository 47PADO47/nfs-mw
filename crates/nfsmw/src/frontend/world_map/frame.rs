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

/// A moving thing on the map: a racer or a police car. It is drawn on every tab and cannot be focused or sent to by GPS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveBlip {
    /// `RACER`, `COP`, `HELICOPTER` or `CHECKPOINT`.
    pub style: &'static str,
    /// Its place on the picture, 0..1 on both axes.
    pub map: [f32; 2],
    /// Where its nose points, degrees clockwise from up the picture.
    pub bearing: f32,
    /// A cop in a chase (its icon flashes blue and red).
    pub chasing: bool,
}

impl LiveBlip {
    /// The icon file: the car arrow, the helicopter, or the checkpoint's ring.
    pub fn icon(&self) -> &'static str {
        match self.style {
            crate::hud::HELICOPTER => super::control::HELICOPTER_ICON,
            crate::hud::CHECKPOINT => super::control::CHECKPOINT_ICON,
            _ => "Minimap_Icon_Car.tga",
        }
    }

    /// The icon colour `clock` seconds after the map opened: a racer is orange, a cop white, or the light bar's colour in a
    /// chase (`docs/specs/hud-cop-blips.md`); the checkpoint is orange as on the minimap and the helicopter keeps its own colours.
    pub fn rgba(&self, clock: f32) -> [u8; 4] {
        if self.style == crate::hud::RACER {
            return [255, 150, 40, 255];
        }
        if self.style == crate::hud::CHECKPOINT {
            return [255, 160, 0, 255];
        }
        if self.style != crate::hud::COP {
            return [255; 4];
        }
        let tick = (clock / crate::hud::TICK_SECONDS) as u32;
        crate::hud::cop_colour(self.chasing, crate::hud::cop_step_at(tick))
    }
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
    /// The second co-op player's car on the picture, drawn in its own tint; `None` without a second car or calibration.
    pub second_mark: Option<PlayerMark>,
    /// How the active theme dresses the map. When it does, the FEng screen restyles itself dark in place
    /// (`screen/restyle.rs`), keeping its picture, filter switch, button prompts and mouse behaviour.
    pub chrome: Chrome,
    /// The file name of the picture the career stage's map uses (`MapVariant::world_map_texture`).
    pub texture: &'static str,
    /// The name hash of the running race's own picture (`TRACK_MAP_<event id>`, with the route in white), once it is
    /// loaded. It replaces [`WorldMapFrame::texture`].
    pub event_picture: Option<u32>,
    /// The racers and the police cars, drawn on every tab.
    pub live: Vec<LiveBlip>,
    /// Seconds since the map opened, for the themed map's entrance.
    pub clock: f32,
    /// Seconds since the cursor last snapped onto, or off, a blip, for the snap animation.
    pub snap_age: f32,
}
