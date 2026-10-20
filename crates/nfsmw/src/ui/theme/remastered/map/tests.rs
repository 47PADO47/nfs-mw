//! Tests for the Remastered map: what it builds from a snapshot, with no game data and no GPU.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, NodeKind, Rect, Theme, UiNode};

use super::{MapSnapshot, WorldMapViewCell};
use crate::frontend::world_map::{Camera, Chrome, Filter, PlacedBlip, WorldMap, WorldMapFrame};
use crate::hud::{AUTO_SHOP, RACE_SPRINT, SAFE_HOUSE};
use crate::input::Action;
use crate::ui::input_icons::Glyph;
use crate::ui::prompts::Prompt;
use crate::ui::theme::custom::{DriveFeedCell, palette};
use crate::ui::theme::remastered::{RemasteredTheme, palette_skin, skin};
use crate::ui::theme::{ElementId, MenuHint, MenuListCell};

const ASPECT: f32 = 16.0 / 9.0;

fn blip(style: &'static str, map: [f32; 2]) -> PlacedBlip {
    PlacedBlip { style, world: [300.0, 400.0], map }
}

fn snapshot(session: WorldMap) -> MapSnapshot {
    let blips = vec![blip(AUTO_SHOP, [0.5, 0.5]), blip(SAFE_HOUSE, [0.6, 0.5]), blip(RACE_SPRINT, [0.5, 0.6])];
    let frame = WorldMapFrame {
        session,
        blips,
        chrome: Chrome::Drawn,
        texture: "TRACK_MAP",
        clock: 5.0,
        snap_age: 5.0,
        ..Default::default()
    };
    let hints = vec![
        MenuHint { keys: vec![Prompt::Key("Esc".into())], label: "Back", action: Some(Action::MenuBack) },
        MenuHint { keys: vec![Prompt::Icon(Glyph::Horizontal)], label: "Filter", action: None },
    ];
    MapSnapshot { frame, hints, aspect: ASPECT }
}

fn built(snapshot: Option<MapSnapshot>) -> Vec<UiNode> {
    let cell = WorldMapViewCell::new();
    if let Some(snapshot) = snapshot {
        cell.set(snapshot);
    }
    let skin = palette::remastered().skin().merged(crate::ui::theme::custom::card_skin()).merged(skin());
    let skin = skin.merged(palette_skin(&palette::remastered()));
    let theme =
        RemasteredTheme::new(Default::default(), DriveFeedCell::new(), Default::default(), MenuListCell::new(), cell);
    let mut map =
        theme.element(ElementId::WorldMapChrome.slot(), &BuildCx::new(&skin)).expect("Remastered draws the map");
    map.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    map.build(&mut out);
    out.iter().cloned().collect()
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

fn pictures(nodes: &[UiNode]) -> Vec<&UiNode> {
    let map = blackbox_feng::resource_handle("TRACK_MAP");
    nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture == map)).collect()
}

fn icons<'a>(nodes: &'a [UiNode], file: &str) -> Vec<&'a UiNode> {
    let key = blackbox_feng::resource_handle(file);
    nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture == key)).collect()
}

#[test]
fn a_closed_map_draws_nothing() {
    assert!(built(None).is_empty());
}

#[test]
fn the_picture_is_the_screens_height_at_zoom_all_and_grows_with_the_zoom() {
    let nodes = built(Some(snapshot(WorldMap::new())));
    let picture = pictures(&nodes);
    assert_eq!(picture.len(), 1, "the career stage's map, once");
    assert_eq!((picture[0].world.x_axis.x, picture[0].world.y_axis.y), (480.0, 480.0));
    assert_eq!(picture[0].world.w_axis.truncate().truncate(), glam::Vec2::ZERO, "centred");

    let mut zoomed = WorldMap::new();
    zoomed.camera = Camera::at_level(1, [0.5, 0.5]);
    let nodes = built(Some(snapshot(zoomed)));
    assert_eq!(pictures(&nodes)[0].world.x_axis.x, 960.0, "zoom 2 doubles it");
}

#[test]
fn only_the_blips_of_the_tab_are_drawn_as_the_games_icons_and_the_legend_names_them() {
    let nodes = built(Some(snapshot(WorldMap::new())));
    assert_eq!(icons(&nodes, "Minimap_Icon_Chop_Shop.tga").len(), 2, "the blip and its legend row");
    assert_eq!(icons(&nodes, "Minimap_Icon_Safe_House.tga").len(), 2);
    assert_eq!(icons(&nodes, "Minimap_Icon_Event.tga").len(), 0, "a race is not on the Locations tab");
    let all = texts(&nodes);
    for name in ["MAP", "LOCATIONS", "PLAYER", "AUTO SHOP", "SAFE HOUSE", "CAR LOT", "Back", "Filter"] {
        assert!(all.contains(&name), "{name} in {all:?}");
    }

    let mut races = WorldMap::new();
    races.filter = Filter::Races;
    let nodes = built(Some(snapshot(races)));
    assert_eq!(
        icons(&nodes, "Minimap_Icon_Event.tga").len(),
        5,
        "the sprint, and the four legend rows that share its icon"
    );
    assert!(texts(&nodes).contains(&"SPRINT") && texts(&nodes).contains(&"RACES"));
}

#[test]
fn the_legend_can_be_hidden() {
    let mut session = WorldMap::new();
    session.legend_open = false;
    let nodes = built(Some(snapshot(session)));
    assert!(!texts(&nodes).contains(&"PLAYER") && !texts(&nodes).contains(&"AUTO SHOP"));
    assert_eq!(icons(&nodes, "Minimap_Icon_Chop_Shop.tga").len(), 1, "the blip itself stays");
}

#[test]
fn a_blip_outside_the_window_is_not_drawn() {
    let mut zoomed = WorldMap::new();
    zoomed.camera = Camera::at_level(3, [0.5, 0.5]);
    let mut snap = snapshot(zoomed);
    snap.frame.blips[1].map = [0.95, 0.5];
    let nodes = built(Some(snap));
    assert_eq!(icons(&nodes, "Minimap_Icon_Safe_House.tga").len(), 1, "only the legend row");
}

#[test]
fn a_focused_blip_gets_brackets_and_a_label_with_its_distance() {
    let mut session = WorldMap::new();
    session.attached = Some(0);
    session.cursor = Some([0.5, 0.5]);
    let nodes = built(Some(snapshot(session)));
    let label = texts(&nodes).into_iter().find(|t| t.starts_with("AUTO SHOP  ·")).expect("the label");
    assert!(label.ends_with("500 m"), "{label}");
    let brackets = nodes
        .iter()
        .filter(|n| {
            matches!(n.kind, NodeKind::Image { texture: 0, .. }) && n.world.x_axis.x < 8.0 && n.world.y_axis.y < 8.0
        })
        .count();
    assert!(brackets >= 8, "eight strokes of four corner brackets, got {brackets}");
}

#[test]
fn the_brackets_close_in_over_the_snap() {
    let bracket_spread = |age: f32| {
        let mut session = WorldMap::new();
        session.attached = Some(0);
        let mut snap = snapshot(session);
        snap.frame.snap_age = age;
        let nodes = built(Some(snap));
        let xs: Vec<f32> = nodes
            .iter()
            .filter(|n| {
                matches!(n.kind, NodeKind::Image { texture: 0, .. })
                    && n.world.x_axis.x < 3.0
                    && (n.world.y_axis.y - 7.0).abs() < 1e-3
            })
            .map(|n| n.world.w_axis.x)
            .collect();
        xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min)
    };
    assert!(bracket_spread(0.0) > bracket_spread(1.0) + 10.0, "far at the start, near once it has settled");
}

#[test]
fn a_free_pointer_draws_a_cross_and_no_label() {
    let mut session = WorldMap::new();
    session.cursor = Some([0.1, 0.1]);
    let nodes = built(Some(snapshot(session)));
    assert!(!texts(&nodes).iter().any(|t| t.contains("  ·  ")), "no label without a focused blip");
    let strokes = nodes
        .iter()
        .filter(|n| {
            matches!(n.kind, NodeKind::Image { texture: 0, .. })
                && n.world.w_axis.x < -150.0
                && n.world.w_axis.y < -150.0
        })
        .count();
    assert!(strokes >= 4, "the cross's four strokes near the pointer");
}

#[test]
fn the_map_fades_in_as_it_opens() {
    let alpha_of_ground = |clock: f32| {
        let mut snap = snapshot(WorldMap::new());
        snap.frame.clock = clock;
        built(Some(snap))[0].colour[3]
    };
    assert!(alpha_of_ground(0.0) < alpha_of_ground(0.1));
    assert!(alpha_of_ground(0.1) < alpha_of_ground(1.0));
}
