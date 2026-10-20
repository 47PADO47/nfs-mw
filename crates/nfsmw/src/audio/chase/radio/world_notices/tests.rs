use super::*;
use crate::audio::chase::feed::CopFeed;
use blackbox_pursuit::speech::{Backup, Impact, RoadblockHit};
use nfsmw_data::speech_chase::ChaseTune;

const MPH: f32 = 0.447_04;

fn tune() -> Tune {
    super::super::tune_of(&ChaseTune::default())
}

fn crash(delta_mph: f32, closing: f32, before: f32, after: f32) -> Crash {
    Crash { delta_v: delta_mph * MPH, closing, speed_before: before, speed_after: after }
}

fn run(events: &[WorldEvent], speech: &SpeechFeed) -> Vec<Notice> {
    let mut log = EventLog::default();
    for &event in events {
        log.push(event);
    }
    let mut seen = 0;
    notices(&mut seen, &log, &tune(), speech)
}

#[test]
fn a_wall_that_took_most_of_the_speed_is_a_crash_of_the_strength_it_lost() {
    let out = run(&[WorldEvent::PlayerHitWorld(crash(20.0, 20.0, 30.0, 5.0))], &SpeechFeed::default());
    let [Notice::PlayerHitWorld { intensity, crash }] = out[..] else { panic!("{out:?}") };
    assert!(crash);
    assert!((intensity - 0.5).abs() < 0.01, "{intensity}: 20 mph lost is halfway up the 10 to 30 range");
}

#[test]
fn a_scrape_is_a_hit_but_no_crash() {
    let out = run(&[WorldEvent::PlayerHitWorld(crash(15.0, 8.0, 30.0, 24.0))], &SpeechFeed::default());
    assert!(matches!(out[..], [Notice::PlayerHitWorld { crash: false, .. }]), "{out:?}");
}

#[test]
fn a_hit_that_closes_too_slowly_is_not_heard() {
    let events = [
        WorldEvent::PlayerHitWorld(crash(20.0, 3.0, 4.0, 0.0)),
        WorldEvent::PlayerHitTraffic { crash: crash(20.0, 3.0, 4.0, 0.0), semi: false },
        WorldEvent::CopHitTraffic { cop: 7, closing: 3.0 },
    ];
    assert!(run(&events, &SpeechFeed::default()).is_empty());
}

#[test]
fn a_crash_into_traffic_marks_the_crash_time_and_reports_the_hit() {
    let events = [WorldEvent::PlayerHitTraffic { crash: crash(25.0, 20.0, 30.0, 5.0), semi: true }];
    let out = run(&events, &SpeechFeed::default());
    assert!(matches!(out[0], Notice::Crashed));
    assert!(matches!(out[1], Notice::PlayerHitTraffic { semi: true, .. }));
}

#[test]
fn a_prop_only_marks_the_crash_time_when_it_stops_the_car() {
    let hard = WorldEvent::PlayerHitProp(crash(20.0, 20.0, 30.0, 5.0));
    let soft = WorldEvent::PlayerHitProp(crash(20.0, 20.0, 30.0, 25.0));
    assert_eq!(run(&[hard], &SpeechFeed::default()), vec![Notice::Crashed]);
    assert!(run(&[soft], &SpeechFeed::default()).is_empty());
}

#[test]
fn a_ram_carries_its_contact_and_whether_it_was_deliberate() {
    let event = WorldEvent::PlayerHitCop {
        cop: 12,
        crash: crash(30.0, 15.0, 30.0, 25.0),
        impact: Impact::TBone,
        deliberate: false,
    };
    let out = run(&[event], &SpeechFeed::default());
    let [Notice::PlayerHitCop { cop: 12, impact: Impact::TBone, intensity, deliberate: false }] = out[..] else {
        panic!("{out:?}")
    };
    assert_eq!(intensity, 1.0);
}

#[test]
fn a_cop_hitting_a_wall_is_seen_only_when_it_is_in_view() {
    let mut speech = SpeechFeed::default();
    speech.push_cop(CopFeed { id: 4, in_view: true, ..CopFeed::default() });
    speech.push_cop(CopFeed { id: 5, in_view: false, ..CopFeed::default() });
    let events = [
        WorldEvent::CopHitWorld { cop: 4, delta_v: 28.0 * MPH, closing: 14.0 },
        WorldEvent::CopHitWorld { cop: 5, delta_v: 28.0 * MPH, closing: 14.0 },
    ];
    let out = run(&events, &speech);
    assert!(matches!(out[0], Notice::CopHitWorld { cop: 4, visible: true, .. }), "{out:?}");
    assert!(matches!(out[1], Notice::CopHitWorld { cop: 5, visible: false, .. }), "{out:?}");
}

#[test]
fn the_messages_pass_straight_through() {
    let events = [
        WorldEvent::Airborne { altitude: 2.0, secs: 1.5 },
        WorldEvent::BlewByCop { cop: 3 },
        WorldEvent::Tunnel { inside: true },
        WorldEvent::GasStation { by_player: true, at: [1.0, 2.0, 3.0] },
        WorldEvent::RoadblockBuilt { spike_offset: Some(-2) },
        WorldEvent::RoadblockBuilt { spike_offset: None },
        WorldEvent::RoadblockDodged,
        WorldEvent::RoadblockHit(RoadblockHit::Vehicle),
        WorldEvent::TireBlown,
        WorldEvent::BackupRequested(Backup::Rhino),
        WorldEvent::BackupDenied(Backup::SuperPursuit),
    ];
    let out = run(&events, &SpeechFeed::default());
    assert_eq!(out.len(), 10, "a roadblock without a spike strip sends no position: {out:?}");
    assert!(out.contains(&Notice::RoadblockPosition(-2)));
    assert!(out.contains(&Notice::BackupDenied(Backup::SuperPursuit)));
    assert!(out.contains(&Notice::GasStationExploded { by_player: true, at: Vec3::new(1.0, 2.0, 3.0) }));
}

#[test]
fn an_event_is_taken_once() {
    let mut log = EventLog::default();
    log.push(WorldEvent::RoadblockDodged);
    let mut seen = 0;
    let speech = SpeechFeed::default();
    assert_eq!(notices(&mut seen, &log, &tune(), &speech).len(), 1);
    assert!(notices(&mut seen, &log, &tune(), &speech).is_empty());
    log.push(WorldEvent::TireBlown);
    assert_eq!(notices(&mut seen, &log, &tune(), &speech), vec![Notice::TireBlown]);
}
