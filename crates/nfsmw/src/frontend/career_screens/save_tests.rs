use super::*;

#[path = "save_pointer_tests.rs"]
mod pointer;

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn a_new_career_takes_the_first_free_name() {
    assert_eq!(next_alias(&[]), "PLAYER");
    assert_eq!(next_alias(&names(&["MWO"])), "PLAYER");
}

#[test]
fn taken_names_are_skipped_without_case() {
    assert_eq!(next_alias(&names(&["player"])), "PLAYER2");
    assert_eq!(next_alias(&names(&["PLAYER", "Player2"])), "PLAYER3");
}

#[test]
fn a_gap_is_filled_before_the_next_number() {
    assert_eq!(next_alias(&names(&["PLAYER", "PLAYER3"])), "PLAYER2");
}

#[test]
fn past_nine_the_names_stay_short_enough_to_list() {
    let mut taken = names(&["PLAYER"]);
    for n in 2..=9 {
        taken.push(format!("PLAYER{n}"));
    }
    let next = next_alias(&taken);
    assert_eq!(next, "P10");
    assert!(nfsmw_career::savefile::state::is_listable_alias(&next));
}

#[test]
fn every_generated_name_is_listable() {
    for n in 1..=60 {
        let name = alias_candidate(n);
        assert!(nfsmw_career::savefile::state::is_listable_alias(&name), "{name} is not listable");
    }
}

#[test]
fn the_question_fills_the_alias_in() {
    assert_eq!(question(Some("save the alias named %s?"), Some("MWO")), "save the alias named MWO?");
}

#[test]
fn the_question_falls_back_without_text_an_alias_or_a_slot() {
    assert_eq!(question(None, Some("MWO")), SAVE_QUESTION_FALLBACK);
    assert_eq!(question(Some("no slot here"), Some("MWO")), SAVE_QUESTION_FALLBACK);
    assert_eq!(question(Some("save %s?"), None), SAVE_QUESTION_FALLBACK);
}

#[test]
fn yes_saves_only_with_an_alias_and_no_or_back_leave() {
    assert_eq!(answer(true, true), Answer::Save);
    assert_eq!(answer(true, false), Answer::Leave);
    assert_eq!(answer(false, true), Answer::Leave);
    assert_eq!(answer(false, false), Answer::Leave);
}

#[test]
fn the_prompt_labels_are_yes_and_no() {
    assert_eq!(YES_LABEL, 0x70E0_1038);
    assert_eq!(NO_LABEL, 0x417B_25E4);
    assert_eq!(SAVE_QUESTION, 0x39B3_CCBA);
}

#[test]
fn a_page_holds_eight_rows_and_the_selection_picks_it() {
    let aliases: Vec<String> = (0..12).map(|i| format!("A{i}")).collect();
    assert_eq!(page_rows(&aliases, 0).len(), ROWS);
    assert_eq!(ROWS, 8);
    assert_eq!(page_start(7), 0);
    assert_eq!(page_start(8), 8);
    assert_eq!(page_rows(&aliases, 8), names(&["A8", "A9", "A10", "A11"]));
}

#[test]
fn the_title_says_load_or_delete() {
    assert_eq!(title_label(ListMode::Load), 0x0018_ECFF);
    assert_eq!(title_label(ListMode::Delete), 0x56B0_0632, "the DELETE label, \"Delete\"");
    assert_ne!(title_label(ListMode::Delete), title_label(ListMode::Load));
}

#[test]
fn the_selection_stays_inside_the_list() {
    assert_eq!(moved(0, 3, false), 0);
    assert_eq!(moved(2, 3, true), 2);
    assert_eq!(moved(0, 3, true), 1);
    assert_eq!(moved(0, 0, true), 0);
}

#[test]
fn the_list_mode_is_read_back_as_set() {
    set_list_mode(ListMode::Delete);
    assert_eq!(list_mode(), ListMode::Delete);
    set_list_mode(ListMode::Load);
    assert_eq!(list_mode(), ListMode::Load);
}

/// The prompt's package answers Left and Right for the game but not Accept or Back (skipped without `NFSMW_GAME_DIR`).
#[test]
fn accept_and_back_on_the_prompt_leave_for_the_safe_house_menu() {
    use std::sync::Arc;

    use crate::frontend::screens::Screens;
    use crate::settings::{Partial, Settings, test_caps};
    use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

    let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let assets = Arc::new(UiAssets::load(&dir).unwrap());
    for button in [pad::ACCEPT, pad::BACK] {
        let mut screens = Screens::new(Catalog::load(&dir, &SCREEN_FILES), assets.clone());
        let (mut settings, mut changed) = (Settings::from(Partial::default()), Partial::default());
        let mut env = crate::frontend::logic::Env {
            settings: &mut settings,
            changed: &mut changed,
            caps: test_caps::native(),
            mods: None,
        };
        let args = Args { safehouse: true, ..Args::default() };
        screens.open(screen::MC_MAIN, args, true, &mut env);
        for mask in std::iter::repeat_n(0, 60).chain([button; 3]).chain([0; 3]).chain(std::iter::repeat_n(0, 60)) {
            screens.update(1.0 / 60.0, mask, &mut env);
        }
        assert_eq!(screens.top(), Some(screen::MAIN_MENU_SUB), "mask {button:#x} leaves the prompt");
    }
}
