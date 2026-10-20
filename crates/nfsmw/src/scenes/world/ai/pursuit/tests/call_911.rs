//! The free-roam 911 call (`docs/specs/ai-pursuit-heat.md` section 5.3): damage and hard traffic hits with no chase on
//! raise a call, which starts the player's 911 timer, lets the cops in, and lets a patrol that sees the player chase it
//! whatever its speed.

use super::free_roam::{patrol, renderer, see};
use super::*;
use crate::scenes::world::ai::cop_manager::roam::{FALLBACK_AFTER, Level, Tuning};
use crate::scenes::world::ai::traffic_events::{Sighting, starts_pursuit};

const MPH: f32 = 0.447_04;

fn world() -> TrafficWorld {
    let mut world = TrafficWorld::new(cop_road());
    let rows = vec![Level { cost: 400, hits: 3, lifetime: 45.0 }];
    world.cops.roam.set_data(Tuning::default(), rows);
    world
}

fn smash(world: &mut TrafficWorld, id: u32, cost: u32) {
    world.note_smacked(&[(id, cost)]);
}

#[test]
fn a_call_clears_the_session_lockout_and_a_slow_patrol_sighting_then_starts_a_chase() {
    let mut world = world();
    world.cops.start_session();
    assert!(!world.cops.spawning_enabled());
    let slow = |world: &TrafficWorld| {
        let sight = Sighting { player_speed: 5.0, ..world.sighting(1) };
        starts_pursuit(&world.events(), &sight)
    };
    world.seen_by_cops = true;
    assert!(!slow(&world), "a seen car at 5 m/s is no reason to chase");
    world.start_911_call(45.0);
    assert!(world.cops.spawning_enabled(), "Force911State clears the lockout");
    assert!(world.sighting(1).has_911);
    assert!(slow(&world), "a running 911 timer is reason enough");
}

#[test]
fn the_timer_runs_down_on_the_police_clock_and_the_call_ends() {
    let mut world = world();
    world.start_911_call(45.0);
    for _ in 0..(44 * 4) {
        see(&mut world, false);
    }
    assert!(world.sighting(1).has_911, "44 s of 45");
    for _ in 0..(2 * 4) {
        see(&mut world, false);
    }
    assert!(!world.sighting(1).has_911);
}

#[test]
fn the_damage_of_smashed_objects_raises_the_call_when_the_speech_does_not_answer() {
    let mut world = world();
    world.cops.start_session();
    for id in 1..=4 {
        smash(&mut world, id, 100);
        see(&mut world, false);
        assert!(!world.sighting(1).has_911, "{id} of 400");
    }
    assert!(!world.cops.spawning_enabled());
    for _ in 0..((FALLBACK_AFTER as u32 + 2) * 4) {
        see(&mut world, false);
    }
    assert!(world.sighting(1).has_911, "the call is made once the speech has had its time");
    assert!(world.cops.spawning_enabled());
}

#[test]
fn hard_traffic_hits_raise_the_call_and_light_ones_do_not() {
    let mut world = world();
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    see(&mut world, false);
    for _ in 0..5 {
        world.cops.roam.tick(1.5, false, 1);
        world.note_traffic_hits(&[(0, 12.0 * MPH)]);
    }
    assert!(!world.cops.roam.threshold_met(), "grazes do not count");
    for _ in 0..3 {
        world.cops.roam.tick(1.5, false, 1);
        world.note_traffic_hits(&[(0, 28.0 * MPH)]);
    }
    assert!(world.cops.roam.threshold_met(), "three hard hits");
}

#[test]
fn a_chase_clears_the_counters() {
    let mut world = world();
    see(&mut world, false);
    smash(&mut world, 1, 300);
    world.start_pursuit(1, vec![wave()], player());
    see(&mut world, true);
    world.end_pursuit(blackbox_pursuit::EndReason::None);
    see(&mut world, false);
    smash(&mut world, 2, 99);
    assert!(!world.cops.roam.threshold_met(), "the counters started again after the chase");
    assert!(world.call_911_status().contains("damage 99 of 400"), "{}", world.call_911_status());
}

#[test]
fn the_feed_carries_what_the_player_did_and_the_heat_row_once() {
    let mut world = world();
    see(&mut world, false);
    smash(&mut world, 1, 150);
    let tuning = world.roam_tuning();
    let feed = world.roam_feed(&tuning).expect("a free-roam frame");
    assert_eq!(feed.heat, 1);
    assert!(feed.armed);
    assert_eq!(feed.events.len(), 1);
    assert!(matches!(feed.events[0], crate::audio::RoamEvent::Smashed { cost: 150 }));
    assert!(world.roam_feed(&tuning).unwrap().events.is_empty(), "the events are told once");
}

#[test]
fn no_free_roam_frame_while_a_chase_runs() {
    let mut world = world();
    see(&mut world, false);
    world.start_pursuit(1, vec![wave()], player());
    let tuning = world.roam_tuning();
    assert!(world.roam_feed(&tuning).is_none());
}

#[test]
fn an_evade_restarts_the_speech_and_allows_a_second_call() {
    let mut world = world();
    see(&mut world, false);
    world.start_911_call(45.0);
    let tuning = world.roam_tuning();
    assert!(!world.roam_feed(&tuning).unwrap().restarted);
    world.start_pursuit(1, vec![wave()], player());
    world.end_pursuit(blackbox_pursuit::EndReason::Evaded);
    assert!(world.roam_feed(&tuning).unwrap().restarted, "the lockout restarts the speech");
    let before = world.sighting(1).has_911;
    world.start_911_call(180.0);
    assert!(before && world.sighting(1).has_911);
    assert!(world.call_911_status().contains("2 calls"), "{}", world.call_911_status());
}

#[test]
fn the_console_shows_the_call_with_no_chase() {
    let mut world = world();
    assert!(world.pursuit_status().contains("no 911 call"), "{}", world.pursuit_status());
    let line = world.dev_911_call();
    assert!(line.contains("911 call on"), "{line}");
    assert!(world.pursuit_status().contains("911 call on"));
}
