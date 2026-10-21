//! Tests for the Remastered minimap: the window it reads from the package, and what it draws from the heat and the chase. They
//! run on a hand-made package tree, with no game data and no GPU.

use blackbox_feng::fe_hash_upper;
use blackbox_ui::{BuildCx, DrawList, LayoutCx, NodeKind, Rect, Skin, Theme, UiNode, UiTree};
use glam::{Mat4, Vec3};

use super::{FLAME_OBJECT, MAP_RADIUS, Window, lit_segments, multiplier};
use crate::hud::{HudState, PursuitHud, roots};
use crate::ui::present::Screen;
use crate::ui::theme::custom::{DriveFeedCell, palette};
use crate::ui::theme::original::{FengSource, FrameCell};
use crate::ui::theme::remastered::RemasteredTheme;
use crate::ui::theme::remastered::skin::{keys, palette_skin, skin};
use crate::ui::theme::{ElementId, MenuListCell};

const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };
const CENTRE: (f32, f32) = (-190.0, 120.0);
/// The map pieces' own colour: white, as the package has them.
const WHITE: [u8; 4] = [255; 4];

fn test_skin() -> Skin {
    palette::remastered().skin().merged(skin()).merged(palette_skin(&palette::remastered()))
}

fn node(index: usize, name: u32, parent: Option<usize>, kind: NodeKind) -> UiNode {
    UiNode {
        index,
        name_hash: name,
        parent,
        kind,
        z: 100.0,
        visible: true,
        colour: WHITE,
        world_colour: WHITE,
        ..UiNode::default()
    }
}

fn image(texture: u32) -> NodeKind {
    NodeKind::Image { texture, uv: [0.0, 0.0, 1.0, 1.0], mask: None, mask_rotation: [0.5, 0.5, 0.0], mask_uv: [0.0; 4] }
}

/// A HUD package with the backing disc at `CENTRE` and the map group scrolled off it, turned by `turn` radians, one piece, the
/// arrow and the heat icon.
fn package(turn: f32, group_visible: bool, group_alpha: u8) -> UiTree {
    let mut map = node(0, fe_hash_upper("TRACK_MAP"), None, NodeKind::Group);
    map.z = 200.0;
    map.visible = group_visible;
    map.world_colour = [255, 255, 255, group_alpha];
    map.world = Mat4::from_translation(Vec3::new(CENTRE.0 + 31.0, CENTRE.1 - 17.0, 0.0)) * Mat4::from_rotation_z(turn);
    let mut backing = node(4, fe_hash_upper("TRACKMAPTARGETRING"), None, NodeKind::Group);
    backing.world = Mat4::from_translation(Vec3::new(CENTRE.0, CENTRE.1, 0.0));
    let piece = node(1, fe_hash_upper("TRACK_MAP1"), Some(0), image(5));
    let arrow = node(2, fe_hash_upper("PLAYERCARINDICATOR"), None, image(6));
    let flame = node(3, FLAME_OBJECT, None, image(7));
    let mut list = DrawList::new();
    for n in [map, piece, arrow, flame, backing] {
        list.push(n);
    }
    list.into_tree(|n| n.visible && !matches!(n.kind, NodeKind::Group))
}

fn built(tree: UiTree, state: &HudState) -> Vec<UiNode> {
    let frame = FrameCell::new();
    frame.set(FengSource::Hud, tree);
    let feed = DriveFeedCell::new();
    feed.set(state, SCREEN);
    let skin = test_skin();
    let mut component = RemasteredTheme::new(frame, feed, Default::default(), MenuListCell::new(), Default::default())
        .element(ElementId::Minimap.slot(), &BuildCx::new(&skin))
        .expect("the minimap is Remastered");
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    out.iter().cloned().collect()
}

fn quads_of(nodes: &[UiNode], colour: [u8; 4]) -> usize {
    nodes.iter().filter(|n| n.text.is_none() && n.colour == colour).count()
}

#[test]
fn the_window_stays_on_the_backing_disc_and_north_follows_the_map_group() {
    let window = Window::of(&package(0.0, true, 255)).expect("the group is shown");
    assert_eq!(window.centre, CENTRE);
    assert_eq!(window.radius, MAP_RADIUS);
    assert!(window.north.0.abs() < 1e-5 && (window.north.1 + 1.0).abs() < 1e-5, "{:?}", window.north);
    // Turned a quarter to the right, the map's north lies to the right of the window.
    let turned = Window::of(&package(std::f32::consts::FRAC_PI_2, true, 255)).expect("the group is shown");
    assert!((turned.north.0 - 1.0).abs() < 1e-5 && turned.north.1.abs() < 1e-5, "{:?}", turned.north);
}

#[test]
fn a_hidden_or_missing_group_draws_nothing() {
    assert!(Window::of(&package(0.0, false, 255)).is_none());
    assert!(built(package(0.0, false, 255), &HudState::default()).is_empty());
    let mut empty = DrawList::new();
    empty.push(node(0, 1, None, NodeKind::Group));
    assert!(Window::of(&empty.into_tree(|_| true)).is_none());
}

#[test]
fn the_heat_shows_as_a_multiplier_and_as_lit_segments() {
    assert_eq!((multiplier(0.0), multiplier(3.8), multiplier(25.0)), (1, 3, 10));
    assert_eq!((lit_segments(0.4), lit_segments(3.8), lit_segments(25.0)), (0, 3, 10));
    let nodes = built(package(0.0, true, 255), &HudState { vehicle_heat: 3.8, ..Default::default() });
    let labels: Vec<_> = nodes.iter().filter_map(|n| n.text.as_deref()).collect();
    assert_eq!(labels, ["N", "x3"]);
    // Each segment is the same number of quads: seven are dim, three are lit.
    let skin = test_skin();
    let dim = quads_of(&nodes, skin.colour(keys::MINIMAP_HEAT_DIM).expect("dim"));
    let lit = quads_of(&nodes, skin.colour(keys::MINIMAP_HEAT).expect("lit"));
    assert!(dim > 0 && dim.is_multiple_of(7), "{dim}");
    assert!(lit >= dim / 7 * 3, "{lit} lit against {dim} dim");
    let one = built(package(0.0, true, 255), &HudState { vehicle_heat: 9.5, ..Default::default() });
    assert_eq!(quads_of(&one, skin.colour(keys::MINIMAP_HEAT_DIM).expect("dim")), dim / 7, "one segment is left dim");
}

#[test]
fn the_package_pieces_are_darkened_and_the_arrow_takes_the_accent() {
    let nodes = built(package(0.0, true, 255), &HudState::default());
    let skin = test_skin();
    let map = skin.colour(keys::MINIMAP_MAP).expect("the map tint");
    let accent = skin.colour(keys::UNIT_HIGHLIGHT).expect("the accent");
    let textured: Vec<_> =
        nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture != 0)).collect();
    assert!(textured.iter().any(|n| n.colour == map), "the piece in the map tint");
    assert!(textured.iter().any(|n| n.colour == accent), "the arrow in the accent");
    assert!(textured.iter().all(|n| n.parent.is_none()), "the tinted copies carry no stale links");
}

#[test]
fn the_minimap_has_one_ring_for_the_heat_and_shows_nothing_of_the_chase() {
    let calm = built(package(0.0, true, 255), &HudState::default());
    let giving_up = PursuitHud { secs: 9.0, evade: 0.5, cooldown: true, busted: -0.5, ..Default::default() };
    let chase = built(package(0.0, true, 255), &HudState { pursuit: Some(giving_up), ..Default::default() });
    assert_eq!(chase.len(), calm.len(), "the cool-down and the meter are the pursuit panel's, not the minimap's");
}

#[test]
fn the_group_fades_the_whole_minimap() {
    let faded = built(package(0.0, true, 128), &HudState::default());
    assert!(!faded.is_empty());
    assert!(faded.iter().all(|n| n.colour[3] <= 128), "nothing is brighter than the group");
}

#[test]
fn the_original_minimap_claims_the_heat_group_so_the_remastered_one_replaces_it() {
    assert!(roots::minimap().roots.contains(&0xC46A_80A9));
    assert!(!roots::minimap_tiles().roots.contains(&fe_hash_upper("TRACK_MAP")), "the tiles are the pieces alone");
}
