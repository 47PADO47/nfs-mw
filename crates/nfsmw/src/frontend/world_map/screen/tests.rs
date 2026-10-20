//! The map's screen against the install's `WorldMapMain.fng`: the objects the logic draws with exist, and a frame puts
//! the blips, the list, the cursor and the card where they belong. Needs the game (set `NFSMW_GAME_DIR`).

use std::collections::HashMap;

use blackbox_feng::{ObjectRef, PackageId, Runtime};
use game_install::GameDir;
use nfsmw_career::regions::MapVariant;

use super::pools;
use super::{EMPTY, WorldMapLogic};
use crate::frontend::ids::screen::WORLD_MAP;
use crate::frontend::logic::{Cx, ScreenLogic};
use crate::frontend::world_map::{Camera, Filter, PlacedBlip, WorldMap, WorldMapFrame};
use crate::hud::{AUTO_SHOP, CAR_LOT, RACE_SPRINT, SAFE_HOUSE};
use crate::settings::{Partial, Settings};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

struct Harness {
    rt: Runtime,
    package: PackageId,
    assets: UiAssets,
}

impl Harness {
    fn open() -> Self {
        let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
        let assets = UiAssets::load(&dir).unwrap();
        let package = Catalog::load(&dir, &SCREEN_FILES).find(WORLD_MAP).cloned().unwrap();
        let mut rt = Runtime::new();
        let package = rt.load(package);
        Self { rt, package, assets }
    }

    /// One tick of the logic over `frame`, as the frontend runs it.
    fn tick(&mut self, frame: &WorldMapFrame) {
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let (mut commands, mut memory) = (Vec::new(), HashMap::new());
        let mut cx = Cx {
            rt: &mut self.rt,
            package: self.package,
            assets: &self.assets,
            settings: &mut settings,
            changed: &mut changed,
            caps: crate::settings::test_caps::native(),
            commands: &mut commands,
            memory: &mut memory,
            key_texts: &[],
            world_map: Some(frame),
            mods: None,
            name: WORLD_MAP,
        };
        WorldMapLogic { variant: MapVariant::Full, last_attached: None }.tick(&mut cx, 1.0 / 60.0);
    }

    fn object(&self, guid: u32) -> ObjectRef {
        self.rt.find_guid(self.package, guid).unwrap_or_else(|| panic!("no object {guid:#x}"))
    }

    fn hidden(&self, guid: u32) -> bool {
        self.rt.object(self.object(guid)).unwrap().hidden
    }

    fn text(&self, guid: u32) -> String {
        self.rt.object(self.object(guid)).unwrap().text.clone().unwrap_or_default()
    }

    fn position(&self, guid: u32) -> [f32; 2] {
        let p = self.rt.position(self.object(guid)).unwrap();
        [p.x, p.y]
    }
}

fn blip(style: &'static str, map: [f32; 2]) -> PlacedBlip {
    PlacedBlip { style, world: [0.0, 0.0], map }
}

fn frame(blips: Vec<PlacedBlip>) -> WorldMapFrame {
    WorldMapFrame { session: WorldMap::new(), blips, ..Default::default() }
}

fn assert_close(a: [f32; 2], b: [f32; 2]) {
    assert!((a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3, "{a:?} != {b:?}");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_package_has_every_object_the_map_draws_with() {
    let h = Harness::open();
    for slots in pools::POOLS.iter().map(|(_, s)| *s) {
        for &guid in slots {
            h.object(guid);
        }
    }
    for row in pools::ROWS {
        h.object(row.text);
        h.object(row.icon_group);
        h.object(row.icon);
    }
    for guid in [
        pools::PICTURE,
        pools::CURSOR,
        pools::CARD,
        pools::CARD_TITLE,
        pools::CARD_DISTANCE,
        pools::CARD_ICON,
        pools::TAB,
        pools::ZOOM,
    ] {
        h.object(guid);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_safe_house_is_drawn_on_the_picture_and_the_other_pools_are_parked() {
    let mut h = Harness::open();
    h.tick(&frame(vec![blip(SAFE_HOUSE, [0.5, 0.5])]));
    let first = pools::slots(SAFE_HOUSE)[0];
    assert!(!h.hidden(first));
    assert_close(h.position(first), [-111.0, 11.0]);
    assert!(h.hidden(pools::slots(SAFE_HOUSE)[1]), "the second safe house pool has no blip");
    assert!(h.hidden(pools::slots(AUTO_SHOP)[0]), "no auto shop blip, so its pool is parked");
    assert!(h.hidden(pools::CARD), "nothing attached: no card");
    assert!(h.hidden(pools::CURSOR));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_list_names_the_blips_and_the_tab_and_zoom_read_out() {
    let mut h = Harness::open();
    let mut frame = frame(vec![blip(SAFE_HOUSE, [0.5, 0.5]), blip(AUTO_SHOP, [0.2, 0.2])]);
    h.tick(&frame);
    assert_eq!(h.text(pools::ROWS[0].text), "Safe house   0 m");
    assert_eq!(h.text(pools::ROWS[1].text), "Auto shop   0 m", "the tab's list is the locations");
    assert_eq!(h.text(pools::TAB), "LOCATIONS");
    assert_eq!(h.text(pools::ZOOM), "Zoom x1.0");

    frame.session.filter = Filter::Races;
    frame.session.camera = Camera::at_level(1, [0.5, 0.5]);
    frame.blips.push(blip(RACE_SPRINT, [0.5, 0.5]));
    h.tick(&frame);
    assert_eq!(h.text(pools::TAB), "RACES");
    assert_eq!(h.text(pools::ZOOM), "Zoom x2.0");
    assert_eq!(h.text(pools::ROWS[0].text), "Sprint   0 m");
    assert!(h.hidden(pools::ROWS[1].text), "one race on the map: one row");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn zoom_crops_the_picture_and_parks_the_blips_outside_the_view() {
    let mut h = Harness::open();
    let mut frame = frame(vec![blip(SAFE_HOUSE, [0.5, 0.5]), blip(CAR_LOT, [0.1, 0.1])]);
    frame.session.camera = Camera::at_level(1, [0.5, 0.5]);
    h.tick(&frame);
    let picture = h.object(pools::PICTURE);
    let uv = h.rt.object(picture).unwrap().data.uv();
    assert_eq!(uv, [0.25, 0.25, 0.75, 0.75]);
    assert!(!h.hidden(pools::slots(SAFE_HOUSE)[0]), "the centre is in the view");
    assert!(h.hidden(pools::slots(SAFE_HOUSE)[1]));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn focusing_a_blip_moves_the_cursor_onto_it_and_fills_the_card() {
    let mut h = Harness::open();
    let mut frame = frame(vec![blip(SAFE_HOUSE, [0.5, 0.5])]);
    frame.session.attached = Some(0);
    h.tick(&frame);
    assert!(!h.hidden(pools::CURSOR));
    assert_close(h.position(pools::CURSOR), [-111.0, 11.0]);
    assert!(!h.hidden(pools::CARD));
    assert_eq!(h.text(pools::CARD_TITLE), "Safe house");
    assert_eq!(h.text(pools::ROWS[0].text), "Safe house   0 m", "the row names the focused blip");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn an_empty_tab_says_so_in_the_first_row() {
    let mut h = Harness::open();
    h.tick(&frame(Vec::new()));
    assert_eq!(h.text(pools::ROWS[0].text), EMPTY);
    assert!(h.hidden(pools::ROWS[0].icon_group));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_remastered_map_hides_the_feng_list_and_card_for_the_themed_legend() {
    let mut h = Harness::open();
    let mut frame = frame(vec![blip(SAFE_HOUSE, [0.5, 0.5])]);
    frame.chrome = crate::frontend::world_map::Chrome::Recoloured;
    frame.session.attached = Some(0);
    h.tick(&frame);
    // The Remastered map recolours the FEng screen dark in place and hides its own list rows and card; the themed
    // legend (drawn by the frontend) replaces them. The picture, the blips and the cursor still draw, so the map keeps
    // its fullscreen picture, filter switch, button prompts and mouse behaviour.
    assert!(!h.hidden(pools::slots(SAFE_HOUSE)[0]), "the blip is still drawn");
    assert!(!h.hidden(pools::CURSOR), "the cursor still snaps onto the focused blip");
    assert!(h.hidden(pools::ROWS[0].text), "the FEng list row is hidden for the themed legend");
    assert!(h.hidden(pools::CARD), "the FEng card is hidden; the themed legend shows the distance");
}
