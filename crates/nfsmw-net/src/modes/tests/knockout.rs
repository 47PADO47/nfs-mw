use super::*;
use crate::results::{FLAG_DNF, FLAG_KNOCKED_OUT, FLAG_LEFT};

fn knocked(events: &[ModeEvent]) -> Vec<u8> {
    events
        .iter()
        .filter_map(|event| match event {
            ModeEvent::KnockedOut { player, .. } => Some(player.0),
            _ => None,
        })
        .collect()
}

#[test]
fn the_last_of_each_lap_is_knocked_out_and_spectates() {
    let mut run = Run::new(Mode::Knockout, 3, &[1, 2, 3, 4]);
    run.lap(1, 1, 30_000);
    run.lap(2, 1, 31_000);
    let third = run.lap(3, 1, 32_000);
    assert_eq!(third[1..], [ModeEvent::KnockedOut { player: p(4), lap: 1 }, ModeEvent::Spectate { player: p(4) }]);
    assert!(knocked(&run.lap(4, 1, 40_000)).is_empty(), "a knocked-out racer's lap does not count");
    run.lap(1, 2, 30_000);
    assert_eq!(knocked(&run.lap(3, 2, 30_000)), [2]);
    run.tick += 600;
    let last_lap = run.lap(3, 3, 29_000);
    assert_eq!(knocked(&last_lap), [1]);
    assert_eq!(over(&last_lap), Some(OverReason::LastStanding));
    assert!(last_lap.contains(&ModeEvent::Finished { player: p(3), place: 1, time_ms: 10_000 }));
    assert_eq!(run.order(), [3, 1, 2, 4], "the first out is last");
    let placings = run.mode.placings();
    assert_eq!(placings[0].flags, 0);
    assert!(placings[1..].iter().all(|placing| placing.flags == FLAG_KNOCKED_OUT | FLAG_DNF));
}

#[test]
fn a_departure_before_the_knockout_is_that_laps_knockout() {
    let mut run = Run::new(Mode::Knockout, 2, &[1, 2, 3]);
    run.leave(3);
    assert!(knocked(&run.lap(1, 1, 30_000)).is_empty());
    assert!(knocked(&run.lap(2, 1, 31_000)).is_empty(), "the leaver already took lap 1's place");
    let lap_two = run.lap(1, 2, 30_000);
    assert_eq!(knocked(&lap_two), [2]);
    assert_eq!(over(&lap_two), Some(OverReason::LastStanding));
    assert_eq!(run.order(), [1, 2, 3]);
    assert_eq!(run.mode.placings()[2].flags, FLAG_LEFT | FLAG_DNF);
}

#[test]
fn a_leader_who_leaves_after_the_lap_does_not_spare_the_last() {
    let mut run = Run::new(Mode::Knockout, 3, &[1, 2, 3, 4]);
    run.lap(1, 1, 30_000);
    run.lap(2, 1, 31_000);
    run.leave(1);
    assert_eq!(knocked(&run.lap(3, 1, 32_000)), [4], "three were in, two completed: the last is out");
}

#[test]
fn a_single_player_left_by_disconnects_wins_at_once() {
    let mut run = Run::new(Mode::Knockout, 5, &[1, 2, 3]);
    run.tick += 120;
    run.leave(2);
    run.tick += 60;
    let last = run.leave(3);
    assert_eq!(
        last,
        [
            ModeEvent::Finished { player: p(1), place: 1, time_ms: 3_000 },
            ModeEvent::Over { reason: OverReason::LastStanding }
        ]
    );
    assert_eq!(run.order(), [1, 3, 2], "the later departure ranks higher");
}

#[test]
fn a_bust_counts_as_the_laps_knockout() {
    let mut run = Run::new(Mode::Knockout, 2, &[1, 2, 3]);
    run.sim(SimEvent::Busted { player: p(2) });
    run.lap(1, 1, 30_000);
    assert!(knocked(&run.lap(3, 1, 31_000)).is_empty());
}

#[test]
fn knockout_needs_two_players() {
    let setup = ModeSetup::new(Mode::Knockout, rules(2), vec![p(1)], START);
    assert_eq!(AnyMode::build(&setup), Err(RuleError::TooFewPlayers(2)));
}
