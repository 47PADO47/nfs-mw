use nfsmw_data::pursuit::{RepRates, RepTable};

use super::*;

#[test]
fn a_crossing_counts_each_ten_second_multiple() {
    assert_eq!(steps_crossed(0.0, 10.0), 1);
    assert_eq!(steps_crossed(9.9, 10.1), 1);
    assert_eq!(steps_crossed(0.0, 35.0), 3);
    assert_eq!(steps_crossed(12.0, 19.0), 0, "no multiple of 10 in between");
}

#[test]
fn no_movement_or_a_backwards_or_nan_clock_crosses_nothing() {
    assert_eq!(steps_crossed(10.0, 10.0), 0);
    assert_eq!(steps_crossed(15.0, 5.0), 0);
    assert_eq!(steps_crossed(f32::NAN, 10.0), 0);
    assert_eq!(steps_crossed(0.0, f32::NAN), 0);
}

#[test]
fn the_first_kill_is_times_one_and_kills_in_the_window_raise_it_to_three() {
    let mut rep = RepState::default();
    assert_eq!(rep.wreck(250, 10.0), 250);
    assert_eq!(rep.wreck(250, 10.0), 500);
    assert_eq!(rep.wreck(250, 10.0), 750);
    assert_eq!(rep.wreck(250, 10.0), 750, "the multiplier stops at three");
    assert_eq!(rep.points(), 2250);
    assert_eq!(rep.cops_destroyed, 4);
}

#[test]
fn a_kill_after_the_window_runs_out_is_times_one_again() {
    let mut rep = RepState::default();
    rep.wreck(250, 10.0);
    rep.tick_window(10.0);
    assert_eq!(rep.wreck(250, 10.0), 250);
}

#[test]
fn the_window_runs_down_and_stops_at_zero() {
    let mut rep = RepState::default();
    rep.wreck(100, 10.0);
    rep.tick_window(4.0);
    assert!((rep.window_secs - 6.0).abs() < 1e-6);
    rep.tick_window(10.0);
    assert_eq!(rep.window_secs, 0.0);
}

#[test]
fn the_time_in_sight_banks_the_rate_per_step() {
    let mut rep = RepState::default();
    rep.time_in_sight(0.0, 25.0, 100);
    assert_eq!(rep.normal, 200);
    assert_eq!(rep.points(), 200);
}

#[test]
fn each_wreck_costs_five_thousand_and_each_new_damaged_cop_two_hundred_fifty() {
    let mut rep = RepState::default();
    rep.wreck(0, 0.0);
    assert_eq!(rep.cost, 5000);
    rep.damaged_to(2);
    assert_eq!(rep.cost, 5500);
    rep.damaged_to(2);
    assert_eq!(rep.cost, 5500, "a cop counted once costs once");
    rep.damaged_to(3);
    assert_eq!(rep.cost, 5750);
}

#[test]
fn a_chase_reset_clears_the_totals_and_keeps_the_table() {
    let table = RepTable {
        rates: vec![RepRates { per_ten_seconds: 500, destroy_bonus_secs: 10.0 }],
        destroy: vec![("copgto".to_owned(), vec![2500])],
    };
    let mut rep = RepState { table, ..RepState::default() };
    rep.wreck(2500, 10.0);
    rep.time_in_sight(0.0, 10.0, 500);
    rep.reset();
    assert_eq!((rep.points(), rep.cost, rep.cops_destroyed), (0, 0, 0));
    assert_eq!(rep.table.rate(1).per_ten_seconds, 500);
    assert_eq!(rep.table.destroy_points("copgto", 1), 2500);
}
