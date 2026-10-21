//! The split-screen HUD without the install: a stand-in package with the real names (`test_support.rs`).

use blackbox_feng::{PackageId, Runtime, UiNode, UiTree};
use glam::Vec3;

use super::place::{Placement, fit, half_rect};
use super::seat::SeatHud;
use super::test_support::{SCRIPT_INIT, guid, load};
use super::{Kind, Seat};
use crate::hud::race::{Count, Message, RaceHud};
use crate::hud::state::HudState;
use crate::settings::HudLayout;
use crate::ui::present::Screen;

fn screen(width: f32, height: f32) -> Screen {
    Screen { width, height, pixels_per_point: 1.0 }
}

fn node(tree: &UiTree, guid: u32) -> &UiNode {
    tree.nodes.iter().find(|n| n.guid == guid).expect("the object is in the package")
}

fn visible(rt: &Runtime, id: PackageId, guid: u32) -> bool {
    node(&rt.tree(id), guid).visible
}

fn text(rt: &Runtime, id: PackageId, guid: u32) -> String {
    node(&rt.tree(id), guid).text.clone().unwrap_or_default()
}

fn x_of(tree: &UiTree, guid: u32) -> f32 {
    node(tree, guid).world.transform_point3(Vec3::ZERO).x
}

fn race() -> RaceHud {
    RaceHud { place: 2, racers: 2, lap: Some((1, 3)), clock: 65.5, progress: 0.4, count: None }
}

/// The seat of the first player over the stand-in package.
fn seat(kind: Kind) -> (Runtime, PackageId, SeatHud) {
    let (rt, id) = load("HUD_Player1.fng");
    let seat = SeatHud::new(&rt, id, kind, Seat::First);
    (rt, id, seat)
}

#[test]
fn each_player_and_kind_has_its_package_and_layout_number() {
    let names = [
        (Kind::Race, Seat::First, "HUD_Player1.fng", 3),
        (Kind::Race, Seat::Second, "HUD_Player2.fng", 4),
        (Kind::Drag, Seat::First, "HUD_Drag_Player1.fng", 5),
        (Kind::Drag, Seat::Second, "HUD_Drag_Player2.fng", 6),
    ];
    for (kind, seat, package, number) in names {
        assert_eq!(kind.package(seat), package);
        assert_eq!(kind.layout_number(seat), number);
    }
    assert_eq!(Kind::of(true), Kind::Drag);
    assert_eq!(Kind::of(false), Kind::Race);
    assert_eq!(Kind::Race.texture_pack(), "GLOBAL/HUDTEXSPLIT.BIN");
    assert_eq!(Kind::Drag.texture_pack(), "GLOBAL/HUDTEXDRAGSPLIT.BIN");
    assert_eq!((Seat::First.index(), Seat::Second.index()), (0, 1));
}

#[test]
fn the_speed_shows_whole_units_without_a_leading_zero() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    seat.apply(&mut rt, &HudState { speed: 187.5 / 3.6, ..Default::default() }, false);
    assert_eq!(text(&rt, id, guid::DIGIT_1), "7");
    assert_eq!(text(&rt, id, guid::DIGIT_2), "8");
    assert_eq!(text(&rt, id, guid::DIGIT_3), "1");
    seat.apply(&mut rt, &HudState { speed: 7.5 / 3.6, ..Default::default() }, false);
    assert!(visible(&rt, id, guid::DIGIT_1));
    assert!(!visible(&rt, id, guid::DIGIT_2) && !visible(&rt, id, guid::DIGIT_3));
}

#[test]
fn the_rpm_bar_grows_from_its_left_edge_and_turns_red_at_the_red_line() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    let bar = rt.find_guid(id, guid::BAR).unwrap();
    let base = HudState { max_rpm: 8000.0, red_line: 7000.0, ..Default::default() };
    for (rpm, width) in [(0.0, 0.0), (4000.0, 52.0), (8000.0, 104.0), (12000.0, 104.0)] {
        seat.apply(&mut rt, &HudState { rpm, ..base.clone() }, false);
        let size = rt.size(bar).unwrap();
        assert!((size.x - width).abs() < 1e-3, "{rpm} rpm: {}", size.x);
        let at = rt.position(bar).unwrap();
        assert!((at.x - size.x / 2.0 + 52.0).abs() < 1e-3, "the left edge stays at -52: {}", at.x);
    }
    seat.apply(&mut rt, &HudState { rpm: 6900.0, ..base.clone() }, false);
    assert_eq!(rt.script_of(bar), Some(SCRIPT_INIT));
    seat.apply(&mut rt, &HudState { rpm: 7000.0, ..base }, false);
    assert_eq!(rt.script_of(bar), Some(super::names::SCRIPT_REDLINE));
}

#[test]
fn the_gear_digit_names_reverse_neutral_and_the_gear() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    for (gear, letter) in [(-1, "R"), (0, "N"), (1, "1"), (6, "6")] {
        seat.apply(&mut rt, &HudState { gear, ..Default::default() }, false);
        assert_eq!(text(&rt, id, guid::GEAR), letter);
    }
}

#[test]
fn the_drag_gear_fades_while_shifting_and_the_race_gear_keeps_its_colour() {
    let (mut rt, id, mut drag) = seat(Kind::Drag);
    drag.apply(&mut rt, &HudState { gear: 2, shifting: true, ..Default::default() }, false);
    assert_eq!(node(&rt.tree(id), guid::GEAR).colour, [255, 255, 255, 0x88]);
    drag.apply(&mut rt, &HudState { gear: 2, ..Default::default() }, false);
    assert_eq!(node(&rt.tree(id), guid::GEAR).colour, [255, 255, 255, 255]);
    let (mut rt, id, mut plain) = seat(Kind::Race);
    plain.apply(&mut rt, &HudState { gear: 2, shifting: true, ..Default::default() }, false);
    assert_eq!(node(&rt.tree(id), guid::GEAR).colour, [255, 255, 255, 255], "as authored");
}

#[test]
fn the_shift_light_plays_its_script_while_the_gearbox_wants_the_next_gear() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    let light = rt.find_guid(id, guid::SHIFT_LIGHT).unwrap();
    seat.apply(&mut rt, &HudState { shift_light: true, ..Default::default() }, false);
    assert_eq!(rt.script_of(light), Some(0x02DD_C8F0));
    seat.apply(&mut rt, &HudState::default(), false);
    assert_eq!(rt.script_of(light), Some(SCRIPT_INIT));
}

#[test]
fn the_nitrous_gauge_shows_with_a_tank_and_its_bar_empties_from_180_degrees() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    seat.apply(&mut rt, &HudState::default(), false);
    assert!(!visible(&rt, id, guid::NITROUS) && !visible(&rt, id, guid::NITROUS_BAR));
    seat.apply(&mut rt, &HudState { has_nos: true, nos: 0.25, ..Default::default() }, false);
    assert!(visible(&rt, id, guid::NITROUS) && visible(&rt, id, guid::NITROUS_BAR));
    let tree = rt.tree(id);
    let blackbox_feng::NodeKind::Image { mask_rotation, .. } = node(&tree, guid::NITROUS_BAR).kind else {
        panic!("the bar is an image")
    };
    assert!((mask_rotation[2] - 135.0).abs() < 1e-3, "{}", mask_rotation[2]);
}

#[test]
fn the_race_readout_shows_with_a_race_and_hides_without() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    seat.apply(&mut rt, &HudState::default(), false);
    for g in [guid::INFO, guid::CLOCK, guid::PLACE, guid::CLOCK_ICON] {
        assert!(!visible(&rt, id, g), "{g} without a race");
    }
    seat.apply(&mut rt, &HudState { race: Some(race()), ..Default::default() }, false);
    for g in [guid::INFO, guid::CLOCK, guid::PLACE, guid::CLOCK_ICON] {
        assert!(visible(&rt, id, g), "{g} in a race");
    }
    assert_eq!(text(&rt, id, guid::PLACE), "2");
    assert!(text(&rt, id, guid::CLOCK).contains(':'), "{}", text(&rt, id, guid::CLOCK));
}

#[test]
fn a_drag_package_keeps_the_gear_when_there_is_no_race() {
    let (mut rt, id, mut seat) = seat(Kind::Drag);
    seat.apply(&mut rt, &HudState::default(), false);
    assert!(visible(&rt, id, guid::INFO), "the race information holds the gear in a drag package");
    assert!(!visible(&rt, id, guid::CLOCK));
}

#[test]
fn the_countdown_and_the_message_line_show_only_while_they_run() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    seat.apply(&mut rt, &HudState { race: Some(race()), ..Default::default() }, false);
    assert!(!visible(&rt, id, guid::COUNTDOWN) && !visible(&rt, id, guid::MESSAGE));
    let counting = RaceHud { count: Some(Count::Three), ..race() };
    seat.apply(&mut rt, &HudState { race: Some(counting), ..Default::default() }, false);
    assert!(visible(&rt, id, guid::COUNTDOWN));
    let line = Message { serial: 1, text: "CHECKPOINT".into() };
    seat.apply(&mut rt, &HudState { race: Some(race()), message: Some(line), ..Default::default() }, false);
    assert!(visible(&rt, id, guid::MESSAGE));
    assert!(!visible(&rt, id, guid::COUNTDOWN));
}

#[test]
fn what_nothing_feeds_stays_hidden() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    seat.apply(&mut rt, &HudState { race: Some(race()), has_nos: true, ..Default::default() }, true);
    assert!(!visible(&rt, id, guid::WRONG_WAY));
    assert!(visible(&rt, id, guid::CLUSTER) && visible(&rt, id, guid::DIGIT_1));
}

#[test]
fn the_minimap_layer_shows_with_the_map_flag() {
    let (mut rt, id, mut seat) = seat(Kind::Race);
    seat.apply(&mut rt, &HudState::default(), false);
    assert!(!visible(&rt, id, guid::LEFT) && !visible(&rt, id, guid::MAP_BACKING));
    seat.apply(&mut rt, &HudState::default(), true);
    assert!(visible(&rt, id, guid::LEFT) && visible(&rt, id, guid::MAP_BACKING));
}

#[test]
fn a_player_has_half_the_window_the_first_above_the_second() {
    let s = screen(1920.0, 1080.0);
    assert_eq!(half_rect(Seat::First, s), [0.0, 0.0, 1920.0, 540.0]);
    assert_eq!(half_rect(Seat::Second, s), [0.0, 540.0, 1920.0, 540.0]);
}

#[test]
fn the_layout_fits_windows_narrower_than_4_by_3() {
    assert_eq!(fit(screen(1440.0, 1080.0)), 1.0);
    assert_eq!(fit(screen(2560.0, 1080.0)), 1.0);
    assert!((fit(screen(1080.0, 1080.0)) - 0.75).abs() < 1e-6);
    assert!((fit(screen(540.0, 1080.0)) - 0.375).abs() < 1e-6);
    assert_eq!(fit(screen(0.0, 0.0)), 1.0);
}

#[test]
fn the_gauges_and_the_map_move_out_in_a_wide_window() {
    let (rt, id) = load("HUD_Player1.fng");
    let placement = Placement::new(&rt, id, &[]);
    let before = rt.tree(id);
    let mut wide = before.clone();
    placement.apply(&mut wide, screen(1920.0, 1080.0), HudLayout::Pc);
    assert!((x_of(&wide, guid::CLUSTER) - x_of(&before, guid::CLUSTER) - 120.0).abs() < 1e-3, "the gauges: right");
    assert!((x_of(&wide, guid::MAP_BACKING) - x_of(&before, guid::MAP_BACKING) + 120.0).abs() < 1e-3, "the map: left");
    let mut square = before.clone();
    placement.apply(&mut square, screen(1440.0, 1080.0), HudLayout::Pc);
    assert!((x_of(&square, guid::CLUSTER) - x_of(&before, guid::CLUSTER)).abs() < 1e-3, "4:3 is as authored");
}

#[test]
fn a_narrow_window_scales_the_whole_layout_down() {
    let (rt, id) = load("HUD_Player1.fng");
    let placement = Placement::new(&rt, id, &[]);
    let before = rt.tree(id);
    let mut narrow = before.clone();
    placement.apply(&mut narrow, screen(1080.0, 1080.0), HudLayout::Pc);
    let ratio = x_of(&narrow, guid::CLUSTER) / x_of(&before, guid::CLUSTER);
    assert!((ratio - 0.75).abs() < 1e-4, "{ratio}");
}

#[test]
fn the_rule_between_the_halves_spans_a_window_wider_than_it() {
    let (rt, id) = load("HUD_Player1.fng");
    let placement = Placement::new(&rt, id, &[]);
    let width = |s: Screen| {
        let mut tree = rt.tree(id);
        placement.apply(&mut tree, s, HudLayout::Pc);
        node(&tree, guid::DIVIDER).world.x_axis.length()
    };
    assert!((width(screen(1440.0, 1080.0)) - 1200.0).abs() < 1e-2, "as authored at 4:3");
    let ultrawide = screen(5120.0, 1440.0);
    assert!((width(ultrawide) - ultrawide.width / ultrawide.scale()).abs() < 1e-1, "the window's width in units");
}

#[test]
fn the_drag_shift_light_shows_only_while_a_shift_is_due() {
    let (mut rt, id, mut seat) = seat(Kind::Drag);
    seat.apply(&mut rt, &HudState::default(), false);
    assert!(!visible(&rt, id, guid::SHIFT_LIGHT));
    seat.apply(&mut rt, &HudState { shift_light: true, ..Default::default() }, false);
    assert!(visible(&rt, id, guid::SHIFT_LIGHT));
    seat.apply(&mut rt, &HudState::default(), false);
    assert!(!visible(&rt, id, guid::SHIFT_LIGHT));
}
