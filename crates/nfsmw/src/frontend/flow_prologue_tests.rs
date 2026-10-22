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

/// The folder lookup of an install that knows the starter preset as a ride of the E46 GTR.
fn e46_folder(preset: &str) -> Option<String> {
    (preset == "M3GTRCAREERSTART").then(|| "BMWM3GTRE46".to_owned())
}

#[test]
fn a_prologue_race_forces_its_preset_on_the_car_type_it_is_made_for() {
    let data = data(vec![race("16.1.0", Some("M3GTRCAREERSTART"))]);
    // Not free roam's `BMWM3GTR`: the preset's parts are the E46's, and only that folder has their solids.
    assert_eq!(forced_car(&data, "16.1.0", e46_folder).as_deref(), Some("BMWM3GTRE46:M3GTRCAREERSTART"));
}

#[test]
fn a_preset_the_install_does_not_know_falls_back_to_the_e46_folder() {
    let data = data(vec![race("16.1.0", Some("M3GTRCAREERSTART"))]);
    assert_eq!(forced_car(&data, "16.1.0", |_| None).as_deref(), Some("BMWM3GTRE46:M3GTRCAREERSTART"));
}

#[test]
fn a_race_that_forces_no_car_or_is_not_in_the_data_gives_none() {
    let data = data(vec![race("1.2.3", None)]);
    assert_eq!(forced_car(&data, "1.2.3", e46_folder), None);
    assert_eq!(forced_car(&data, "16.9.9", e46_folder), None);
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
