//! Tests for the cluster as drawn: the golden node counts per state, the colours of the rev strip and the speed scale,
//! the hidden bars, the pulse and the place in the content box. They build the cluster through the theme's
//! `Speedometer` slot from a feed set by hand, with no game data and no GPU.
//!
//! Draw order (`mod.rs`): the plate, the gear box (cue fill, edge, digit, cue chevron), the digits and the unit, the
//! speed scale's 24 segments, the rev strip's 20, the bar row (each bar a label and its segments), the advanced row.

use std::sync::Arc;

use blackbox_ui::{BuildCx, Component, DrawList, LayoutCx, Rect, Skin, Theme, UiNode};

use super::super::plate::{Plate, PlateStyle};
use super::super::tokens::{Colours, CustomLook, skin};
use super::super::{CustomFeed, CustomFeedCell, CustomTheme, RadioFeedCell};
use super::frame::{GAP, REV_ROW, Rows, TALL, WIDTH};
use crate::hud::{HudState, HudTelemetry, TireCondition, VehicleCondition};
use crate::settings::hud::{HudPalette, HudSettings};
use crate::settings::{SpeedUnits, Transmission};
use crate::ui::present::Screen;
use crate::ui::theme::ElementId;

const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };
const GOLD: [u8; 4] = [255, 200, 80, 255];
const C: Colours = Colours::BRIEF;
/// The fixed parts: the gear box's edge and digit, the digits and the unit, the scale and the rev strip.
const GEAR: usize = 5;
const READOUT: usize = 2;
const SCALE: usize = 24;
const REVS: usize = 20;
/// A bar: its label and its segments.
const NITROUS: usize = 1 + 10;
const SPEEDBREAKER: usize = 1 + 10;
const BOOST: usize = 1 + 5;

fn token_skin(high_contrast: bool) -> Skin {
    skin(CustomLook { palette: HudPalette::Default, high_contrast }, GOLD)
}

/// Full opacity, so the colours are the tokens' own.
fn opaque() -> HudSettings {
    HudSettings { opacity: 1.0, ..HudSettings::default() }
}

fn feed(hud: HudState, settings: HudSettings, elapsed: f32) -> CustomFeed {
    CustomFeed {
        hud: Arc::new(hud),
        settings,
        screen: SCREEN,
        elapsed_s: elapsed,
        delta_s: 1.0 / 60.0,
        ..CustomFeed::default()
    }
}

/// The nodes the cluster builds for `feed` through `cell` (whose memory carries over between calls).
fn built_in(cell: &CustomFeedCell, feed: CustomFeed) -> Vec<UiNode> {
    let skin = token_skin(feed.settings.high_contrast);
    cell.set(feed);
    let theme = CustomTheme::new(RadioFeedCell::default(), cell.clone(), Default::default());
    let mut cluster = theme.element(ElementId::Speedometer.slot(), &BuildCx::new(&skin)).expect("the cluster");
    cluster.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    cluster.build(&mut out);
    out.iter().cloned().collect()
}

/// The nodes of one frame of `hud` from a fresh cell.
fn built(hud: HudState, settings: HudSettings) -> Vec<UiNode> {
    built_in(&CustomFeedCell::new(), feed(hud, settings, 0.0))
}

/// How many nodes the speed row's plate draws at scale 1.
fn plate_nodes(high_contrast: bool) -> usize {
    let skin = token_skin(high_contrast);
    let plate = Plate::new(Rect::new(0.0, 0.0, WIDTH, 48.0), PlateStyle::SURFACE, &LayoutCx::new(&skin, 0.0));
    let mut out = DrawList::new();
    plate.build(&mut out);
    out.len()
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

fn cruising() -> HudState {
    HudState {
        speed: 30.0,
        rpm: 4000.0,
        gear: 3,
        has_nos: true,
        nos: 0.6,
        has_turbo: true,
        boost_psi: 9.0,
        speedbreaker: 0.4,
        ..HudState::default()
    }
}

#[test]
fn golden_node_counts_per_state() {
    let plate = plate_nodes(false);
    assert_eq!(plate, 13, "a rounded plate: six corner strips top and bottom and the middle");
    let fixed = plate + GEAR + READOUT + SCALE + REVS;
    // Idle: no nitrous, no turbo, the speedbreaker only.
    assert_eq!(built(HudState::default(), opaque()).len(), fixed + SPEEDBREAKER, "idle");
    let cruise = fixed + NITROUS + SPEEDBREAKER + BOOST;
    assert_eq!(built(cruising(), opaque()).len(), cruise, "cruising with nitrous and a turbo");
    // The shift cue adds the box's fill and the chevron's two strokes.
    let cue = HudState { shift_light: true, ..cruising() };
    assert_eq!(built(cue, opaque()).len(), cruise + 1 + 2, "shift cue");
    // The burn changes colours and sizes, not the parts.
    let telemetry = HudTelemetry { nos_burning: true, ..HudTelemetry::default() };
    assert_eq!(built(HudState { telemetry, ..cruising() }, opaque()).len(), cruise, "nitrous burning");
    // Advanced: rpm, psi, load and lateral g, and the tyres (sound 1, punctured 2, blown 4, sound 1).
    let tires = [TireCondition::Sound, TireCondition::Punctured, TireCondition::Blown, TireCondition::Sound];
    let condition = Some(VehicleCondition { tires, ..VehicleCondition::default() });
    let advanced = HudSettings { advanced_telemetry: true, ..opaque() };
    assert_eq!(built(HudState { condition, ..cruising() }, advanced).len(), cruise + 4 + 8, "advanced");
    assert_eq!(
        built(HudState::default(), advanced).len(),
        fixed + SPEEDBREAKER + 3,
        "advanced, missing fields left out"
    );
    // High contrast outlines the plate.
    let contrast = HudSettings { high_contrast: true, ..opaque() };
    assert_eq!(built(HudState::default(), contrast).len(), plate_nodes(true) + fixed - plate + SPEEDBREAKER);
    assert_eq!(plate_nodes(true), plate + 4);
}

#[test]
fn the_cluster_shows_the_digits_the_unit_and_the_gear() {
    let metric = HudState { speed: 83.2, use_mph: false, gear: 4, ..HudState::default() };
    let nodes = built(metric.clone(), opaque());
    assert_eq!(&texts(&nodes)[..3], ["4", "299", "KM/H"], "the gear, then the digits and the unit");
    let imperial = HudState { use_mph: true, ..metric };
    let cell = CustomFeedCell::new();
    let nodes = built_in(&cell, CustomFeed { units: SpeedUnits::Imperial, ..feed(imperial, opaque(), 0.0) });
    assert_eq!(&texts(&nodes)[..3], ["4", "186", "MPH"]);
    let reverse = built(HudState { gear: -1, ..HudState::default() }, opaque());
    assert_eq!(texts(&reverse)[0], "R");
}

#[test]
fn nitrous_and_boost_hide_without_the_part() {
    let without = texts(&built(HudState::default(), opaque())).join(" ");
    assert!(!without.contains("N2O") && !without.contains("BOOST"), "{without}");
    assert!(without.contains("SB"), "the speedbreaker is always there: {without}");
    let with = texts(&built(cruising(), opaque())).join(" ");
    assert!(with.contains("N2O") && with.contains("BOOST"), "{with}");
}

#[test]
fn the_rev_strip_lights_in_its_bands_and_the_hot_bands_are_taller() {
    let hud = HudState {
        rpm: 7600.0,
        max_rpm: 8000.0,
        red_line: 7000.0,
        telemetry: HudTelemetry { shift_up_rpm: Some(6000.0), ..HudTelemetry::default() },
        ..HudState::default()
    };
    let nodes = built(hud, opaque());
    let start = plate_nodes(false) + GEAR + READOUT + SCALE;
    let strip = &nodes[start..start + REVS];
    // 7600 of 8000 lights 19 segments of 400 rpm: the shift point ends segment 14, the red line falls in segment 17.
    for (index, node) in strip.iter().enumerate() {
        let expected = match index {
            0..=14 => C.text_secondary,
            15 | 16 => C.caution,
            17 | 18 => C.danger,
            _ => C.rule,
        };
        assert_eq!(node.colour, expected, "segment {index}");
        let tall = match index {
            0..=14 => 1.0,
            _ => TALL,
        };
        assert!((node.world.y_axis.y - REV_ROW * tall).abs() < 1e-4, "segment {index} height {}", node.world.y_axis.y);
    }
}

#[test]
fn the_speed_scale_lights_the_presented_speed() {
    // 50 m/s is 180 km/h: half of the 360 km/h scale, 12 of 24 segments. The first frame starts on the speed.
    let nodes = built(HudState { speed: 50.0, ..HudState::default() }, opaque());
    let start = plate_nodes(false) + GEAR + READOUT;
    let scale = &nodes[start..start + SCALE];
    assert!(scale[..12].iter().all(|node| node.colour == C.text), "lit");
    assert!(scale[12..].iter().all(|node| node.colour == C.rule), "empty");
}

#[test]
fn the_scale_glides_to_a_new_speed_while_the_digits_jump() {
    let cell = CustomFeedCell::new();
    built_in(&cell, feed(HudState { speed: 0.0, ..HudState::default() }, opaque(), 0.0));
    let nodes = built_in(&cell, feed(HudState { speed: 50.0, ..HudState::default() }, opaque(), 1.0 / 60.0));
    assert_eq!(texts(&nodes)[1], "180", "the digits are the raw speed");
    let start = plate_nodes(false) + GEAR + READOUT;
    let lit = nodes[start..start + SCALE].iter().filter(|node| node.colour == C.text).count();
    assert!(lit < 12, "one frame in, the scale is still on its way: {lit} lit");
}

#[test]
fn the_shift_cue_fills_the_box_and_darkens_the_digit_and_shifting_dims_it() {
    let cue = built(HudState { shift_light: true, gear: 2, ..HudState::default() }, opaque());
    let start = plate_nodes(false);
    assert_eq!(cue[start].colour, C.caution, "the box fills with caution");
    let digit = cue.iter().find(|node| node.text.as_deref() == Some("2")).expect("the gear");
    assert_eq!(digit.colour, C.critical_text, "a dark digit on the fill");
    let manual = HudState {
        rpm: 6500.0,
        telemetry: HudTelemetry { shift_up_rpm: Some(6000.0), ..HudTelemetry::default() },
        ..HudState::default()
    };
    let cell = CustomFeedCell::new();
    let nodes =
        built_in(&cell, CustomFeed { transmission: Transmission::Manual, ..feed(manual.clone(), opaque(), 0.0) });
    assert_eq!(nodes[start].colour, C.caution, "manual past the shift point cues too");
    let automatic = built(manual, opaque());
    assert_ne!(automatic[start].colour, C.caution, "the automatic box shifts by itself");
    let shifting = built(HudState { shifting: true, gear: 2, ..HudState::default() }, opaque());
    let digit = shifting.iter().find(|node| node.text.as_deref() == Some("2")).expect("the gear");
    assert_eq!(digit.colour[3], (255.0_f32 * super::revs::SHIFTING_ALPHA).round() as u8, "dimmed while shifting");
}

#[test]
fn the_nitrous_pulse_brightens_once_and_motion_reduction_keeps_it_still() {
    let burning =
        HudState { telemetry: HudTelemetry { nos_burning: true, ..HudTelemetry::default() }, nos: 1.0, ..cruising() };
    let label = |nodes: &[UiNode]| nodes.iter().find(|node| node.text.as_deref() == Some("N2O")).expect("N2O").clone();
    let cell = CustomFeedCell::new();
    built_in(&cell, feed(cruising(), opaque(), 0.0));
    let start = built_in(&cell, feed(burning.clone(), opaque(), 0.1));
    assert_eq!(label(&start).colour, C.text, "the label brightens on the edge");
    let later = built_in(&cell, feed(burning.clone(), opaque(), 1.0));
    assert_eq!(label(&later).colour, C.text_secondary, "and settles while the burn goes on");
    // With motion the segments grow at the start of the pulse; without it they keep their size.
    let segment_height = |nodes: &[UiNode]| {
        let at = nodes.iter().position(|node| node.text.as_deref() == Some("N2O")).expect("N2O");
        nodes[at + 1].world.y_axis.y
    };
    assert!(segment_height(&start) > segment_height(&later), "a moving pulse grows the bar");
    let reduced = HudSettings { motion_reduction: true, ..opaque() };
    let still = CustomFeedCell::new();
    built_in(&still, feed(cruising(), reduced, 0.0));
    let highlighted = built_in(&still, feed(burning, reduced, 0.1));
    assert_eq!(label(&highlighted).colour, C.text, "a static highlight");
    assert!((segment_height(&highlighted) - segment_height(&later)).abs() < 1e-4, "no growth");
}

#[test]
fn the_cluster_sits_in_the_bottom_right_of_the_content_box() {
    // 1080p is 16:9: the box is the window, 853.3 units wide; the safe inset is 3.5% of 480.
    let nodes = built(HudState::default(), opaque());
    let right = nodes.iter().map(|node| node.world.w_axis.x + node.world.x_axis.x / 2.0).fold(f32::MIN, f32::max);
    let bottom = nodes.iter().map(|node| node.world.w_axis.y + node.world.y_axis.y / 2.0).fold(f32::MIN, f32::max);
    let inset = 0.035 * 480.0;
    let edge = 480.0 * 16.0 / 9.0 / 2.0 - inset;
    assert!((right - edge).abs() < 1e-2, "the right edge on the safe area: {right} against {edge}");
    assert!(bottom <= 240.0 - inset + 1e-2, "nothing below the safe area: {bottom}");
    let rows = Rows::for_text_size(1.0);
    assert!(rows.revs.bottom > rows.speed.top() + GAP - 1e-4, "the rows stack upwards without overlapping");
}
