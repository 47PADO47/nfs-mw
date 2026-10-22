use super::*;
use blackbox_race::KMH_TO_MS;

fn speedtrap(players: &[u8]) -> Run {
    let players = players.iter().copied().map(PlayerId).collect();
    let mut setup = ModeSetup::new(Mode::Speedtrap, rules(0), players, START);
    setup.trap_thresholds_kmh = vec![88.0, 100.0];
    Run::with(setup)
}

fn pass(run: &mut Run, player: u8, trap: u8, kmh: f32) -> Vec<ModeEvent> {
    run.feed(ModeInput::TrapPass { player: p(player), trap, kmh })
}

#[test]
fn a_pass_pays_only_above_the_threshold_and_the_racers_best() {
    let mut run = speedtrap(&[1]);
    assert!(pass(&mut run, 1, 0, 80.0).is_empty(), "under the 88 km/h threshold");
    let first = pass(&mut run, 1, 0, 180.0);
    let (speed, total) = match first[..] {
        [ModeEvent::TrapScored { speed, total, .. }] => (speed, total),
        _ => panic!("{first:?}"),
    };
    assert!((speed - 180.0 * KMH_TO_MS).abs() < 1e-3 && (total - speed).abs() < 1e-6);
    assert!(pass(&mut run, 1, 0, 150.0).is_empty(), "slower than the best at that trap");
    assert!(pass(&mut run, 1, 7, 300.0).is_empty(), "no such trap");
    pass(&mut run, 1, 1, 200.0);
    let points = run.mode.placings()[0].points;
    assert!((points - 380.0 * KMH_TO_MS).abs() < 1e-3, "{points}");
}

#[test]
fn points_rank_ahead_of_the_finish_time() {
    let mut run = speedtrap(&[1, 2]);
    pass(&mut run, 1, 0, 120.0);
    pass(&mut run, 2, 0, 200.0);
    run.finish(1, 50_000);
    let last = run.finish(2, 70_000);
    assert_eq!(over(&last), Some(OverReason::Finished));
    assert_eq!(run.order(), [2, 1], "more points win though slower to the finish");
}

#[test]
fn equal_points_go_to_the_faster_finish() {
    let mut run = speedtrap(&[1, 2]);
    pass(&mut run, 1, 0, 150.0);
    pass(&mut run, 2, 0, 150.0);
    run.finish(2, 50_000);
    run.finish(1, 55_000);
    assert_eq!(run.order(), [2, 1]);
}

#[test]
fn a_racer_who_leaves_ranks_last_whatever_the_points() {
    let mut run = speedtrap(&[1, 2]);
    pass(&mut run, 1, 0, 250.0);
    run.leave(1);
    pass(&mut run, 2, 0, 100.0);
    run.finish(2, 60_000);
    assert_eq!(run.order(), [2, 1]);
    assert!(run.mode.placings()[1].points > 0.0, "the points stay in the record");
}

#[test]
fn passes_after_the_finish_do_not_count() {
    let mut run = speedtrap(&[1, 2]);
    run.finish(1, 40_000);
    assert!(pass(&mut run, 1, 0, 250.0).is_empty());
}
