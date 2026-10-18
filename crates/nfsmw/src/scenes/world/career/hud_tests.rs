//! Tests of the career HUD's pure rules: the bounty row, the heat meter and what the HUD state gets.

use super::*;
use crate::hud::PursuitHud;

#[test]
fn the_bounty_row_sums_the_banked_bounty_and_the_pending_points() {
    assert_eq!(bounty_row(43_116, 25_000), 68_116);
    assert_eq!(bounty_row(0, 0), 0);
    assert_eq!(bounty_row(u32::MAX, 1), u32::MAX);
}

#[test]
fn the_heat_meter_stays_within_its_top() {
    assert_eq!(meter_heat(-1.0), 0.0);
    assert_eq!(meter_heat(1.5), 1.5);
    assert_eq!(meter_heat(12.0), 10.0);
    assert_eq!(meter_heat(f32::NAN), 0.0);
}

#[test]
fn a_car_without_a_record_has_no_bounty_and_no_heat() {
    assert_eq!(CareerHud::new(None, 0), CareerHud { on: true, bounty: 0, car_heat: 0.0 });
}

#[test]
fn the_row_and_the_car_heat_go_into_the_state_outside_a_chase() {
    let hud = CareerHud::new(Some((5_000, 2.5)), 1_000);
    let mut state = HudState { vehicle_heat: 4.0, ..Default::default() };
    hud.apply(&mut state);
    assert_eq!(state.career_bounty, Some(6_000));
    assert_eq!(state.vehicle_heat, 2.5, "the car's heat, not the police heat");
}

#[test]
fn during_a_chase_the_heat_stays_the_police_heat() {
    let hud = CareerHud::new(Some((5_000, 2.5)), 0);
    let mut state = HudState { vehicle_heat: 4.0, pursuit: Some(PursuitHud::default()), ..Default::default() };
    hud.apply(&mut state);
    assert_eq!(state.career_bounty, Some(5_000));
    assert_eq!(state.vehicle_heat, 4.0);
}

#[test]
fn outside_career_mode_the_state_is_left_alone() {
    let mut state = HudState { vehicle_heat: 4.0, ..Default::default() };
    CareerHud::default().apply(&mut state);
    assert_eq!(state.career_bounty, None);
    assert_eq!(state.vehicle_heat, 4.0);
}
