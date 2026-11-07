use super::*;
use nfsmw_data::gameplay::RaceParams;

fn trigger(key: u32, at: Vec3) -> BoxTrigger {
    BoxTrigger {
        key,
        name: format!("t{key}"),
        position: at,
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

fn character(key: u32, name: &str, skill: i32, boss: bool) -> Character {
    Character {
        key,
        name: name.to_owned(),
        skill_level: skill,
        min_performance: 0.5,
        preset_ride: None,
        car_type: None,
        is_boss: boss,
        racer_name: None,
        force_start: None,
    }
}

fn race(opponents: Vec<u32>) -> Race {
    Race {
        key: 1,
        name: "race_bin_1/1_1_1_sprint".into(),
        event: None,
        event_icon: "p2p".into(),
        race_type: Some(RaceType::Sprint),
        region: None,
        laps: 1,
        start: Some(Placement { position: Vec3::new(0.0, 0.0, 0.0), heading_deg: 0.0 }),
        finish: Some(10),
        checkpoints: vec![11, 12],
        barriers: Vec::new(),
        params: RaceParams { opponents, ..Default::default() },
        story: Default::default(),
    }
}

fn data() -> GameplayData {
    GameplayData {
        triggers: vec![
            trigger(10, Vec3::new(0.0, 0.0, 300.0)),
            trigger(11, Vec3::new(0.0, 0.0, 100.0)),
            trigger(12, Vec3::new(0.0, 0.0, 200.0)),
        ],
        characters: vec![
            character(20, "race_bin_1/character1", 80, false),
            character(21, "race_bin_1/boss", 90, true),
            character(22, "character_smart", 0, false),
        ],
        ..Default::default()
    }
}

#[test]
fn the_course_has_the_checkpoints_in_order_and_the_finish() {
    let course = course_of(&data(), &race(Vec::new()), &RaceOptions::default()).unwrap();
    assert_eq!(course.checkpoints.len(), 2);
    assert_eq!(course.checkpoints[0].centre.z, 100.0);
    assert_eq!(course.finish.unwrap().centre.z, 300.0);
    assert_eq!(course.laps, 1);
}

#[test]
fn the_stored_extents_are_full_sizes_across_and_a_half_height() {
    // A 2 x 40 x 40 trigger is 2 m deep, 40 m wide and 40 m above and below: a point 39 m up, 19 m across and 0.9 m along
    // its heading is inside, one just past any of the three is not.
    let gate = gate_of(&trigger(1, Vec3::ZERO));
    assert!(gate.contains(Vec3::new(19.0, 39.0, 0.9)));
    assert!(!gate.contains(Vec3::new(21.0, 0.0, 0.0)));
    assert!(!gate.contains(Vec3::new(0.0, 0.0, 1.1)));
    assert!(!gate.contains(Vec3::new(0.0, 41.0, 0.0)));
}

#[test]
fn the_course_carries_the_time_bonuses_and_the_speed_traps() {
    let mut d = data();
    d.triggers[1].time_bonus = 25.0;
    d.spheres = vec![nfsmw_data::gameplay::SphereTrigger {
        key: 50,
        name: "trap".into(),
        position: Vec3::new(1.0, 2.0, 3.0),
        radius: 20.0,
        threshold_speed: 88.0,
    }];
    let mut r = race(Vec::new());
    r.params.speed_traps = vec![50, 51];
    let course = course_of(&d, &r, &RaceOptions::default()).unwrap();
    assert_eq!(course.bonus(1), 25.0);
    assert_eq!(course.bonus(2), 0.0);
    assert_eq!(course.traps.len(), 1, "a trap that was not read is skipped");
    assert_eq!(course.traps[0].threshold_kmh, 88.0);
    assert_eq!(config_of(&r).speed_traps, 2);
}

#[test]
fn a_race_without_gates_has_no_course() {
    let mut empty = race(Vec::new());
    empty.finish = None;
    empty.checkpoints.clear();
    assert!(course_of(&data(), &empty, &RaceOptions::default()).is_err());
}

#[test]
fn the_lap_option_replaces_the_stored_laps() {
    let options = RaceOptions { laps: Some(4), ..Default::default() };
    assert_eq!(laps_of(&race(Vec::new()), &options), 4);
    assert_eq!(laps_of(&race(Vec::new()), &RaceOptions::default()), 1);
}

#[test]
fn a_rolling_start_has_no_countdown() {
    let mut r = race(Vec::new());
    assert_eq!(config_of(&r).countdown, COUNTDOWN_SECS);
    r.params.rolling_start = true;
    assert_eq!(config_of(&r).countdown, 0.0);
}

#[test]
fn a_listed_race_uses_its_own_characters_with_their_skill_in_a_career() {
    let r = race(vec![20, 21]);
    let career = RaceOptions { career: true, ..Default::default() };
    let opponents = opponents_of(&data(), &r, &career, &|_| None);
    assert_eq!(opponents.len(), 2);
    assert_eq!(opponents[0].skill, 0.8);
    assert_eq!(opponents[1].skill, 0.9);
}

#[test]
fn a_quick_race_replaces_a_boss_and_uses_the_difficulty_table() {
    let r = race(vec![21]);
    let quick = RaceOptions { difficulty: Some(33), ..Default::default() };
    let opponents = opponents_of(&data(), &r, &quick, &|_| None);
    assert_eq!(opponents.len(), 1);
    assert_eq!(opponents[0].name, "character_smart");
    assert_eq!(opponents[0].skill, 0.15);
    let hard = RaceOptions { difficulty: Some(100), ..Default::default() };
    assert_eq!(opponents_of(&data(), &r, &hard, &|_| None)[0].skill, 1.0);
}

#[test]
fn only_career_uses_the_characters_minimum_performance_bias() {
    let mut d = data();
    d.characters[0].min_performance = 0.7;
    let r = race(vec![d.characters[0].key]);
    let quick = opponents_of(&d, &r, &RaceOptions::default(), &|_| None);
    assert_eq!(quick[0].bias, 0.0);
    let career = opponents_of(&d, &r, &RaceOptions { career: true, ..Default::default() }, &|_| None);
    assert_eq!(career[0].bias, 0.7);
}

#[test]
fn a_race_with_no_opponents_gets_the_asked_count_of_generic_ones() {
    let r = race(Vec::new());
    assert!(opponents_of(&data(), &r, &RaceOptions::default(), &|_| None).is_empty());
    let three = RaceOptions { opponents: Some(3), ..Default::default() };
    assert_eq!(opponents_of(&data(), &r, &three, &|_| None).len(), 3);
}

#[test]
fn a_preset_ride_resolves_to_its_car() {
    let mut d = data();
    d.characters[0].preset_ride = Some("BL5".into());
    let r = race(vec![20]);
    let opponents = opponents_of(&d, &r, &RaceOptions { career: true, ..Default::default() }, &|name| {
        (name == "BL5").then(|| "mustanggt".to_owned())
    });
    assert_eq!(opponents[0].car.as_deref(), Some("mustanggt"));
}

#[test]
fn a_boss_race_in_a_career_fixes_the_rivals_leaderboard_name_and_keeps_the_rivals_car() {
    let mut d = data();
    d.characters[1].preset_ride = Some("BL10".into());
    let mut r = race(vec![21]);
    r.name = "race_bin_10/10_2_1_sprint".into();
    r.story.boss_race = true;
    let career = RaceOptions { career: true, ..Default::default() };
    let resolve = |name: &str| (name == "BL10").then(|| "CAYMANS".to_owned());
    let opponents = opponents_of(&d, &r, &career, &resolve);
    let rival = &opponents[0];
    assert_eq!(rival.name_label.as_deref(), Some("BLACKLIST_RIVAL_10_LEADERBOARD"));
    assert_eq!(rival.car.as_deref(), Some("CAYMANS"));
    // A quick race of the same race plays the generic racer, with a drawn name.
    let quick = RaceOptions::default();
    assert_eq!(opponents_of(&d, &r, &quick, &resolve)[0].name_label, None);
}

#[test]
fn a_career_characters_racer_name_is_its_fixed_name() {
    let mut d = data();
    d.characters[0].racer_name = Some("BLACKLIST_RIVAL_16_LEADERBOARD".into());
    let r = race(vec![20, 21]);
    let career = RaceOptions { career: true, ..Default::default() };
    let opponents = opponents_of(&d, &r, &career, &|_| None);
    assert_eq!(opponents[0].name_label.as_deref(), Some("BLACKLIST_RIVAL_16_LEADERBOARD"));
    assert_eq!(opponents[1].name_label, None, "a street rival draws its name");
    let quick = RaceOptions::default();
    assert_eq!(opponents_of(&d, &r, &quick, &|_| None)[0].name_label, None);
}

#[test]
fn the_stock_pick_is_stable_and_differs_between_racers() {
    let pool: Vec<String> = (0..20).map(|i| format!("car{i}")).collect();
    assert_eq!(stock_car(&pool, 7, 1), stock_car(&pool, 7, 1));
    let picks: std::collections::HashSet<_> = (0..6).filter_map(|i| stock_car(&pool, 7, i)).collect();
    assert!(picks.len() > 2, "{picks:?}");
    assert_eq!(stock_car(&[], 1, 1), None);
}

#[test]
fn a_positive_glue_is_scaled_down_for_a_weak_driver_and_a_negative_one_for_a_strong_one() {
    assert!((scale_glue(1.0, 0.0, false) - 0.33).abs() < 1e-5);
    assert!((scale_glue(1.0, 0.5, false) - 0.66).abs() < 1e-5);
    assert!((scale_glue(1.0, 1.0, false) - 1.0).abs() < 1e-5);
    assert!((scale_glue(-1.0, 0.0, false) + 1.0).abs() < 1e-5);
    assert!((scale_glue(-1.0, 1.0, false) + 0.66).abs() < 1e-5);
    assert!((scale_glue(1.0, 1.0, true) - 0.5).abs() < 1e-5, "halved in a speed-trap race");
}

#[test]
fn only_the_types_the_world_can_run_are_runnable() {
    assert!(runnable(Some(RaceType::Drag)));
    assert!(!runnable(Some(RaceType::CashGrab)));
    assert!(!runnable(None));
}

/// The side bins 18 to 21 (tollbooth challenges, the challenge series, the collectors; the OPM test bin 20 is a developer bin nothing enters): every race
/// of them is one of the runnable types, and has a course or is a pursuit challenge with a statistic to reach.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_race_of_the_side_bins_can_be_started() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let data = nfsmw_data::gameplay::load(&dir).unwrap();
    let side: Vec<_> = data
        .races
        .iter()
        .filter(|r| ["18.", "19.", "21."].iter().any(|bin| crate::career::race_event_id(r).starts_with(bin)))
        .collect();
    assert!(side.len() > 50, "{} side bin races", side.len());
    for race in side {
        assert!(runnable(race.race_type), "{} is {:?}", race.name, race.race_type);
        let challenge = race.race_type == Some(RaceType::Challenge);
        let ok = match challenge {
            true => {
                race.params.milestone.is_some()
                    && crate::scenes::world::race::challenge::Stat::of(race.params.milestone.as_deref().unwrap())
                        .is_some()
            }
            false => course_of(&data, race, &RaceOptions::default()).is_ok(),
        };
        assert!(ok, "{} cannot start", race.name);
    }
}

/// Walks a straight line through every gate of every race of the install and checks that the race ends with the
/// walker first, with the laps counted.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_install_course_can_be_finished_by_walking_through_its_gates() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let data = nfsmw_data::gameplay::load(&dir).unwrap();
    let (mut ran, mut skipped) = (0, 0);
    for race in data.races.iter().filter(|r| runnable(r.race_type)) {
        let options = RaceOptions::default();
        let Ok(course) = course_of(&data, race, &options) else {
            skipped += 1;
            continue;
        };
        let start = course.start;
        let mut engine = blackbox_race::Race::new(course.clone(), config_of(race), &[(true, start), (false, start)]);
        engine.start_now();
        let mut at = start;
        let mut finished = false;
        let mut gates: Vec<Vec3> = course.checkpoints.iter().map(|g| g.centre).collect();
        gates.extend(course.finish.iter().map(|g| g.centre));
        for _ in 0..course.laps {
            for centre in &gates {
                let steps = (at.distance(*centre) / 4.0).ceil().max(1.0) as u32;
                for step in 1..=steps {
                    let to = at.lerp(*centre, step as f32 / steps as f32);
                    finished |=
                        engine.move_racer(0, to).iter().any(|e| matches!(e, blackbox_race::RaceEvent::Finished { .. }));
                }
                at = *centre;
            }
        }
        assert!(finished, "{} was not finished: {:?}", race.name, engine.racers()[0].progress);
        assert_eq!(engine.places()[0], 1, "{}", race.name);
        ran += 1;
    }
    eprintln!("{ran} courses finished, {skipped} races without a course");
    assert!(ran > 200, "{ran}");
}
