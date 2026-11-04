//! The weather edge of a chase: the line fires once on its rising edge, names its speech event, and a chase that opens
//! already raining says nothing; it is a pilot's line, so a chase with no helicopter flying says nothing either.
//! Synthetic feeds only.

use super::cue_line;
use super::cues::{Cue, Cues, RAIN_THRESHOLD};
use super::feed::HeliFeed;
use crate::audio::PursuitFeed;

fn feed(secs: f32, rain: f32) -> PursuitFeed {
    let mut feed = no_heli(secs, rain);
    feed.speech.heli = Some(HeliFeed { id: 99, active: true, ..HeliFeed::default() });
    feed
}

fn no_heli(secs: f32, rain: f32) -> PursuitFeed {
    PursuitFeed { heat: 1, intensity: 1.0, secs, secs_to_evade: 30.0, rain_intensity: rain, ..Default::default() }
}

fn play(cues: &mut Cues, frames: &[PursuitFeed]) -> Vec<Cue> {
    frames.iter().flat_map(|f| cues.step(Some(f))).collect()
}

#[test]
fn rain_rising_across_the_threshold_says_the_weather_once_per_rise() {
    let mut c = Cues::new();
    let wet = |s: f32| feed(s, RAIN_THRESHOLD + 0.2);
    let said = play(&mut c, &[feed(0.0, 0.0), wet(0.1), wet(0.2), wet(0.3)]);
    assert_eq!(said, vec![Cue::Rain], "one rise, one line");
    let second = play(&mut c, &[feed(0.4, 0.0), wet(0.5)]);
    assert_eq!(second, vec![Cue::Rain], "a fresh rise after it stopped says it again");
}

#[test]
fn a_chase_that_opens_in_the_rain_says_nothing_for_it() {
    let mut c = Cues::new();
    let said = play(&mut c, &[feed(0.0, 1.0), feed(0.1, 1.0)]);
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn a_chase_that_ends_forgets_the_rain() {
    let mut c = Cues::new();
    play(&mut c, &[feed(0.0, 0.0), feed(0.1, 0.0)]);
    assert_eq!(c.step(None), None);
    // The next chase opens in the rain: its first frame is a baseline, not a rising edge.
    let next = play(&mut c, &[feed(0.0, 1.0)]);
    assert!(next.is_empty(), "a stale dry from the old chase must not make a rise: {next:?}");
}

#[test]
fn the_weather_cue_names_its_speech_event() {
    assert_eq!(cue_line(Cue::Rain), "anytimeevents_weatherreport");
}

#[test]
fn the_weather_is_a_pilots_line_and_needs_a_flying_helicopter() {
    let mut c = Cues::new();
    let said = play(&mut c, &[no_heli(0.0, 0.0), no_heli(0.1, 1.0)]);
    assert!(said.is_empty(), "no helicopter, no weather report: {said:?}");
    let mut leaving = feed(0.3, 1.0);
    leaving.speech.heli = Some(HeliFeed { id: 99, active: false, ..HeliFeed::default() });
    let said = play(&mut c, &[no_heli(0.2, 0.0), leaving]);
    assert!(said.is_empty(), "a helicopter on its way out does not report: {said:?}");
    let said = play(&mut c, &[feed(0.4, 0.0), feed(0.5, 1.0)]);
    assert_eq!(said, vec![Cue::Rain]);
}
