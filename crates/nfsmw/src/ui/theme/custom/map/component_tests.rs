//! Tests for the map component on synthetic feeds: what each mode builds, that the roads are clipped to the ellipse,
//! that the cached roads are not queried again every frame, and the chip.

use std::sync::Arc;

use blackbox_minimap::Orientation;
use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Skin, UiNode};

use super::CustomMap;
use crate::hud::{HudState, MapPosition, MapRoad, MapRoads, MapRoadsHandle, NavRoute, RoadClass, next_turn};
use crate::settings::hud::{HudSettings, MapMode};
use crate::ui::present::Screen;
use crate::ui::theme::custom::feed::{CustomFeed, CustomFeedCell};
use crate::ui::theme::custom::tokens::{CustomLook, skin};

const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

fn token_skin() -> Skin {
    skin(CustomLook::default(), [255, 200, 80, 255])
}

fn car() -> MapPosition {
    MapPosition { position: [0.0, 0.0], heading: [1.0, 0.0], orientation: Orientation::North }
}

fn roads(points: Vec<Vec<[f32; 2]>>) -> MapRoadsHandle {
    let roads = points.into_iter().map(|points| MapRoad { class: RoadClass::Side, points }).collect();
    MapRoadsHandle::new(MapRoads::from_roads(roads))
}

fn feed(hud: HudState, mode: MapMode) -> CustomFeed {
    CustomFeed {
        hud: Arc::new(hud),
        settings: HudSettings { map_mode: mode, ..HudSettings::default() },
        screen: SCREEN,
        ..CustomFeed::default()
    }
}

/// The nodes the map builds for `feed` in `cell`, whose map memory carries over from the frames before.
fn built(cell: &CustomFeedCell, feed: CustomFeed) -> Vec<UiNode> {
    cell.set(feed);
    let skin = token_skin();
    let mut map = CustomMap::new(cell.clone());
    map.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    map.build(&mut out);
    out.iter().cloned().collect()
}

fn on_map(roads: Option<MapRoadsHandle>) -> HudState {
    HudState { map_car: Some(car()), map_roads: roads, ..HudState::default() }
}

#[test]
fn off_draws_nothing_and_centered_draws_at_least_the_arrow() {
    assert!(built(&CustomFeedCell::new(), feed(on_map(None), MapMode::Off)).is_empty());
    let arrow = built(&CustomFeedCell::new(), feed(on_map(None), MapMode::Centered));
    assert!(!arrow.is_empty(), "the player's arrow");
    let hidden = HudState { visible: false, ..on_map(None) };
    assert!(built(&CustomFeedCell::new(), feed(hidden, MapMode::Centered)).is_empty());
    let nowhere = HudState { map_car: None, ..on_map(None) };
    assert!(built(&CustomFeedCell::new(), feed(nowhere, MapMode::Centered)).is_empty(), "no car, no overlay");
}

#[test]
fn roads_are_clipped_to_the_ellipse_not_its_box() {
    // At 1080p the box is 480 units high: the Centered ellipse is 220.8 by 163.2 units for 300 m, 0.736 units a metre.
    // North-up, (271, 204) m is at about (200, -150) units: inside the ellipse's box, outside the ellipse.
    let corner = vec![vec![[250.0, 190.0], [290.0, 215.0]]];
    let through = vec![vec![[-100.0, 0.0], [100.0, 0.0]]];
    let bare = built(&CustomFeedCell::new(), feed(on_map(None), MapMode::Centered)).len();
    let cornered = built(&CustomFeedCell::new(), feed(on_map(Some(roads(corner))), MapMode::Centered)).len();
    assert_eq!(cornered, bare, "a road in the ellipse's box corner draws nothing");
    let crossed = built(&CustomFeedCell::new(), feed(on_map(Some(roads(through))), MapMode::Centered)).len();
    assert!(crossed > bare, "a road through the middle draws");
}

#[test]
fn the_roads_far_away_are_not_drawn() {
    let far = vec![vec![[5000.0, 0.0], [5100.0, 0.0]]];
    let bare = built(&CustomFeedCell::new(), feed(on_map(None), MapMode::Centered)).len();
    assert_eq!(built(&CustomFeedCell::new(), feed(on_map(Some(roads(far))), MapMode::Centered)).len(), bare);
}

#[test]
fn a_still_car_keeps_its_cached_roads() {
    let cell = CustomFeedCell::new();
    let network = roads(vec![vec![[-100.0, 0.0], [100.0, 0.0]]]);
    built(&cell, feed(on_map(Some(network.clone())), MapMode::Centered));
    // The car moves a little each frame: the set is not queried again (`refresh` says so), and the roads still draw.
    for step in 1..5 {
        let moved = MapPosition { position: [step as f32 * 5.0, 0.0], ..car() };
        let hud = HudState { map_car: Some(moved), ..on_map(Some(network.clone())) };
        assert!(!built(&cell, feed(hud, MapMode::Centered)).is_empty());
        let queried = cell.map_memory().with(|memory| memory.local.refresh(&network, [step as f32 * 5.0, 0.0], 300.0));
        assert!(!queried, "frame {step} reused the cached set");
    }
}

#[test]
fn compact_shows_the_turn_chip_only_with_a_route() {
    assert!(built(&CustomFeedCell::new(), feed(on_map(None), MapMode::Compact)).is_empty(), "no route, no chip");
    let path = vec![[0.0, 0.0], [347.0, 0.0], [347.0, 100.0]];
    let route = NavRoute { next_turn: next_turn(&path, &[1], [0.0, 0.0]), target: [347.0, 100.0], path };
    let hud = HudState { route: Some(route), ..on_map(None) };
    let nodes = built(&CustomFeedCell::new(), feed(hud, MapMode::Compact));
    let texts: Vec<&str> = nodes.iter().filter_map(|node| node.text.as_deref()).collect();
    assert_eq!(texts, ["350 m"]);
}
