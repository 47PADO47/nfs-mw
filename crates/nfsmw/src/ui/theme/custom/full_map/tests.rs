//! Tests for the Custom full-screen map: the projection, the marks and the legend under the frontend's filter, and what
//! the component builds from a fixed snapshot, with no game data and no GPU.

use blackbox_minimap::Calibration;
use blackbox_ui::{BuildCx, DrawList, LayoutCx, Rect, Skin, Theme, UiNode};
use glam::Vec2;

use super::fit::{self, FIT_MARGIN};
use super::places::{self, kind_of};
use super::{FullMapCell, FullMapSnapshot};
use crate::frontend::world_map::{Camera, Chrome, Filter, PlacedBlip, PlayerMark, WorldMap, WorldMapFrame};
use crate::hud::{
    AUTO_SHOP, CAR_LOT, MapRoad, MapRoads, MapRoadsHandle, NavRoute, RACE_SPRINT, ROADBLOCK, RoadClass, SAFE_HOUSE,
};
use crate::input::Action;
use crate::ui::prompts::Prompt;
use crate::ui::theme::custom::map::marks::MarkKind;
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

fn roads(lines: Vec<Vec<[f32; 2]>>) -> MapRoadsHandle {
    let roads = lines.into_iter().map(|points| MapRoad { class: RoadClass::Main, points }).collect();
    MapRoadsHandle::new(MapRoads::from_roads(roads))
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

#[test]
fn a_network_without_a_calibration_is_fitted_into_the_middle_of_the_picture() {
    let network = roads(vec![vec![[0.0, 0.0], [1000.0, 0.0]], vec![[0.0, 0.0], [0.0, 500.0]]]);
    let fitted = fit::fitted(network.roads()).expect("the network has an extent");
    let middle = fitted.to_map([500.0, 250.0]);
    close(Vec2::from(middle), Vec2::splat(0.5));
    let left = fitted.to_map([0.0, 250.0])[0];
    assert!((left - FIT_MARGIN / (1.0 + 2.0 * FIT_MARGIN)).abs() < 1e-5, "the margin is kept: {left}");
    assert_eq!(fit::fitted(&MapRoads::default()), None, "no roads, no fit");
    assert_eq!(fit::fitted(roads(vec![vec![[5.0, 5.0], [5.0, 5.0]]]).roads()), None, "a point has no extent");
}

#[test]
fn the_marks_are_the_current_tabs_blips_and_the_routes_end() {
    let kinds = |filter| places::marks(&map(filter, None)).iter().map(|mark| mark.kind).collect::<Vec<_>>();
    assert_eq!(kinds(Filter::Locations), [MarkKind::SafeHouse, MarkKind::Shop]);
    assert_eq!(kinds(Filter::Races), [MarkKind::Race]);
    assert_eq!(kinds(Filter::Pursuit), [MarkKind::Roadblock]);
    let route = NavRoute { path: vec![[0.0, 0.0], [300.0, 0.0]], target: [300.0, 0.0], next_turn: None };
    let routed = WorldMapFrame { route: Some(route), ..map(Filter::Locations, None) };
    let marks = places::marks(&routed);
    let last = marks.last().expect("the route's end");
    assert_eq!((last.kind, last.at), (MarkKind::Target, [300.0, 0.0]));
    assert_eq!(places::marks(&routed)[0].at, [-300.0, 200.0], "a blip is marked at its world place");
    assert_eq!(kind_of(CAR_LOT), MarkKind::Shop);
}

#[test]
fn the_legend_rows_are_the_tabs_styles_with_their_counts() {
    let rows = places::legend_rows(&map(Filter::Locations, None));
    let named: Vec<(&str, usize)> = rows.iter().map(|row| (row.name.as_str(), row.count)).collect();
    assert_eq!(named, [("Auto shop", 1), ("Safe house", 1), ("Car lot", 0)]);
    let races = places::legend_rows(&map(Filter::Races, None));
    assert_eq!(races.len(), Filter::Races.styles().len());
    assert!(races.iter().all(|row| row.kind == MarkKind::Race));
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

#[test]
fn the_roads_and_the_route_add_their_strokes() {
    let bare = built(Some(snapshot(map(Filter::Locations, None)))).len();
    let network = roads(vec![vec![[-500.0, 0.0], [500.0, 0.0]], vec![[0.0, -500.0], [0.0, 500.0]]]);
    let with_roads = WorldMapFrame { roads: Some(network.clone()), ..map(Filter::Locations, None) };
    let roaded = built(Some(snapshot(with_roads.clone()))).len();
    assert!(roaded > bare, "the roads draw");
    let route = NavRoute { path: vec![[0.0, 0.0], [0.0, 300.0]], target: [0.0, 300.0], next_turn: None };
    let routed = WorldMapFrame { route: Some(route), ..with_roads };
    assert!(built(Some(snapshot(routed))).len() > roaded, "the route and its end draw");
    // Without the city's calibration the network is fitted to the screen, so its roads still draw.
    let fitted = WorldMapFrame { calibration: None, roads: Some(network), ..map(Filter::Locations, None) };
    let unfitted = WorldMapFrame { calibration: None, ..map(Filter::Locations, None) };
    assert!(built(Some(snapshot(fitted))).len() > built(Some(snapshot(unfitted))).len());
}
