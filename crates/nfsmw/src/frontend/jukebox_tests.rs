use super::*;

/// Ten songs: all on in the menus, the even ones in game too.
fn state(songs: usize) -> State {
    let playable: Vec<(bool, bool)> = (0..songs).map(|n| (true, n % 2 == 0)).collect();
    State::new(Jukebox::defaults(&playable))
}

#[test]
fn accept_leaves_without_changing_mode() {
    let mut s = state(10);
    assert_eq!(s.press(PAD_ACCEPT), Action::Leave);
    assert_eq!(s.jukebox.mode_of(0), Some(Mode::Both));
}

#[test]
fn accept_on_an_empty_table_does_nothing() {
    let mut s = state(0);
    assert_eq!(s.press(PAD_ACCEPT), Action::Leave);
    assert_eq!(s.press(PAD_DOWN), Action::Nothing);
}

#[test]
fn up_and_down_move_the_highlight_within_the_table() {
    let mut s = state(10);
    assert_eq!(s.press(PAD_UP), Action::Nothing, "already at the top");
    assert_eq!(s.press(PAD_DOWN), Action::Moved);
    assert_eq!(s.selected, 1);
    for _ in 0..20 {
        s.press(PAD_DOWN);
    }
    assert_eq!(s.selected, 9, "stops at the last song");
}

#[test]
fn selection_scrolls_a_four_row_window() {
    let mut s = state(10);
    for _ in 0..4 {
        s.press(PAD_DOWN);
    }
    assert_eq!((s.selected, s.page_start()), (4, 1));
    for _ in 0..20 {
        s.press(PAD_DOWN);
    }
    assert_eq!((s.selected, s.page_start()), (9, 6));
    for _ in 0..4 {
        s.press(PAD_UP);
    }
    assert_eq!((s.selected, s.page_start()), (5, 5));
}

#[test]
fn with_re_order_on_the_arrows_move_the_song_and_its_place_follows() {
    let mut s = state(10);
    s.press(PAD_DOWN);
    assert_eq!(s.press(RE_ORDER), Action::Moved);
    assert!(s.reorder);
    assert_eq!(s.press(PAD_UP), Action::Saved);
    assert_eq!(s.selected, 0);
    assert_eq!(s.jukebox.entries()[0].song, 1, "song 1 moved above song 0");
    assert_eq!(s.press(PAD_LEFT), Action::Nothing, "no page turn while re-ordering");
}

#[test]
fn l1_and_r1_set_the_order_and_save() {
    let mut s = state(3);
    assert!(!s.jukebox.random);
    assert_eq!(s.press(PAD_LTRIGGER), Action::Saved);
    assert!(s.jukebox.random);
    assert_eq!(s.press(PAD_RTRIGGER), Action::Saved);
    assert!(!s.jukebox.random);
}

#[test]
fn preview_plays_the_selected_song_without_changing_selection_or_order() {
    let mut s = state(10);
    s.press(PAD_DOWN);
    s.press(PAD_DOWN);
    s.press(RE_ORDER);
    assert_eq!(s.press(PREVIEW), Action::Preview(2));
    assert_eq!(s.selected, 2);
    assert_eq!(s.jukebox.entries()[2].song, 2, "Preview does not move a song");
    s.press(PAD_UP);
    assert_eq!(s.press(PREVIEW), Action::Preview(2), "preview follows the grabbed song's installed index");
}

#[test]
fn preview_and_grab_do_nothing_without_a_song() {
    let mut s = state(0);
    assert_eq!(s.press(PREVIEW), Action::Nothing);
    assert_eq!(s.press(RE_ORDER), Action::Nothing);
    assert!(!s.reorder);
}

#[test]
fn back_leaves() {
    let mut s = state(3);
    assert_eq!(s.press(PAD_BACK), Action::Leave);
}

#[test]
fn the_row_text_is_the_number_and_the_mode() {
    assert_eq!(number(0), "01");
    assert_eq!(number(11), "12");
    assert_eq!(title_line("Tilted", &mode_text(Mode::Both), false), "Tilted");
    assert_eq!(title_line("Tilted", "Off", true), "> Tilted");
    assert_eq!(mode_text(Mode::Off), Text::Game(0x9CCE_9F86).show(), "the game's Off");
    assert_eq!(mode_text(Mode::Menu), Text::Game(0x5278_C50B).show(), "the game's Menu");
    assert_eq!(mode_text(Mode::InGame), Text::Game(0x5C1B_351C).show(), "the game's Racing");
    assert_eq!(mode_text(Mode::Both), Text::Game(0x9CCE_64C4).show(), "the game's All");
    assert_eq!(order_word(true), Text::Game(0xA9C9_C8F7), "the game's Random");
    assert_eq!(order_word(false), Text::Game(0x4CA3_6B89), "the game's Sequential");
}

#[path = "jukebox_screen_tests.rs"]
mod screen_tests;

#[test]
fn left_and_right_change_selected_track_mode_without_turning_the_page() {
    let mut s = state(10);
    s.press(PAD_DOWN);
    let selected = s.selected;
    let first_page = s.page_start();
    let before_other = s.jukebox.mode_of(0);
    assert_eq!(s.jukebox.mode_of(selected), Some(Mode::Menu));
    assert_eq!(s.press(PAD_RIGHT), Action::Saved);
    assert_eq!(s.jukebox.mode_of(selected), Some(Mode::InGame));
    assert_eq!(s.selected, selected);
    assert_eq!(s.page_start(), first_page);
    assert_eq!(s.jukebox.mode_of(0), before_other);
    assert_eq!(s.press(PAD_LEFT), Action::Saved);
    assert_eq!(s.jukebox.mode_of(selected), Some(Mode::Menu));
    s.reorder = true;
    assert_eq!(s.press(PAD_RIGHT), Action::Nothing);
    assert_eq!(s.jukebox.mode_of(selected), Some(Mode::Menu));
    assert_eq!(s.press(PAD_ACCEPT), Action::Leave);
}
