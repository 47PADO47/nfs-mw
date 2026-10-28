//! The feed over made-up cars, and the numbers of the real install (set `MW2012_GAME_DIR` to the 2012 install and run with
//! `--ignored`).

use std::path::PathBuf;

use blackbox_modding::sdk::VehicleState;
use chameleon_vehicle::DriftParams;

use super::{Feed, MESSAGE_EVERY, message_text, params_from_install};

const DT: f32 = 1.0 / 60.0;

/// A car doing 25 m/s, 20 degrees crooked, on the ground.
fn drifting_car() -> VehicleState {
    VehicleState { speed: 25.0, slip_angle: 20.0_f32.to_radians(), on_ground: true, ..VehicleState::default() }
}

fn run(feed: &mut Feed, car: &VehicleState, seconds: f32) -> (f32, Vec<String>) {
    let mut gain = 0.0;
    let mut messages = Vec::new();
    for _ in 0..(seconds / DT).round() as u32 {
        let tick = feed.tick(car, DT);
        gain += tick.gain;
        messages.extend(tick.message);
    }
    (gain, messages)
}

#[test]
fn a_car_that_drives_straight_earns_nothing_and_shows_nothing() {
    let mut feed = Feed::new(DriftParams::default());
    let straight = VehicleState { speed: 30.0, on_ground: true, ..VehicleState::default() };
    assert_eq!(run(&mut feed, &straight, 3.0), (0.0, Vec::new()));
}

#[test]
fn a_drift_earns_the_burn_rate_after_its_delay() {
    let mut feed = Feed::new(DriftParams::default());
    let (gain, _) = run(&mut feed, &drifting_car(), 5.0);
    // 5 s, the first 0.75 s unpaid, at 6 units of 100 a second.
    assert!((gain - 4.25 * 0.06).abs() < 0.01, "{gain}");
}

#[test]
fn the_hud_text_is_refreshed_while_the_drift_pays_and_not_before() {
    let mut feed = Feed::new(DriftParams::default());
    let (_, early) = run(&mut feed, &drifting_car(), 0.7);
    assert!(early.is_empty(), "nothing pays yet: {early:?}");
    let (_, messages) = run(&mut feed, &drifting_car(), 2.0);
    let expected = (2.0 / MESSAGE_EVERY) as usize;
    assert!(messages.len().abs_diff(expected) <= 1, "{} texts in 2 s", messages.len());
    assert!(messages.iter().all(|text| text.starts_with("DRIFTING ") && text.ends_with(" m")), "{messages:?}");
}

#[test]
fn a_hit_or_the_air_ends_the_drift_and_a_reset_forgets_it() {
    let mut feed = Feed::new(DriftParams::default());
    run(&mut feed, &drifting_car(), 2.0);
    let hit = VehicleState { impact: 0.6, ..drifting_car() };
    assert_eq!(feed.tick(&hit, DT).gain, 0.0);
    let (gain, _) = run(&mut feed, &VehicleState { on_ground: false, ..drifting_car() }, 1.0);
    assert_eq!(gain, 0.0, "a car in the air does not start a drift");
    run(&mut feed, &drifting_car(), 2.0);
    feed.reset();
    let (gain, _) = run(&mut feed, &drifting_car(), 0.5);
    assert_eq!(gain, 0.0, "after a reset the delay starts again");
}

#[test]
fn the_text_is_the_2012_ticker() {
    assert_eq!(message_text(152.4), "DRIFTING 152 m");
    assert_eq!(message_text(0.0), "DRIFTING 0 m");
}

#[test]
fn an_install_without_car_bundles_gives_no_numbers() {
    let nothing = std::env::temp_dir().join("nfsmw-2012-content-no-install");
    assert!(params_from_install(&nothing).is_none());
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_install_gives_the_burn_payout_in_the_rule() {
    let Some(dir) = std::env::var_os("MW2012_GAME_DIR").map(PathBuf::from) else { return };
    let params = params_from_install(&dir).expect("a car bundle reads");
    assert_eq!(params, DriftParams::default());
}
