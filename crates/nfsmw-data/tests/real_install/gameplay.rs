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
fn finish_lines_carry_the_red_flare_and_checkpoints_none() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let (mut finishes, mut with_flares) = (0, 0);
    let mut spacings: Vec<i32> = Vec::new();
    for race in &data.races {
        for key in &race.checkpoints {
            let trigger = data.trigger(*key).expect("checkpoint");
            assert!(trigger.particle_effect.is_none(), "{} shows an effect", trigger.name);
        }
        let Some(finish) = race.finish.and_then(|key| data.trigger(key)) else { continue };
        finishes += 1;
        let Some(effect) = finish.particle_effect.as_deref() else { continue };
        with_flares += 1;
        assert_eq!(effect, "fxgame_flare_red", "{}", finish.name);
        let spacing = finish.flare_spacing.unwrap_or(0.0);
        assert!(spacing > 0.0, "{} has no spacing", finish.name);
        spacings.push(spacing as i32);
    }
    spacings.sort_unstable();
    spacings.dedup();
    println!("flare spacings: {spacings:?}");
    // 69 finish lines have none (the D-Day story shells among them); the spacings run from 4 to 16 m, road by road.
    assert!(with_flares == 148 && with_flares <= finishes, "{with_flares} of {finishes} finish lines have flares");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_race_names_its_photo_backdrop_and_finish_camera() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let texture = |t: gameplay::RaceType| {
        let mut names: Vec<&str> = data
            .races
            .iter()
            .filter(|r| r.race_type == Some(t))
            .filter_map(|r| r.params.photo_texture.as_deref())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    };
    assert_eq!(texture(RaceType::Tollbooth), ["PHOTOFINISH_TOOBOOTH"]);
    assert_eq!(texture(RaceType::Drag), ["PHOTOFINISH_DRAG"]);
    assert_eq!(texture(RaceType::Circuit), ["PHOTOFINISH_SPRINT"]);
    assert!(data.races.iter().filter(|r| r.race_type.is_some()).all(|r| r.params.finish_camera.is_some()));
    let numbered = data.races.iter().filter_map(|r| r.params.finish_camera.as_deref());
    assert!(numbered.clone().any(|name| name == "DefaultFinish") && numbered.clone().any(|name| name == "PHoldFin_17"));
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

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn post_race_activities_and_outros_are_read() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let chained: Vec<_> = data.races.iter().filter(|r| r.params.post_race_activity.is_some()).collect();
    let outros: Vec<(String, &str)> =
        data.races.iter().filter_map(|r| r.story.outro_nis.as_deref().map(|nis| (r.name.clone(), nis))).collect();
    println!("CHAINED {} OUTROS {outros:?}", chained.len());
    for race in &chained {
        let next = race.params.post_race_activity.unwrap();
        println!(
            "CHAIN {} -> {}",
            race.name,
            data.races.iter().find(|r| r.key == next).map_or("?", |r| r.name.as_str())
        );
    }
    assert!(!chained.is_empty(), "races chain through PostRaceActivity");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_challenge_ladders_have_their_unlock_lists_and_starts() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let id = |r: &gameplay::Race| r.event.clone().unwrap_or_else(|| r.name.clone());
    let side = |r: &&gameplay::Race| ["18.", "19.", "21."].iter().any(|bin| id(r).starts_with(bin));
    let mut open: Vec<String> =
        data.races.iter().filter(side).filter(|r| r.params.initially_unlocked).map(id).collect();
    open.sort();
    println!("OPEN {open:?}");
    for race in data.races.iter().filter(side).filter(|r| !r.params.unlock_races.is_empty()).take(4) {
        let next: Vec<String> =
            race.params.unlock_races.iter().filter_map(|k| data.races.iter().find(|r| r.key == *k)).map(id).collect();
        println!("UNLOCKS {} -> {next:?}", id(race));
    }
    assert!(open.contains(&"19.8.1".to_owned()), "the first pursuit challenge starts open: {open:?}");
    assert!(data.races.iter().filter(side).any(|r| !r.params.unlock_races.is_empty()), "a win opens races");
}
