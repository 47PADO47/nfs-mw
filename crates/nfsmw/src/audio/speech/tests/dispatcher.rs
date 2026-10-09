use nfsmw_data::speech::SpeechTune;

use super::{FakeVoice, event};
use crate::audio::speech::random::Rng;
use crate::audio::speech::rules::Context;
use crate::audio::speech::{Dispatcher, Outcome, Request};

const TICK: f32 = 0.1;

fn dispatcher(events: Vec<nfsmw_data::speech::SpeechEvent>) -> Dispatcher {
    Dispatcher::new(events, SpeechTune::default(), Rng::new(7))
}

fn run(d: &mut Dispatcher, voice: &mut FakeVoice, seconds: f32) {
    run_in(d, voice, seconds, Context::default());
}

fn run_in(d: &mut Dispatcher, voice: &mut FakeVoice, seconds: f32, context: Context) {
    for _ in 0..(seconds / TICK).round() as usize {
        d.update(TICK, context, voice);
    }
}

fn ask(d: &mut Dispatcher, id: u32) -> Outcome {
    d.request(id, Request::default())
}

#[test]
fn a_request_is_said_once_when_the_voice_is_free() {
    let mut d = dispatcher(vec![event(1, |_| {})]);
    let mut voice = FakeVoice::default();
    assert_eq!(ask(&mut d, 1), Outcome::Queued);
    run(&mut d, &mut voice, 1.0);
    assert_eq!(voice.plays, [(1, 0, false)]);
    assert_eq!(d.waiting(), 0);
}

#[test]
fn unknown_events_and_the_template_are_refused() {
    let mut d = dispatcher(vec![event(0, |_| {}), event(1, |_| {})]);
    assert_eq!(ask(&mut d, 9), Outcome::Unknown);
    assert_eq!(ask(&mut d, 0), Outcome::Unknown);
    assert!(d.find("EVENT1").is_some());
    assert_eq!(d.events().len(), 1);
}

#[test]
fn lines_never_overlap_and_go_by_priority_then_arrival() {
    let patient = |id, priority| event(id, |e| (e.priority, e.expiry) = (priority, 100.0));
    let mut d = dispatcher(vec![patient(1, 50), patient(2, 80), patient(3, 50), patient(4, 50)]);
    let mut voice = FakeVoice::default();
    for id in [1, 3, 2, 4] {
        ask(&mut d, id);
    }
    // The first to be said is the highest priority; the others wait for the voice to be free.
    run(&mut d, &mut voice, 1.0);
    assert_eq!(voice.ids(), [2]);
    for expected in [vec![2, 1], vec![2, 1, 3], vec![2, 1, 3, 4]] {
        voice.busy = false;
        run(&mut d, &mut voice, 3.0);
        assert_eq!(voice.ids(), expected);
        assert!(voice.plays.iter().all(|p| !p.2), "nothing was cut");
    }
}

#[test]
fn a_waiting_request_is_refreshed_not_doubled() {
    let mut d = dispatcher(vec![event(1, |e| e.init_delay = 5.0)]);
    assert_eq!(ask(&mut d, 1), Outcome::Queued);
    assert_eq!(ask(&mut d, 1), Outcome::Refreshed);
    assert_eq!(d.waiting(), 1);
}

#[test]
fn an_interrupt_cuts_a_line_that_may_be_interrupted() {
    let mut d = dispatcher(vec![
        event(1, |e| e.interruptable = true),
        event(2, |e| {
            e.interrupt = true;
            e.priority = 60;
        }),
    ]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 0.5);
    ask(&mut d, 2);
    run(&mut d, &mut voice, 0.5);
    assert_eq!(voice.plays, [(1, 0, false), (2, 0, true)]);
}

#[test]
fn an_interrupt_waits_for_a_line_that_may_not_be_interrupted() {
    let mut d = dispatcher(vec![
        event(1, |e| e.interruptable = false),
        event(2, |e| {
            e.interrupt = true;
            e.expiry = 100.0;
        }),
    ]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 0.5);
    ask(&mut d, 2);
    run(&mut d, &mut voice, 2.0);
    assert_eq!(voice.ids(), [1], "the interrupt is held back");
    voice.busy = false;
    run(&mut d, &mut voice, 0.5);
    // The line is over; an interrupt does not wait out the silence the first one enforces.
    assert_eq!(voice.plays, [(1, 0, false), (2, 0, false)]);
}

#[test]
fn a_stronger_interrupt_cuts_an_interrupt_that_may_not_be_cut() {
    let mut d = dispatcher(vec![
        event(1, |e| {
            e.interrupt = true;
            e.interruptable = false;
            e.priority = 70;
        }),
        event(2, |e| {
            e.interrupt = true;
            e.priority = 70;
        }),
        event(3, |e| {
            e.interrupt = true;
            e.interruptable = false;
            e.priority = 90;
        }),
    ]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 0.5);
    ask(&mut d, 2);
    run(&mut d, &mut voice, 0.5);
    assert_eq!(voice.ids(), [1], "equal priority does not cut");
    ask(&mut d, 3);
    run(&mut d, &mut voice, 0.5);
    assert_eq!(voice.plays, [(1, 0, false), (3, 0, true)]);
}

#[test]
fn a_finished_line_enforces_silence_for_the_normal_lines_behind_it() {
    let mut d = dispatcher(vec![event(1, |e| e.enforce_dead_air = 2.0), event(2, |_| {})]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 0.5);
    voice.busy = false;
    ask(&mut d, 2);
    run(&mut d, &mut voice, 1.5);
    assert_eq!(voice.ids(), [1], "still inside the two seconds of silence");
    run(&mut d, &mut voice, 1.0);
    assert_eq!(voice.ids(), [1, 2]);
}

#[test]
fn a_request_that_waits_too_long_is_dropped() {
    let mut d = dispatcher(vec![event(1, |e| e.expiry = 1.0), event(2, |e| e.expiry = 30.0)]);
    let mut voice = FakeVoice { busy: true, ..FakeVoice::default() };
    ask(&mut d, 1);
    ask(&mut d, 2);
    run(&mut d, &mut voice, 2.0);
    assert_eq!(d.waiting(), 1);
    voice.busy = false;
    run(&mut d, &mut voice, 1.0);
    assert_eq!(voice.ids(), [2]);
}

#[test]
fn the_initial_delay_holds_a_request_back_and_the_expiry_starts_after_it() {
    let mut d = dispatcher(vec![event(1, |e| {
        e.init_delay = 3.0;
        e.expiry = 1.0;
    })]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 2.5);
    assert!(voice.plays.is_empty());
    run(&mut d, &mut voice, 1.0);
    assert_eq!(voice.ids(), [1], "kept past its expiry because the clock started when the delay ended");
}

#[test]
fn an_event_is_not_repeated_inside_its_interval() {
    let mut d = dispatcher(vec![event(1, |e| {
        e.interval = 10.0;
        e.expiry = 100.0;
        e.enforce_dead_air = 0.0;
    })]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 0.5);
    voice.busy = false;
    run(&mut d, &mut voice, 0.5);
    assert_eq!(d.history().count(1), 1, "heard once the line ended");
    ask(&mut d, 1);
    run(&mut d, &mut voice, 5.0);
    assert_eq!(voice.ids(), [1]);
    run(&mut d, &mut voice, 6.0);
    assert_eq!(voice.ids(), [1, 1]);
}

#[test]
fn an_event_with_a_limit_is_dropped_once_it_is_over() {
    let mut d = dispatcher(vec![event(1, |e| {
        e.max_playback = 1;
        e.interval = 0.0;
        e.enforce_dead_air = 0.0;
        e.do_not_dropout = true;
    })]);
    let mut voice = FakeVoice::default();
    for _ in 0..4 {
        ask(&mut d, 1);
        run(&mut d, &mut voice, 0.3);
        voice.busy = false;
        run(&mut d, &mut voice, 0.3);
    }
    // The original compares with `>`: a limit of one lets the event through twice.
    assert_eq!(voice.ids(), [1, 1]);
    assert_eq!(d.waiting(), 0);
}

#[test]
fn a_line_counts_as_heard_after_three_seconds_or_when_it_ends() {
    let mut d = dispatcher(vec![event(1, |_| {}), event(2, |_| {})]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 2.0);
    assert_eq!(d.history().count(1), 0);
    run(&mut d, &mut voice, 1.5);
    assert_eq!(d.history().count(1), 1);
    voice.busy = false;
    run(&mut d, &mut voice, 0.5);
    assert_eq!(d.history().count(1), 1, "counted once");
    voice.busy = false;
    ask(&mut d, 2);
    run(&mut d, &mut voice, 3.0);
    voice.busy = false;
    run(&mut d, &mut voice, 0.5);
    assert_eq!(d.history().count(2), 1);
    assert_eq!(d.history().last_event(), Some(2));
}

#[test]
fn requesting_an_event_withdraws_the_ones_it_recalls() {
    let mut d = dispatcher(vec![event(1, |e| e.init_delay = 9.0), event(2, |e| e.recall = vec![1])]);
    ask(&mut d, 1);
    assert_eq!(d.waiting(), 1);
    ask(&mut d, 2);
    assert_eq!(d.waiting(), 1);
    let mut voice = FakeVoice::default();
    run(&mut d, &mut voice, 1.0);
    assert_eq!(voice.ids(), [2]);
}

#[test]
fn a_dependent_event_follows_the_one_it_depends_on() {
    let mut d = dispatcher(vec![
        event(1, |e| e.enforce_dead_air = 0.0),
        event(2, |e| {
            e.dep_follow = vec![1];
            e.interval = 0.0;
            e.expiry = 100.0;
            e.enforce_dead_air = 0.0;
        }),
        event(3, |e| e.enforce_dead_air = 0.0),
    ]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 2);
    run(&mut d, &mut voice, 1.0);
    assert!(voice.plays.is_empty(), "nothing said yet for it to follow");
    ask(&mut d, 1);
    run(&mut d, &mut voice, 0.5);
    voice.busy = false;
    run(&mut d, &mut voice, 0.5);
    voice.busy = false;
    run(&mut d, &mut voice, 0.5);
    assert_eq!(voice.ids(), [1, 2]);
    // Something else was said since: the dependent event is not due again.
    ask(&mut d, 3);
    run(&mut d, &mut voice, 0.5);
    voice.busy = false;
    run(&mut d, &mut voice, 0.5);
    ask(&mut d, 2);
    run(&mut d, &mut voice, 1.0);
    assert_eq!(voice.ids(), [1, 2, 3]);
}

#[test]
fn the_state_of_the_pursuit_gates_events() {
    let mut d = dispatcher(vec![event(1, |e| {
        e.min_heat = 3;
        e.min_player_speed = 40.0;
        e.expiry = 100.0;
    })]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run_in(&mut d, &mut voice, 1.0, Context { heat: 1, player_speed: 80.0 });
    run_in(&mut d, &mut voice, 1.0, Context { heat: 4, player_speed: 20.0 });
    assert!(voice.plays.is_empty());
    run_in(&mut d, &mut voice, 1.0, Context { heat: 4, player_speed: -80.0 });
    assert_eq!(voice.ids(), [1], "speed counts without its sign");
}

#[test]
fn the_dead_air_an_event_needs_is_counted_from_the_last_line() {
    let mut d = dispatcher(vec![
        event(1, |e| e.enforce_dead_air = 0.0),
        event(2, |e| {
            e.dead_air = 4.0;
            e.expiry = 100.0;
        }),
    ]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 0.5);
    voice.busy = false;
    ask(&mut d, 2);
    run(&mut d, &mut voice, 3.0);
    assert_eq!(voice.ids(), [1]);
    run(&mut d, &mut voice, 2.0);
    assert_eq!(voice.ids(), [1, 2]);
    // With nothing said before, there is plenty of dead air.
    let mut fresh = dispatcher(vec![event(2, |e| e.dead_air = 4.0)]);
    let mut other = FakeVoice::default();
    ask(&mut fresh, 2);
    run(&mut fresh, &mut other, 0.5);
    assert_eq!(other.ids(), [2]);
}

#[test]
fn an_event_without_a_recording_is_dropped_and_the_next_one_goes() {
    let mut d = dispatcher(vec![event(1, |e| e.priority = 90), event(2, |_| {})]);
    let mut voice = FakeVoice { mute: vec![1], ..FakeVoice::default() };
    ask(&mut d, 1);
    ask(&mut d, 2);
    run(&mut d, &mut voice, 1.0);
    assert_eq!(voice.ids(), [2]);
    assert_eq!(d.waiting(), 0);
}

#[test]
fn speech_that_is_off_takes_no_requests() {
    let mut d = dispatcher(vec![event(1, |_| {})]);
    d.set_enabled(false);
    assert_eq!(ask(&mut d, 1), Outcome::Muted);
    d.set_enabled(true);
    assert_eq!(ask(&mut d, 1), Outcome::Queued);
}

#[test]
fn requests_are_thinned_along_the_pursuit_ramp() {
    let tune = SpeechTune { dropoff_ramp: [30.0, 500.0] };
    let exempt = event(2, |e| (e.do_not_dropout, e.enforce_dead_air) = (true, 0.0));
    let mut d = Dispatcher::new(vec![event(1, |e| e.enforce_dead_air = 0.0), exempt], tune, Rng::new(3));
    let probability = |d: &mut Dispatcher, secs| {
        d.set_pursuit_secs(secs);
        d.keep_probability()
    };
    assert_eq!(probability(&mut d, 0.0), 1.0);
    assert_eq!(probability(&mut d, 30.0), 1.0);
    assert!((probability(&mut d, 265.0) - 0.5).abs() < 1e-6);
    assert_eq!(probability(&mut d, 500.0), 0.0);
    assert_eq!(probability(&mut d, 900.0), 0.0);

    // At the end of the ramp a repeated request is always thinned out, unless the event is exempt. The first
    // request of an event is never thinned.
    let mut voice = FakeVoice::default();
    let mut said = |d: &mut Dispatcher, id| {
        d.request(id, Request::default());
        run(d, &mut voice, 0.5);
        voice.busy = false;
        run(d, &mut voice, 0.5);
    };
    d.set_pursuit_secs(900.0);
    said(&mut d, 1);
    said(&mut d, 2);
    assert_eq!(ask(&mut d, 1), Outcome::Thinned);
    assert_eq!(ask(&mut d, 2), Outcome::Queued);
}

#[test]
fn reset_forgets_the_queue_and_the_history() {
    let mut d = dispatcher(vec![event(1, |_| {}), event(2, |e| e.init_delay = 9.0)]);
    let mut voice = FakeVoice::default();
    ask(&mut d, 1);
    run(&mut d, &mut voice, 4.0);
    ask(&mut d, 2);
    d.reset();
    assert_eq!((d.waiting(), d.history().count(1)), (0, 0));
}

#[test]
fn the_pursuit_ramp_without_a_tune_never_thins() {
    let d = dispatcher(vec![]);
    assert_eq!(d.keep_probability(), 1.0);
}
