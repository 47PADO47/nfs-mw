//! Tests for the Remastered cluster: what each part builds from the drive feed. They run on a `HudState` set by hand, with
//! no game data and no GPU.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, Rect, Skin, Theme, UiNode};

use super::RemasteredTheme;
use super::skin::{keys, palette_skin, skin};
use crate::hud::HudState;
use crate::ui::present::Screen;
use crate::ui::theme::custom::DriveFeedCell;
use crate::ui::theme::custom::palette;
use crate::ui::theme::{ElementId, MenuListCell};

const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

/// The skin the registry builds for the Remastered HUD under the Remastered palette.
fn test_skin() -> Skin {
    palette::remastered().skin().merged(skin()).merged(palette_skin(&palette::remastered()))
}

fn built(element: ElementId, state: &HudState) -> Option<Vec<UiNode>> {
    let feed = DriveFeedCell::new();
    feed.set(state, SCREEN);
    let skin = test_skin();
    let mut component = RemasteredTheme::new(feed, MenuListCell::new(), Default::default())
        .element(element.slot(), &BuildCx::new(&skin))?;
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
    let skin = test_skin();
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
    let mp = nodes.iter().find(|node| node.text.as_deref() == Some("MP")).expect("the MP");
    assert_eq!(mp.colour, test_skin().colour(keys::UNIT_HIGHLIGHT).expect("MP is gold"));
    let h = nodes.iter().find(|node| node.text.as_deref() == Some("H")).expect("the H");
    assert_eq!(h.colour, test_skin().colour(keys::UNIT).expect("H is muted"));
}

#[test]
fn the_speedometer_shows_kilometres_per_hour_without_the_highlight() {
    let state = HudState { speed: 20.0, use_mph: false, gear: 3, ..Default::default() };
    let nodes = built(ElementId::Speedometer, &state).expect("the speedometer is Remastered");
    assert_eq!(texts(&nodes), ["72", "3", "KM", "/H"]);
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
    let skin = test_skin();
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
fn the_card_follows_the_arc_so_its_top_left_is_cut_away() {
    let skin = test_skin();
    let nodes = built(ElementId::Tachometer, &HudState::default()).expect("the tachometer is Remastered");
    let card_colour = skin.colour(keys::CLUSTER_BG).expect("the card colour");
    let card: Vec<&UiNode> = nodes.iter().filter(|node| node.colour == card_colour).collect();
    let left_of = |node: &&UiNode| node.world.w_axis.x - node.world.x_axis.x / 2.0;
    let top = card.iter().min_by(|a, b| a.world.w_axis.y.total_cmp(&b.world.w_axis.y)).expect("a top strip");
    let bottom = card.iter().max_by(|a, b| a.world.w_axis.y.total_cmp(&b.world.w_axis.y)).expect("a bottom strip");
    assert!(left_of(top) > left_of(bottom) + 20.0, "the top strip starts well right of the bottom one");
    let right_of = |node: &&UiNode| node.world.w_axis.x + node.world.x_axis.x / 2.0;
    assert!(right_of(top) > right_of(bottom) + 5.0, "and the right side leans out at the top");
}

#[test]
fn the_gear_has_a_lighter_panel_and_the_dial_a_light_line_under_it() {
    let skin = test_skin();
    let nodes = built(ElementId::Tachometer, &HudState::default()).expect("the tachometer is Remastered");
    let panel = skin.colour(keys::GEAR_PANEL).expect("the gear panel colour");
    let panels = nodes.iter().filter(|node| node.colour == panel);
    assert!(panels.count() > 10, "the gear panel, in strips that follow the card's leaning side");
    assert!(texts(&nodes).len() >= 8, "and the numbers of the scale");
}

#[test]
fn the_nitrous_bar_shows_only_on_a_car_with_nitrous() {
    let without = HudState { has_nos: false, ..Default::default() };
    let with = HudState { has_nos: true, nos: 0.5, ..Default::default() };
    assert!(built(ElementId::Nitrous, &without).expect("the bar is Remastered").is_empty());
    let nodes = built(ElementId::Nitrous, &with).expect("the bar is Remastered");
    assert_eq!(texts(&nodes), ["N2O"]);
    assert!(nodes.len() > 20, "the pill, its border, the fill with its point, and the label");
}

#[test]
fn the_speed_breaker_shows_its_bar_and_the_gear_indicator_draws_nothing() {
    let state = HudState { speedbreaker: 0.5, ..Default::default() };
    let nodes = built(ElementId::Speedbreaker, &state).expect("the bar is Remastered");
    assert_eq!(texts(&nodes), ["SPEEDBREAKER"]);
    assert!(built(ElementId::GearIndicator, &state).expect("hidden, not left to Original").is_empty());
}

#[test]
fn the_supercharger_shows_only_on_a_car_with_forced_induction() {
    let without = HudState { has_turbo: false, boost_psi: 10.0, ..Default::default() };
    let with = HudState { has_turbo: true, boost_psi: 10.0, ..Default::default() };
    assert!(built(ElementId::Turbo, &without).expect("the bar is Remastered").is_empty());
    assert_eq!(texts(&built(ElementId::Turbo, &with).expect("the bar is Remastered")), ["SUPERCHARGER"]);
}

#[test]
fn the_dial_draws_over_the_gauge_bars_and_the_card_under_them() {
    // A lower depth draws on top, so the arch is at a lower depth than the bars (the default) and the card above them.
    let dial = built(ElementId::Tachometer, &HudState::default()).expect("the tachometer is Remastered");
    let bar = built(ElementId::Nitrous, &HudState { has_nos: true, nos: 0.5, ..Default::default() })
        .expect("the bar is Remastered");
    let card_colour = test_skin().colour(keys::CLUSTER_BG).expect("the card colour");
    let arch_z = dial.iter().filter(|node| node.colour != card_colour).map(|node| node.z).fold(f32::MAX, f32::min);
    let bar_z = bar.iter().map(|node| node.z).fold(f32::MAX, f32::min);
    let card_z = dial.iter().filter(|node| node.colour == card_colour).map(|node| node.z).fold(f32::MIN, f32::max);
    assert!(arch_z < bar_z, "the arch draws over the bars");
    assert!(card_z > bar_z, "the card draws under the bars");
}

#[test]
fn the_pursuit_timer_has_the_minutes_and_seconds_in_the_accent_and_the_hundredths_in_white() {
    let pursuit = crate::hud::PursuitHud { secs: 137.43, ..Default::default() };
    let state = HudState { pursuit: Some(pursuit), ..Default::default() };
    let nodes = built(ElementId::PursuitMeter, &state).expect("the pursuit board is Remastered");
    assert_eq!(texts(&nodes), ["43", "2:17.", "PURSUIT TIME"]);
    let colour_of = |text: &str| nodes.iter().find(|n| n.text.as_deref() == Some(text)).expect(text).colour;
    let skin = test_skin();
    assert_eq!(colour_of("2:17."), skin.colour(keys::UNIT_HIGHLIGHT).expect("gold"));
    assert_eq!(colour_of("43"), skin.colour(keys::VALUE).expect("white"));
    // The two parts meet: the minutes end where the hundredths start.
    let x_of = |text: &str| nodes.iter().find(|n| n.text.as_deref() == Some(text)).expect(text).world.w_axis.x;
    assert!(x_of("2:17.") < x_of("43"));
}

#[test]
fn no_chase_draws_no_timer_and_the_evade_bar_and_the_wrecked_count_appear_with_them() {
    assert!(built(ElementId::PursuitMeter, &HudState::default()).expect("the pursuit board is Remastered").is_empty());
    let calm = crate::hud::PursuitHud { secs: 5.0, ..Default::default() };
    let evading = crate::hud::PursuitHud { secs: 5.0, evade: 0.5, cooldown: true, cops_destroyed: 3, ..calm };
    let count = |pursuit| {
        let state = HudState { pursuit: Some(pursuit), ..Default::default() };
        built(ElementId::PursuitMeter, &state).expect("the pursuit board is Remastered")
    };
    let (calm, evading) = (count(calm), count(evading));
    assert!(evading.len() > calm.len(), "the bar and the count add nodes");
    assert!(texts(&evading).contains(&"3 WRECKED") && !texts(&calm).iter().any(|t| t.contains("WRECKED")));
}
