use nfsmw_career::rapsheet::PursuitHistory;
use nfsmw_career::stable::Stable;

use super::*;

#[test]
fn pursuit_length_shows_as_a_time_and_the_other_statistics_as_numbers() {
    assert_eq!(value_text(0, 1800 * 4000), "30:00");
    assert_eq!(value_text(1, 1_234), "1,234");
    assert_eq!(threshold_text(0, 600.0), "10:00");
    assert_eq!(threshold_text(1, 25.0), "25");
}

#[test]
fn a_ranked_player_is_listed_in_place_of_a_threshold() {
    let mut history = PursuitHistory::default();
    history.best[1].value = 150;
    let stable = Stable::empty();
    let built = rows(None, &stable, &history, 1, false);
    assert_eq!(built.len(), 15);
    assert_eq!(built[0], RankingRow::Player { rank: 1 }, "150 cops involved is the first single-pursuit row");
    assert_eq!(player_index(&built), 0);
}

#[test]
fn an_unranked_player_is_the_last_row_with_rank_sixteen() {
    let built = rows(None, &Stable::empty(), &PursuitHistory::default(), 9, true);
    assert_eq!(built.len(), 16);
    assert_eq!(built[15], RankingRow::Player { rank: UNRANKED });
    assert_eq!(player_index(&built), 15);
}

#[test]
fn the_window_starts_so_the_player_is_in_view() {
    assert_eq!(start_top(0, 15), 0);
    assert_eq!(start_top(6, 16), 1, "rank 7 scrolls once");
    assert_eq!(start_top(15, 16), 6, "the last full window, rows 7 to 16");
    assert_eq!(start_top(15, 12), 2, "a short list stops at its last full window");
}

#[test]
fn the_window_follows_the_cursor_and_the_cursor_stops_at_the_ends() {
    assert_eq!(follow(0, 9, VISIBLE), 0);
    assert_eq!(follow(0, 10, VISIBLE), 1);
    assert_eq!(follow(5, 2, VISIBLE), 2);
    assert_eq!(move_cursor(0, 16, Step::Up), 0);
    assert_eq!(move_cursor(15, 16, Step::Down), 15);
    assert_eq!(move_cursor(3, 16, Step::Down), 4);
}
