use super::*;
use crate::results::{FLAG_BUSTED, FLAG_DNF, FLAG_LEFT};

#[test]
fn circuit_ranks_by_finish_time_and_keeps_the_best_lap() {
    let mut run = Run::new(Mode::Circuit, 2, &[1, 2, 3]);
    assert_eq!(run.lap(1, 1, 30_000), [ModeEvent::LapCompleted { player: p(1), lap: 1, time_ms: 30_000, best: true }]);
    run.lap(2, 1, 31_000);
    run.lap(3, 1, 33_000);
    assert_eq!(run.lap(1, 2, 29_000), [ModeEvent::LapCompleted { player: p(1), lap: 2, time_ms: 29_000, best: true }]);
    assert_eq!(run.finish(1, 59_000), [ModeEvent::Finished { player: p(1), place: 1, time_ms: 59_000 }]);
    assert_eq!(run.lap(2, 2, 32_000), [ModeEvent::LapCompleted { player: p(2), lap: 2, time_ms: 32_000, best: false }]);
    run.finish(2, 63_000);
    let last = run.finish(3, 70_000);
    assert_eq!(over(&last), Some(OverReason::Finished));
    assert!(run.mode.is_over());
    let placings = run.mode.placings();
    assert_eq!(run.order(), [1, 2, 3]);
    assert_eq!(placings[0].best_lap_ms, Some(29_000));
    assert_eq!(placings[1].best_lap_ms, Some(31_000));
    assert_eq!(placings[2].time_ms, Some(70_000));
    assert!(placings.iter().all(|placing| placing.flags == 0));
}

#[test]
fn an_equal_finish_time_puts_the_lower_player_id_ahead() {
    let mut run = Run::new(Mode::Sprint, 0, &[4, 7]);
    assert_eq!(run.finish(7, 50_000), [ModeEvent::Finished { player: p(7), place: 1, time_ms: 50_000 }]);
    run.finish(4, 50_000);
    assert_eq!(run.order(), [4, 7], "the tie goes to the lower id, not to the first event");
    let places: Vec<u8> = run.mode.placings().iter().map(|placing| placing.place).collect();
    assert_eq!(places, [1, 2], "places stay unique on a tie");
}

#[test]
fn a_reset_penalty_is_added_to_the_finish_time_and_can_cost_the_win() {
    let mut run = Run::new(Mode::Sprint, 0, &[1, 2]);
    let penalty = run.feed(ModeInput::Reset { player: p(1), penalty_ms: 5_000 });
    assert_eq!(penalty, [ModeEvent::Penalty { player: p(1), ms: 5_000 }]);
    assert_eq!(run.finish(1, 60_000), [ModeEvent::Finished { player: p(1), place: 1, time_ms: 65_000 }]);
    run.finish(2, 62_000);
    assert_eq!(run.order(), [2, 1], "the finish place was provisional; the record is final");
    let free = run.feed(ModeInput::Reset { player: p(2), penalty_ms: 5_000 });
    assert!(free.is_empty(), "no penalty after the finish");
}

#[test]
fn a_disconnect_mid_race_is_a_dnf_behind_every_finisher() {
    let mut run = Run::new(Mode::Circuit, 1, &[1, 2, 3]);
    run.feed(ModeInput::Car(CarSample { player: p(2), laps: 0, percent: 80.0, ..CarSample::default() }));
    run.step();
    assert!(run.leave(2).is_empty(), "a departure has no event of its own");
    run.finish(3, 40_000);
    let last = run.finish(1, 41_000);
    assert_eq!(over(&last), Some(OverReason::Finished));
    assert_eq!(run.order(), [3, 1, 2]);
    let left = run.mode.placings()[2];
    assert_eq!(left.flags, FLAG_LEFT | FLAG_DNF);
    assert_eq!(left.time_ms, None);
}

#[test]
fn a_finisher_who_leaves_keeps_the_result() {
    let mut run = Run::new(Mode::Sprint, 0, &[1, 2]);
    run.finish(1, 30_000);
    run.leave(1);
    run.finish(2, 31_000);
    let first = run.mode.placings()[0];
    assert_eq!((first.player, first.flags, first.time_ms), (p(1), 0, Some(30_000)));
}

#[test]
fn everyone_busted_ends_the_race_with_no_winner() {
    let mut run = Run::new(Mode::Circuit, 2, &[1, 2]);
    assert_eq!(run.sim(SimEvent::Busted { player: p(1) }), [ModeEvent::Spectate { player: p(1) }]);
    run.tick += 60;
    let last = run.sim(SimEvent::Busted { player: p(2) });
    assert_eq!(last, [ModeEvent::Spectate { player: p(2) }, ModeEvent::Over { reason: OverReason::AllBusted }]);
    let placings = run.mode.placings();
    assert_eq!(run.order(), [2, 1], "the later bust is the better place");
    assert!(placings.iter().all(|placing| placing.flags == FLAG_BUSTED | FLAG_DNF));
}

#[test]
fn the_last_racer_leaving_ends_the_race_empty() {
    let mut run = Run::new(Mode::Sprint, 0, &[3]);
    assert_eq!(over(&run.leave(3)), Some(OverReason::Empty));
}

#[test]
fn racers_still_on_the_course_rank_by_laps_then_progress() {
    let mut run = Run::new(Mode::Circuit, 3, &[1, 2, 3]);
    run.feed(ModeInput::Car(CarSample { player: p(1), laps: 1, percent: 10.0, ..CarSample::default() }));
    run.feed(ModeInput::Car(CarSample { player: p(2), laps: 1, percent: 60.0, ..CarSample::default() }));
    run.feed(ModeInput::Car(CarSample { player: p(3), laps: 0, percent: 99.0, ..CarSample::default() }));
    assert_eq!(run.order(), [2, 1, 3]);
    assert!(!run.mode.is_over());
}

#[test]
fn inputs_before_the_start_tick_and_unknown_players_are_ignored() {
    let mut run = Run::new(Mode::Sprint, 0, &[1, 2]);
    run.tick = START - 1;
    assert!(run.finish(1, 1_000).is_empty(), "before the start");
    run.tick = START;
    assert!(run.finish(9, 1_000).is_empty(), "not in the race");
    assert!(run.mode.placings().iter().all(|placing| placing.time_ms.is_none()));
}

#[test]
fn a_finished_race_reads_nothing_more() {
    let mut run = Run::new(Mode::Sprint, 0, &[1]);
    run.finish(1, 10_000);
    assert!(run.mode.is_over());
    assert!(run.lap(1, 1, 5_000).is_empty());
    assert!(run.sim(SimEvent::Busted { player: p(1) }).is_empty());
}
