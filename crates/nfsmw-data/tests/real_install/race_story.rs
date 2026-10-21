//! The story fields of the races (`gameplay/story.rs`) against docs/specs/career-scripts-events.md sections 1.1 and 2.1.

use nfsmw_data::gameplay::{self, GameplayData, Race};
use nfsmw_data::shared;

use crate::install;

/// The race with this event id (`16.1.0`); the prologue races store their event id in the race.
fn event<'a>(data: &'a GameplayData, id: &str) -> &'a Race {
    data.races.iter().find(|r| r.event.as_deref() == Some(id)).unwrap_or_else(|| panic!("no race with event {id}"))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_story_fields_are_in_the_gameplay_schema() {
    let Some(dir) = install() else { return };
    let db = shared::attributes(&dir).expect("attributes");
    let class = db.class("gameplay").expect("the gameplay class");
    for name in [
        "IntroMessageID",
        "OutroMessageID",
        "IntroMovie",
        "IntroNIS",
        "QuickRaceNIS",
        "OutroNIS",
        "OutroMovie",
        "PlayerCarType",
        "PlayerCarPerformance",
    ] {
        assert!(class.field(name).is_some(), "{name} is not a gameplay field");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_prologue_intros_match_the_documented_table() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    let step1 = event(&data, "16.1.0");
    assert_eq!(step1.story.intro_movie.as_deref(), Some("storyfmv_rac01"));
    assert_eq!(step1.story.intro_nis.as_deref(), Some("IntroNisDD"));

    let step2 = event(&data, "16.2.2");
    assert_eq!(step2.story.intro_movie.as_deref(), Some("storyfmv_cro06_coh06a"));
    assert_eq!(step2.story.intro_nis.as_deref(), Some("GenericStart"));

    let step3 = event(&data, "16.2.3");
    assert_eq!(step3.story.intro_movie.as_deref(), Some("storyfmv_raz08"));
    assert_eq!(step3.story.intro_nis.as_deref(), Some("IntroNis05"));

    let step4 = event(&data, "16.1.1");
    assert_eq!(step4.story.intro_nis.as_deref(), Some("IntroNis10"));
    // `docs/provenance/career-phone.md` names this message screen.
    assert_eq!(step4.story.intro_message_id, Some(2));

    let step6 = event(&data, "16.2.1");
    assert_eq!(step6.story.intro_movie.as_deref(), Some("storyfmv_pin11"));
    assert_eq!(step6.story.intro_nis.as_deref(), Some("IntroNis04"));
    assert_eq!(step6.story.outro_movie.as_deref(), Some("storyfmv_bus12"));

    let epic = data.race_named("race_bin_01/epic_pursuit").expect("the final race");
    assert_eq!(epic.story.intro_movie.as_deref(), Some("Storyfmv_her136"));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn race_story_census() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    println!("CENSUS races={}", data.races.len());
    type Check = fn(&Race) -> bool;
    let fields: [(&str, Check); 9] = [
        ("intro_message_id", |r| r.story.intro_message_id.is_some()),
        ("intro_movie", |r| r.story.intro_movie.is_some()),
        ("intro_nis", |r| r.story.intro_nis.is_some()),
        ("quick_race_nis", |r| r.story.quick_race_nis.is_some()),
        ("outro_nis", |r| r.story.outro_nis.is_some()),
        ("outro_message_id", |r| r.story.outro_message_id.is_some()),
        ("outro_movie", |r| r.story.outro_movie.is_some()),
        ("player_car_type", |r| r.story.player_car_type.is_some()),
        ("player_car_performance", |r| r.story.player_car_performance.is_some()),
    ];
    for (name, set) in fields {
        println!("CENSUS {name}={}", data.races.iter().filter(|r| set(r)).count());
    }

    let id = |r: &Race| r.event.clone().unwrap_or_else(|| r.name.clone());
    let mut bl: Vec<(String, String)> = data
        .races
        .iter()
        .filter_map(|r| {
            r.story.intro_nis.as_deref().filter(|n| n.starts_with("IntroNisBL")).map(|n| (n.to_owned(), id(r)))
        })
        .collect();
    bl.sort();
    let mut names: Vec<&str> = bl.iter().map(|(n, _)| n.as_str()).collect();
    names.dedup();
    println!("CENSUS IntroNisBL distinct={}", names.len());
    for (nis, event) in &bl {
        println!("CENSUS IntroNisBL {nis} <- {event}");
    }
    let names_of = |set: fn(&Race) -> bool| -> Vec<String> { data.races.iter().filter(|r| set(r)).map(id).collect() };
    println!("CENSUS intro_message_id races {:?}", names_of(|r| r.story.intro_message_id.is_some()));
    println!("CENSUS outro_nis races {:?}", names_of(|r| r.story.outro_nis.is_some()));
    println!("CENSUS outro_movie races {:?}", names_of(|r| r.story.outro_movie.is_some()));

    for event_id in ["16.1.0", "16.2.2", "16.2.3", "16.1.1", "16.8.1", "16.2.1"] {
        let race = event(&data, event_id);
        println!(
            "CENSUS prologue {event_id} {} car={:?} perf={:?}",
            race.name, race.story.player_car_type, race.story.player_car_performance
        );
    }
}
