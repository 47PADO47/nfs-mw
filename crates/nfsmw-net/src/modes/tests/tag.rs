use super::*;
use crate::results::FLAG_LEFT;

fn tag(players: &[u8], round_ticks: u32, first: Option<u8>) -> (Run, Vec<ModeEvent>) {
    let players = players.iter().copied().map(PlayerId).collect();
    let mut setup = ModeSetup::new(Mode::Tag, rules(0), players, START);
    setup.tag_round_ticks = round_ticks;
    setup.tag_first = first.map(PlayerId);
    let mut mode = AnyMode::build(&setup).expect("a valid setup");
    let mut out = Vec::new();
    mode.begin(&mut out);
    (Run { mode, tick: START }, out)
}

/// Feeds a car sample of `player` at `x` metres moving at `vx` m/s along x.
fn car(run: &mut Run, player: u8, x: f32, vx: f32) {
    let sample =
        CarSample { player: p(player), position: [x, 0.0, 0.0], velocity: [vx, 0.0, 0.0], ..Default::default() };
    run.feed(ModeInput::Car(sample));
}

fn holder(run: &Run) -> u8 {
    match &run.mode {
        AnyMode::Tag(tag) => tag.holder().0,
        other => panic!("not tag: {other:?}"),
    }
}

#[test]
fn the_first_holder_is_announced_at_the_start() {
    let (_, begin) = tag(&[3, 5], 600, None);
    assert_eq!(begin, [ModeEvent::TagPassed { from: None, to: p(3) }], "the lowest id when none is chosen");
    let (_, chosen) = tag(&[3, 5], 600, Some(5));
    assert_eq!(chosen, [ModeEvent::TagPassed { from: None, to: p(5) }]);
}

#[test]
fn the_contact_rule_is_inclusive_at_the_distance_and_speed_limits() {
    assert!(is_tag(TAG_DISTANCE, TAG_MIN_RELATIVE_SPEED));
    assert!(!is_tag(TAG_DISTANCE + 0.001, 5.0), "just outside the distance");
    assert!(!is_tag(1.0, TAG_MIN_RELATIVE_SPEED - 0.001), "resting contact does not tag");
    assert!(!is_tag(f32::NAN, 5.0));
}

#[test]
fn a_tag_passes_at_exactly_the_distance_and_not_beyond() {
    let (mut run, _) = tag(&[1, 2], 6_000, None);
    car(&mut run, 1, 0.0, 2.0);
    car(&mut run, 2, TAG_DISTANCE + 0.01, 0.0);
    assert!(run.step().is_empty(), "4.01 m is not contact");
    car(&mut run, 1, 0.0, 2.0);
    car(&mut run, 2, TAG_DISTANCE, 0.0);
    assert_eq!(run.step(), [ModeEvent::TagPassed { from: Some(p(1)), to: p(2) }]);
    assert_eq!(holder(&run), 2);
}

#[test]
fn stale_samples_do_not_tag() {
    let (mut run, _) = tag(&[1, 2], 6_000, None);
    car(&mut run, 1, 0.0, 2.0);
    car(&mut run, 2, 1.0, 0.0);
    run.tick += 1;
    car(&mut run, 1, 0.0, 2.0);
    assert!(run.step().is_empty(), "player 2's sample is from the tick before");
}

#[test]
fn the_new_holder_cannot_tag_back_at_once() {
    let (mut run, _) = tag(&[1, 2], 6_000, None);
    car(&mut run, 1, 0.0, 3.0);
    car(&mut run, 2, 1.0, 0.0);
    run.step();
    assert_eq!(holder(&run), 2);
    for _ in 1..TAG_BACK_TICKS {
        car(&mut run, 1, 0.0, 3.0);
        car(&mut run, 2, 1.0, 0.0);
        assert!(run.step().is_empty(), "protected at tick {}", run.tick);
    }
    car(&mut run, 1, 0.0, 3.0);
    car(&mut run, 2, 1.0, 0.0);
    assert_eq!(run.step(), [ModeEvent::TagPassed { from: Some(p(2)), to: p(1) }]);
}

#[test]
fn the_nearest_free_player_is_tagged_and_ties_go_to_the_lower_id() {
    let (mut run, _) = tag(&[1, 2, 3], 6_000, None);
    car(&mut run, 1, 0.0, 2.0);
    car(&mut run, 3, 2.0, 0.0);
    car(&mut run, 2, 3.0, 0.0);
    assert_eq!(run.step(), [ModeEvent::TagPassed { from: Some(p(1)), to: p(3) }]);
    let (mut tie, _) = tag(&[1, 2, 3], 6_000, None);
    car(&mut tie, 1, 0.0, 2.0);
    car(&mut tie, 3, -2.0, 0.0);
    car(&mut tie, 2, 2.0, 0.0);
    assert_eq!(tie.step(), [ModeEvent::TagPassed { from: Some(p(1)), to: p(2) }]);
}

#[test]
fn the_least_time_as_it_wins_when_time_is_up() {
    let (mut run, _) = tag(&[1, 2], 300, None);
    run.steps(100);
    car(&mut run, 1, 0.0, 2.0);
    car(&mut run, 2, 1.0, 0.0);
    run.step();
    let rest = run.steps(199);
    assert!(over(&rest).is_none());
    assert_eq!(over(&run.step()), Some(OverReason::TimeUp));
    assert!(run.step().is_empty(), "nothing after the end");
    let placings = run.mode.placings();
    assert_eq!(run.order(), [1, 2], "player 1 was it for 101 ticks, player 2 for 199");
    assert_eq!(placings[0].time_ms, Some(ticks_ms(101)));
    assert_eq!(placings[1].time_ms, Some(ticks_ms(199)));
}

fn ticks_ms(ticks: u32) -> u32 {
    crate::results::ticks_to_ms(ticks)
}

#[test]
fn the_holder_leaving_passes_the_tag_to_the_least_time_as_it() {
    let (mut run, _) = tag(&[1, 2, 3], 6_000, Some(2));
    run.steps(10);
    car(&mut run, 2, 0.0, 2.0);
    car(&mut run, 3, 1.0, 0.0);
    run.step();
    assert_eq!(holder(&run), 3);
    assert_eq!(run.leave(3), [ModeEvent::TagPassed { from: Some(p(3)), to: p(1) }], "player 1 was never it");
    assert_eq!(run.leave(2), [ModeEvent::Over { reason: OverReason::LastStanding }]);
    assert_eq!(run.order(), [1, 3, 2], "players who left rank last, by their time as it");
    assert_eq!(run.mode.placings()[2].flags, FLAG_LEFT);
}

#[test]
fn tag_needs_two_players() {
    let setup = ModeSetup::new(Mode::Tag, rules(0), vec![p(1)], START);
    assert_eq!(AnyMode::build(&setup), Err(RuleError::TooFewPlayers(2)));
}
