//! Tests for the Custom full-screen map: the projection, the legend under the frontend's filter, the game's own picture
//! and icons it draws, and what the component builds from a fixed snapshot, with no game data and no GPU.

use blackbox_feng::resource_handle;
use blackbox_minimap::Calibration;
use blackbox_ui::{BuildCx, DrawList, LayoutCx, NodeKind, Rect, Skin, Theme, UiNode};
use glam::Vec2;

use super::fit;
use super::places;
use super::{FullMapCell, FullMapSnapshot};
use crate::frontend::world_map::control::{CHECKPOINT_ICON, icon_name};
use crate::frontend::world_map::{Camera, Chrome, Filter, LiveBlip, PlacedBlip, PlayerMark, WorldMap, WorldMapFrame};
use crate::hud::{AUTO_SHOP, COP, HELICOPTER, NavRoute, RACE_SPRINT, ROADBLOCK, SAFE_HOUSE};
use crate::input::Action;
use crate::ui::prompts::Prompt;
use crate::ui::theme::custom::menu::MenuLook;
use crate::ui::theme::custom::tokens::{self, CustomLook};
use crate::ui::theme::custom::{CustomFeedCell, CustomTheme, RadioFeedCell, palette};
use crate::ui::theme::{ElementId, MenuHint, default_skin};

const ASPECT: f32 = 16.0 / 9.0;
/// A city 2 km wide centred on the world's origin: the origin is the picture's middle.
const CITY: Calibration = Calibration { origin: [-1000.0, -1000.0], width: 2000.0 };

fn close(a: Vec2, b: Vec2) {
    assert!((a - b).length() < 1e-3, "{a:?} is not {b:?}");
}

fn blip(style: &'static str, world: [f32; 2]) -> PlacedBlip {
    PlacedBlip { style, world, map: CITY.to_map(world) }
}

fn map(filter: Filter, attached: Option<usize>) -> WorldMapFrame {
    let mut session = WorldMap::new();
    session.filter = filter;
    session.attached = attached;
    let blips = vec![
        blip(SAFE_HOUSE, [-300.0, 200.0]),
        blip(AUTO_SHOP, [1000.0, 0.0]),
        blip(RACE_SPRINT, [400.0, -500.0]),
        blip(ROADBLOCK, [50.0, 50.0]),
    ];
    WorldMapFrame {
        session,
        blips,
        player: [0.0, 0.0],
        player_mark: Some(PlayerMark { map: CITY.to_map([0.0, 0.0]), bearing: 90.0 }),
        chrome: Chrome::Drawn,
        calibration: Some(CITY),
        ..Default::default()
    }
}

fn hints() -> Vec<MenuHint> {
    vec![MenuHint { keys: vec![Prompt::Key("Esc".into())], label: "Back".into(), action: Some(Action::MenuBack) }]
}

fn snapshot(frame: WorldMapFrame) -> FullMapSnapshot {
    FullMapSnapshot { frame, hints: hints(), aspect: ASPECT }
}

fn skin() -> Skin {
    default_skin()
        .merged(palette::bevy().skin())
        .merged(tokens::skin(CustomLook::default(), palette::bevy().secondary))
        .merged(MenuLook::default().skin())
}

fn built(snapshot: Option<FullMapSnapshot>) -> Vec<UiNode> {
    let cell = FullMapCell::new();
    if let Some(snapshot) = snapshot {
        cell.set(snapshot);
    }
    let skin = skin();
    let theme = CustomTheme::new(RadioFeedCell::default(), CustomFeedCell::default(), cell);
    let mut map = theme.element(ElementId::WorldMapChrome.slot(), &BuildCx::new(&skin)).expect("Custom draws the map");
    map.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    map.build(&mut out);
    out.iter().cloned().collect()
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

#[test]
fn the_view_fits_the_picture_to_the_screens_height_and_follows_the_camera() {
    let all = fit::view(CITY, Camera::zoom_all());
    close(all.to_screen([0.0, 0.0]), Vec2::ZERO);
    // The picture's top left corner (its left edge, its top edge in the world) is the full picture's top left.
    close(all.to_screen([-1000.0, 1000.0]), Vec2::new(-240.0, -240.0));
    close(all.to_screen([1000.0, -1000.0]), Vec2::new(240.0, 240.0));
    let closer = fit::view(CITY, Camera::at_level(1, [0.5, 0.5]));
    close(closer.to_screen([100.0, -50.0]), all.to_screen([100.0, -50.0]) * 2.0);
}

#[test]
fn a_world_point_lands_where_the_frontend_picks_its_blip() {
    // The cursor picks a blip by its place on the picture under the camera; the map draws it from its world place.
    for camera in [Camera::zoom_all(), Camera::at_level(2, [0.3, 0.6]), Camera::at_level(3, [0.8, 0.2])] {
        let view = fit::view(CITY, camera);
        for world in [[0.0, 0.0], [250.0, -400.0], [-900.0, 800.0]] {
            close(view.to_screen(world), fit::on_screen(camera, CITY.to_map(world)));
        }
    }
    close(fit::cursor_on_screen([0.5, 0.5]), Vec2::ZERO);
}

/// How many nodes draw the picture `file`'s texture.
fn drawing(nodes: &[UiNode], file: &str) -> usize {
    let texture = resource_handle(file);
    nodes.iter().filter(|node| matches!(node.kind, NodeKind::Image { texture: t, .. } if t == texture)).count()
}

#[test]
fn the_current_tabs_blips_are_the_games_own_icons() {
    let icons = |filter| built(Some(snapshot(map(filter, None))));
    let locations = icons(Filter::Locations);
    assert_eq!(drawing(&locations, icon_name(SAFE_HOUSE)), 2, "the blip and the legend's row");
    assert_eq!(drawing(&locations, icon_name(AUTO_SHOP)), 2);
    assert_eq!(drawing(&locations, icon_name(RACE_SPRINT)), 0, "a race is not on the Locations tab");
    let races = icons(Filter::Races);
    assert!(drawing(&races, icon_name(RACE_SPRINT)) >= 2, "the sprint and the legend's rows");
    assert_eq!(drawing(&races, icon_name(SAFE_HOUSE)), 0, "a location is not on the Races tab");
    let pursuit = icons(Filter::Pursuit);
    assert!(drawing(&pursuit, icon_name(ROADBLOCK)) >= 1, "the roadblock draws the game's event icon");
}

#[test]
fn the_blips_icon_lies_where_the_cursor_picks_it() {
    let camera = Camera::at_level(2, [0.35, 0.4]);
    let mut frame = map(Filter::Locations, None);
    frame.session.camera = camera;
    let nodes = built(Some(snapshot(frame.clone())));
    let texture = resource_handle(icon_name(SAFE_HOUSE));
    let node = nodes
        .iter()
        .find(|node| {
            matches!(node.kind, NodeKind::Image { texture: t, .. } if t == texture) && node.world.w_axis.z > 10.0
        })
        .expect("the blip's icon");
    let expected = fit::on_screen(camera, frame.blips[0].map);
    close(node.world.w_axis.truncate().truncate(), expected);
}

#[test]
fn the_picture_is_the_games_map_and_follows_the_camera_and_the_race_picture_replaces_it() {
    let mut frame = map(Filter::Locations, None);
    frame.texture = "World_Map_Test.tga";
    let picture = |frame: &WorldMapFrame| built(Some(snapshot(frame.clone())));
    let nodes = picture(&frame);
    assert_eq!(drawing(&nodes, "World_Map_Test.tga"), 1, "the career stage's map is drawn once");
    let side = |nodes: &[UiNode], file: &str| {
        let texture = resource_handle(file);
        let node = nodes.iter().find(|node| matches!(node.kind, NodeKind::Image { texture: t, .. } if t == texture));
        node.map(|node| node.world.x_axis.truncate().length())
    };
    assert_eq!(side(&nodes, "World_Map_Test.tga").map(f32::round), Some(480.0), "the screen's height at Zoom All");
    frame.session.camera = Camera::at_level(1, [0.5, 0.5]);
    assert_eq!(side(&picture(&frame), "World_Map_Test.tga").map(f32::round), Some(960.0), "twice as large at level 1");
    frame.event_picture = Some(77);
    let race = picture(&frame);
    assert_eq!(drawing(&race, "World_Map_Test.tga"), 0, "the race's own picture replaces the stage's");
    assert!(race.iter().any(|node| matches!(node.kind, NodeKind::Image { texture: 77, .. })));
}

#[test]
fn the_cars_and_the_helicopter_are_the_games_icons_too() {
    let mut frame = map(Filter::Locations, None);
    frame.live = vec![
        LiveBlip { style: COP, map: [0.4, 0.4], bearing: 0.0, chasing: true },
        LiveBlip { style: HELICOPTER, map: [0.6, 0.6], bearing: 90.0, chasing: false },
    ];
    let nodes = built(Some(snapshot(frame)));
    assert!(drawing(&nodes, "Minimap_Icon_Car.tga") >= 3, "the cop, the player's car and the legend's");
    assert_eq!(drawing(&nodes, crate::frontend::world_map::control::HELICOPTER_ICON), 1);
}

#[test]
fn the_routes_end_is_the_checkpoint_icon_and_needs_the_calibration() {
    let route = NavRoute { path: vec![[0.0, 0.0], [300.0, 0.0]], target: [300.0, 0.0], next_turn: None };
    let routed = WorldMapFrame { route: Some(route.clone()), ..map(Filter::Locations, None) };
    let bare = built(Some(snapshot(map(Filter::Locations, None))));
    let nodes = built(Some(snapshot(routed.clone())));
    assert_eq!(drawing(&nodes, CHECKPOINT_ICON), drawing(&bare, CHECKPOINT_ICON) + 1);
    assert!(nodes.len() > bare.len(), "the route's strokes and its end draw");
    let lost = WorldMapFrame { calibration: None, ..routed };
    assert_eq!(drawing(&built(Some(snapshot(lost))), CHECKPOINT_ICON), drawing(&bare, CHECKPOINT_ICON));
}

#[test]
fn the_legend_rows_are_the_tabs_styles_with_their_counts() {
    let rows = places::legend_rows(&map(Filter::Locations, None));
    let named: Vec<(&str, usize)> = rows.iter().map(|row| (row.name.as_str(), row.count)).collect();
    assert_eq!(named, [("Auto shop", 1), ("Safe house", 1), ("Car lot", 0)]);
    let races = places::legend_rows(&map(Filter::Races, None));
    assert_eq!(races.len(), Filter::Races.styles().len());
    assert_eq!(races.iter().find(|row| row.style == RACE_SPRINT).map(|row| row.count), Some(1));
}

#[test]
fn the_custom_theme_claims_the_world_map_slot_and_a_closed_map_draws_nothing() {
    let skin = skin();
    let theme = CustomTheme::new(RadioFeedCell::default(), CustomFeedCell::default(), FullMapCell::new());
    assert!(theme.element(ElementId::WorldMapChrome.slot(), &BuildCx::new(&skin)).is_some());
    assert!(built(None).is_empty());
}

#[test]
fn an_open_map_draws_the_title_the_tabs_the_footer_and_the_legend_of_the_current_tab() {
    let nodes = built(Some(snapshot(map(Filter::Locations, None))));
    let shown = texts(&nodes);
    for word in ["MAP", "LOCATIONS", "RACES", "PURSUIT", "Back", "Safe house", "Auto shop", "Player"] {
        assert!(shown.contains(&word), "{word} is drawn: {shown:?}");
    }
    assert!(!shown.contains(&"Sprint"), "a race is not in the Locations legend");
    let races = built(Some(snapshot(map(Filter::Races, None))));
    assert!(texts(&races).contains(&"Sprint") && !texts(&races).contains(&"Safe house"));
}

#[test]
fn the_legend_key_hides_the_legend() {
    let mut frame = map(Filter::Locations, None);
    frame.session.legend_open = false;
    let nodes = built(Some(snapshot(frame)));
    assert!(!texts(&nodes).contains(&"Safe house"), "no legend");
    assert!(texts(&nodes).contains(&"MAP"), "the title stays");
}

#[test]
fn the_focused_blip_has_a_card_with_its_distance_from_the_player() {
    let nodes = built(Some(snapshot(map(Filter::Locations, Some(1)))));
    let shown = texts(&nodes);
    assert!(shown.contains(&"1.0 km"), "the auto shop is 1 km away: {shown:?}");
    assert!(shown.contains(&crate::i18n::t!("map-card-gps").as_str()));
    let unfocused = built(Some(snapshot(map(Filter::Locations, None))));
    assert!(!texts(&unfocused).contains(&"1.0 km"), "no card without a focused blip");
    let nowhere = WorldMapFrame { player_mark: None, ..map(Filter::Locations, Some(1)) };
    assert!(!texts(&built(Some(snapshot(nowhere)))).contains(&"1.0 km"), "no distance without the player's place");
}
