use super::leaders::{Entry, LeaderRow, Leaders, Phase, Rotation, Words, group_digits};

fn entry(place: u32, player: bool, name: &str, percent: f32) -> Entry {
    Entry { place, player, row: LeaderRow { name: name.into(), percent, points: 0.0, out: false, busted: false } }
}

/// A race of 4000 m with the player first and three racers behind.
fn sprint() -> Leaders {
    Leaders::pick(
        vec![
            entry(3, false, "CARLA", 60.0),
            entry(1, true, "stock", 70.0),
            entry(4, false, "DOM", 10.0),
            entry(2, false, "ABEL", 69.0),
        ],
        4000.0,
        false,
    )
}

#[test]
fn the_rows_are_the_places_in_order() {
    let leaders = sprint();
    let names: Vec<&str> = leaders.rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["stock", "ABEL", "CARLA", "DOM"]);
    assert_eq!(leaders.player, Some(0));
    assert_eq!(leaders.racers, 4);
    assert_eq!(leaders.shown_rows(), 4);
}

#[test]
fn more_than_four_racers_show_the_four_best() {
    let entries = (1..=6).map(|p| entry(p, p == 5, &format!("R{p}"), 100.0 - 10.0 * p as f32)).collect();
    let leaders = Leaders::pick(entries, 1000.0, false);
    assert_eq!(leaders.rows.len(), 4);
    assert_eq!(leaders.rows[3].name, "R4");
    assert_eq!(leaders.racers, 6);
    assert_eq!(leaders.shown_rows(), 4);
    // The player is fifth: no row is the player's, but the gaps are still measured from the player.
    assert_eq!(leaders.player, None);
    assert_eq!(leaders.player_percent, 50.0);
    assert_eq!(leaders.status(0, false, &Words::default()), "+1,312 FT");
}

#[test]
fn a_race_of_one_racer_hides_the_board() {
    let leaders = Leaders::pick(vec![entry(1, true, "stock", 5.0)], 1000.0, false);
    assert_eq!(leaders.shown_rows(), 0);
    let two = Leaders::pick(vec![entry(1, true, "stock", 5.0), entry(2, false, "A", 4.0)], 1000.0, false);
    assert_eq!(two.shown_rows(), 2);
}

#[test]
fn the_player_row_says_dashes_and_the_others_their_gap_in_feet() {
    let leaders = sprint();
    let words = Words::default();
    assert_eq!(leaders.status(0, false, &words), "-----");
    // 1 percent of 4000 m is 40 m, 131.2 ft.
    assert_eq!(leaders.status(1, false, &words), "-131 FT");
    // 10 percent is 400 m, 1312.3 ft: the thousands are grouped.
    assert_eq!(leaders.status(2, false, &words), "-1,312 FT");
    // 60 percent is 2400 m, 7874 ft.
    assert_eq!(leaders.status(3, false, &words), "-7,874 FT");
}

#[test]
fn the_metric_setting_prints_metres() {
    let leaders = sprint();
    let words = Words::default();
    assert_eq!(leaders.status(1, true, &words), "-40 M");
    assert_eq!(leaders.status(3, true, &words), "-2,400 M");
}

#[test]
fn a_racer_ahead_of_the_player_has_a_plus() {
    let leaders = Leaders::pick(
        vec![entry(1, false, "LEAD", 55.0), entry(2, true, "stock", 50.0), entry(3, false, "TAIL", 49.0)],
        2000.0,
        false,
    );
    let words = Words::default();
    // 5 percent of 2000 m is 100 m, 328.084 ft.
    assert_eq!(leaders.status(0, false, &words), "+328 FT");
    assert_eq!(leaders.status(1, false, &words), "-----");
    assert_eq!(leaders.status(2, false, &words), "-66 FT");
}

#[test]
fn a_racer_that_is_out_says_its_state_instead_of_a_gap() {
    let mut leaders = sprint();
    leaders.rows[2].out = true;
    leaders.rows[3].busted = true;
    let words = Words::default();
    assert_eq!(leaders.status(2, false, &words), "K.O.");
    assert_eq!(leaders.status(3, false, &words), "BUSTED");
}

#[test]
fn a_speed_trap_race_prints_the_points_in_the_speed_unit() {
    let mut leaders = sprint();
    leaders.speed_trap = true;
    leaders.rows[0].points = 300.0;
    let words = Words::default();
    assert_eq!(leaders.status(0, true, &words), "300 KM/H");
    // 300 km/h is 186 mph.
    assert_eq!(leaders.status(0, false, &words), "186 MPH");
}

#[test]
fn the_digits_are_grouped_by_threes() {
    for (value, text) in [(0.0, "0"), (999.0, "999"), (1000.0, "1,000"), (1234567.4, "1,234,567"), (12345.6, "12,346")]
    {
        assert_eq!(group_digits(value), text, "{value}");
    }
}

#[test]
fn the_rows_swap_their_text_every_ninety_updates() {
    let mut rotation = Rotation::default();
    assert_eq!(rotation.phase(), Phase::Names);
    // 89 updates of 1/60 s: not yet.
    rotation.advance(89.0 / 60.0 + 0.001);
    assert!(!rotation.take_leave());
    rotation.advance(1.0 / 60.0);
    assert!(rotation.take_leave(), "the 90th update starts the leave");
    assert!(rotation.leaving());
    assert!(!rotation.take_leave(), "the leave starts once");
    rotation.swap();
    assert_eq!(rotation.phase(), Phase::Gaps);
    assert!(!rotation.leaving());
    // And back, a period later.
    rotation.advance(1.6);
    assert!(rotation.take_leave());
    rotation.swap();
    assert_eq!(rotation.phase(), Phase::Names);
}

#[test]
fn a_long_frame_still_starts_the_leave() {
    let mut rotation = Rotation::default();
    rotation.advance(10.0);
    assert!(rotation.take_leave());
}
