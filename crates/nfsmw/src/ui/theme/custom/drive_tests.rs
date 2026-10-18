//! Tests for the Custom drive cluster: what the tachometer, the speedometer, the gear indicator and the three meters
//! build from the drive feed. They run on a `HudState` set by hand, with no game data and no GPU.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, Rect, Theme, UiNode};

use super::drive_feed::DriveFeedCell;
use super::palette;
use super::skin::drive;
use super::{CustomTheme, card_skin};
use crate::hud::HudState;
use crate::ui::present::Screen;
use crate::ui::theme::ElementId;

/// The Bevy palette's primary: lit dots take the primary token now (it was a cooler white at alpha 220).
const REV_LIT: [u8; 4] = [255, 255, 255, 255];
const REV_WARN: [u8; 4] = [255, 200, 80, 255];
const REV_REDLINE: [u8; 4] = [255, 80, 90, 255];
const TRACK: [u8; 4] = [255, 255, 255, 40];
const NOS: [u8; 4] = [210, 185, 90, 255];
const BOOST: [u8; 4] = [255, 170, 80, 255];
const SPEEDBREAKER: [u8; 4] = [150, 255, 170, 255];
const CARD: [u8; 4] = [0, 0, 0, 170];
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

/// A state with the red line at 6500 rpm on a 7000 scale (about 0.93 of the rev gauge).
fn revving(rpm: f32) -> HudState {
    HudState { rpm, max_rpm: 6000.0, red_line: 6500.0, ..Default::default() }
}

#[test]
fn the_speedometer_shows_the_speed_and_the_unit() {
    let state = HudState { speed: 20.0, use_mph: false, ..Default::default() };
    assert_eq!(texts(&built(ElementId::Speedometer, &state)), ["72", "KM/H"]);
}

#[test]
fn the_speedometer_switches_to_miles_per_hour() {
    let state = HudState { speed: 20.0, use_mph: true, ..Default::default() };
    assert_eq!(texts(&built(ElementId::Speedometer, &state)), ["44", "MPH"]);
}

#[test]
fn the_tachometer_draws_the_card_and_arc_dots() {
    let nodes = built(ElementId::Tachometer, &revving(0.0));
    assert!(count(&nodes, CARD) > 1, "the card is drawn");
    assert!(count(&nodes, TRACK) > 0, "at idle every dot is unlit");
}

#[test]
fn the_tachometer_has_number_labels() {
    let state = revving(0.0);
    let nodes = built(ElementId::Tachometer, &state);
    let labels = texts(&nodes);
    assert!(labels.contains(&"1"), "should have label 1");
    assert!(labels.len() >= 4, "should have at least 4 labels");
}

#[test]
fn the_rev_arc_lights_dots_below_the_rev_fraction() {
    let nodes = built(ElementId::Tachometer, &revving(3500.0));
    assert!(count(&nodes, REV_LIT) > 0, "some dots should be lit");
    assert!(count(&nodes, TRACK) > 0, "some dots should be unlit");
}

#[test]
fn the_rev_arc_turns_amber_then_red_near_and_past_the_red_line() {
    let nodes = built(ElementId::Tachometer, &revving(6790.0));
    assert!(count(&nodes, REV_LIT) > 0, "lit cyan dots");
    assert!(count(&nodes, REV_WARN) > 0, "amber warning dots");
    assert!(count(&nodes, REV_REDLINE) > 0, "red redline dots");
    assert!(count(&nodes, TRACK) > 0, "unlit dots past RPM");
}

#[test]
fn the_gear_shows_the_gear_number() {
    let state = HudState { gear: 3, ..Default::default() };
    let nodes = built(ElementId::GearIndicator, &state);
    assert_eq!(texts(&nodes), ["3"]);
}

#[test]
fn reverse_and_neutral_have_their_own_letters() {
    let reverse = built(ElementId::GearIndicator, &HudState { gear: -1, ..Default::default() });
    assert_eq!(texts(&reverse), ["R"]);
    let neutral = built(ElementId::GearIndicator, &HudState { gear: 0, ..Default::default() });
    assert_eq!(texts(&neutral), ["N"]);
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
    assert_eq!(texts(&nodes), ["N2O"]);
    assert!(count(&nodes, NOS) > 0, "the fill is drawn");
}

#[test]
fn the_boost_meter_fills_to_the_boost_and_shows_only_on_a_turbo_car() {
    assert!(built(ElementId::Turbo, &HudState::default()).is_empty());
    let state = HudState { has_turbo: true, boost_psi: 15.0, ..Default::default() };
    let nodes = built(ElementId::Turbo, &state);
    assert!(count(&nodes, BOOST) > 0, "boost fill is drawn");
}

#[test]
fn the_speed_breaker_meter_shows_when_it_has_a_value_or_is_active() {
    assert!(built(ElementId::Speedbreaker, &HudState::default()).is_empty());
    let state = HudState { speedbreaker: 0.5, ..Default::default() };
    let nodes = built(ElementId::Speedbreaker, &state);
    assert_eq!(texts(&nodes), ["SB"]);
    assert!(count(&nodes, SPEEDBREAKER) > 0, "the fill is not drawn");
}

#[test]
fn the_drive_keys_are_in_the_skin() {
    let skin = card_skin().merged(palette::bevy().skin());
    assert!(skin.colour(drive::REV_REDLINE).is_some());
    assert!(skin.colour(drive::GEAR_SHIFT).is_some());
    assert!(skin.colour(drive::ARC_NUMBER).is_some());
    assert!(skin.font(drive::FONT).is_some());
}
