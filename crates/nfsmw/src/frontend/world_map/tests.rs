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

fn press_accept() -> MapInput {
    MapInput { accept: true, ..Default::default() }
}

#[test]
fn the_map_opens_on_locations_zoomed_out() {
    let map = WorldMap::new();
    assert_eq!(map.filter, Filter::Locations);
    assert_eq!(map.camera, Camera::zoom_all());
    assert_eq!(map.attached, None);
    assert!(!map.prompt);
}

#[test]
fn the_filter_tabs_cycle_and_wrap_both_ways() {
    assert_eq!(Filter::Locations.next(), Filter::Races);
    assert_eq!(Filter::Races.next(), Filter::Locations);
    assert_eq!(Filter::Locations.previous(), Filter::Races);
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
fn the_cursor_attaches_to_the_nearest_blip_of_the_filter() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    assert_eq!(map.attached, Some(0), "the race is nearer but another filter, so the safe house is taken");
}

#[test]
fn nothing_attaches_outside_the_pick_radius() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.5, 0.5]), &blips());
    assert_eq!(map.attached, None);
    map.frame(&MapInput::default(), &blips());
    assert_eq!(map.attached, None, "no cursor, nothing attached");
}

#[test]
fn nearest_picks_the_closest_within_the_radius_and_ties_go_early() {
    let points = [[0.5, 0.5], [0.52, 0.5], [0.9, 0.9]];
    assert_eq!(nearest(&points, [0.51, 0.5], PICK_RADIUS), Some(0), "tie: the earlier point");
    assert_eq!(nearest(&points, [0.53, 0.5], PICK_RADIUS), Some(1));
    assert_eq!(nearest(&points, [0.0, 0.0], PICK_RADIUS), None);
}

#[test]
fn enter_asks_then_confirms_the_gps_for_the_attached_blip() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    assert_eq!(map.frame(&press_accept(), &blips()), Outcome::Stay);
    assert!(map.prompt, "the first Enter opens the prompt");
    assert_eq!(map.frame(&press_accept(), &blips()), Outcome::ActivateGps(0));
}

#[test]
fn right_click_opens_the_same_prompt() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    let secondary = MapInput { secondary: true, ..hover([0.2, 0.2]) };
    assert_eq!(map.frame(&secondary, &blips()), Outcome::Stay);
    assert!(map.prompt);
}

#[test]
fn the_prompt_keeps_its_blip_when_the_cursor_moves_away() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    map.frame(&press_accept(), &blips());
    map.frame(&hover([0.8, 0.8]), &blips());
    assert_eq!(map.attached, Some(0), "the prompt still names the safe house");
    assert_eq!(map.frame(&press_accept(), &blips()), Outcome::ActivateGps(0));
}

#[test]
fn enter_with_nothing_attached_does_nothing() {
    let mut map = WorldMap::new();
    assert_eq!(map.frame(&press_accept(), &blips()), Outcome::Stay);
    assert!(!map.prompt);
}

#[test]
fn back_cancels_the_prompt_before_it_closes_the_map() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    map.frame(&press_accept(), &blips());
    let back = MapInput { back: true, ..Default::default() };
    assert_eq!(map.frame(&back, &blips()), Outcome::Stay);
    assert!(!map.prompt);
    assert_eq!(map.frame(&back, &blips()), Outcome::Close);
}

#[test]
fn switching_the_tab_drops_the_prompt_and_the_attachment() {
    let mut map = WorldMap::new();
    map.frame(&hover([0.2, 0.2]), &blips());
    map.frame(&press_accept(), &blips());
    map.frame(&MapInput { next_filter: true, ..hover([0.2, 0.2]) }, &blips());
    assert!(!map.prompt);
    assert_eq!(map.attached, Some(1), "the race under the cursor is now the one shown");
}

#[test]
fn zoom_all_puts_the_camera_back() {
    let mut map = WorldMap::new();
    map.frame(&MapInput { zoom_lines: 3.0, ..Default::default() }, &blips());
    assert!(map.camera.zoom() > 1.0);
    map.frame(&MapInput { zoom_all: true, ..Default::default() }, &blips());
    assert_eq!(map.camera, Camera::zoom_all());
}

#[test]
fn the_distance_is_straight_line_metres() {
    let blip = PlacedBlip { style: CAR_LOT, world: [300.0, 400.0], map: [0.0, 0.0] };
    assert!((distance_metres([0.0, 0.0], &blip) - 500.0).abs() < 1e-3);
}
