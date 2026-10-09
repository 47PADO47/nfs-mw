//! `Speech` over fake recordings and a fake sound output.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ea_audio::Pcm;
use nfsmw_data::speech::SpeechTune;

use super::event;
use crate::audio::speech::random::Rng;
use crate::audio::speech::{Lines, Outcome, Sink, Speech};

/// What the fake output was asked to do.
#[derive(Default)]
struct Log {
    /// The first sample of each line started (a marker for which line it was).
    started: Vec<i16>,
    stops: u32,
    busy: bool,
    volume: f32,
}

struct FakeSink(Rc<RefCell<Log>>);

impl Sink for FakeSink {
    fn busy(&self) -> bool {
        self.0.borrow().busy
    }

    fn play(&mut self, pcm: &Pcm) -> Result<(), String> {
        let mut log = self.0.borrow_mut();
        log.started.push(pcm.samples[0]);
        log.busy = true;
        Ok(())
    }

    fn stop(&mut self) {
        let mut log = self.0.borrow_mut();
        log.stops += 1;
        log.busy = false;
    }

    fn set_volume(&mut self, amplitude: f32) {
        self.0.borrow_mut().volume = amplitude;
    }
}

/// Recordings by event: the marker sample of the line.
struct FakeLines(HashMap<u32, i16>);

impl Lines for FakeLines {
    fn has(&self, event: u32) -> bool {
        self.0.contains_key(&event)
    }

    fn take(&mut self, event: u32, _speaker: u16) -> Result<Pcm, String> {
        let marker = self.0.get(&event).ok_or("no recording")?;
        Ok(Pcm { sample_rate: 1000, channels: 1, samples: vec![*marker; 500], loop_range: None })
    }

    fn bank_take(&self, bank: usize, take: usize) -> Result<Pcm, String> {
        Ok(Pcm { sample_rate: 1000, channels: 1, samples: vec![(bank * 10 + take) as i16; 250], loop_range: None })
    }

    fn summary(&self) -> String {
        "fake".into()
    }
}

fn speech() -> (Speech, Rc<RefCell<Log>>) {
    let log = Rc::new(RefCell::new(Log::default()));
    let events = vec![
        event(1, |e| e.enforce_dead_air = 0.0),
        event(2, |e| {
            e.interrupt = true;
            e.enforce_dead_air = 0.0;
        }),
        event(3, |_| {}),
    ];
    let lines = FakeLines(HashMap::from([(1, 11), (2, 22)]));
    let speech =
        Speech::new(events, SpeechTune::default(), Box::new(lines), Box::new(FakeSink(log.clone())), Rng::new(1));
    (speech, log)
}

fn run(speech: &mut Speech, seconds: f32) {
    for _ in 0..(seconds * 10.0).round() as usize {
        speech.update(0.1);
    }
}

#[test]
fn a_request_is_said_through_the_sink() {
    let (mut speech, log) = speech();
    assert_eq!(speech.request(1, 0), Outcome::Queued);
    run(&mut speech, 0.5);
    assert_eq!(log.borrow().started, [11]);
    assert_eq!(speech.said(), 1);
}

#[test]
fn an_interrupt_stops_the_line_in_progress_first() {
    let (mut speech, log) = speech();
    speech.request(1, 0);
    run(&mut speech, 0.5);
    speech.request(2, 0);
    run(&mut speech, 0.5);
    assert_eq!(log.borrow().started, [11, 22]);
    assert_eq!(log.borrow().stops, 1);
}

#[test]
fn an_event_without_recordings_is_dropped_and_the_reason_kept() {
    let (mut speech, log) = speech();
    speech.request(3, 0);
    run(&mut speech, 0.5);
    assert!(log.borrow().started.is_empty());
    assert_eq!(speech.dispatcher().waiting(), 0);
    assert_eq!(speech.last_error(), Some("event3: no recording"));
    assert_eq!(speech.said(), 0);
}

#[test]
fn the_volume_reaches_the_output_and_zero_refuses_requests() {
    let (mut speech, log) = speech();
    speech.set_volume(0.5);
    assert_eq!(log.borrow().volume, 0.5);
    assert_eq!(speech.request(1, 0), Outcome::Queued);
    speech.silence();
    assert_eq!(speech.dispatcher().waiting(), 0);
    assert_eq!(log.borrow().stops, 1);
    speech.set_volume(0.0);
    assert_eq!(speech.request(1, 0), Outcome::Muted);
    speech.set_volume(0.7);
    assert_eq!(speech.request(1, 0), Outcome::Queued);
}

#[test]
fn a_line_can_be_played_by_hand_over_the_one_in_progress() {
    let (mut speech, log) = speech();
    speech.request(1, 0);
    run(&mut speech, 0.5);
    let secs = speech.play_now(2, 0).unwrap();
    assert!((secs - 0.5).abs() < 1e-6);
    assert_eq!(log.borrow().started, [11, 22]);
    assert!(speech.play_now(3, 0).is_err());
    let secs = speech.play_take(4, 2).unwrap();
    assert!((secs - 0.25).abs() < 1e-6);
    assert_eq!(log.borrow().started.last(), Some(&42));
}
