use blackbox_modding::content::{nitrous_profile_path, parse_nitrous_profile};

use super::*;

fn sample(id: u64, looped: bool) -> StereoSample {
    StereoSample { id, frames: vec![[0.25, -0.25]; 4], rate: 48_000, looped }
}

#[test]
fn the_loop_and_the_end_come_from_the_burn_sequence_and_the_start_from_the_other() {
    let found = assign(vec![sample(1, true), sample(2, false)], vec![sample(3, false)]);
    assert_eq!(found.looped.as_ref().map(|s| s.id), Some(1));
    assert_eq!(found.end.as_ref().map(|s| s.id), Some(2));
    assert_eq!(found.start.as_ref().map(|s| s.id), Some(3));
}

#[test]
fn a_sequence_that_is_missing_leaves_its_sound_out() {
    let found = assign(vec![sample(1, true)], vec![]);
    assert!(found.start.is_none() && found.end.is_none() && found.looped.is_some());
    assert!(assign(vec![], vec![]).is_empty());
    assert!(assign(vec![], vec![sample(5, true)]).is_empty(), "a looping sample is never a start");
}

#[test]
fn the_profile_parses_with_the_games_own_reader_and_names_only_what_was_found() {
    let found = assign(vec![sample(1, true), sample(2, false)], vec![sample(3, false)]);
    let profile = parse_nitrous_profile(&profile_text(&found)).unwrap();
    let audio = profile.audio.unwrap();
    assert_eq!(audio.start.as_deref(), Some("NITROUS/MW2012/START.WAV"));
    assert_eq!(audio.looped.as_deref(), Some("NITROUS/MW2012/LOOP.WAV"));
    assert_eq!(audio.end.as_deref(), Some("NITROUS/MW2012/END.WAV"));
    assert!(audio.replace_original);
    let partial = assign(vec![sample(1, true)], vec![]);
    let audio = parse_nitrous_profile(&profile_text(&partial)).unwrap().audio.unwrap();
    assert!(audio.start.is_none() && audio.end.is_none());
}

#[test]
fn the_files_land_where_the_game_looks_for_the_profile() {
    let dir = std::env::temp_dir().join(format!("nfsmw-2012-nitrous-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let found = assign(vec![sample(1, true), sample(2, false)], vec![sample(3, false)]);
    write(&dir, &found).unwrap();
    let wanted = dir.join("files").join(nitrous_profile_path(VARIANT));
    assert!(wanted.is_file(), "{}", wanted.display());
    for name in ["START.WAV", "LOOP.WAV", "END.WAV"] {
        assert!(dir.join("files/NITROUS/MW2012").join(name).is_file());
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_expression_search_finds_a_string_anywhere_in_a_block() {
    assert!(contains(b"..Players.LocalPlayer.NitrousState.UsingNitrous(NOT)..", BURN_EXPRESSION));
    assert!(!contains(b"NitrousState.Other", BURN_EXPRESSION));
    assert!(!contains(b"", BURN_EXPRESSION));
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_real_install_has_a_looping_burn_sound_and_two_one_shots() {
    let root = std::env::var_os("MW2012_GAME_DIR").map(std::path::PathBuf::from).expect("MW2012_GAME_DIR");
    let source = crate::source::Source::open(&root).unwrap();
    let found = find_in(&source.game_logic_bundles()).expect("the nitrous sequences are in GAMELOGIC");
    let seconds = |s: &Option<StereoSample>| s.as_ref().map(|s| s.frames.len() as f32 / s.rate as f32);
    println!("start {:?} loop {:?} end {:?}", seconds(&found.start), seconds(&found.looped), seconds(&found.end));
    assert!(found.looped.is_some() && found.end.is_some() && found.start.is_some());
}
