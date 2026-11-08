use super::*;
use crate::scenes::world::ai::trace_frame::{HeliFrame, StripFrame};

fn cop(key: u32, kind: &'static str) -> CopFrame {
    CopFrame {
        key,
        render_key: key + 100,
        model: "copmidsize".into(),
        kind,
        position: [key as f32, 0.0, 0.0],
        velocity: [0.0; 3],
        forward: [0.0, 0.0, 1.0],
        speed: 0.0,
        desired_speed: 0.0,
        drive_target: [0.0; 3],
        goal: None,
        steer: 0.0,
        gas: 0.0,
        brake: 0.0,
        nos: false,
        searching: false,
        fleeing: false,
        engaged: false,
        sees_player: false,
        wrecked: false,
        damage: 0.0,
        offscreen_secs: 0.0,
        roadblock: None,
        support: None,
        action: None,
        speed_limit: None,
        reverse_gear: false,
        goal_slot: None,
        goal_arrived: false,
    }
}

fn kinds(found: &[Found]) -> Vec<&'static str> {
    found.iter().map(|f| f.kind).collect()
}

#[test]
fn a_chase_start_spawns_and_joins_and_its_end_removes_the_cops() {
    let mut tracker = Tracker::default();
    let patrol = PoliceFrame { cops: vec![cop(1, "patrol")], ..Default::default() };
    assert_eq!(kinds(&tracker.events(&patrol, None)), ["spawn"]);
    let chase = PoliceFrame {
        pursuing: true,
        heat_level: Some(2),
        cops: vec![cop(1, "chase"), cop(2, "chase")],
        ..Default::default()
    };
    let started = tracker.events(&chase, Some([0.0; 3]));
    assert_eq!(kinds(&started), ["pursuit_start", "pursuit_join", "spawn", "pursuit_join"]);
    assert_eq!(tracker.pursuit_id(&chase).as_deref(), Some("remake-pursuit-1"));
    let raised = PoliceFrame { heat_level: Some(3), ..chase.clone() };
    assert_eq!(kinds(&tracker.events(&raised, Some([0.0; 3]))), ["heat_change"]);
    let ended = PoliceFrame { end_reason: "Evaded".into(), ..Default::default() };
    let end = tracker.events(&ended, Some([0.0; 3]));
    assert_eq!(kinds(&end), ["pursuit_leave", "unspawn", "pursuit_leave", "unspawn", "pursuit_evaded", "pursuit_end"]);
    assert_eq!(end[1].detail["reason"], "pursuit_end");
    assert_eq!(end[1].detail["distance_to_player_m"], 1.0);
}

#[test]
fn removal_reasons_come_from_the_last_snapshot() {
    let quiet = PoliceFrame::default();
    let wrecked = CopFrame { wrecked: true, ..cop(1, "chase") };
    assert_eq!(retire_reason(&wrecked, &quiet, &quiet, false), "wrecked");
    let unseen = CopFrame { offscreen_secs: 9.0, ..cop(1, "chase") };
    assert_eq!(retire_reason(&unseen, &quiet, &quiet, false), "offscreen");
    assert_eq!(retire_reason(&cop(1, "chase"), &quiet, &quiet, true), "roadblock_takeover");
    let locked = PoliceFrame { lockout_secs: 30.0, ..Default::default() };
    assert_eq!(retire_reason(&cop(1, "chase"), &quiet, &locked, false), "lockout");
    assert_eq!(retire_reason(&cop(1, "chase"), &quiet, &quiet, false), "unknown");
}

#[test]
fn roadblocks_and_the_helicopter_come_and_go_with_their_poses() {
    let mut tracker = Tracker::default();
    let car = CopFrame { roadblock: Some(4), ..cop(7, "roadblock") };
    let strip = StripFrame { roadblock: 4, index: 0, position: [1.0, 2.0, 3.0], axis: [1.0, 0.0, 0.0] };
    let heli = HeliFrame {
        position: [0.0, 40.0, 0.0],
        forward: [0.0, 0.0, 1.0],
        velocity: [0.0; 3],
        mode: "Following".into(),
        in_sight: true,
        fuel_secs: 90.0,
    };
    let up = PoliceFrame {
        pursuing: true,
        cops: vec![car],
        strips: vec![strip],
        heli: Some(heli),
        speech: vec!["BackupRequested".into()],
        ..Default::default()
    };
    let found = tracker.events(&up, None);
    assert!(kinds(&found).starts_with(&["pursuit_start", "roadblock_create"]));
    let block = found.iter().find(|f| f.kind == "roadblock_create").unwrap();
    assert_eq!(block.detail["cars"][0]["id"], "remake-7:1");
    assert_eq!(block.detail["spike_strips"][0]["position"][2], 3.0);
    assert!(kinds(&found).contains(&"heli_spawn"));
    assert!(kinds(&found).contains(&"backup_request"));
    let down = PoliceFrame { pursuing: true, ..Default::default() };
    let gone = kinds(&tracker.events(&down, None));
    assert!(gone.contains(&"roadblock_remove") && gone.contains(&"heli_despawn") && gone.contains(&"unspawn"));
}

#[test]
fn a_roadblock_car_is_no_chase_member_until_the_chase_recruits_it() {
    let mut tracker = Tracker::default();
    let car = CopFrame { roadblock: Some(4), ..cop(7, "roadblock") };
    let standing = PoliceFrame { pursuing: true, heat_level: Some(5), cops: vec![car.clone()], ..Default::default() };
    let found = tracker.events(&standing, Some([0.0; 3]));
    assert!(!kinds(&found).contains(&"pursuit_join"), "a roadblock car only spawns: {:?}", kinds(&found));
    assert_eq!(super::super::records::cop(&car, in_pursuit(&car, &standing), None).role(), "roadblock_cop");
    let recruited = PoliceFrame { cops: vec![CopFrame { roadblock: None, ..cop(7, "chase") }], ..standing };
    let joined = kinds(&tracker.events(&recruited, Some([0.0; 3])));
    assert_eq!(joined.iter().filter(|k| **k == "pursuit_join").count(), 1, "the recruit joins once: {joined:?}");
}
