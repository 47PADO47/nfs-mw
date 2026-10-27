//! Tests that the map's colours are the active theme's: no fixed colour in the map's sources, and a theme's tokens reach
//! the strokes and shapes it builds.

use std::sync::Arc;

use blackbox_minimap::Orientation;
use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Roles, Skin, Tokens, UiNode};

use super::CustomMap;
use crate::hud::{HudState, MapPosition, MapRoad, MapRoads, MapRoadsHandle, NavRoute, RoadClass};
use crate::settings::hud::{HudSettings, MapMode};
use crate::ui::present::Screen;
use crate::ui::theme::custom::feed::{CustomFeed, CustomFeedCell};
use crate::ui::theme::custom::tokens::{CustomLook, skin};

const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

/// The sources of the map and the full map (not their tests). A colour in them must come from the skin.
const SOURCES: [(&str, &str); 14] = [
    ("map/backdrop.rs", include_str!("backdrop.rs")),
    ("map/chip.rs", include_str!("chip.rs")),
    ("map/draw.rs", include_str!("draw.rs")),
    ("map/icons.rs", include_str!("icons.rs")),
    ("map/marks.rs", include_str!("marks.rs")),
    ("map/mod.rs", include_str!("mod.rs")),
    ("map/modes.rs", include_str!("modes.rs")),
    ("map/picture.rs", include_str!("picture.rs")),
    ("map/roads.rs", include_str!("roads.rs")),
    ("full_map/card.rs", include_str!("../full_map/card.rs")),
    ("full_map/chrome.rs", include_str!("../full_map/chrome.rs")),
    ("full_map/fit.rs", include_str!("../full_map/fit.rs")),
    ("full_map/mod.rs", include_str!("../full_map/mod.rs")),
    ("full_map/places.rs", include_str!("../full_map/places.rs")),
];

/// What a fixed colour looks like in Rust: an array literal of byte channels, a hex byte list, or an `rgb` helper.
const FIXED_COLOUR: [&str; 6] = ["[255,", "[0, 0, 0", "0x", "255, 255", "from_rgb", "Color::"];

/// The code of a source file: everything before its inline test module, with the line endings normalised.
fn code_of(source: &str) -> String {
    let source = source.replace("\r\n", "\n");
    source.split("\n#[cfg(test)]\nmod tests {").next().unwrap_or(&source).to_owned()
}

#[test]
fn the_maps_sources_hold_no_fixed_colour() {
    for (name, source) in SOURCES {
        for (n, line) in code_of(source).lines().enumerate() {
            let line = line.trim_start();
            if line.starts_with("//") {
                continue;
            }
            for pattern in FIXED_COLOUR {
                assert!(!line.contains(pattern), "{name}:{} holds a fixed colour ({pattern}): {line}", n + 1);
            }
        }
    }
}

#[test]
fn the_maps_strokes_and_shapes_are_drawn_in_theme_keys_not_the_huds_own() {
    // The map draws in `theme.*` tokens (`token_keys::`). The `custom.*` keys are the HUD's own (`keys::`); a label or a
    // plate of the full map's chrome may name one, a stroke or a shape colour may not.
    for (name, source) in SOURCES {
        for line in code_of(source).lines() {
            let line = line.trim_start();
            let draws = line.contains("colour(cx") || line.contains("Polyline");
            let bare = line.match_indices("keys::").any(|(at, _)| !line[..at].ends_with("token_"));
            assert!(!(draws && bare), "{name} draws a stroke in a HUD key: {line}");
        }
    }
}

fn car() -> MapPosition {
    MapPosition { position: [0.0, 0.0], heading: [1.0, 0.0], orientation: Orientation::North }
}

fn feed(hud: HudState) -> CustomFeed {
    CustomFeed {
        hud: Arc::new(hud),
        settings: HudSettings { map_mode: MapMode::Centered, ..HudSettings::default() },
        screen: SCREEN,
        ..CustomFeed::default()
    }
}

/// The nodes the map builds for `hud` in a skin carrying `tokens`.
fn built(hud: HudState, tokens: &Tokens) -> Vec<UiNode> {
    let skin: Skin = skin(CustomLook::default(), tokens.secondary).merged(tokens.skin());
    let cell = CustomFeedCell::new();
    cell.set(feed(hud));
    let mut map = CustomMap::new(cell);
    map.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    map.build(&mut out);
    out.iter().cloned().collect()
}

fn roads() -> MapRoadsHandle {
    let road = MapRoad { class: RoadClass::Side, points: vec![[-100.0, 0.0], [100.0, 0.0]] };
    MapRoadsHandle::new(MapRoads::from_roads(vec![road]))
}

fn has_rgb(nodes: &[UiNode], rgb: [u8; 3]) -> bool {
    nodes.iter().any(|node| node.colour[..3] == rgb)
}

fn themed(roles: Roles) -> Tokens {
    Tokens { roles, ..Tokens::default() }
}

#[test]
fn the_roads_take_the_themes_line_colour() {
    let hud = HudState { map_car: Some(car()), map_roads: Some(roads()), ..HudState::default() };
    let first = themed(Roles { line_minor: Some([1, 2, 3, 255]), ..Roles::default() });
    let second = themed(Roles { line_minor: Some([9, 8, 7, 255]), ..Roles::default() });
    let (a, b) = (built(hud.clone(), &first), built(hud, &second));
    assert!(has_rgb(&a, [1, 2, 3]) && !has_rgb(&a, [9, 8, 7]), "the first theme's side roads");
    assert!(has_rgb(&b, [9, 8, 7]) && !has_rgb(&b, [1, 2, 3]), "the second theme's side roads");
}

#[test]
fn a_theme_with_no_roles_draws_its_roads_in_a_tone_derived_from_its_muted_colour() {
    let hud = HudState { map_car: Some(car()), map_roads: Some(roads()), ..HudState::default() };
    let tokens = Tokens { muted: [40, 50, 60, 255], ..Tokens::default() };
    assert!(has_rgb(&built(hud, &tokens), [40, 50, 60]), "line_minor derives from muted");
}

#[test]
fn the_route_takes_the_themes_guide_colour() {
    let route = NavRoute { path: vec![[0.0, 0.0], [100.0, 0.0]], target: [100.0, 0.0], next_turn: None };
    let hud = HudState { map_car: Some(car()), route: Some(route), ..HudState::default() };
    let tokens = themed(Roles { guide: Some([11, 22, 33, 255]), ..Roles::default() });
    assert!(has_rgb(&built(hud.clone(), &tokens), [11, 22, 33]), "the route and its end");
    let other = themed(Roles { guide: Some([44, 55, 66, 255]), ..Roles::default() });
    assert!(!has_rgb(&built(hud, &other), [11, 22, 33]));
}

#[test]
fn the_players_arrow_is_the_themes_primary_colour() {
    let hud = HudState { map_car: Some(car()), ..HudState::default() };
    let tokens = Tokens { primary: [70, 80, 90, 255], ..Tokens::default() };
    assert!(has_rgb(&built(hud, &tokens), [70, 80, 90]));
}
