use super::event;
use crate::audio::speech::history::History;
use crate::audio::speech::rules::{Actor, Asked, Context, Env, Verdict, judge};

fn env(history: &History, now: f32) -> Env<'_> {
    Env { now, dead_air: 99.0, history, playing: None, context: Context::default() }
}

fn asked(entry: f32) -> Asked {
    Asked { entry, actor: None }
}

fn near(distance: f32) -> Option<Actor> {
    Some(Actor { distance, on_screen: true, line_of_sight: true })
}

#[test]
fn a_fresh_request_of_a_plain_event_is_kept() {
    let history = History::default();
    assert_eq!(judge(&event(1, |_| {}), &asked(0.0), &env(&history, 0.0)), Verdict::Keep);
}

#[test]
fn a_request_older_than_its_expiry_is_ditched() {
    let history = History::default();
    let e = event(1, |e| e.expiry = 5.0);
    assert_eq!(judge(&e, &asked(0.0), &env(&history, 5.0)), Verdict::Keep);
    assert_eq!(judge(&e, &asked(0.0), &env(&history, 5.1)), Verdict::Ditch);
}

#[test]
fn a_unit_too_far_away_or_out_of_sight_has_to_wait() {
    let history = History::default();
    let e = event(1, |e| {
        e.culling_range = 100.0;
        e.on_screen_only = true;
        e.require_line_of_sight = true;
    });
    let at = |actor| Asked { entry: 0.0, actor };
    assert_eq!(judge(&e, &at(near(50.0)), &env(&history, 0.0)), Verdict::Keep);
    assert_eq!(judge(&e, &at(near(150.0)), &env(&history, 0.0)), Verdict::Defer);
    let hidden = Some(Actor { distance: 50.0, on_screen: false, line_of_sight: true });
    assert_eq!(judge(&e, &at(hidden), &env(&history, 0.0)), Verdict::Defer);
    let blocked = Some(Actor { distance: 50.0, on_screen: true, line_of_sight: false });
    assert_eq!(judge(&e, &at(blocked), &env(&history, 0.0)), Verdict::Defer);
    // A request without a unit skips these checks.
    assert_eq!(judge(&e, &asked(0.0), &env(&history, 0.0)), Verdict::Keep);
}

#[test]
fn a_timed_dependency_looks_back_that_many_seconds() {
    let mut history = History::default();
    history.touch(7, 10.0);
    history.touch(8, 12.0);
    let e = event(1, |e| {
        e.dep_follow = vec![7];
        e.back_time = 5.0;
        e.expiry = 100.0;
    });
    assert_eq!(judge(&e, &asked(0.0), &env(&history, 14.0)), Verdict::Keep);
    assert_eq!(judge(&e, &asked(0.0), &env(&history, 16.0)), Verdict::Defer);
}

#[test]
fn a_dependency_is_met_by_the_line_being_said() {
    // Something else was said before: the rule looks at the last event said, or at the one being said.
    let mut history = History::default();
    history.touch(5, 0.0);
    let e = event(1, |e| e.dep_follow = vec![7]);
    let mut playing = env(&history, 0.0);
    assert_eq!(judge(&e, &asked(0.0), &playing), Verdict::Defer);
    playing.playing = Some(7);
    assert_eq!(judge(&e, &asked(0.0), &playing), Verdict::Keep);
}

#[test]
fn the_history_remembers_the_last_ten_events() {
    let mut history = History::default();
    for id in 1..=12 {
        history.touch(id, id as f32);
    }
    assert_eq!(history.last_event(), Some(12));
    assert_eq!((history.count(3), history.last_time(3)), (1, Some(3.0)));
    history.touch(3, 20.0);
    assert_eq!((history.count(3), history.last_time(3)), (2, Some(20.0)));
    history.clear();
    assert_eq!((history.count(3), history.last_event()), (0, None));
}
