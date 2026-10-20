//! Losing the suspect, the quadrant lines, regaining contact, the bust, and a reset.

use super::super::*;
use super::support::*;

/// A chase that has passed its opening and talks as a strategy chase.
fn in_strategy(seed: u64) -> Chase {
    let mut chase = Chase::new(seed);
    chase.snap.cops = vec![cop(10, 40.0), cop(11, 70.0)];
    chase.run(16.0);
    assert_eq!(chase.machine.focus(), Focus::Strategy, "{}", chase.machine.status());
    chase
}

/// The cops lose sight; the pursuit is cooling down with `secs` to run.
fn lose(chase: &mut Chase, secs: f32) {
    chase.lose_sight();
    chase.snap.pursuit.cooling_down = true;
    chase.snap.pursuit.cooldown_remaining = secs;
}

#[test]
fn the_opening_hands_over_to_the_strategy_flow() {
    let chase = in_strategy(1);
    assert!(
        chase.radio.played_lines().contains(&Line::InitiatePursuit)
            || chase.radio.played_lines().contains(&Line::LocationReport)
    );
}

#[test]
fn losing_the_suspect_says_lost_visual_then_the_break_away() {
    let mut chase = in_strategy(2);
    lose(&mut chase, 60.0);
    chase.run(12.0);
    let lines = chase.radio.lines();
    assert!(lines.contains(&Line::LostVisual), "{lines:?}");
    assert!(lines.contains(&Line::LostSuspect), "no cop in view: the last speaker says lost suspect: {lines:?}");
    let away = chase.radio.first(Line::DispBreakAway).expect("dispatch reports the break away");
    assert_eq!(away.speaker, DISPATCH);
    assert_eq!(away.params.road, Some(5));
    assert_eq!(chase.machine.focus(), Focus::Lost);
}

#[test]
fn the_quadrant_lines_follow_in_order() {
    let mut chase = in_strategy(3);
    lose(&mut chase, 90.0);
    chase.run(40.0);
    let lines = chase.radio.played_lines();
    let at = |line: Line| lines.iter().position(|l| *l == line);
    assert!(at(Line::QuadrantForming).is_some(), "{lines:?}");
    let forming = at(Line::QuadrantForming).unwrap();
    let after = [Line::PossibleSuspect, Line::SuspectPossiblyGone, Line::QuadrantMoving, Line::OtherLead];
    let next = after.iter().filter_map(|l| at(*l)).min().expect("one stray remark");
    assert!(next > forming);
    if let Some(wrong) = at(Line::WrongSuspect) {
        assert!(wrong > at(Line::PossibleSuspect).unwrap(), "a wrong suspect follows a possible one");
    }
}

#[test]
fn dispatch_calls_the_time_expired_once_when_the_pursuit_is_gone() {
    let mut chase = in_strategy(4);
    lose(&mut chase, 30.0);
    chase.run(25.0);
    chase.snap.pursuit.cooling_down = false;
    chase.snap.pursuit.evaded = true;
    chase.run(5.0);
    chase.snap.pursuit.exists = false;
    chase.run(40.0);
    assert_eq!(chase.radio.count(Line::DispTimeExpired), 1, "{:?}", chase.radio.lines());
}

#[test]
fn contact_regained_in_time_reinitiates_the_pursuit() {
    let mut chase = in_strategy(5);
    lose(&mut chase, 90.0);
    chase.run(14.0);
    assert_eq!(chase.machine.focus(), Focus::Lost, "{}", chase.machine.status());
    chase.regain_sight();
    chase.snap.pursuit.cooling_down = false;
    chase.run(8.0);
    let again = chase.radio.lines();
    assert!(
        again.contains(&Line::ReinitiatePursuit)
            || again.contains(&Line::Spotted)
            || again.contains(&Line::RegainVisual),
        "{again:?}"
    );
    assert!(matches!(chase.machine.focus(), Focus::Pursuit | Focus::Strategy), "{}", chase.machine.status());
}

#[test]
fn a_reinitiated_pursuit_after_a_long_loss_says_how_long() {
    let mut chase = in_strategy(6);
    lose(&mut chase, 200.0);
    chase.run(60.0);
    chase.regain_sight();
    chase.snap.pursuit.cooling_down = false;
    chase.run(6.0);
    let again = chase.radio.first(Line::ReinitiatePursuit).expect("re-initiate");
    assert_eq!(again.params.lost_for, Some(LostFor::ForAWhile));
}

#[test]
fn a_chase_abandoned_for_good_starts_over() {
    let mut chase = in_strategy(7);
    lose(&mut chase, 400.0);
    chase.snap.pursuit.exists = false;
    chase.run(170.0);
    assert_eq!(chase.machine.focus(), Focus::Pursuit, "{}", chase.machine.status());
    assert!(chase.radio.effects.contains(&Effect::ResetHistory));
}

#[test]
fn a_bust_silences_the_machine() {
    let mut chase = in_strategy(8);
    chase.notices.push(Notice::Busted);
    chase.run(1.0);
    let before = chase.radio.log.len();
    chase.snap.cops.push(cop(11, 20.0));
    chase.run(10.0);
    assert_eq!(chase.machine.focus(), Focus::Terminal);
    assert_eq!(chase.radio.log.len(), before, "nothing is requested after the bust");
}

#[test]
fn a_forced_bailout_ends_with_a_high_intensity_bailout() {
    let mut chase = in_strategy(9);
    chase.notices.push(Notice::ForcedBailout);
    chase.run(1.0);
    let bail = chase.radio.first(Line::Bailout).expect("a bail-out line");
    assert_eq!(bail.params.bailout, Some(Bailout::HighIntensity));
    assert_eq!(chase.machine.focus(), Focus::Terminal);
}

#[test]
fn a_restart_resets_everything() {
    let mut chase = in_strategy(10);
    chase.notices.push(Notice::Restart);
    chase.snap.pursuit.exists = false;
    chase.snap.cops.clear();
    chase.run(1.0);
    assert_eq!(chase.machine.focus(), Focus::Pursuit);
    assert!(chase.machine.cops().is_empty());
}
