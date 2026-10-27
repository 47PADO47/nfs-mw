//! Tests for the map component on synthetic feeds: what each mode builds, that the roads are clipped to the ellipse,
//! that the cached roads are not queried again every frame, and the chip.

use std::sync::Arc;

use blackbox_feng::fe_hash_upper;
use blackbox_minimap::{Calibration, Orientation};
use blackbox_ui::{Component, DrawList, LayoutCx, NodeKind, Rect, Skin, UiNode, UiTree};
use glam::{Mat4, Vec3};

use super::CustomMap;
use crate::hud::{
    HudState, MapPicture, MapPosition, MapRoad, MapRoads, MapRoadsHandle, NavRoute, RaceRoute, RaceRouteHandle,
    RoadClass, next_turn,
};
use crate::settings::hud::{HudSettings, MapMode};
use crate::ui::present::Screen;
use crate::ui::theme::custom::feed::{CustomFeed, CustomFeedCell};
use crate::ui::theme::custom::tokens::{CustomLook, skin};
use crate::ui::theme::original::{FengSource, FrameCell};

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
    // At 1080p the box is 480 units high: the Centered ellipse's half-axes are 110.4 by 55.2 units for 300 m, 0.368
    // units a metre. North-up, (271, 204) m is at about (100, -75) units from the car: outside the ellipse.
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
    let texts = |nodes: &[UiNode]| nodes.iter().filter_map(|node| node.text.clone()).collect::<Vec<_>>();
    let bare = built(&CustomFeedCell::new(), feed(on_map(None), MapMode::Compact));
    assert!(!bare.is_empty() && texts(&bare).is_empty(), "no route, no chip: only the small overlay's arrow");
    let path = vec![[0.0, 0.0], [347.0, 0.0], [347.0, 100.0]];
    let route = NavRoute { next_turn: next_turn(&path, &[1], [0.0, 0.0]), target: [347.0, 100.0], path };
    let hud = HudState { route: Some(route), ..on_map(None) };
    let nodes = built(&CustomFeedCell::new(), feed(hud, MapMode::Compact));
    assert_eq!(texts(&nodes), ["350 m"]);
}

#[test]
fn the_race_route_is_drawn_on_every_overlay() {
    let samples = vec![([-100.0, 0.0], 0.0), ([100.0, 0.0], 200.0), ([100.0, 100.0], 300.0)];
    let lap = RaceRouteHandle::new(RaceRoute::from_samples(samples, 300.0, 300.0, true).unwrap());
    let racing = HudState { race_route: Some(lap), ..on_map(None) };
    let bare = built(&CustomFeedCell::new(), feed(on_map(None), MapMode::Centered)).len();
    assert!(built(&CustomFeedCell::new(), feed(racing.clone(), MapMode::Centered)).len() > bare);
    assert!(built(&CustomFeedCell::new(), feed(racing.clone(), MapMode::Expanded)).len() > bare);
    let compact_bare = built(&CustomFeedCell::new(), feed(on_map(None), MapMode::Compact)).len();
    assert!(built(&CustomFeedCell::new(), feed(racing, MapMode::Compact)).len() > compact_bare);
}

fn picture() -> Arc<MapPicture> {
    Arc::new(MapPicture::new("MINI_MAP", Calibration { origin: [-3000.0, -3000.0], width: 6400.0 }))
}

/// How many nodes draw one of the picture's 64 tiles.
fn tile_nodes(nodes: &[UiNode]) -> usize {
    let tiles: Vec<u32> = (0..64).map(|n| fe_hash_upper(&format!("MINI_MAP_CHOP{n}"))).collect();
    let is_tile = |node: &&UiNode| matches!(node.kind, NodeKind::Image { texture, .. } if tiles.contains(&texture));
    nodes.iter().filter(is_tile).count()
}

#[test]
fn the_games_picture_is_drawn_from_its_tiles_and_replaces_the_roads() {
    let through = vec![vec![[-100.0, 0.0], [100.0, 0.0]]];
    let plain = HudState { map_picture: None, ..on_map(Some(roads(through.clone()))) };
    assert_eq!(
        tile_nodes(&built(&CustomFeedCell::new(), feed(plain, MapMode::Centered))),
        0,
        "roads without a picture"
    );
    let pictured = HudState { map_picture: Some(picture()), ..on_map(Some(roads(through))) };
    let nodes = built(&CustomFeedCell::new(), feed(pictured, MapMode::Centered));
    assert!(tile_nodes(&nodes) > 0, "the tiles under the car draw");
    let hidden = HudState { map_picture: Some(picture()), ..on_map(None) };
    assert_eq!(tile_nodes(&built(&CustomFeedCell::new(), feed(hidden, MapMode::Off))), 0, "off draws nothing");
}

#[test]
fn a_picture_the_car_is_off_draws_no_tiles() {
    let far = MapPosition { position: [90_000.0, 90_000.0], ..car() };
    let hud = HudState { map_car: Some(far), map_picture: Some(picture()), ..HudState::default() };
    assert_eq!(tile_nodes(&built(&CustomFeedCell::new(), feed(hud, MapMode::Centered))), 0);
}

fn icon_node(name: &str, texture: u32) -> UiNode {
    UiNode {
        name_hash: fe_hash_upper(name),
        kind: NodeKind::Image {
            texture,
            uv: [0.0, 0.0, 1.0, 1.0],
            mask: None,
            mask_rotation: [0.5, 0.5, 0.0],
            mask_uv: [0.0, 0.0, 1.0, 1.0],
        },
        world: Mat4::from_scale(Vec3::new(14.0, 16.0, 1.0)),
        colour: [255; 4],
        ..UiNode::default()
    }
}

/// The nodes the map builds for `feed`, with the game's icons from a HUD tree that holds `icons`.
fn built_with_icons(feed: CustomFeed, icons: Vec<UiNode>) -> Vec<UiNode> {
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, UiTree { nodes: icons, draw_order: Vec::new() });
    let cell = CustomFeedCell::new();
    cell.set(feed);
    let skin = token_skin();
    let mut map = CustomMap::new(cell).with_frame(frame);
    map.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    map.build(&mut out);
    out.iter().cloned().collect()
}

#[test]
fn the_games_icons_draw_the_marks_and_the_arrow_where_the_package_has_them() {
    let path = vec![[0.0, 0.0], [100.0, 0.0]];
    let route = NavRoute { next_turn: None, target: [100.0, 0.0], path };
    let hud = HudState { route: Some(route), ..on_map(None) };
    let uses = |nodes: &[UiNode], key: u32| {
        nodes.iter().any(|node| matches!(node.kind, NodeKind::Image { texture, .. } if texture == key))
    };
    let icons = vec![icon_node("MMICON_CHECKPOINT", 4242), icon_node("PLAYERCARINDICATOR", 777)];
    let with = built_with_icons(feed(hud.clone(), MapMode::Centered), icons);
    assert!(uses(&with, 4242), "the route's end is the checkpoint icon");
    assert!(uses(&with, 777), "the arrow is the game's");
    let without = built_with_icons(feed(hud, MapMode::Centered), Vec::new());
    assert!(!uses(&without, 4242) && !uses(&without, 777), "no package icon: the shapes");
    assert!(!without.is_empty());
}
