//! Tests for the race layout: the rival gap's wording, the strip's fields per race type and detail, the board's rows,
//! the intro card's lines, and the strip leaving at the results.

use std::sync::Arc;

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};
use nfsmw_data::gameplay::RaceType;

use super::gap::{Side, gap_words};
use super::intro::intro_lines;
use super::leaderboard::board_rows;
use super::strip::{Figure, Progress, RaceStrip, StripModel};
use crate::hud::{HudContext, HudState, LeaderRow, Leaders, RaceHud, RaceInfo, RaceIntro, RaceStage, RivalGap};
use crate::settings::hud::HudSettings;
use crate::ui::theme::custom::feed::{CustomFeed, CustomFeedCell};
use crate::ui::theme::custom::mode::Detail;
use crate::ui::theme::custom::tokens::{CustomLook, skin};

fn gap(seconds: f32, ahead: bool) -> RivalGap {
    RivalGap { seconds, ahead, name: "Valdez".into() }
}

fn race(lap: Option<(u32, u32)>) -> RaceHud {
    RaceHud { place: 2, racers: 6, lap, clock: 42.0, progress: 0.5, count: None }
}

fn info(kind: RaceType) -> RaceInfo {
    RaceInfo {
        kind,
        next_gate: Some([0.0; 3]),
        gate: Some(4),
        gates: 12,
        remaining_m: Some(4200.0),
        rival_gap: Some(gap(1.42, true)),
    }
}

fn racing(kind: RaceType, lap: Option<(u32, u32)>) -> HudState {
    HudState {
        race: Some(race(lap)),
        race_info: Some(info(kind)),
        context: HudContext { race_stage: Some(RaceStage::Running), race_points: Some(812.4), ..HudContext::default() },
        ..HudState::default()
    }
}

#[test]
fn the_gap_is_a_size_and_a_side_never_a_sign() {
    assert_eq!(gap_words(&gap(1.42, true)), "1.42 s AHEAD");
    assert_eq!(gap_words(&gap(0.85, false)), "0.85 s BEHIND");
    assert_eq!(gap_words(&gap(0.004, false)), "LEVEL", "the same time at two decimals");
    assert_eq!(Side::of(&gap(0.004, true)), Side::Level);
    assert!(!gap_words(&gap(3.0, false)).contains('-'), "never a signed number");
}

#[test]
fn a_sprint_strip_has_the_checkpoint_the_bar_the_distance_and_the_gap() {
    let strip = StripModel::of(&racing(RaceType::Sprint, None), Detail::Balanced).expect("a race");
    assert_eq!((strip.place, strip.racers), (2, 6));
    assert_eq!(strip.progress, Some(Progress::Gate { now: 4, of: 12 }));
    assert_eq!(strip.bar, Some(0.5));
    assert_eq!(strip.remaining_m, Some(4200.0));
    assert_eq!(strip.rival, Some(gap(1.42, true)));
    assert_eq!(strip.figure, None);
}

#[test]
fn a_lapped_race_says_the_lap_and_the_final_lap() {
    let strip = StripModel::of(&racing(RaceType::Circuit, Some((2, 3))), Detail::Balanced).expect("a race");
    assert_eq!(strip.progress, Some(Progress::Lap { now: 2, of: 3, last: false }));
    let last = StripModel::of(&racing(RaceType::Circuit, Some((3, 3))), Detail::Balanced).expect("a race");
    assert_eq!(last.progress, Some(Progress::Lap { now: 3, of: 3, last: true }));
}

#[test]
fn minimal_keeps_the_place_and_the_lap_only() {
    let strip = StripModel::of(&racing(RaceType::Circuit, Some((1, 3))), Detail::Minimal).expect("a race");
    assert!(strip.progress.is_some());
    assert_eq!((strip.bar, strip.remaining_m, strip.rival), (None, None, None));
}

#[test]
fn the_races_not_ranked_by_time_show_no_gap() {
    for kind in [RaceType::Drag, RaceType::SpeedTrap, RaceType::Tollbooth] {
        let strip = StripModel::of(&racing(kind, None), Detail::Advanced).expect("a race");
        assert_eq!(strip.rival, None, "{kind:?}");
    }
    let drag = StripModel::of(&racing(RaceType::Drag, None), Detail::Advanced).expect("a race");
    assert_eq!((drag.progress, drag.bar), (None, None), "a drag strip is the place and the distance");
    assert_eq!(drag.remaining_m, Some(4200.0));
    let trap = StripModel::of(&racing(RaceType::SpeedTrap, None), Detail::Balanced).expect("a race");
    assert_eq!(trap.figure, Some(Figure::Points(812.4)));
    let toll = StripModel::of(&racing(RaceType::Tollbooth, None), Detail::Balanced).expect("a race");
    assert_eq!(toll.figure, Some(Figure::TimeLeft(42.0)));
}

#[test]
fn missing_race_data_is_absent_from_the_strip() {
    assert_eq!(StripModel::of(&HudState::default(), Detail::Advanced), None, "no race, no strip");
    let bare = HudState { race: Some(race(None)), ..HudState::default() };
    let strip = StripModel::of(&bare, Detail::Advanced).expect("a race");
    assert_eq!((strip.progress, strip.remaining_m, strip.rival), (None, None, None));
}

fn leaders(names: &[&str], player: usize) -> Leaders {
    let rows = names
        .iter()
        .map(|name| LeaderRow { name: (*name).into(), percent: 50.0, points: 0.0, out: false, busted: false })
        .collect();
    Leaders {
        rows,
        racers: names.len() as u32,
        player: Some(player),
        player_percent: 50.0,
        length: 1000.0,
        speed_trap: false,
    }
}

#[test]
fn the_board_shows_the_rows_around_the_player_and_the_gap_on_the_rival() {
    let board = leaders(&["A", "Valdez", "Me", "D"], 2);
    let rival = gap(0.85, false);
    let rows = board_rows(&board, 3, Detail::Balanced, Some(&rival));
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["Valdez", "Me", "D"], "ahead, the player, behind");
    assert_eq!(rows[0].status.as_deref(), Some("0.85 s BEHIND"));
    assert!(rows[1].player && rows[1].place == 3);
    assert_eq!(board_rows(&board, 3, Detail::Advanced, None).len(), 4, "every row in Advanced");
    assert!(board_rows(&board, 3, Detail::Minimal, None).is_empty(), "hidden in Minimal");
    let leading = board_rows(&leaders(&["Me", "B", "C"], 0), 1, Detail::Balanced, None);
    assert_eq!(leading.len(), 2, "the leader and the racer behind");
}

#[test]
fn a_racer_out_of_the_race_keeps_its_state_word() {
    let mut board = leaders(&["A", "Me"], 1);
    board.rows[0].out = true;
    let rows = board_rows(&board, 2, Detail::Advanced, None);
    assert_eq!(rows[0].status.as_deref(), Some("K.O."));
}

#[test]
fn the_intro_card_says_only_what_the_race_has() {
    let full = RaceIntro {
        name: Some("Rosewood Loop".into()),
        kind: RaceType::Circuit,
        laps: Some(3),
        opponents: vec!["Ronnie".into(), "Izzy".into()],
    };
    assert_eq!(intro_lines(&full), ["CIRCUIT", "Rosewood Loop", "3 laps \u{b7} 2 opponents", "Ronnie, Izzy"]);
    let bare = RaceIntro { name: None, kind: RaceType::Sprint, laps: None, opponents: Vec::new() };
    assert_eq!(intro_lines(&bare), ["SPRINT"]);
}

/// The nodes the strip builds for `hud` on the second of two frames a second apart (past the mode's crossfade).
fn strip_nodes(hud: HudState, settings: HudSettings) -> usize {
    let cell = CustomFeedCell::new();
    let hud = Arc::new(hud);
    let mut strip = RaceStrip::new(cell.clone());
    let skin = skin(CustomLook::default(), [255, 200, 80, 255]);
    for now in [1.0, 2.0] {
        cell.set(CustomFeed { hud: Arc::clone(&hud), settings, elapsed_s: now, ..CustomFeed::default() });
        strip.layout(&LayoutCx::new(&skin, now), Rect::SCREEN);
    }
    let mut out = DrawList::new();
    strip.build(&mut out);
    out.len()
}

#[test]
fn the_strip_draws_while_racing_and_leaves_at_the_results_or_with_race_info_off() {
    let settings = HudSettings::default();
    assert!(strip_nodes(racing(RaceType::Sprint, None), settings) > 0);
    let mut results = racing(RaceType::Sprint, None);
    results.context.race_stage = Some(RaceStage::Results);
    assert_eq!(strip_nodes(results, settings), 0, "the results flow has the race");
    let off = HudSettings { race_info: false, ..settings };
    assert_eq!(strip_nodes(racing(RaceType::Sprint, None), off), 0);
}
