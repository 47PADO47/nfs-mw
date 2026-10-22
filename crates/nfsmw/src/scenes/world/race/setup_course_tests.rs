//! The course a stored race makes: the gates it aims at are its `Checkpoint` list, then its finish line, and nothing else
//! (`docs/specs/race-goals.md` section 2).

use blackbox_race::{Race as Engine, RaceConfig, RaceEvent};
use glam::Vec3;
use nfsmw_data::gameplay::{BoxTrigger, GameplayData, Placement, Race, RaceParams, RaceType};

use super::setup::{RaceOptions, course_of};

fn trigger(key: u32, z: f32) -> BoxTrigger {
    BoxTrigger {
        key,
        name: format!("t{key}"),
        position: Vec3::new(0.0, 0.0, z),
        heading_deg: 0.0,
        dimensions: [2.0, 40.0, 40.0],
        one_shot: false,
        fire_on_exit: false,
        particle_effect: None,
        flare_spacing: None,
        time_bonus: 0.0,
        resets_player: false,
    }
}

/// A sprint with two listed checkpoints (11 at 100 m, 12 at 200 m), a finish line (10 at 500 m) and a trigger the list
/// leaves out (13 at 300 m), like `checkpoint7` of `16_2_2_sprint` and `ai_cp5` of `16_1_0_partial_dday`.
fn sprint() -> (GameplayData, Race) {
    let data = GameplayData {
        triggers: vec![trigger(10, 500.0), trigger(11, 100.0), trigger(12, 200.0), trigger(13, 300.0)],
        ..Default::default()
    };
    let race = Race {
        key: 1,
        name: "race_bin_16/16_2_2_sprint".into(),
        event: None,
        event_icon: "p2p".into(),
        race_type: Some(RaceType::Sprint),
        region: None,
        laps: 1,
        start: Some(Placement { position: Vec3::ZERO, heading_deg: 0.0 }),
        finish: Some(10),
        checkpoints: vec![11, 12],
        barriers: Vec::new(),
        params: RaceParams::default(),
        story: Default::default(),
    };
    (data, race)
}

#[test]
fn a_trigger_the_checkpoint_list_leaves_out_is_not_a_goal() {
    let (data, race) = sprint();
    let course = course_of(&data, &race, &RaceOptions::default()).unwrap();
    let along: Vec<f32> = course.checkpoints.iter().map(|gate| gate.centre.z).collect();
    assert_eq!(along, [100.0, 200.0], "the unlisted trigger at 300 m is no checkpoint");
    assert_eq!(course.finish.unwrap().centre.z, 500.0, "the finish line ends the course");
}

#[test]
fn driving_through_the_unlisted_trigger_and_the_last_checkpoint_leaves_the_race_running_until_the_finish() {
    let (data, race) = sprint();
    let course = course_of(&data, &race, &RaceOptions::default()).unwrap();
    let mut engine = Engine::new(course, RaceConfig::default(), &[(true, Vec3::ZERO), (false, Vec3::ZERO)]);
    engine.start_now();
    let mut events = Vec::new();
    for z in 0..=450 {
        events.extend(engine.move_racer(0, Vec3::new(0.0, 0.0, z as f32)));
    }
    let checkpoints = events.iter().filter(|e| matches!(e, RaceEvent::Checkpoint { .. })).count();
    assert_eq!(checkpoints, 2, "the trigger at 300 m raises nothing");
    assert!(!events.iter().any(|e| matches!(e, RaceEvent::Finished { .. } | RaceEvent::Over)), "{events:?}");
    for z in 451..=550 {
        events.extend(engine.move_racer(0, Vec3::new(0.0, 0.0, z as f32)));
    }
    assert!(events.iter().any(|e| matches!(e, RaceEvent::Finished { racer: 0, .. })));
    assert_eq!(events.last(), Some(&RaceEvent::Over));
}
