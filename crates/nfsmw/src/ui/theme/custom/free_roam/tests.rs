//! Tests for free roam: the discovery chips' memory (nearest first, at most two, never twice), the chips' words and the
//! extras line.

use nfsmw_data::gameplay::RaceType;

use super::chips::chip_words;
use super::discovery::{Discovery, MAX_CHIPS, RADIUS_M};
use super::extras::{ExtrasLine, along_route};
use crate::hud::{HudContext, HudState, NavMarker, NavMarkerKind, NavRoute};
use crate::settings::SpeedUnits;

const FADE: f32 = 0.25;

fn race(at: [f32; 2], event: &str) -> NavMarker {
    NavMarker { kind: NavMarkerKind::Race(RaceType::Sprint), position: at, name: Some(event.into()), discovered: true }
}

fn shop(at: [f32; 2]) -> NavMarker {
    NavMarker { kind: NavMarkerKind::AutoShop, position: at, name: None, discovered: true }
}

/// The names (or kinds) of the chips still staying.
fn staying(discovery: &Discovery) -> Vec<f32> {
    discovery.chips().iter().filter(|chip| chip.left.is_none()).map(|chip| chip.distance_m).collect()
}

#[test]
fn the_nearest_places_within_the_radius_come_up_at_most_two() {
    let markers =
        [race([100.0, 0.0], "1.1"), shop([50.0, 0.0]), race([200.0, 0.0], "1.2"), shop([RADIUS_M + 1.0, 0.0])];
    let mut discovery = Discovery::default();
    discovery.update(&markers, Some([0.0, 0.0]), false, 1.0, FADE);
    assert_eq!(staying(&discovery), vec![50.0, 100.0], "the nearest two, nearest first");
    assert_eq!(discovery.chips().len(), MAX_CHIPS);
}

#[test]
fn a_place_the_player_cannot_go_to_or_with_no_car_has_no_chip() {
    let locked = NavMarker { discovered: false, ..race([10.0, 0.0], "1.1") };
    let mut discovery = Discovery::default();
    discovery.update(&[locked], Some([0.0, 0.0]), false, 1.0, FADE);
    assert!(discovery.chips().is_empty(), "a locked race is not announced");
    discovery.update(&[shop([10.0, 0.0])], None, false, 2.0, FADE);
    assert!(discovery.chips().is_empty(), "no car, no distance");
}

#[test]
fn a_chip_follows_the_car_then_leaves_and_never_comes_back() {
    let markers = [shop([100.0, 0.0])];
    let mut discovery = Discovery::default();
    discovery.update(&markers, Some([0.0, 0.0]), false, 1.0, FADE);
    discovery.update(&markers, Some([40.0, 0.0]), false, 2.0, FADE);
    assert_eq!(staying(&discovery), vec![60.0], "the distance follows the car");
    // The car drives away: the chip leaves, fades out, and is gone.
    discovery.update(&markers, Some([1000.0, 0.0]), false, 3.0, FADE);
    assert_eq!(discovery.chips()[0].left, Some(3.0));
    discovery.update(&markers, Some([1000.0, 0.0]), false, 3.0 + FADE, FADE);
    assert!(discovery.chips().is_empty());
    // It comes back near: the place was announced already.
    discovery.update(&markers, Some([0.0, 0.0]), false, 10.0, FADE);
    assert!(discovery.chips().is_empty(), "a discovered place is never announced again");
}

#[test]
fn reaching_a_place_hands_over_to_the_entry_card_and_counts_as_announced() {
    let markers = [shop([20.0, 0.0])];
    let mut discovery = Discovery::default();
    discovery.update(&markers, Some([0.0, 0.0]), false, 1.0, FADE);
    discovery.update(&markers, Some([19.0, 0.0]), true, 2.0, FADE);
    assert!(staying(&discovery).is_empty(), "the entry card takes over");
    discovery.update(&markers, Some([0.0, 0.0]), false, 5.0, FADE);
    assert!(discovery.chips().is_empty());
}

#[test]
fn outside_free_roam_no_place_is_offered_so_the_chips_leave() {
    let markers = [shop([20.0, 0.0])];
    let mut discovery = Discovery::default();
    discovery.update(&markers, Some([0.0, 0.0]), false, 1.0, FADE);
    discovery.update(&[], Some([0.0, 0.0]), false, 2.0, FADE);
    assert!(staying(&discovery).is_empty());
}

#[test]
fn a_chip_says_a_races_type_or_that_a_place_is_near() {
    let mut discovery = Discovery::default();
    discovery.update(&[race([10.0, 0.0], "9.9.9"), shop([20.0, 0.0])], Some([0.0, 0.0]), false, 1.0, FADE);
    let words: Vec<(String, String)> = discovery.chips().iter().map(chip_words).collect();
    // No language table in the tests: the track reads as its event id.
    assert_eq!(words[0], ("Sprint".to_owned(), "9.9.9".to_owned()));
    assert_eq!(words[1], ("Nearby".to_owned(), "Auto shop".to_owned()));
}

#[test]
fn the_extras_line_shows_only_what_the_game_has() {
    let none = HudState::default();
    assert_eq!(ExtrasLine::of(&none).text(SpeedUnits::Metric), None, "outside a career, no cash or bounty");
    let career = HudState { cash: Some(12500), career_bounty: Some(4500), ..HudState::default() };
    assert_eq!(ExtrasLine::of(&career).text(SpeedUnits::Metric).as_deref(), Some("$12,500   Bounty 4,500"));
    let routed = HudState {
        route: Some(NavRoute {
            path: vec![[0.0, 0.0], [1000.0, 0.0], [1000.0, 1500.0]],
            target: [1000.0, 1500.0],
            next_turn: None,
        }),
        context: HudContext { player: Some([0.0, 0.0]), ..HudContext::default() },
        ..HudState::default()
    };
    assert_eq!(ExtrasLine::of(&routed).text(SpeedUnits::Metric).as_deref(), Some("GPS 2.5 km"));
}

#[test]
fn the_route_left_is_measured_from_its_nearest_point() {
    let path = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    assert_eq!(along_route(&path, [100.0, 10.0]), Some(10.0 + 100.0), "from the corner on");
    assert_eq!(along_route(&[], [0.0, 0.0]), None);
}
