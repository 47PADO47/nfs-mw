use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_NOT_FOUND};
use nfsmw_modding::abi::{BbName, BbRaceDef, BbVec3};
use nfsmw_modding::host::RacesBackend;

use super::RacesPort;
use crate::modding::races_feed::{self, Now, Outcome, RaceRow, Request, Snapshot};

fn row(id: u32, name: &str) -> RaceRow {
    RaceRow {
        id,
        name: name.into(),
        kind: 1,
        laps: 3,
        racers: 4,
        start: [1.0, 2.0, 3.0],
        checkpoints: vec![[10.0, 0.0, 0.0], [20.0, 5.0, 0.0]],
        done: false,
    }
}

fn def(key: &str) -> BbRaceDef {
    BbRaceDef {
        size: 0,
        kind: 0,
        laps: 1,
        reward_cash: 500,
        start: BbVec3 { x: 0.0, y: 0.0, z: 0.0 },
        start_heading: 0.5,
        key: BbName::new(key).unwrap(),
        name: BbName::new("A race").unwrap(),
        opponents: 3,
    }
}

fn point(x: f32) -> BbVec3 {
    BbVec3 { x, y: 0.0, z: 0.0 }
}

fn published() {
    races_feed::clear();
    races_feed::publish(Snapshot {
        rows: vec![row(0, "race_bin_01/1_1_1_a"), row(7, "race_bin_01/1_1_2_b")],
        now: Some(Now { id: 7, lap: 2, checkpoint: 3, position: 1, elapsed: 12.5 }),
        results: vec![(0, Outcome { position: 2, time: 61.0, won: false, winnings: 0 })],
    });
}

#[test]
fn the_list_and_the_checkpoints_come_from_the_snapshot() {
    published();
    let port = RacesPort;
    assert_eq!(port.race_count("m").unwrap(), 2);
    let info = port.race_at("m", 1).unwrap();
    assert_eq!((info.id, info.kind, info.laps, info.racers), (7, 1, 3, 4));
    assert_eq!(info.name.text(), Some("race_bin_01/1_1_2_b"));
    assert_eq!(info.pos, BbVec3 { x: 1.0, y: 2.0, z: 3.0 });
    assert_eq!(port.race_at("m", 2), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.race_checkpoint_count("m", 7).unwrap(), 2);
    assert_eq!(port.race_checkpoint_at("m", 7, 1).unwrap(), BbVec3 { x: 20.0, y: 5.0, z: 0.0 });
    assert_eq!(port.race_checkpoint_at("m", 7, 2), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.race_checkpoint_count("m", 99), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn the_running_race_and_the_results_are_read() {
    published();
    let port = RacesPort;
    let now = port.race_current("m").unwrap();
    assert_eq!((now.active, now.id, now.lap, now.checkpoint, now.position), (1, 7, 2, 3, 1));
    assert_eq!(now.elapsed, 12.5);
    let result = port.race_results("m", 0).unwrap();
    assert_eq!((result.position, result.won, result.winnings), (2, 0, 0));
    assert_eq!(port.race_results("m", 7), Err(BB_ERR_NOT_FOUND), "no result before the race ends");
    races_feed::publish(Snapshot::default());
    assert_eq!(port.race_current("m").unwrap().active, 0);
}

#[test]
fn a_start_an_abort_and_the_tunings_are_queued_for_the_world() {
    published();
    let port = RacesPort;
    port.race_start("m", 7).unwrap();
    port.race_abort("m").unwrap();
    port.race_set_opponent_skill("m", 0, 0.25).unwrap();
    port.race_set_reward("m", 0, 900).unwrap();
    assert_eq!(
        races_feed::take_requests(),
        vec![Request::Start(7), Request::Abort, Request::Skill(0, 0.25), Request::Reward(0, 900)]
    );
    assert_eq!(port.race_start("m", 5), Err(BB_ERR_NOT_FOUND), "an unknown race");
    assert_eq!(port.race_set_opponent_skill("m", 0, 1.5), Err(BB_ERR_ARG), "skill is 0 to 1");
    assert_eq!(port.race_set_reward("m", 4, 1), Err(BB_ERR_NOT_FOUND));
    assert!(races_feed::take_requests().is_empty(), "a refused call queues nothing");
}

#[test]
fn a_registered_race_is_on_the_list_at_once_and_can_be_started() {
    published();
    let port = RacesPort;
    let id = port.race_register("m", &def("m:loop"), &[point(10.0), point(20.0)]).unwrap();
    assert!(id >= races_feed::CUSTOM_BASE);
    assert_eq!(port.race_count("m").unwrap(), 3);
    assert_eq!(port.race_checkpoint_count("m", id).unwrap(), 2);
    port.race_start("m", id).unwrap();
    assert_eq!(races_feed::take_requests(), vec![Request::Start(id)]);
    assert_eq!(races_feed::customs_after(0).len(), 1, "the world builds it on its next frame");
}

#[test]
fn a_race_key_has_to_be_the_mods_own_and_new() {
    published();
    let port = RacesPort;
    assert_eq!(port.race_register("m", &def("other:loop"), &[point(1.0)]), Err(BB_ERR_ARG), "another mod's id");
    assert_eq!(port.race_register("m", &def("loop"), &[point(1.0)]), Err(BB_ERR_ARG), "no mod id");
    assert_eq!(port.race_register("m", &def("m:loop"), &[]), Err(BB_ERR_ARG), "no checkpoints");
    let mut bad = def("m:loop");
    bad.kind = 99;
    assert_eq!(port.race_register("m", &bad, &[point(1.0)]), Err(BB_ERR_ARG), "no such kind");
    let mut nan = def("m:loop");
    nan.start.x = f32::NAN;
    assert_eq!(port.race_register("m", &nan, &[point(1.0)]), Err(BB_ERR_ARG));
    port.race_register("m", &def("m:loop"), &[point(1.0)]).unwrap();
    assert_eq!(port.race_register("m", &def("m:loop"), &[point(1.0)]), Err(BB_ERR_ARG), "the key is taken");
}
