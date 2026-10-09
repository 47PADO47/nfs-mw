//! The state the game hands the music, and the console command's number parsing.

use super::super::director::control_value;
use super::super::glue::percent;
use super::super::state::{MusicInput, MusicState, PURSUIT_SETS, Pursuit, pursuit_set_for_heat};

#[test]
fn a_pursuit_is_clamped_to_what_the_music_accepts() {
    assert_eq!(Pursuit::new(0, 0.5).set, 1);
    assert_eq!(Pursuit::new(9, 0.5).set, PURSUIT_SETS);
    assert_eq!(Pursuit::new(2, -1.0).intensity, 0.0);
    assert_eq!(Pursuit::new(2, 7.0).intensity, 1.0);
    assert_eq!(Pursuit::new(2, f32::NAN).intensity, 0.0);
}

#[test]
fn the_hooks_start_steer_and_end_a_pursuit() {
    let mut input = MusicInput::default();
    assert_eq!(input.state(), MusicState::default());

    // Steering outside a pursuit does nothing.
    input.set_pursuit_intensity(1.0);
    input.set_pursuit_set(3);
    assert_eq!(input.state().pursuit, None);

    input.start_pursuit(2, 0.25);
    assert_eq!(input.state().pursuit, Some(Pursuit { set: 2, intensity: 0.25 }));
    input.set_pursuit_intensity(0.75);
    input.set_pursuit_set(4);
    assert_eq!(input.state().pursuit, Some(Pursuit { set: 4, intensity: 0.75 }));
    input.end_pursuit();
    assert_eq!(input.state().pursuit, None);
}

#[test]
fn the_race_hooks_set_the_flag() {
    let mut input = MusicInput::default();
    input.start_race();
    assert!(input.state().racing);
    input.end_race();
    assert!(!input.state().racing);
    assert_eq!(MusicState::chased(3, 0.5).pursuit, Some(Pursuit { set: 3, intensity: 0.5 }));
}

#[test]
fn heat_picks_a_set_from_one_to_four() {
    let sets: Vec<u8> = [1.0, 1.9, 2.0, 2.9, 3.0, 3.9, 4.0, 5.0].map(pursuit_set_for_heat).to_vec();
    assert_eq!(sets, [1, 1, 2, 2, 3, 3, 4, 4]);
    assert_eq!(pursuit_set_for_heat(0.0), 1);
    assert_eq!(pursuit_set_for_heat(99.0), PURSUIT_SETS);
    assert_eq!(pursuit_set_for_heat(f32::NAN), 1);
}

#[test]
fn intensity_maps_onto_the_control_range() {
    assert_eq!(control_value(0.0), 0);
    assert_eq!(control_value(1.0), 127);
    assert_eq!(control_value(0.5), 64);
    assert_eq!(control_value(-3.0), 0);
    assert_eq!(control_value(3.0), 127);
}

#[test]
fn the_console_reads_percentages() {
    assert_eq!(percent("50"), Ok(0.5));
    assert_eq!(percent("100%"), Ok(1.0));
    assert!(percent("101").is_err());
    assert!(percent("-1").is_err());
    assert!(percent("loud").is_err());
}
