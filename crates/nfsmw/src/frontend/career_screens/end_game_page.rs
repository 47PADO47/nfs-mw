//! The end-game pages (`docs/specs/career-rapsheet-2.md` section 3): `GameWon.fng` and the three `_ENDGAME` rap sheet pages.
//! Their classes are one tiny logic whose only job is to run the end-game step when the page is done, and the step is the
//! chain of `flow_endgame.rs`. Here a page is done when Accept or Back closes it, which takes the chain to the next page.
//! The package's own scripts still run, so the page draws as the original's does.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};

use crate::frontend::ids::screen;
use crate::frontend::logic::{Command, Cx, ScreenLogic};

/// The end-game pages of the port, in lower case: `GameWon.fng` and the rap sheet pages the counter pushes.
const END_GAME_PAGES: [&str; 4] = [
    screen::GAME_WON,
    screen::RAP_SHEET_LOGIN_ENDGAME,
    screen::RAP_SHEET_LOGIN2_ENDGAME,
    screen::RAP_SHEET_MAIN_ENDGAME,
];

/// The logic of an end-game page: Accept or Back closes it, and the end-game chain opens the next page.
pub struct EndGamePage;

impl ScreenLogic for EndGamePage {
    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_ACCEPT | PAD_BACK => cx.send(Command::Pop),
            _ => {}
        }
    }
}

/// Whether `name` (any case) is an end-game page.
pub fn is_end_game_page(name: &str) -> bool {
    END_GAME_PAGES.iter().any(|page| name.eq_ignore_ascii_case(page))
}

/// The logic for `name` when it is an end-game page.
pub fn make(name: &str) -> Option<Box<dyn ScreenLogic>> {
    match is_end_game_page(name) {
        true => Some(Box::new(EndGamePage)),
        false => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_win_screen_and_the_three_rap_sheet_pages_are_end_game_pages() {
        assert!(is_end_game_page("GameWon.fng"));
        assert!(is_end_game_page("rapsheetlogin_endgame.fng"));
        assert!(is_end_game_page("RapSheetLogin2_ENDGAME.fng"));
        assert!(is_end_game_page("RapSheetMain_ENDGAME.fng"));
    }

    #[test]
    fn the_normal_rap_sheet_and_other_screens_are_not() {
        assert!(!is_end_game_page("RapSheetLogin.fng"));
        assert!(!is_end_game_page("RapSheetMain.fng"));
        assert!(!is_end_game_page("Credits.fng"));
        assert!(make("MainMenu_Sub.fng").is_none());
        assert!(make("GameWon.fng").is_some());
    }
}
