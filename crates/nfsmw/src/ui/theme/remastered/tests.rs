//! Tests for the Remastered cluster: what each part builds from the drive feed. They run on a `HudState` set by hand, with
//! no game data and no GPU.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, Rect, Theme, UiNode};

use super::RemasteredTheme;
use super::skin::{keys, skin};
use crate::hud::HudState;
use crate::ui::present::Screen;
use crate::ui::theme::ElementId;
use crate::ui::theme::custom::DriveFeedCell;

const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

/// The nodes the element in `element`'s slot builds from `state`, or `None` when the theme does not provide the slot.
fn built(element: ElementId, state: &HudState) -> Option<Vec<UiNode>> {
    let feed = DriveFeedCell::new();
    feed.set(state, SCREEN);
    let skin = skin();
    let mut component = RemasteredTheme::new(feed).element(element.slot(), &BuildCx::new(&skin))?;
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    Some(out.iter().cloned().collect())
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

#[test]
fn the_theme_fills_the_cluster_and_hides_the_speed_breaker_and_falls_back_for_the_rest() {
    let theme = RemasteredTheme::default();
    let skin = skin();
    let cx = BuildCx::new(&skin);
    for element in [ElementId::Tachometer, ElementId::Speedometer, ElementId::Nitrous, ElementId::Speedbreaker] {
        assert!(theme.element(element.slot(), &cx).is_some(), "{} is Remastered", element.name());
    }
    assert!(theme.element(ElementId::RadioCard.slot(), &cx).is_none(), "the rest falls back to Original");
}

#[test]
fn the_speedometer_shows_the_speed_the_unit_with_its_gold_h_and_the_gear() {
    // 83.2 m/s is about 186 mph: `display_speed` cuts to whole units.
    let state = HudState { speed: 83.2, use_mph: true, gear: 4, ..Default::default() };
    let nodes = built(ElementId::Speedometer, &state).expect("the speedometer is Remastered");
    assert_eq!(texts(&nodes), ["186", "4", "MP", "H"]);
    let h = nodes.iter().find(|node| node.text.as_deref() == Some("H")).expect("the H");
    assert_eq!(h.colour, skin().colour(keys::UNIT_HIGHLIGHT).expect("the highlight colour"));
}

#[test]
fn the_speedometer_shows_kilometres_per_hour_without_the_highlight() {
    let state = HudState { speed: 20.0, use_mph: false, gear: 3, ..Default::default() };
    let nodes = built(ElementId::Speedometer, &state).expect("the speedometer is Remastered");
    assert_eq!(texts(&nodes), ["72", "3", "KM/H"]);
}

#[test]
fn the_gear_shows_reverse_and_neutral_as_letters() {
    let reverse = HudState { gear: -1, ..Default::default() };
    let neutral = HudState { gear: 0, ..Default::default() };
    let reverse_nodes = built(ElementId::Speedometer, &reverse).expect("the speedometer is Remastered");
    let neutral_nodes = built(ElementId::Speedometer, &neutral).expect("the speedometer is Remastered");
    assert!(texts(&reverse_nodes).contains(&"R"));
    assert!(texts(&neutral_nodes).contains(&"N"));
}

#[test]
fn the_tachometer_numbers_each_thousand_rpm_up_to_the_scale() {
    // 4500 rpm under a 8250 maximum: the face is the 9000 one, so the numbers run 1 to 9.
    let state = HudState { rpm: 4500.0, max_rpm: 8250.0, ..Default::default() };
    let nodes = built(ElementId::Tachometer, &state).expect("the tachometer is Remastered");
    let numbers: Vec<&str> = texts(&nodes);
    assert_eq!(numbers, ["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
}

#[test]
fn the_arc_lights_up_to_the_revs_and_turns_gold_past_the_red_line() {
    let skin = skin();
    // Revs at the top of the scale with the red line half way: the arc has lit and gold parts, and no unlit white.
    let high = HudState { rpm: 8900.0, max_rpm: 8250.0, red_line: 4500.0, ..Default::default() };
    let colours: Vec<[u8; 4]> = built(ElementId::Tachometer, &high)
        .expect("the tachometer is Remastered")
        .iter()
        .filter(|node| node.text.is_none())
        .map(|node| node.colour)
        .collect();
    assert!(colours.contains(&skin.colour(keys::ARC_REDLINE).expect("gold")));
    assert!(!colours.contains(&skin.colour(keys::ARC_DIM).expect("dim")));
}

#[test]
fn the_nitrous_bar_shows_only_on_a_car_with_nitrous() {
    let without = HudState { has_nos: false, ..Default::default() };
    let with = HudState { has_nos: true, nos: 0.5, ..Default::default() };
    assert!(built(ElementId::Nitrous, &without).expect("the bar is Remastered").is_empty());
    let nodes = built(ElementId::Nitrous, &with).expect("the bar is Remastered");
    assert_eq!(texts(&nodes), ["N2O"]);
    assert!(nodes.len() > 2, "the track, the glow, the lit segments and the label");
}

#[test]
fn the_speed_breaker_shows_its_bar_and_the_gear_indicator_draws_nothing() {
    let state = HudState { speedbreaker: 0.5, ..Default::default() };
    let nodes = built(ElementId::Speedbreaker, &state).expect("the bar is Remastered");
    assert_eq!(texts(&nodes), ["BRAKE"]);
    assert!(built(ElementId::GearIndicator, &state).expect("hidden, not left to Original").is_empty());
}

#[test]
fn the_boost_bar_shows_only_on_a_car_with_forced_induction() {
    let without = HudState { has_turbo: false, boost_psi: 10.0, ..Default::default() };
    let with = HudState { has_turbo: true, boost_psi: 10.0, ..Default::default() };
    assert!(built(ElementId::Turbo, &without).expect("the bar is Remastered").is_empty());
    assert_eq!(texts(&built(ElementId::Turbo, &with).expect("the bar is Remastered")), ["BOOST"]);
}
