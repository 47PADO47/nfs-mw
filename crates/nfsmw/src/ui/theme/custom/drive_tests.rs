//! Tests for the Custom drive cluster: what the tachometer, the speedometer, the gear indicator and the three meters build
//! from the drive feed. They run on a `HudState` set by hand, with no game data and no GPU.
//!
//! A rounded shape and a `LinearProgress` bar are several quads, so the tests count the plain segments and check where the
//! shapes sit, not how many quads each shape uses.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, Rect, Theme, UiNode};

use super::drive_feed::DriveFeedCell;
use super::drive_layout::{self, Corner, SEGMENTS};
use super::skin::drive;
use super::{CustomTheme, card_skin, palette};
use crate::hud::HudState;
use crate::ui::present::Screen;
use crate::ui::theme::ElementId;

// The palette's secondary (the accent) and border, as the Bevy palette sets them; the status colours are fixed.
const ACCENT: [u8; 4] = [120, 220, 255, 255];
const REV_LIT: [u8; 4] = ACCENT;
const REV_WARN: [u8; 4] = [255, 190, 60, 255];
const REV_REDLINE: [u8; 4] = [255, 80, 90, 255];
const TRACK: [u8; 4] = [255, 255, 255, 40];
const PIP_ON: [u8; 4] = ACCENT;
const PIP_OFF: [u8; 4] = [255, 255, 255, 40];
const NOS: [u8; 4] = ACCENT;
const BOOST: [u8; 4] = ACCENT;
const SPEEDBREAKER: [u8; 4] = ACCENT;
const SHIFT: [u8; 4] = [255, 80, 90, 255];

/// A 16:9 window at 1080p, one point per pixel.
const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

/// The nodes `element` builds from `state`.
fn built(element: ElementId, state: &HudState) -> Vec<UiNode> {
    let feed = DriveFeedCell::new();
    feed.set(state, SCREEN);
    let skin = card_skin().merged(palette::bevy().skin());
    let mut component = CustomTheme::new(Default::default(), feed)
        .element(element.slot(), &BuildCx::new(&skin))
        .expect("the drive cluster is Custom components");
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    out.iter().cloned().collect()
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

fn count(nodes: &[UiNode], colour: [u8; 4]) -> usize {
    nodes.iter().filter(|node| node.colour == colour).count()
}

/// The horizontal centres of the shape nodes in `colour`. Text is left out: a glyph can share a shape's colour.
fn centres_x(nodes: &[UiNode], colour: [u8; 4]) -> Vec<f32> {
    nodes.iter().filter(|node| node.text.is_none() && node.colour == colour).map(|node| node.world.w_axis.x).collect()
}

/// Whether every node in `colour` has its centre inside `rect`'s horizontal span, and there is at least one.
fn all_within(nodes: &[UiNode], colour: [u8; 4], rect: Rect) -> bool {
    let xs = centres_x(nodes, colour);
    !xs.is_empty() && xs.iter().all(|x| *x >= rect.min_x() - 1e-3 && *x <= rect.max_x() + 1e-3)
}

/// Whether a shape node's colour is `target` to within a channel's tolerance. A `LinearProgress` fill is shaded a little
/// lighter and darker than its colour, so the meter tests match it by nearness, not equality.
fn near(nodes: &[UiNode], target: [u8; 4]) -> Vec<&UiNode> {
    nodes
        .iter()
        .filter(|node| {
            node.text.is_none() && (0..3).all(|c| (i32::from(node.colour[c]) - i32::from(target[c])).abs() <= 40)
        })
        .collect()
}

/// A state with the red line at 6500 rpm on a 7000 scale (about 0.93 of the rev gauge).
fn revving(rpm: f32) -> HudState {
    HudState { rpm, max_rpm: 6000.0, red_line: 6500.0, ..Default::default() }
}

fn corner() -> Corner {
    Corner::of(SCREEN)
}

#[test]
fn the_speedometer_shows_the_speed_and_the_unit() {
    // 20 m/s is 72 km/h: `display_speed` cuts to whole units.
    let state = HudState { speed: 20.0, use_mph: false, ..Default::default() };
    assert_eq!(texts(&built(ElementId::Speedometer, &state)), ["72", "KM/H"]);
}

#[test]
fn the_speedometer_switches_to_miles_per_hour() {
    let state = HudState { speed: 20.0, use_mph: true, ..Default::default() };
    // 20 m/s is 44.7 mph, cut to 44.
    assert_eq!(texts(&built(ElementId::Speedometer, &state)), ["44", "MPH"]);
}

#[test]
fn the_tachometer_draws_the_card_and_one_plain_segment_per_slice_of_the_scale() {
    let nodes = built(ElementId::Tachometer, &revving(0.0));
    assert!(count(&nodes, [0, 0, 0, 170]) > 1, "the card is drawn");
    assert_eq!(count(&nodes, TRACK), SEGMENTS, "at idle every segment is unlit");
}

#[test]
fn the_rev_gauge_lights_the_segments_below_the_rev_fraction_in_cyan() {
    // Half the scale: the first 16 of 32 segments are lit, all cyan, since none is past the warning zone.
    let nodes = built(ElementId::Tachometer, &revving(3500.0));
    assert_eq!(count(&nodes, REV_LIT), 16);
    assert_eq!(count(&nodes, TRACK), 16);
}

#[test]
fn the_rev_gauge_turns_amber_then_red_near_and_past_the_red_line() {
    // 0.97 of the scale, with the red line at 0.93: 27 cyan, 3 amber in the warning zone, 1 red past the red line, and
    // the last segment unlit.
    let nodes = built(ElementId::Tachometer, &revving(6790.0));
    assert_eq!(count(&nodes, REV_LIT), 27);
    assert_eq!(count(&nodes, REV_WARN), 3);
    assert_eq!(count(&nodes, REV_REDLINE), 1);
    assert_eq!(count(&nodes, TRACK), 1);
}

#[test]
fn the_gear_box_shows_the_gear_and_lights_its_pip() {
    let state = HudState { gear: 3, ..Default::default() };
    let nodes = built(ElementId::GearIndicator, &state);
    assert_eq!(texts(&nodes), ["3"]);
    // Pips: reverse, neutral, then first to sixth. Gear 3 is the fifth pip (index 4).
    let lit = drive_layout::pip(corner(), 4);
    assert!(all_within(&nodes, PIP_ON, lit), "the lit pip is not the fifth one");
    let xs = centres_x(&nodes, PIP_OFF);
    assert!(!xs.is_empty() && xs.iter().all(|x| *x < lit.min_x() || *x > lit.max_x()), "a dim pip is lit");
}

#[test]
fn reverse_and_neutral_have_their_own_letters_and_pips() {
    let reverse = built(ElementId::GearIndicator, &HudState { gear: -1, ..Default::default() });
    assert_eq!(texts(&reverse), ["R"]);
    assert!(all_within(&reverse, PIP_ON, drive_layout::pip(corner(), 0)), "reverse lights the first pip");
    let neutral = built(ElementId::GearIndicator, &HudState { gear: 0, ..Default::default() });
    assert_eq!(texts(&neutral), ["N"]);
    assert!(all_within(&neutral, PIP_ON, drive_layout::pip(corner(), 1)), "neutral lights the second pip");
}

#[test]
fn a_gear_past_the_last_pip_lights_the_last_pip() {
    let nodes = built(ElementId::GearIndicator, &HudState { gear: 9, ..Default::default() });
    assert!(all_within(&nodes, PIP_ON, drive_layout::pip(corner(), 7)), "the last pip is the lit one");
}

#[test]
fn the_gear_letter_turns_red_on_the_shift_light() {
    let nodes = built(ElementId::GearIndicator, &HudState { shift_light: true, ..Default::default() });
    assert!(count(&nodes, SHIFT) > 0, "the gear is not red on the shift light");
}

#[test]
fn the_nitrous_meter_shows_only_on_a_car_with_nitrous_and_fills_to_its_value() {
    assert!(built(ElementId::Nitrous, &HudState::default()).is_empty(), "no nitrous, no meter");
    let state = HudState { has_nos: true, nos: 0.5, ..Default::default() };
    let nodes = built(ElementId::Nitrous, &state);
    assert_eq!(texts(&nodes), ["NOS"]);
    let bar = drive_layout::meter_bar(corner(), 0);
    let half = Rect::new(bar.min_x() + bar.w / 4.0, bar.y, bar.w / 2.0, bar.h);
    let fill = near(&nodes, NOS);
    assert!(!fill.is_empty(), "the fill is drawn");
    assert!(fill.iter().all(|n| n.world.w_axis.x <= half.max_x() + 1e-3), "the fill does not stop at half of the bar");
}

#[test]
fn the_boost_meter_fills_to_the_boost_and_shows_only_on_a_turbo_car() {
    assert!(built(ElementId::Turbo, &HudState::default()).is_empty());
    // 15 psi is half of the 30 psi scale: the fill is about half of the meter bar.
    let state = HudState { has_turbo: true, boost_psi: 15.0, ..Default::default() };
    let nodes = built(ElementId::Turbo, &state);
    let bar = drive_layout::meter_bar(corner(), 1);
    let widest = near(&nodes, BOOST).iter().map(|n| n.world.x_axis.x).fold(0.0, f32::max);
    assert!((widest - bar.w / 2.0).abs() < 1.5, "fill width {widest} against a bar of {}", bar.w);
}

#[test]
fn the_speed_breaker_meter_shows_when_it_has_a_value_or_is_active() {
    assert!(built(ElementId::Speedbreaker, &HudState::default()).is_empty());
    let state = HudState { speedbreaker: 0.5, ..Default::default() };
    let nodes = built(ElementId::Speedbreaker, &state);
    assert_eq!(texts(&nodes), ["SB"]);
    assert!(!near(&nodes, SPEEDBREAKER).is_empty(), "the fill is not drawn");
}

#[test]
fn the_drive_keys_are_in_the_skin() {
    assert!(card_skin().colour(drive::REV_REDLINE).is_some());
    assert!(card_skin().colour(drive::GEAR_SHIFT).is_some());
    assert!(card_skin().font(drive::FONT).is_some());
}
