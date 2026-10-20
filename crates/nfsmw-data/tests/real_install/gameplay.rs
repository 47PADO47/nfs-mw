//! The gameplay class of `GLOBAL/gameplay.bin` against docs/formats/gameplay-races.md.

use nfsmw_data::gameplay::{self, ActivityKind, RaceType};

use crate::install;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-2
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn races_triggers_and_activity_zones_match_documented_numbers() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    assert_eq!(data.races.len(), 255, "races with a racestart");
    let kinds = |t: RaceType| data.races.iter().filter(|r| r.race_type == Some(t)).count();
    assert_eq!(kinds(RaceType::Sprint), 51, "p2p");
    assert_eq!(kinds(RaceType::Tollbooth), 68);
    assert_eq!(kinds(RaceType::Challenge), 36);
    assert_eq!(kinds(RaceType::Circuit), 32);
    assert_eq!(kinds(RaceType::SpeedTrap), 31);
    assert_eq!(kinds(RaceType::Knockout), 20);
    assert_eq!(kinds(RaceType::Drag), 11);
    assert!(data.races.iter().all(|r| r.race_type.is_some() || r.event_icon.is_empty()));

    // Every checkpoint and finish line refers to a box trigger that was read.
    for race in &data.races {
        for key in race.checkpoints.iter().chain(race.finish.iter()) {
            assert!(data.trigger(*key).is_some(), "{} refers to an unread trigger {key:08X}", race.name);
        }
    }

    let sprint = data.races.iter().find(|r| r.name == "race_bin_10/10_2_1_sprint").expect("10.2.1 sprint");
    assert_eq!(sprint.event_icon, "p2p");
    assert_eq!(sprint.race_type, Some(RaceType::Sprint));
    assert_eq!(sprint.laps, 1);
    assert_eq!(sprint.region.as_deref(), Some("coastal"));
    assert_eq!(sprint.checkpoints.len(), 9);
    let start = sprint.start.expect("start grid");
    let want = [-4404.865, 209.6045, 1511.824];
    assert!(
        [start.position.x, start.position.y, start.position.z].iter().zip(want).all(|(a, b)| close(*a, b)),
        "start {start:?}"
    );
    assert!(close(start.heading_deg, -198.9169));

    let finish = data.trigger(sprint.finish.expect("finish")).expect("finish trigger");
    assert_eq!(finish.dimensions, [3.0, 65.0, 5.0]);
    assert_eq!(finish.particle_effect.as_deref(), Some("fxgame_flare_red"));
    assert_eq!(finish.flare_spacing, Some(5.0));
    assert!(!finish.one_shot && !finish.fire_on_exit);

    let first = data.trigger(sprint.checkpoints[0]).expect("checkpoint 1");
    assert_eq!(first.name, "race_bin_10/10_2_1_sprint/checkpoint1");
    assert_eq!(first.dimensions, [2.0, 60.0, 40.0]);

    let shops = data.markers.iter().filter(|m| m.kind == ActivityKind::AutoShop).count();
    let houses = data.markers.iter().filter(|m| m.kind == ActivityKind::SafeHouse).count();
    assert_eq!((shops, houses), (8, 4));
    let zone = data.markers.iter().find(|m| m.name == "menu_gate_customshop/zone_customshop_1").expect("shop 1");
    let want = [-4523.472, 214.6646, 698.1821];
    assert!(
        [zone.position.x, zone.position.y, zone.position.z].iter().zip(want).all(|(a, b)| close(*a, b)),
        "shop position {zone:?}"
    );
    assert!(close(zone.radius, 6.0));
    assert_eq!(zone.icon_index, Some(15));
    let respawn = zone.respawn.expect("respawn marker");
    assert!(close(respawn.heading_deg, 82.59451));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn race_parameters_and_characters_match_documented_numbers() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    let sprint = data.race_named("race_bin_10/10_2_1_sprint").expect("10.2.1 sprint");
    assert!(sprint.params.do_countdown && !sprint.params.rolling_start, "{:?}", sprint.params);
    assert_eq!(sprint.params.difficulty, 100);
    assert!(!sprint.params.opponents.is_empty(), "a rival");

    let races = &data.races;
    let with = |f: &dyn Fn(&gameplay::RaceParams) -> bool| races.iter().filter(|r| f(&r.params)).count();
    assert_eq!(with(&|p| p.opponents.is_empty()), 108, "races without opponents");
    assert_eq!(with(&|p| p.opponents.len() == 3), 101, "street races");
    assert_eq!(with(&|p| p.rolling_start), 38, "rolling starts");
    assert_eq!(with(&|p| p.do_countdown), 217, "countdown races");
    assert_eq!(with(&|p| p.looping), 53, "looping races");
    assert_eq!(with(&|p| p.knockouts_per_lap == 1), 20, "knockout races");
    assert_eq!(with(&|p| p.rank_by_points), 32, "speed-trap ranking");
    assert_eq!(with(&|p| p.traffic_pattern.as_deref() == Some("drag")), 11, "drag traffic");

    for race in races {
        for key in &race.params.opponents {
            assert!(data.character(*key).is_some(), "{} names an unread opponent {key:08X}", race.name);
        }
    }
    let street = data.character(sprint.params.opponents[0]).expect("an opponent");
    assert!(street.skill_level > 0 && street.min_performance >= 0.0, "{street:?}");
    assert!(data.characters.len() >= 300, "314 characters in the install, got {}", data.characters.len());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn tollbooth_bonuses_and_speed_trap_spheres_are_read() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let toll = data.race_named("race_bin_01/2_4_3_r_tollbooth").expect("tollbooth race");
    let bonus: Vec<f32> = toll.checkpoints.iter().filter_map(|k| data.trigger(*k)).map(|t| t.time_bonus).collect();
    assert_eq!(bonus.len(), 15, "every checkpoint reads");
    assert_eq!(bonus.iter().filter(|b| **b > 0.0).count(), 4, "four time bonus checkpoints: {bonus:?}");
    assert!(bonus.contains(&160.0) && bonus.contains(&35.0) && bonus.contains(&25.0), "{bonus:?}");
    let trap = data.race_named("race_bin_01/1_5_3_speedtrap").expect("speed trap race");
    assert_eq!(trap.params.speed_traps.len(), 9);
    for key in &trap.params.speed_traps {
        let sphere = data.sphere(*key).expect("every trap is a sphere trigger");
        assert!(sphere.radius >= 20.0 && sphere.threshold_speed == 88.0, "{sphere:?}");
    }
    let every_trap_resolves =
        data.races.iter().flat_map(|r| r.params.speed_traps.iter()).all(|k| data.sphere(*k).is_some());
    assert!(every_trap_resolves, "every speed trap of every race is a sphere");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn pursuit_challenges_carry_their_milestone_and_threshold() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let challenges: Vec<_> = data.races.iter().filter(|r| r.race_type == Some(RaceType::Challenge)).collect();
    assert_eq!(challenges.len(), 36, "36 challenge events");
    let mut names: Vec<&str> = challenges.iter().filter_map(|r| r.params.milestone.as_deref()).collect();
    names.sort_unstable();
    names.dedup();
    println!("MILESTONES {names:?}");
    let epic = data.race_named("race_bin_01/epic_pursuit").expect("the final race");
    assert_eq!(epic.params.milestone.as_deref(), Some("pursuit_length"));
    let hit = data.race_named("race_bin_challenge/challenge_18_cops_hit").expect("challenge 18");
    assert_eq!(hit.params.threshold, 12.0);
    assert_eq!(hit.params.force_heat_level, 4);
}
