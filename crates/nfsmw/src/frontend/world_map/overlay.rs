//! The map's state while it is open, as one resource: the session, the blips (read once, at open), the player and the
//! city calibration. The frontend writes it; the map's screen draws its frame (`screen/`).

use bevy_ecs::prelude::Resource;
use blackbox_minimap::{Calibration, bearing_degrees};

use super::{Chrome, Filter, PlacedBlip, PlayerMark, WorldMap, WorldMapFrame};
use crate::hud::{MapBlip, MapPosition};

#[derive(Resource, Default)]
pub struct WorldMapOverlay {
    pub open: bool,
    pub session: WorldMap,
    pub calibration: Option<Calibration>,
    /// The blips of the open map, placed on the picture. Read at open: the paused world reports none.
    pub blips: Vec<PlacedBlip>,
    /// The player's place in the world (render space metres), for the distances.
    pub player: [f32; 2],
    /// The player's car on the picture, once a calibration places it.
    pub player_mark: Option<PlayerMark>,
    /// The picture of the career stage's map (`MapVariant::world_map_texture`); empty until the map opens.
    pub texture: &'static str,
    /// Seconds since the map opened, advanced each frame (`tick`).
    pub clock: f32,
    /// The clock when the cursor last snapped onto, or off, a blip, and the blip it was on then.
    snap_since: f32,
    snapped: Option<usize>,
}

impl WorldMapOverlay {
    /// Opens a fresh map: the Locations tab, the whole picture, nothing attached.
    pub fn open(&mut self, calibration: Option<Calibration>) {
        self.open = true;
        self.session = WorldMap::new();
        self.calibration = calibration;
        self.blips.clear();
        self.clock = 0.0;
        self.snap_since = 0.0;
        self.snapped = None;
    }

    /// Advances the map's clock by `dt` seconds, and notes when the cursor changes its blip (the snap animation).
    pub fn tick(&mut self, dt: f32) {
        self.clock += dt;
        if self.session.attached != self.snapped {
            self.snapped = self.session.attached;
            self.snap_since = self.clock;
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.session = WorldMap::new();
        self.blips.clear();
    }

    /// Places the blips the scene reported. Without a calibration there is nothing to place.
    pub fn refresh(&mut self, blips: &[MapBlip], player: Option<&MapPosition>) {
        self.player = player.map_or(self.player, |p| p.position);
        self.player_mark = match (player, &self.calibration) {
            (Some(p), Some(calibration)) => {
                Some(PlayerMark { map: calibration.to_map(p.position), bearing: bearing_degrees(p.heading) })
            }
            _ => None,
        };
        self.blips = match self.calibration {
            Some(calibration) => placed(blips, &calibration),
            None => Vec::new(),
        };
    }

    /// The frame the map's screen draws this tick. `chrome` says how an alternative theme dresses the map: when it does,
    /// it draws the map's list, card, tab and zoom as its own overlay (so the FEng screen leaves those to it).
    pub fn frame(&self, chrome: Chrome) -> WorldMapFrame {
        WorldMapFrame {
            session: self.session.clone(),
            blips: self.blips.clone(),
            player: self.player,
            player_mark: self.player_mark,
            chrome,
            texture: self.texture,
            clock: self.clock,
            snap_age: self.clock - self.snap_since,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    const CITY: Calibration = Calibration { origin: [-1224.8894, -1591.1449], width: 6659.332 };

    #[test]
    fn only_the_map_filters_blips_are_placed_on_the_picture() {
        use crate::hud::{AUTO_SHOP, COP, RACE_SPRINT};
        let blips = [
            MapBlip { style: COP, position: [0.0, 0.0], active: false, heading: None },
            MapBlip { style: AUTO_SHOP, position: [0.0, 0.0], active: false, heading: None },
            MapBlip { style: RACE_SPRINT, position: [100.0, 0.0], active: false, heading: None },
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
        use blackbox_minimap::Orientation;
        let at = MapPosition { position: [5.0, 6.0], heading: [0.0, 1.0], orientation: Orientation::North };
        overlay.refresh(&[], Some(&at));
        assert_eq!(overlay.player, [5.0, 6.0]);
        overlay.close();
        assert!(!overlay.open && overlay.blips.is_empty());
        assert_eq!(overlay.session.filter, Filter::DEFAULT);
    }

    #[test]
    fn the_clock_runs_and_a_snap_restarts_when_the_attached_blip_changes() {
        let mut overlay = WorldMapOverlay::default();
        overlay.open(None);
        overlay.tick(1.0);
        overlay.tick(0.5);
        assert_eq!(overlay.frame(Chrome::Drawn).clock, 1.5);
        overlay.session.attached = Some(2);
        overlay.tick(0.25);
        assert_eq!(overlay.frame(Chrome::Drawn).snap_age, 0.0, "a new blip restarts the snap");
        overlay.tick(0.25);
        assert!((overlay.frame(Chrome::Drawn).snap_age - 0.25).abs() < 1e-6);
        overlay.session.attached = None;
        overlay.tick(0.1);
        assert_eq!(overlay.frame(Chrome::Drawn).snap_age, 0.0, "so does leaving it");
    }

    #[test]
    fn the_frame_carries_the_session_and_the_blips() {
        let mut overlay = WorldMapOverlay::default();
        overlay.open(Some(CITY));
        overlay.session.filter = Filter::Races;
        let frame = overlay.frame(Chrome::Recoloured);
        assert_eq!(frame.session.filter, Filter::Races);
        assert!(frame.blips.is_empty());
        assert!(frame.chrome.themed());
    }
}
