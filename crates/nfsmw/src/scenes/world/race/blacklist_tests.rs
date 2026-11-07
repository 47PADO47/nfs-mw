//! Installed event identity, settings and rival equipment through production readers.
use super::*;
use crate::career::{CareerData, race_event_id};
use nfsmw_data::car::{
    CarTables,
    physics::{PhysicsData, performance::preset},
};

#[test]
#[ignore = "needs installed Blacklist free-event data"]
fn installed_blacklist_free_events_do_not_launch_rival_duels() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let gameplay = nfsmw_data::gameplay::load(&dir).unwrap();
    let mut duels = Vec::new();
    for bin in gameplay.bins.iter().filter(|bin| (1..=15).contains(&bin.number)) {
        for key in &bin.world_races {
            let race = gameplay.races.iter().find(|race| race.key == *key).unwrap();
            let opponents: Vec<_> = race.params.opponents.iter().filter_map(|key| gameplay.character(*key)).collect();
            if race.story.boss_race || opponents.iter().any(|character| character.is_boss) {
                duels.push((bin.number, race.name.clone()));
            }
        }
    }
    assert!(duels.is_empty(), "free Blacklist events contain rival duels: {duels:?}");
}

#[test]
#[ignore = "needs installed Blacklist collections and preset equipment"]
fn installed_blacklist_events_resolve_exact_settings_and_cars() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let gameplay = nfsmw_data::gameplay::load(&dir).unwrap();
    let data = CareerData::from_parts(gameplay.clone(), &[], None, None);
    let tables = CarTables::load(&dir).unwrap();
    let physics = PhysicsData::load(&dir).unwrap();
    let options = RaceOptions { career: true, ..Default::default() };
    let resolver = |name: &str| {
        tables.preset(name).map(|p| format!("{}:{}", preset::vehicle_name(physics.database(), p), p.preset_name))
    };
    let mut reused = 0;
    let mut presets = std::collections::BTreeSet::new();
    let mut checked = 0;
    for bin in &data.bins {
        let source = gameplay.bins.iter().find(|source| source.number == i32::from(bin.number)).unwrap();
        for (keys, events) in [(&source.world_races, &bin.races), (&source.boss_races, &bin.rival_events)] {
            for (key, event) in keys.iter().zip(events) {
                let race = start::resolve(&gameplay, event).unwrap();
                assert_eq!(race.key, *key, "bin {} event {event} starts another collection", bin.number);
                assert_eq!(setup::laps_of(race, &options), race.laps.max(1) as u32);
                let opponents = setup::opponents_of(&gameplay, race, &options, &resolver);
                assert_eq!(opponents.len(), race.params.opponents.len());
                for (opponent, key) in opponents.iter().zip(&race.params.opponents) {
                    let character = gameplay.character(*key).unwrap();
                    assert_eq!(opponent.name, character.name);
                    assert_eq!(opponent.bias, character.min_performance);
                    if let Some(name) = &character.preset_ride {
                        assert_eq!(opponent.car, resolver(name));
                        presets.insert(name.clone());
                    }
                }
                checked += 1;
                reused += usize::from(event.ends_with(".r"));
            }
        }
    }
    assert_eq!(data.bins.len(), 15);
    assert!(checked > 100 && reused > 0);
    let mut upgraded = 0;
    for name in &presets {
        let ride = tables.preset(name).unwrap();
        let vehicle = preset::vehicle_name(physics.database(), ride);
        assert_eq!(blackbox_attrib::vlt_hash(vehicle), ride.vehicle_key as u32, "preset must keep its vehicle record");
        let package = preset::read(physics.database(), vehicle, name).unwrap();
        eprintln!("BLACKLIST_PRESET {name} {} package={package:?}", ride.car_type_name);
        if let Some(package) = package {
            physics.car_with_performance(vehicle, package).unwrap();
            upgraded += usize::from(package.levels.iter().any(|level| *level > 0));
        }
    }
    assert_eq!(presets.len(), 15, "each Blacklist rival needs its own installed preset");
    assert!(upgraded > 0, "installed preset equipment must be read");
    let reused_race = start::resolve(&gameplay, "2.2.1.r").unwrap();
    assert!(!reused_race.story.boss_race);
    assert!(reused_race.story.player_car_type.is_none());
    assert_ne!(reused_race.key, start::resolve(&gameplay, "2.2.1").unwrap().key);
    eprintln!("BLACKLIST_EVENTS checked={checked} reused={reused} example={}", race_event_id(reused_race));
}

#[test]
#[ignore = "needs installed world and rival cars; production event loading without a GPU"]
fn installed_blacklist_world_launch_keeps_owned_car_and_rival_preset() {
    use crate::app::test_backend::NullBackend;
    use crate::input::ActionState;
    use crate::scenes::world::{DriveOptions, Options, WorldScene};
    use crate::viewer::Scene;
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = nfsmw_data::gameplay::load(&dir).unwrap();
    let boss_key = data.bins.iter().find(|bin| bin.number == 14).unwrap().boss_races[0];
    let boss = data.races.iter().find(|race| race.key == boss_key).unwrap();
    let tables = CarTables::load(&dir).unwrap();
    for event in ["2.2.1.r".to_owned(), race_event_id(boss), "16.1.0".to_owned()] {
        let race = start::resolve(&data, &event).unwrap();
        let mut scene = WorldScene::open(
            &dir,
            Options {
                at: Some([2151.0, 1399.0]),
                height: 40.0,
                heading: 45.0,
                pitch: -20.0,
                fog_distance: 1500.0,
                wait_for_load: true,
                drive: Some(DriveOptions { car: "CARRERAGT".into(), script: None }),
            },
        )
        .unwrap();
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        scene.init(&mut renderer).unwrap();
        assert!(scene.start_career_event(&event));
        let mut staged = false;
        for _ in 0..20_000 {
            scene.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
            if scene.race.as_ref().is_some_and(|session| session.engine.is_some()) {
                staged = true;
                break;
            }
        }
        assert!(staged, "{event}: event did not stage");
        let session = scene.race.as_ref().unwrap();
        assert_eq!(session.origin.race.key, race.key);
        assert_eq!(session.options, RaceOptions { career: true, ..Default::default() });
        assert_eq!(session.plan.len(), race.params.opponents.len() + 1, "rival must not silently drop from the grid");
        let expected_car = race
            .story
            .player_car_type
            .as_deref()
            .and_then(|name| tables.preset(name))
            .map_or("CARRERAGT", |ride| ride.car_type_name.as_str());
        assert!(scene.players.lead().unwrap().car_name.eq_ignore_ascii_case(expected_car));
        if let Some(name) = &race.story.player_car_type {
            let ride = tables.preset(name).unwrap();
            let vehicle = preset::vehicle_name(&tables.attributes, ride);
            assert_eq!(scene.players.lead().unwrap().sound_car.as_deref(), Some(vehicle));
            assert_eq!(blackbox_attrib::vlt_hash(vehicle), ride.vehicle_key as u32);
        }
        if race.story.boss_race {
            assert!(session.plan[1].model.as_ref().unwrap().ends_with(":BL14"));
        }
        scene.close(&mut renderer);
    }
}
