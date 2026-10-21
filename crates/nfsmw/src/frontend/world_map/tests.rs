use super::camera::MAX_ZOOM;
use super::legend::VISIBLE_ROWS;
use super::*;
use crate::hud::{AUTO_SHOP, CAR_LOT, RACE_SPRINT, SAFE_HOUSE};

fn blip(style: &'static str, map: [f32; 2]) -> PlacedBlip {
    PlacedBlip { style, world: [map[0] * 1000.0, map[1] * 1000.0], map }
}

/// A safe house beside a sprint race (the race is exactly under the cursor, so a naive pick would take it), and a
/// car lot far away.
fn blips() -> Vec<PlacedBlip> {
    vec![blip(SAFE_HOUSE, [0.21, 0.2]), blip(RACE_SPRINT, [0.2, 0.2]), blip(CAR_LOT, [0.8, 0.8])]
}

fn hover(at: [f32; 2]) -> MapInput {
    MapInput { cursor: Some(at), ..Default::default() }
}

fn click(at: [f32; 2]) -> MapInput {
    MapInput { cursor: Some(at), click: true, ..Default::default() }
}

fn press_accept() -> MapInput {
    MapInput { accept: true, ..Default::default() }
}

#[test]
fn the_map_opens_on_locations_zoomed_out() {
    let map = WorldMap::new();
    assert_eq!(map.filter, Filter::Locations);
    assert_eq!(map.camera, Camera::zoom_all());
    assert_eq!(map.attached, None);
    assert_eq!(map.cursor, None);
    assert_eq!(map.list_scroll, 0);
}

#[test]
fn the_filter_tabs_cycle_and_wrap_both_ways() {
    assert_eq!(Filter::Locations.next(), Filter::Races);
    assert_eq!(Filter::Races.next(), Filter::Pursuit);
    assert_eq!(Filter::Pursuit.next(), Filter::Locations);
    assert_eq!(Filter::Locations.previous(), Filter::Pursuit);
    assert_eq!(Filter::Races.previous(), Filter::Locations);
}

#[test]
fn each_filter_shows_only_its_own_styles() {
    assert!(Filter::Locations.contains(AUTO_SHOP));
    assert!(Filter::Locations.contains(SAFE_HOUSE));
    assert!(Filter::Locations.contains(CAR_LOT));
    assert!(!Filter::Locations.contains(RACE_SPRINT));
    assert!(Filter::Races.contains(RACE_SPRINT));
    assert!(!Filter::Races.contains(SAFE_HOUSE));
    assert!(Filter::Pursuit.contains(crate::hud::ROADBLOCK));
    assert!(!Filter::Races.contains(crate::hud::ROADBLOCK));
}

#[test]
fn the_list_is_the_blips_of_the_selected_type() {
    let mut map = WorldMap::new();
    assert_eq!(map.listed(&blips()), vec![0, 2]);
    map.frame(&MapInput { next_filter: true, ..Default::default() }, &blips());
    assert_eq!(map.listed(&blips()), vec![1]);
}

#[test]
fn zoom_all_is_the_whole_picture_and_zooming_stays_inside_it() {
    let mut camera = Camera::zoom_all();
    assert_eq!(camera.to_view([0.25, 0.75]), [0.25, 0.75]);
    camera.step(4.0);
    assert!(camera.zoom() > 1.0);
    camera.step(-100.0);
    assert_eq!(camera.zoom(), 1.0, "zooming out stops at Zoom All");
    camera.step(1000.0);
    assert_eq!(camera.zoom(), MAX_ZOOM, "zooming in stops at the maximum");
    let half = 0.5 / camera.zoom();
    assert!(camera.centre.iter().all(|c| *c >= half - 1e-6 && *c <= 1.0 - half + 1e-6));
}

#[test]
fn panning_cannot_leave_the_picture() {
    let mut camera = Camera::zoom_all();
    camera.step(3.0);
    camera.pan([-50.0, 50.0]);
    let half = 0.5 / camera.zoom();
    assert!((camera.centre[0] - half).abs() < 1e-6, "dragged to the left edge");
    assert!((camera.centre[1] - (1.0 - half)).abs() < 1e-6, "dragged to the top edge");
}

#[test]
fn hovering_a_blip_focuses_the_nearest_of_the_filter() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    assert_eq!(map.cursor, Some([0.2, 0.2]), "the cursor follows the mouse");
    assert_eq!(map.attached, Some(0), "the race is nearer but another filter, so the safe house is focused");
}

#[test]
fn hovering_off_a_blip_on_the_picture_unfocuses() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    assert_eq!(map.attached, Some(0));
    map.frame(&hover([0.5, 0.5]), &blips());
    assert_eq!(map.attached, None, "moving off a blip (still on the picture) unfocuses");
}

#[test]
fn clicking_a_blip_asks_for_gps() {
    let mut map = WorldMap::new();
    assert_eq!(map.frame(&click([0.2, 0.2]), &blips()), Outcome::AskGps(0), "a click on a blip asks for GPS");
    assert_eq!(map.attached, Some(0));
}

#[test]
fn nearest_picks_the_closest_within_the_radius_and_ties_go_early() {
    let points = [[0.5, 0.5], [0.52, 0.5], [0.9, 0.9]];
    assert_eq!(nearest(&points, [0.51, 0.5], PICK_RADIUS), Some(0), "tie: the earlier point");
    assert_eq!(nearest(&points, [0.53, 0.5], PICK_RADIUS), Some(1));
    assert_eq!(nearest(&points, [0.0, 0.0], PICK_RADIUS), None);
}

#[test]
fn enter_asks_for_gps_for_the_focused_blip() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    assert_eq!(map.frame(&press_accept(), &blips()), Outcome::AskGps(0));
}

#[test]
fn right_click_asks_for_gps_too() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    let secondary = MapInput { secondary: true, ..Default::default() };
    assert_eq!(map.frame(&secondary, &blips()), Outcome::AskGps(0));
}

#[test]
fn a_frame_with_no_cursor_keeps_the_focus() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    map.frame(&MapInput::default(), &blips());
    assert_eq!(map.attached, Some(0), "a key frame (no cursor) keeps the focus");
    assert_eq!(map.frame(&press_accept(), &blips()), Outcome::AskGps(0));
}

#[test]
fn enter_with_nothing_focused_does_nothing() {
    let mut map = WorldMap::new();
    assert_eq!(map.frame(&press_accept(), &blips()), Outcome::Stay);
    assert_eq!(map.attached, None);
}

#[test]
fn scrolling_zooms_over_the_picture_and_scrolls_the_list_off_it() {
    let mut map = WorldMap::new();
    // Off the picture (over the legend) the wheel does not zoom.
    map.frame(&MapInput { cursor: Some([1.5, 0.5]), zoom_lines: 3.0, ..Default::default() }, &blips());
    assert_eq!(map.camera, Camera::zoom_all(), "scrolling off the picture does not zoom");
    // Over the picture it zooms.
    map.frame(&MapInput { cursor: Some([0.5, 0.5]), zoom_lines: 3.0, ..Default::default() }, &blips());
    assert!(map.camera.zoom() > 1.0, "scrolling over the picture zooms");
}

#[test]
fn scrolling_off_the_picture_moves_the_list_within_its_bounds() {
    let many: Vec<PlacedBlip> = (0..20).map(|i| blip(SAFE_HOUSE, [0.1, i as f32 / 20.0])).collect();
    let mut map = WorldMap::new();
    let off = |lines| MapInput { cursor: Some([1.5, 0.5]), zoom_lines: lines, ..Default::default() };
    map.frame(&off(-2.0), &many);
    assert_eq!(map.list_scroll, 2, "wheel down advances the list");
    map.frame(&off(1.0), &many);
    assert_eq!(map.list_scroll, 1, "wheel up goes back");
    map.frame(&off(-100.0), &many);
    assert_eq!(map.list_scroll, 20 - VISIBLE_ROWS, "the list stops at the last page");
}

#[test]
fn back_unfocuses_then_closes() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    let back = MapInput { back: true, ..Default::default() };
    assert_eq!(map.frame(&back, &blips()), Outcome::Stay);
    assert_eq!(map.attached, None, "back unfocuses first");
    assert_eq!(map.frame(&back, &blips()), Outcome::Close, "then it closes");
}

#[test]
fn switching_the_tab_drops_the_focus_and_the_scroll() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    map.list_scroll = 3;
    map.frame(&MapInput { next_filter: true, ..Default::default() }, &blips());
    assert_eq!(map.attached, None, "switching the tab unfocuses");
    assert_eq!(map.list_scroll, 0, "and resets the list scroll");
}

#[test]
fn zoom_all_puts_the_camera_back() {
    let mut map = WorldMap::new();
    map.frame(&MapInput { cursor: Some([0.5, 0.5]), zoom_lines: 3.0, ..Default::default() }, &blips());
    assert!(map.camera.zoom() > 1.0);
    map.frame(&MapInput { zoom_all: true, ..Default::default() }, &blips());
    assert_eq!(map.camera, Camera::zoom_all());
}

#[test]
fn the_distance_is_straight_line_metres() {
    let blip = PlacedBlip { style: CAR_LOT, world: [300.0, 400.0], map: [0.0, 0.0] };
    assert!((distance_metres([0.0, 0.0], &blip) - 500.0).abs() < 1e-3);
}

#[test]
fn every_map_icon_file_is_a_texture_of_the_install() {
    let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let assets = crate::ui::UiAssets::load(&dir).unwrap();
    for file in control::MAP_ICON_FILES {
        let key = blackbox_feng::resource_handle(file);
        assert!(assets.texture(key).is_some(), "{file} is not a texture of the install");
    }
}
