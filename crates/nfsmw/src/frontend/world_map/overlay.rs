//! The map's state while it is open, as one resource: the session, the blips (read once, at open), the player and the
//! city calibration. The frontend writes it; the map's screen draws its frame (`screen/`).

use bevy_ecs::prelude::Resource;
use blackbox_minimap::{Calibration, bearing_degrees};

use super::{Chrome, Filter, LiveBlip, PlacedBlip, PlayerMark, WorldMap, WorldMapFrame};
use crate::frontend::challenge_series::maps::Maps;
use crate::hud::{CHECKPOINT, COP, HELICOPTER, MapBlip, MapPosition, MapRoadsHandle, NavRoute, RACER};
use crate::ui::UiAssets;

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
    /// The second co-op player's car on the picture, once a calibration places it.
    pub second_mark: Option<PlayerMark>,
    /// The racers and police cars, placed on the picture; drawn on every tab.
    pub live: Vec<LiveBlip>,
    /// The roads and the GPS route the HUD state had at open, for a theme that draws the map itself. Read at open, as
    /// the blips are.
    pub roads: Option<MapRoadsHandle>,
    pub route: Option<NavRoute>,
    /// The picture of the career stage's map (`MapVariant::world_map_texture`); empty until the map opens.
    pub texture: &'static str,
    /// The running race's own picture, `TRACK_MAP_<event id>`: its name hash, and the read of it.
    event_picture: Option<u32>,
    pictures: Maps,
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
        self.live.clear();
        self.roads = None;
        self.route = None;
        self.event_picture = None;
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

    pub fn close(&mut self, assets: &UiAssets) {
        self.open = false;
        self.session = WorldMap::new();
        self.blips.clear();
        self.live.clear();
        self.roads = None;
        self.route = None;
        self.event_picture = None;
        self.pictures.release(assets);
    }

    /// Starts reading the picture of a running race's event (`TRACK_MAP_<event id>`, route in white). Until it is read the
    /// career stage's map stays.
    pub fn show_event(&mut self, dir: &game_install::GameDir, event: &str) {
        let hash = crate::frontend::challenge_series::board::map_texture(event);
        self.pictures.start_in(dir.clone(), vec![hash]);
        self.event_picture = Some(hash);
    }

    /// Takes a finished read of the event's picture into the assets. Call every frame while the map is open.
    pub fn poll_pictures(&mut self, assets: &UiAssets) {
        self.pictures.poll(assets);
    }

    /// Places the blips the scene reported, and the players' cars. Without a calibration there is nothing to place.
    pub fn refresh(&mut self, blips: &[MapBlip], player: Option<&MapPosition>, second: Option<&MapPosition>) {
        self.player = player.map_or(self.player, |p| p.position);
        self.player_mark = mark_of(player, self.calibration);
        self.second_mark = mark_of(second, self.calibration);
        self.blips = match self.calibration {
            Some(calibration) => placed(blips, &calibration),
            None => Vec::new(),
        };
        self.live = match self.calibration {
            Some(calibration) => live(blips, &calibration),
            None => Vec::new(),
        };
    }

    /// The frame the map's screen draws this tick. `chrome` says how an alternative theme dresses the map: when it does,
    /// it draws the map's list, card, tab and zoom as its own overlay (so the FEng screen leaves those to it).
    pub fn frame(&self, chrome: Chrome) -> WorldMapFrame {
        WorldMapFrame {
            session: self.session.clone(),
            blips: self.blips.clone(),
            live: self.live.clone(),
            event_picture: self.event_picture.filter(|_| self.pictures.loaded()),
            player: self.player,
            player_mark: self.player_mark,
            second_mark: self.second_mark,
            chrome,
            texture: self.texture,
            clock: self.clock,
            snap_age: self.clock - self.snap_since,
            calibration: self.calibration,
            roads: self.roads.clone(),
            route: self.route.clone(),
        }
    }
}

/// A car's mark on the picture: its place and its bearing. `None` without a position or a calibration to place it.
fn mark_of(at: Option<&MapPosition>, calibration: Option<Calibration>) -> Option<PlayerMark> {
    let (Some(at), Some(calibration)) = (at, calibration) else {
        return None;
    };
    Some(PlayerMark { map: calibration.to_map(at.position), bearing: bearing_degrees(at.heading) })
}

/// The racers, the police cars and helicopter and the race's next checkpoint among `blips`, placed on the picture. A thing
/// with no heading points up. A checkpoint shows only while it is active, as in the HUD.
pub fn live(blips: &[MapBlip], calibration: &Calibration) -> Vec<LiveBlip> {
    let mut live: Vec<LiveBlip> = blips
        .iter()
        .filter(|b| matches!(b.style, RACER | COP | HELICOPTER) || (b.style == CHECKPOINT && b.active))
        .map(|b| LiveBlip {
            style: b.style,
            map: calibration.to_map(b.position),
            bearing: b.heading.map_or(0.0, bearing_degrees),
            chasing: b.style == COP && b.active,
        })
        .collect();
    // Drawn in this order, so the checkpoint and then the helicopter lie over the cars they fly above.
    live.sort_by_key(|blip| match blip.style {
        CHECKPOINT => 1,
        HELICOPTER => 2,
        _ => 0,
    });
    live
}

/// The blips of the map's filters, placed on the picture (`Calibration::to_map`). The moving cars are [`live`].
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
    fn moving_cars_and_the_next_checkpoint_are_live_blips_on_every_tab_and_other_blips_are_not() {
        use crate::hud::AUTO_SHOP;
        let blips = [
            MapBlip { style: RACER, position: [100.0, 0.0], active: true, heading: Some([1.0, 0.0]) },
            MapBlip { style: COP, position: [0.0, 0.0], active: true, heading: Some([0.0, 1.0]) },
            MapBlip { style: COP, position: [0.0, 50.0], active: false, heading: None },
            MapBlip { style: CHECKPOINT, position: [0.0, 0.0], active: true, heading: None },
            MapBlip { style: CHECKPOINT, position: [9.0, 9.0], active: false, heading: None },
            MapBlip { style: HELICOPTER, position: [0.0, 5.0], active: false, heading: Some([1.0, 0.0]) },
            MapBlip { style: AUTO_SHOP, position: [0.0, 0.0], active: false, heading: None },
        ];
        let live = live(&blips, &CITY);
        assert_eq!(live.len(), 5, "a racer, two cops, the active checkpoint and the helicopter");
        assert_eq!(live[3].style, CHECKPOINT);
        assert_eq!(live[4].style, HELICOPTER);
        assert_eq!((live[0].style, live[0].map, live[0].bearing), (RACER, CITY.to_map([100.0, 0.0]), 90.0));
        assert!(!live[0].chasing, "a racer is never chasing");
        assert!(live[1].chasing && !live[2].chasing, "only a cop in the chase flashes");
        assert_eq!(live[2].bearing, 0.0, "no heading points up");
    }

    #[test]
    fn a_racer_is_orange_and_a_cop_is_white_or_flashes_blue_and_red() {
        let car = |style, chasing| LiveBlip { style, map: [0.0; 2], bearing: 0.0, chasing };
        assert_eq!(car(RACER, false).rgba(0.0), [255, 150, 40, 255]);
        assert_eq!(car(COP, false).rgba(0.0), [255, 255, 255, 255]);
        assert_eq!(car(HELICOPTER, false).rgba(0.0), [255; 4], "the helicopter keeps its own colours");
        assert_eq!(car(CHECKPOINT, false).rgba(0.0), [255, 160, 0, 255], "the checkpoint is orange");
        assert_eq!(car(CHECKPOINT, false).icon(), "Minimap_Icon_Circuit.tga");
        assert_eq!(car(HELICOPTER, false).icon(), "Helicopter_Minimap_Icon.tga");
        assert_eq!(car(RACER, false).icon(), "Minimap_Icon_Car.tga");
        let chase = [car(COP, true).rgba(0.0), car(COP, true).rgba(0.25)];
        assert_ne!(chase[0], chase[1], "the light bar changes colour");
        assert!(
            chase.iter().all(|c| c[2] == 255 || c[0] == 0xa0 || *c == [0xcc, 0xcc, 0xcc, 255]),
            "blue, grey or red"
        );
    }

    #[test]
    fn the_race_picture_is_used_only_once_it_is_loaded() {
        let mut overlay = WorldMapOverlay::default();
        overlay.open(Some(CITY));
        assert_eq!(overlay.frame(Chrome::Drawn).event_picture, None, "no race");
        // Asking for a race's picture sets it wanted, but nothing has been read: the stage's map stays.
        overlay.event_picture = Some(7);
        assert_eq!(overlay.frame(Chrome::Drawn).event_picture, None, "not loaded yet");
    }

    #[test]
    fn a_closed_map_forgets_its_blips_and_a_new_one_starts_on_locations() {
        let mut overlay = WorldMapOverlay { calibration: Some(CITY), ..Default::default() };
        overlay.session.filter = Filter::Races;
        use blackbox_minimap::Orientation;
        let at = MapPosition { position: [5.0, 6.0], heading: [0.0, 1.0], orientation: Orientation::North };
        overlay.refresh(&[], Some(&at), None);
        assert_eq!(overlay.player, [5.0, 6.0]);
        overlay.close(&UiAssets::empty());
        assert!(!overlay.open && overlay.blips.is_empty());
        assert_eq!(overlay.session.filter, Filter::DEFAULT);
    }

    #[test]
    fn each_co_op_player_has_a_mark_of_their_own() {
        use blackbox_minimap::Orientation;
        let mut overlay = WorldMapOverlay { calibration: Some(CITY), ..Default::default() };
        let at = |position| MapPosition { position, heading: [1.0, 0.0], orientation: Orientation::North };
        let (lead, guest) = (at([5.0, 6.0]), at([100.0, 200.0]));
        overlay.refresh(&[], Some(&lead), Some(&guest));
        assert_eq!(overlay.player_mark.map(|m| m.map), Some(CITY.to_map([5.0, 6.0])));
        assert_eq!(overlay.second_mark.map(|m| m.map), Some(CITY.to_map([100.0, 200.0])));
        assert_eq!(overlay.frame(Chrome::Drawn).second_mark, overlay.second_mark);
        overlay.refresh(&[], Some(&lead), None);
        assert_eq!(overlay.second_mark, None, "a single player has no second mark");
    }

    #[test]
    fn a_second_mark_needs_a_calibration_like_the_first() {
        use blackbox_minimap::Orientation;
        let mut overlay = WorldMapOverlay::default();
        let at = MapPosition { position: [1.0, 2.0], heading: [0.0, 1.0], orientation: Orientation::North };
        overlay.refresh(&[], Some(&at), Some(&at));
        assert_eq!((overlay.player_mark, overlay.second_mark), (None, None));
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

    #[test]
    fn the_frame_carries_the_calibration_the_roads_and_the_route_until_the_map_closes() {
        use crate::hud::{MapRoad, MapRoads, RoadClass};
        let mut overlay = WorldMapOverlay::default();
        overlay.open(Some(CITY));
        let road = MapRoad { class: RoadClass::Main, points: vec![[0.0, 0.0], [10.0, 0.0]] };
        let roads = MapRoadsHandle::new(MapRoads::from_roads(vec![road]));
        let route = NavRoute { path: vec![[0.0, 0.0], [10.0, 0.0]], target: [10.0, 0.0], next_turn: None };
        overlay.roads = Some(roads.clone());
        overlay.route = Some(route.clone());
        let frame = overlay.frame(Chrome::Drawn);
        assert_eq!(frame.calibration, Some(CITY));
        assert!(frame.roads.as_ref().is_some_and(|shared| shared.same(&roads)), "the same roads, not a copy");
        assert_eq!(frame.route, Some(route));
        overlay.close(&UiAssets::empty());
        assert!(overlay.roads.is_none() && overlay.route.is_none(), "a closed map lets the roads go");
    }
}
