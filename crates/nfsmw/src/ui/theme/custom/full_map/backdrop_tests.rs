//! Tests for the full-screen map's backdrop: the scrim over the whole scene and the plate under the picture come from the
//! theme's tokens, fade in with the motion table, and are clearly stronger than the HUD map's own light plate.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Roles, Skin, Tokens, UiNode};

use super::{FullMap, FullMapCell, FullMapSnapshot};
use crate::frontend::world_map::{Chrome, WorldMapFrame};
use crate::ui::theme::custom::map::backdrop::plate_colour;
use crate::ui::theme::custom::menu::MenuLook;
use crate::ui::theme::custom::tokens::{CustomLook, Motion, skin};
use crate::ui::theme::default_skin;

const ASPECT: f32 = 16.0 / 9.0;
const SCRIM: [u8; 4] = [1, 2, 3, 200];
const OVERLAY: [u8; 4] = [4, 5, 6, 240];

fn tokens(scrim: [u8; 4], overlay: [u8; 4]) -> Tokens {
    Tokens { roles: Roles { scrim: Some(scrim), overlay: Some(overlay), ..Roles::default() }, ..Tokens::default() }
}

fn themed_skin(tokens: &Tokens, look: MenuLook) -> Skin {
    default_skin().merged(tokens.skin()).merged(skin(CustomLook::default(), tokens.secondary)).merged(look.skin())
}

fn snapshot() -> FullMapSnapshot {
    let frame = WorldMapFrame { chrome: Chrome::Drawn, ..Default::default() };
    FullMapSnapshot { frame, hints: Vec::new(), aspect: ASPECT }
}

fn open_map() -> FullMapCell {
    let cell = FullMapCell::new();
    cell.set(snapshot());
    cell
}

/// The nodes `map` builds at layout time `now` in `skin`.
fn at(map: &mut FullMap, skin: &Skin, now: f32) -> Vec<UiNode> {
    map.layout(&LayoutCx::new(skin, now), Rect::SCREEN);
    let mut out = DrawList::new();
    map.build(&mut out);
    out.iter().cloned().collect()
}

/// The first node whose colour has these channels, and its alpha.
fn alpha_of(nodes: &[UiNode], rgb: [u8; 3]) -> Option<u8> {
    nodes.iter().find(|node| node.colour[..3] == rgb).map(|node| node.colour[3])
}

fn width_of(nodes: &[UiNode], rgb: [u8; 3]) -> f32 {
    let node = nodes.iter().find(|node| node.colour[..3] == rgb).expect("the node is drawn");
    node.world.x_axis.truncate().length()
}

#[test]
fn the_scrim_covers_the_whole_window_and_the_plate_the_pictures_square() {
    let skin = themed_skin(&tokens(SCRIM, OVERLAY), MenuLook::default());
    let mut map = FullMap::new(open_map());
    at(&mut map, &skin, 5.0);
    let nodes = at(&mut map, &skin, 50.0);
    let window = 480.0 * ASPECT;
    assert!((width_of(&nodes, [1, 2, 3]) - window).abs() < 0.5, "the scrim is as wide as the window");
    assert!(
        (width_of(&nodes, [4, 5, 6]) - 480.0).abs() < 0.5,
        "the plate is the picture's square, the screen's height"
    );
}

#[test]
fn the_backdrop_is_the_themes_scrim_and_overlay_once_open() {
    let skin = themed_skin(&tokens(SCRIM, OVERLAY), MenuLook::default());
    let mut map = FullMap::new(open_map());
    at(&mut map, &skin, 5.0);
    let nodes = at(&mut map, &skin, 50.0);
    assert_eq!(alpha_of(&nodes, [1, 2, 3]), Some(SCRIM[3]));
    assert_eq!(alpha_of(&nodes, [4, 5, 6]), Some(OVERLAY[3]));
    // Another theme, another backdrop.
    let other = themed_skin(&tokens([7, 8, 9, 120], [10, 11, 12, 180]), MenuLook::default());
    let mut again = FullMap::new(open_map());
    at(&mut again, &other, 5.0);
    let nodes = at(&mut again, &other, 50.0);
    assert_eq!((alpha_of(&nodes, [7, 8, 9]), alpha_of(&nodes, [10, 11, 12])), (Some(120), Some(180)));
    assert_eq!(alpha_of(&nodes, [1, 2, 3]), None, "nothing of the first theme is left");
}

#[test]
fn the_backdrop_fades_in_over_the_motion_tables_map_change_and_a_reduced_one_over_its_shorter_time() {
    let tokens = tokens(SCRIM, OVERLAY);
    let full = themed_skin(&tokens, MenuLook::new(1.0, false));
    let mut map = FullMap::new(open_map());
    assert_eq!(alpha_of(&at(&mut map, &full, 10.0), [1, 2, 3]), Some(0), "the first frame starts from nothing");
    let half = at(&mut map, &full, 10.0 + Motion::FULL.mode_change / 2.0);
    let alpha = alpha_of(&half, [1, 2, 3]).expect("drawn");
    assert!(
        (i32::from(alpha) - i32::from(SCRIM[3]) / 2).abs() <= 8,
        "about half way (eased, so it is exactly half at the middle): {alpha}"
    );
    assert_eq!(alpha_of(&at(&mut map, &full, 10.0 + Motion::FULL.mode_change), [1, 2, 3]), Some(SCRIM[3]));
    // With motion reduction the same fade is shorter.
    let reduced = themed_skin(&tokens, MenuLook::new(1.0, true));
    let mut quick = FullMap::new(open_map());
    at(&mut quick, &reduced, 10.0);
    let done = at(&mut quick, &reduced, 10.0 + Motion::REDUCED.mode_change);
    assert_eq!(alpha_of(&done, [1, 2, 3]), Some(SCRIM[3]));
    const { assert!(Motion::REDUCED.mode_change < Motion::FULL.mode_change) };
}

#[test]
fn closing_the_map_starts_the_next_opening_from_nothing() {
    let skin = themed_skin(&tokens(SCRIM, OVERLAY), MenuLook::default());
    let cell = open_map();
    let mut map = FullMap::new(cell.clone());
    at(&mut map, &skin, 5.0);
    assert_eq!(alpha_of(&at(&mut map, &skin, 50.0), [1, 2, 3]), Some(SCRIM[3]));
    cell.clear();
    assert!(at(&mut map, &skin, 51.0).is_empty(), "a closed map draws nothing");
    cell.set(snapshot());
    assert_eq!(alpha_of(&at(&mut map, &skin, 60.0), [1, 2, 3]), Some(0), "closing let the memory go");
}

#[test]
fn the_full_maps_backdrop_is_clearly_stronger_than_the_hud_maps_plate() {
    let skin = themed_skin(&tokens(SCRIM, OVERLAY), MenuLook::default());
    let cx = LayoutCx::new(&skin, 0.0);
    let hud = plate_colour(&cx, 1.0)[3];
    assert!(hud > 0, "the HUD map has a plate");
    assert!(u16::from(SCRIM[3]) >= 2 * u16::from(hud), "the scrim alone is at least twice the HUD plate: {hud}");
    assert!(u16::from(OVERLAY[3]) > 2 * u16::from(hud), "and the plate under the picture is stronger still");
    // With the built-in palette, whose values are the product's.
    let bevy = crate::ui::theme::custom::palette::bevy();
    let built_in = themed_skin(&bevy, MenuLook::default());
    let hud = plate_colour(&LayoutCx::new(&built_in, 0.0), 1.0)[3];
    let roles = bevy.resolved_roles();
    assert!(roles.scrim[3] > 2 * hud && roles.overlay[3] > roles.scrim[3], "{hud} {:?}", roles.scrim);
}
