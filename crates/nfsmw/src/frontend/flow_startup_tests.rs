use super::*;
use crate::frontend::ids::screen::MAIN_MENU;

#[test]
fn a_loaded_profile_goes_straight_to_the_menu() {
    assert_eq!(decide(true, 3), Step::Menu);
    assert_eq!(decide(true, 0), Step::Menu);
}

#[test]
fn with_no_profile_saved_the_game_asks_to_create_one() {
    assert_eq!(decide(false, 0), Step::Create);
}

#[test]
fn saved_profiles_are_listed_even_when_there_is_only_one() {
    assert_eq!(decide(false, 1), Step::List);
    assert_eq!(decide(false, 8), Step::List);
}

#[test]
fn the_title_screen_stays_when_the_movies_are_skipped() {
    let all = boot_steps(true);
    assert_eq!(all, VecDeque::from(BOOT));
    assert_eq!(all.len(), 3);
    assert_eq!(boot_steps(false), VecDeque::from([Boot::Splash]));
}

#[test]
fn the_profile_step_ends_when_its_last_screen_closes() {
    assert!(is_over(true, Stage::Menus, None));
    assert!(!is_over(true, Stage::Menus, Some(MAIN_MENU)), "a screen is still up");
    assert!(!is_over(false, Stage::Menus, None), "no profile step is running");
    assert!(!is_over(true, Stage::Playing, None), "the world has the window");
}

#[test]
fn yes_shows_the_two_autosave_notices_and_then_asks_for_the_name() {
    let question = create_question();
    assert_eq!(question.message, Text::Game(CREATE_QUESTION));
    let Command::Alert(first) = *question.on_yes else { panic!("Yes opens a notice") };
    assert_eq!(first.message, Text::Game(AUTOSAVE_NOTICE_1));
    let Some(then) = first.on_close else { panic!("the first notice leads on") };
    let Command::Alert(second) = *then else { panic!("the second notice follows") };
    assert_eq!(second.message, Text::Game(AUTOSAVE_NOTICE_2));
    let Some(then) = second.on_close else { panic!("the second notice leads on") };
    assert_eq!(*then, Command::EnterName(NameFor::Create));
}

#[test]
fn no_runs_nothing_so_the_game_goes_on_to_the_menu() {
    assert_eq!(create_question().on_no, None);
}

#[test]
fn the_question_and_notices_are_the_originals_labels() {
    assert_eq!(CREATE_QUESTION, 0xBE97_590F);
    assert_eq!((AUTOSAVE_NOTICE_1, AUTOSAVE_NOTICE_2), (0xB01B_E5DD, 0xB380_362F));
}
