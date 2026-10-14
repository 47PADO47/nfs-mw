//! The cues of a chase: which edge asks for which line, once per edge. Synthetic feeds only.

use super::cue_line;
use super::cues::{Cue, Cues, HEAT_JUMP_AFTER_SECS, NO_CONTACT_SECS};
use crate::audio::PursuitFeed;
use crate::audio::speech::random::Rng;

const SEEN: f32 = 1.0;
const UNSEEN: f32 = 0.5;

fn feed(heat: u32, intensity: f32, secs: f32) -> PursuitFeed {
    PursuitFeed { heat, intensity, secs, secs_to_evade: 30.0, player_mph: 80.0 }
}

/// Run `frames` of one chase state, one frame per 0.1 s from `from`, collecting the cues.
fn run(cues: &mut Cues, heat: u32, intensity: f32, from: f32, frames: u32) -> Vec<(f32, Cue)> {
    (0..frames)
        .flat_map(|i| {
            let secs = from + i as f32 * 0.1;
            cues.step(Some(&feed(heat, intensity, secs))).into_iter().map(move |c| (secs, c))
        })
        .collect()
}

fn cues() -> Cues {
    Cues::new(Rng::new(7))
}

#[test]
fn a_long_unseen_spell_says_lost_once_after_the_no_contact_time() {
    let mut c = cues();
    run(&mut c, 1, SEEN, 0.0, 10);
    let fired = run(&mut c, 1, UNSEEN, 1.0, 60);
    assert_eq!(fired.len(), 1, "one lost cue, not one per frame: {fired:?}");
    assert_eq!(fired[0].1, Cue::Lost);
    let at = fired[0].0;
    assert!(at - 1.0 >= NO_CONTACT_SECS - 0.11, "said too early at {at}");
}

#[test]
fn a_short_unseen_spell_says_nothing() {
    let mut c = cues();
    run(&mut c, 1, SEEN, 0.0, 10);
    let mut all = run(&mut c, 1, UNSEEN, 1.0, 5);
    all.extend(run(&mut c, 1, SEEN, 1.5, 20));
    assert!(all.is_empty(), "a flicker of sight is not a lost visual: {all:?}");
}

#[test]
fn a_regained_sight_follows_a_lost_one_once() {
    let mut c = cues();
    run(&mut c, 1, SEEN, 0.0, 10);
    run(&mut c, 1, UNSEEN, 1.0, 40);
    let back = run(&mut c, 1, SEEN, 5.0, 30);
    assert_eq!(back.iter().map(|b| b.1).collect::<Vec<_>>(), vec![Cue::Regained]);
}

#[test]
fn no_regained_without_a_lost_before_it() {
    let mut c = cues();
    let first = run(&mut c, 1, UNSEEN, 0.0, 5);
    let seen = run(&mut c, 1, SEEN, 0.5, 20);
    assert!(first.is_empty() && seen.is_empty(), "the first sighting is not a regain: {first:?} {seen:?}");
}

#[test]
fn each_lost_and_regained_pair_fires_once_per_edge() {
    let mut c = cues();
    run(&mut c, 1, SEEN, 0.0, 10);
    let mut all = Vec::new();
    let mut t = 1.0;
    for _ in 0..3 {
        all.extend(run(&mut c, 1, UNSEEN, t, 30));
        t += 3.0;
        all.extend(run(&mut c, 1, SEEN, t, 20));
        t += 2.0;
    }
    let kinds: Vec<Cue> = all.iter().map(|a| a.1).collect();
    assert_eq!(kinds, vec![Cue::Lost, Cue::Regained, Cue::Lost, Cue::Regained, Cue::Lost, Cue::Regained]);
}

#[test]
fn a_heat_rise_before_the_jump_time_says_nothing() {
    let mut c = cues();
    run(&mut c, 1, SEEN, 0.0, 5);
    let early = c.step(Some(&feed(2, SEEN, HEAT_JUMP_AFTER_SECS - 1.0)));
    assert!(early.is_empty());
}

#[test]
fn a_heat_jump_is_asked_at_most_once_per_crossing() {
    // Over many seeds the coin flip passes about half the time, and staying at the new level never repeats it.
    let mut passed = 0;
    for seed in 1..=200u64 {
        let mut c = Cues::new(Rng::new(seed));
        run(&mut c, 1, SEEN, 0.0, 60);
        let mut jumps = 0;
        let frame = |c: &mut Cues, heat: u32, secs: f32| {
            c.step(Some(&feed(heat, SEEN, secs))).iter().filter(|x| **x == Cue::HeatJump).count()
        };
        jumps += frame(&mut c, 2, 6.0);
        jumps += frame(&mut c, 2, 6.1);
        jumps += frame(&mut c, 2, 6.2);
        assert!(jumps <= 1, "seed {seed} repeated the jump");
        passed += jumps;
    }
    assert!((60..=140).contains(&passed), "the coin is about even: {passed} of 200");
}

#[test]
fn a_heat_rise_across_several_levels_is_one_crossing() {
    // 1 to 4 crosses 2, 3 and 4 in one frame: at most one line.
    let mut c = cues();
    run(&mut c, 1, SEEN, 0.0, 60);
    let jumps = c.step(Some(&feed(4, SEEN, 7.0)));
    assert!(jumps.len() <= 1, "{jumps:?}");
}

#[test]
fn the_first_frame_of_a_chase_only_records_the_heat() {
    let mut c = cues();
    let first = c.step(Some(&feed(3, SEEN, 6.0)));
    assert!(first.is_empty(), "a chase that opens at heat 3 is not a jump: {first:?}");
}

#[test]
fn a_chase_that_ends_forgets_its_lost_sight() {
    let mut c = cues();
    run(&mut c, 1, SEEN, 0.0, 10);
    run(&mut c, 1, UNSEEN, 1.0, 30);
    assert!(c.step(None).is_empty());
    // A new chase: the sight is seen, so no regain is said for the old loss.
    let next = run(&mut c, 1, SEEN, 0.0, 20);
    assert!(next.is_empty(), "{next:?}");
}

#[test]
fn each_cue_names_its_speech_event() {
    assert_eq!(cue_line(Cue::Lost), "lostvisual");
    assert_eq!(cue_line(Cue::Regained), "regainvisual");
    assert_eq!(cue_line(Cue::HeatJump), "heatjump");
}
