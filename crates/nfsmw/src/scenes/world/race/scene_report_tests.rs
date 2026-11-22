//! The prologue's presentation against the install: every scene the six prologue events play runs to its end headless,
//! its car channels all drive a car the data names, and no step is a text card the race data does not ask for. Needs
//! the game (set `NFSMW_GAME_DIR`). With `NFSMW_SCENE_REPORT` set to a file path, the JSON report is written there.

use game_install::GameDir;

use super::prologue;

fn install() -> Option<GameDir> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    Some(GameDir::open(std::path::PathBuf::from(dir)).expect("the install opens"))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_prologue_scene_plays_to_its_end_with_its_cars_bound_and_no_fallback() {
    let dir = install().expect("NFSMW_GAME_DIR");
    let events = prologue(&dir).expect("the prologue report");
    let json = serde_json::to_string_pretty(&events).expect("json");
    if let Some(path) = std::env::var_os("NFSMW_SCENE_REPORT") {
        std::fs::write(path, &json).expect("the report is written");
    }
    println!("{json}");
    assert_eq!(events.len(), 6, "six prologue events");
    let mut failures = Vec::new();
    for event in &events {
        for scene in &event.scenes {
            failures.extend(scene.problems.iter().map(|problem| format!("{} {}: {problem}", event.event, scene.scene)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn only_the_race_that_sets_an_intro_message_shows_a_card() {
    let dir = install().expect("NFSMW_GAME_DIR");
    let events = prologue(&dir).expect("the prologue report");
    let carded: Vec<&str> = events
        .iter()
        .filter(|event| event.steps.iter().any(|step| step.step == "message"))
        .map(|event| event.event.as_str())
        .collect();
    // `16_1_1_circuit` is the only prologue race with an `IntroMessageID` (`story-flow-2.md` F-04).
    assert_eq!(carded, ["16.1.1"]);
}
