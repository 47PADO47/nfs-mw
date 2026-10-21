use super::*;
use nfsmw_data::gameplay::{Race, RaceParams, RaceStory};

/// A race with the event id `event`, which forces the preset `forced` when it is given.
fn race(event: &str, forced: Option<&str>) -> Race {
    Race {
        key: 1,
        name: format!("race_bin_16/{event}"),
        event_icon: "p2p".into(),
        race_type: None,
        event: Some(event.into()),
        region: None,
        laps: 1,
        start: None,
        finish: None,
        checkpoints: Vec::new(),
        barriers: Vec::new(),
        params: RaceParams::default(),
        story: RaceStory { player_car_type: forced.map(str::to_owned), ..RaceStory::default() },
    }
}

fn data(races: Vec<Race>) -> GameplayData {
    GameplayData { races, ..GameplayData::default() }
}

#[test]
fn a_prologue_race_forces_its_preset_on_the_stock_car() {
    let data = data(vec![race("16.1.0", Some("M3GTRCAREERSTART"))]);
    assert_eq!(forced_car(&data, "16.1.0").as_deref(), Some("BMWM3GTR:M3GTRCAREERSTART"));
}

#[test]
fn a_race_that_forces_no_car_or_is_not_in_the_data_gives_none() {
    let data = data(vec![race("1.2.3", None)]);
    assert_eq!(forced_car(&data, "1.2.3"), None);
    assert_eq!(forced_car(&data, "16.9.9"), None);
}

#[test]
fn a_new_career_starts_with_the_first_prologue_race() {
    assert_eq!(FIRST_PROLOGUE_RACE, "16.1.0");
    assert_eq!(prologue_resume(&NodeStates::default(), 16), Some("16.1.0"));
}

#[test]
fn a_save_outside_the_prologue_stage_does_not_restart_it() {
    assert_eq!(prologue_resume(&NodeStates::default(), 15), None);
}
