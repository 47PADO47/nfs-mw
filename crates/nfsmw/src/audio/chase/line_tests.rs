//! The air-support edges of a chase: each new line fires once on its edge, names its speech event, and a chase that
//! opens with the state already set says nothing. Synthetic feeds only.

use super::cue_line;
use super::cues::{Cue, Cues, RAIN_THRESHOLD};
use crate::audio::PursuitFeed;
use crate::audio::speech::random::Rng;

const SEEN: f32 = 1.0;

/// A seen chase at `secs` with the air-support fields the test sets.
fn feed(secs: f32, edit: impl FnOnce(&mut PursuitFeed)) -> PursuitFeed {
    let mut f = PursuitFeed { heat: 1, intensity: SEEN, secs, secs_to_evade: 30.0, ..Default::default() };
    edit(&mut f);
    f
}

/// Steps the chase through `frames` (each its feed) and collects every cue said.
fn play(cues: &mut Cues, frames: Vec<PursuitFeed>) -> Vec<Cue> {
    frames.iter().flat_map(|f| cues.step(Some(f))).collect()
}

fn cues() -> Cues {
    Cues::new(Rng::new(7))
}

#[test]
fn rain_rising_across_the_threshold_says_the_weather_once_per_rise() {
    let mut c = cues();
    let dry = feed(0.0, |f| f.rain_intensity = 0.0);
    let wet = |s: f32| feed(s, |f| f.rain_intensity = RAIN_THRESHOLD + 0.2);
    let said = play(&mut c, vec![dry, wet(0.1), wet(0.2), wet(0.3)]);
    assert_eq!(said, vec![Cue::Rain], "one rise, one line");
    let dry_again = feed(0.4, |f| f.rain_intensity = 0.0);
    let second = play(&mut c, vec![dry_again, wet(0.5)]);
    assert_eq!(second, vec![Cue::Rain], "a fresh rise after it stopped says it again");
}

#[test]
fn a_chase_that_opens_in_the_rain_says_nothing_for_it() {
    let mut c = cues();
    let said = play(&mut c, vec![feed(0.0, |f| f.rain_intensity = 1.0), feed(0.1, |f| f.rain_intensity = 1.0)]);
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn a_roadblock_appearing_says_the_call_once() {
    let mut c = cues();
    let said = play(
        &mut c,
        vec![feed(0.0, |_| {}), feed(0.1, |f| f.roadblock_active = true), feed(0.2, |f| f.roadblock_active = true)],
    );
    assert_eq!(said, vec![Cue::RoadblockCalled]);
}

#[test]
fn a_hit_on_traffic_is_said_on_its_rising_edge_only() {
    let mut c = cues();
    let said = play(
        &mut c,
        vec![
            feed(0.0, |_| {}),
            feed(0.1, |f| f.player_hit = true),
            feed(0.2, |f| f.player_hit = true),
            feed(0.3, |_| {}),
            feed(0.4, |f| f.player_hit = true),
        ],
    );
    assert_eq!(said, vec![Cue::BeenHit, Cue::BeenHit], "two hits, two lines; a held hit is one");
}

#[test]
fn a_hit_while_the_chase_opens_is_not_said() {
    let mut c = cues();
    let said = play(&mut c, vec![feed(0.0, |f| f.player_hit = true), feed(0.1, |f| f.player_hit = true)]);
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn a_cop_wrecked_says_the_kill_once_per_increase() {
    let mut c = cues();
    let said = play(
        &mut c,
        vec![
            feed(0.0, |f| f.cops_destroyed = 2),
            feed(0.1, |f| f.cops_destroyed = 2),
            feed(0.2, |f| f.cops_destroyed = 3),
            feed(0.3, |f| f.cops_destroyed = 3),
            feed(0.4, |f| f.cops_destroyed = 4),
        ],
    );
    assert_eq!(said, vec![Cue::CopDown, Cue::CopDown], "the baseline of 2 is not a kill");
}

#[test]
fn the_helicopter_lost_and_spotted_follow_the_sight_edges_only_with_a_heli() {
    let mut c = cues();
    // Seen, then unseen long enough for Lost, then seen again: Regained.
    let mut frames = vec![feed(0.0, |f| f.heli_present = true)];
    for i in 1..=30 {
        frames.push(feed(i as f32 * 0.1, |f| {
            f.heli_present = true;
            f.intensity = 0.5;
        }));
    }
    frames.push(feed(3.1, |f| f.heli_present = true));
    let said = play(&mut c, frames);
    assert_eq!(said, vec![Cue::Lost, Cue::HeliLost, Cue::Regained, Cue::HeliSpotted]);
}

#[test]
fn no_helicopter_no_heli_line_but_the_ground_sight_still_says_its_own() {
    let mut c = cues();
    let mut frames = vec![feed(0.0, |_| {})];
    for i in 1..=30 {
        frames.push(feed(i as f32 * 0.1, |f| f.intensity = 0.5));
    }
    frames.push(feed(3.1, |_| {}));
    let said = play(&mut c, frames);
    assert_eq!(said, vec![Cue::Lost, Cue::Regained]);
}

#[test]
fn a_chase_that_ends_forgets_the_air_edges() {
    let mut c = cues();
    // The first chase ends with the roadblock and the hit both low.
    play(&mut c, vec![feed(0.0, |_| {}), feed(0.1, |_| {})]);
    assert!(c.step(None).is_empty());
    // The next chase opens with both already high: its first frame is a baseline, not a rising edge.
    let next = play(
        &mut c,
        vec![feed(0.0, |f| {
            f.roadblock_active = true;
            f.player_hit = true;
        })],
    );
    assert!(next.is_empty(), "a stale low from the old chase must not make a rise: {next:?}");
}

#[test]
fn each_air_cue_names_its_speech_event() {
    assert_eq!(cue_line(Cue::Rain), "anytimeevents_weatherreport");
    assert_eq!(cue_line(Cue::RoadblockCalled), "staticroadblock_callforrb");
    assert_eq!(cue_line(Cue::BeenHit), "anytimeevents_collisionworld");
    assert_eq!(cue_line(Cue::HeliLost), "helispecific_helilostvisual");
    assert_eq!(cue_line(Cue::HeliSpotted), "anytimeevents_spotted");
    assert_eq!(cue_line(Cue::CopDown), "anytimeevents_unitdisabled");
}
