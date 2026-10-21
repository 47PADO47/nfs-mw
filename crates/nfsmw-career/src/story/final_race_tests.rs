use super::*;

/// The reference direction for the win tests: the trigger faces along +x.
const WIN_FACING_TEST: [f32; 3] = [1.0, 0.0, 0.0];

/// A point 600 m from the barrier area, outside the 500 m radius.
fn far() -> [f32; 3] {
    [BARRIER_AREA_POS[0] + 600.0, BARRIER_AREA_POS[1], BARRIER_AREA_POS[2]]
}

/// A point 100 m from the barrier area, inside the 500 m radius.
fn near() -> [f32; 3] {
    [BARRIER_AREA_POS[0] + 100.0, BARRIER_AREA_POS[1], BARRIER_AREA_POS[2]]
}

/// A quiet update: far from the barrier, stopped, in no trigger, facing across the win direction.
fn quiet(elapsed_secs: f32) -> FinalRaceInput {
    FinalRaceInput {
        elapsed_secs,
        player_pos: far(),
        player_speed_kmh: 0.0,
        player_forward: [0.0, 1.0, 0.0],
        in_barrier_area: false,
        crossed_win_plane: false,
        in_lose_box: false,
    }
}

/// An update that crosses the win plane at the given speed and facing.
fn crossing(speed_kmh: f32, forward: [f32; 3]) -> FinalRaceInput {
    FinalRaceInput { player_speed_kmh: speed_kmh, player_forward: forward, crossed_win_plane: true, ..quiet(60.0) }
}

/// A race with call 95 already placed, so later updates show only the events under test.
fn race() -> FinalRace {
    let mut race = FinalRace::new(WIN_FACING_TEST);
    race.update(quiet(START_CALL_DELAY_SECS));
    race
}

#[test]
fn start_call_is_placed_once_at_ten_seconds() {
    let mut race = FinalRace::new(WIN_FACING_TEST);
    assert!(race.update(quiet(0.0)).is_empty());
    assert!(race.update(quiet(9.9)).is_empty());
    assert_eq!(race.update(quiet(10.0)), vec![FinalRaceEvent::PlaceCall(95)]);
    assert!(race.update(quiet(11.0)).is_empty());
    assert!(race.update(quiet(20.0)).is_empty());
}

#[test]
fn nothing_fires_up_to_300_seconds_even_far_away() {
    let mut race = race();
    assert!(race.update(quiet(299.0)).is_empty());
    // "more than 300 s": exactly 300 does not count.
    assert!(race.update(quiet(300.0)).is_empty());
}

#[test]
fn barrier_call_needs_more_than_300_seconds_and_more_than_500_metres() {
    let mut race = race();
    // Past 300 s but within 500 m: the Lua reading (C-14) does not fire.
    let close = FinalRaceInput { player_pos: near(), ..quiet(301.0) };
    assert!(race.update(close).is_empty());
    // Past 300 s and more than 500 m away: call 96, the door and the GPS, in that order.
    assert_eq!(
        race.update(quiet(302.0)),
        vec![FinalRaceEvent::PlaceCall(96), FinalRaceEvent::OpenDoor, FinalRaceEvent::GpsTo(GpsTarget::BarrierArea),]
    );
    // Once only.
    assert!(race.update(quiet(303.0)).is_empty());
}

#[test]
fn barrier_distance_is_measured_against_the_500_metre_radius() {
    let just_inside = FinalRaceInput {
        player_pos: [BARRIER_AREA_POS[0] + 499.0, BARRIER_AREA_POS[1], BARRIER_AREA_POS[2]],
        ..quiet(301.0)
    };
    assert!(race().update(just_inside).is_empty());
    let just_outside = FinalRaceInput {
        player_pos: [BARRIER_AREA_POS[0] + 501.0, BARRIER_AREA_POS[1], BARRIER_AREA_POS[2]],
        ..quiet(301.0)
    };
    assert_eq!(race().update(just_outside).len(), 3);
}

#[test]
fn entering_the_barrier_area_hides_the_gps_once_it_is_on() {
    let mut race = race();
    // Inside the area before the GPS is on: nothing to hide.
    let inside_early = FinalRaceInput { in_barrier_area: true, ..quiet(100.0) };
    assert!(race.update(inside_early).is_empty());
    race.update(quiet(301.0));
    let inside = FinalRaceInput { in_barrier_area: true, player_pos: near(), ..quiet(302.0) };
    assert_eq!(race.update(inside), vec![FinalRaceEvent::HideGps]);
    assert!(race.update(inside).is_empty());
}

#[test]
fn a_slow_crossing_is_not_a_win() {
    let mut race = race();
    assert!(race.update(crossing(179.0, WIN_FACING_TEST)).is_empty());
    assert!(!race.is_won());
    assert_eq!(
        race.update(crossing(180.0, WIN_FACING_TEST)),
        vec![FinalRaceEvent::Won, FinalRaceEvent::PlayOutroNis("EndingNis04"), FinalRaceEvent::BailPursuit,]
    );
    assert!(race.is_won());
}

#[test]
fn a_wrong_direction_is_not_a_win() {
    let mut race = race();
    // A direction match of 0.79 is below the 0.8 threshold.
    let off = [0.79, (1.0f32 - 0.79 * 0.79).sqrt(), 0.0];
    assert!(race.update(crossing(200.0, off)).is_empty());
    assert!(!race.is_won());
    // A direction match of 0.9 passes.
    let on = [0.9, (1.0f32 - 0.9 * 0.9).sqrt(), 0.0];
    assert_eq!(race.update(crossing(200.0, on)).len(), 3);
}

#[test]
fn the_direction_match_ignores_the_length_of_the_forward_vector() {
    let mut race = race();
    assert_eq!(race.update(crossing(200.0, [50.0, 0.0, 0.0])).len(), 3);
}

#[test]
fn no_win_without_crossing_the_plane() {
    let mut race = race();
    let not_crossing = FinalRaceInput { crossed_win_plane: false, ..crossing(200.0, WIN_FACING_TEST) };
    assert!(race.update(not_crossing).is_empty());
    assert!(!race.is_won());
}

#[test]
fn a_win_plays_the_outro_and_bails_once() {
    let mut race = race();
    assert_eq!(race.update(crossing(200.0, WIN_FACING_TEST)).len(), 3);
    // Crossing again does not win twice.
    assert!(race.update(crossing(200.0, WIN_FACING_TEST)).is_empty());
}

#[test]
fn a_lose_box_before_the_win_wrecks_the_racer_once() {
    let mut race = race();
    let lose = FinalRaceInput { in_lose_box: true, ..quiet(60.0) };
    assert_eq!(race.update(lose), vec![FinalRaceEvent::Wrecked]);
    assert!(race.update(lose).is_empty());
}

#[test]
fn a_lose_box_after_the_win_is_ignored() {
    let mut race = race();
    assert_eq!(race.update(crossing(200.0, WIN_FACING_TEST)).len(), 3);
    let lose = FinalRaceInput { in_lose_box: true, ..quiet(60.0) };
    assert!(race.update(lose).is_empty());
}

#[test]
fn after_win_is_the_end_game_chain_in_order() {
    let expected = vec![
        EndGameStep::ChallengeComplete,
        EndGameStep::NotifyRaceFinished,
        EndGameStep::RaceOverToPostRace,
        EndGameStep::CareerComplete,
        EndGameStep::Screen("GameWon.fng"),
        EndGameStep::Screen("Credits.fng"),
        EndGameStep::Screen("RapSheetLogin_ENDGAME.fng"),
        EndGameStep::Screen("RapSheetLogin2_ENDGAME.fng"),
        EndGameStep::Screen("RapSheetMain_ENDGAME.fng"),
        EndGameStep::Screen("MainMenu_Sub.fng"),
    ];
    assert_eq!(after_win(), expected);
}
