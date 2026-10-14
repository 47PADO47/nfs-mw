//! The map's state while it is open, as one resource: the session, the blips (read once, at open), the player, the
//! city calibration, and the pictures. The frontend writes it; the egui pass (`view.rs`) draws it.

use bevy_ecs::prelude::Resource;
use blackbox_feng::fe_hash_upper;
use blackbox_minimap::Calibration;

use super::control::ICONS;
use super::view::Textures;
use super::{Filter, PlacedBlip, WorldMap};
use crate::hud::MapBlip;
use crate::ui::UiAssets;

/// A decoded picture: RGBA, `size` in pixels.
pub struct Picture {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

/// The pictures the view uploads once: the city's picture and the blip icons.
#[derive(Default)]
pub struct Pictures {
    pub map: Option<Picture>,
    pub icons: Vec<(&'static str, Picture)>,
}

/// The city's picture: the world-map screen's own texture (`WorldMapMain.fng`, resource 17).
const MAP_PICTURE: &str = "TRACK_MAP.tga";

#[derive(Resource, Default)]
pub struct WorldMapOverlay {
    pub open: bool,
    pub session: WorldMap,
    pub calibration: Option<Calibration>,
    /// The blips of the open map, placed on the picture. Read at open: the paused world reports none.
    pub blips: Vec<PlacedBlip>,
    /// The player's place in the world (render space metres), for the distances.
    pub player: [f32; 2],
    /// Pictures decoded at the first open, taken by the view when it uploads them.
    pub pending: Option<Pictures>,
    /// The uploaded textures, once the view has them.
    pub textures: Option<Textures>,
}

impl WorldMapOverlay {
    /// Opens a fresh map: the Locations tab, the whole picture, nothing attached.
    pub fn open(&mut self, calibration: Option<Calibration>, assets: &UiAssets) {
        self.open = true;
        self.session = WorldMap::new();
        self.calibration = calibration;
        self.blips.clear();
        if self.textures.is_none() && self.pending.is_none() {
            self.pending = Some(load_pictures(assets));
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.session = WorldMap::new();
        self.blips.clear();
    }

    /// Places the blips the scene reported. Without a calibration there is nothing to place.
    pub fn refresh(&mut self, blips: &[MapBlip], player: Option<[f32; 2]>) {
        self.player = player.unwrap_or(self.player);
        self.blips = match self.calibration {
            Some(calibration) => placed(blips, &calibration),
            None => Vec::new(),
        };
    }
}

/// The blips of the map's filters, placed on the picture (`Calibration::to_map`). Other blips (cops, traffic) are not
/// on the map.
pub fn placed(blips: &[MapBlip], calibration: &Calibration) -> Vec<PlacedBlip> {
    blips
        .iter()
        .filter(|b| Filter::ALL.iter().any(|f| f.contains(b.style)))
        .map(|b| PlacedBlip { style: b.style, world: b.position, map: calibration.to_map(b.position) })
        .collect()
}

fn load_pictures(assets: &UiAssets) -> Pictures {
    let map = picture(assets, MAP_PICTURE);
    if map.is_none() {
        log::warn!("world map: the picture {MAP_PICTURE} is not in the install's packages");
    }
    let icons = ICONS.iter().filter_map(|name| picture(assets, name).map(|p| (*name, p))).collect();
    Pictures { map, icons }
}

/// A texture from the install, `None` when the install does not have it or its pixels do not fit its size.
fn picture(assets: &UiAssets, name: &str) -> Option<Picture> {
    let image = assets.image(fe_hash_upper(name))?;
    let size = [image.width as usize, image.height as usize];
    if image.rgba.len() != size[0] * size[1] * 4 {
        return None;
    }
    Some(Picture { size, rgba: image.rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CITY: Calibration = Calibration { origin: [-1224.8894, -1591.1449], width: 6659.332 };

    #[test]
    fn only_the_map_filters_blips_are_placed_on_the_picture() {
        use crate::hud::{AUTO_SHOP, COP, RACE_SPRINT};
        let blips = [
            MapBlip { style: COP, position: [0.0, 0.0], heading: [1.0, 0.0], active: false },
            MapBlip { style: AUTO_SHOP, position: [0.0, 0.0], heading: [1.0, 0.0], active: false },
            MapBlip { style: RACE_SPRINT, position: [100.0, 0.0], heading: [1.0, 0.0], active: false },
        ];
        let placed = placed(&blips, &CITY);
        assert_eq!(placed.len(), 2, "the cop is not on the map");
        assert_eq!(placed[0].style, AUTO_SHOP);
        assert_eq!(placed[1].map, CITY.to_map([100.0, 0.0]));
    }

    #[test]
    fn a_closed_map_forgets_its_blips_and_a_new_one_starts_on_locations() {
        let mut overlay = WorldMapOverlay { calibration: Some(CITY), ..Default::default() };
        overlay.session.filter = Filter::Races;
        overlay.refresh(&[], Some([5.0, 6.0]));
        assert_eq!(overlay.player, [5.0, 6.0]);
        overlay.close();
        assert!(!overlay.open && overlay.blips.is_empty());
        assert_eq!(overlay.session.filter, Filter::DEFAULT);
    }
}
