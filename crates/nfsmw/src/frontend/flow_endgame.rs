//! The end game: once the final race is won and its results are answered, the front end shows `GameWon.fng`, then runs the
//! end-game chain over the frozen game (`docs/specs/story-flow-2.md` section 5.4). The steps are [`EndGame`]
//! (`rapsheet::session`): the credits, the three `_ENDGAME` rap sheet pages and the safe house menu. Each page closes with
//! Accept or Back (`career_screens/end_game_page.rs`), which runs the next step.

use nfsmw_career::rapsheet::{EndGame, EndGamePage};
use nfsmw_career::safehouse::FrontEndContext;

use super::*;
use crate::frontend::ids::screen;

/// The screen a page of the chain opens.
pub(super) fn screen_of(page: EndGamePage) -> &'static str {
    match page {
        EndGamePage::Credits => screen::CREDITS,
        EndGamePage::LoginEndgame => screen::RAP_SHEET_LOGIN_ENDGAME,
        EndGamePage::Login2Endgame => screen::RAP_SHEET_LOGIN2_ENDGAME,
        EndGamePage::MainEndgame => screen::RAP_SHEET_MAIN_ENDGAME,
        EndGamePage::MainMenuSub => screen::MAIN_MENU_SUB,
    }
}

impl Frontend {
    /// The final race is won and answered: the chain starts over the game, with the stage paused so the screens take the pad.
    pub(in crate::frontend) fn start_end_game(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("career: the final race is won; the end game runs");
        self.save_settings(env.changed);
        let mut context = FrontEndContext::CAREER;
        self.end_game = Some(EndGame::start(&mut context));
        self.end_game_shown = None;
        self.latch = true;
        self.screens.clear();
        self.paused.set(true);
        self.stage = Stage::Paused;
        // The win screen comes first; its exit (Accept or Back, `end_game_page.rs`) runs the chain's first page.
        let commands = self.screens.open(screen::GAME_WON, Args::default(), true, env);
        self.end_game_shown = Some(screen::GAME_WON.to_owned());
        self.apply(commands, host, env);
    }

    /// Called every frame while the front end is over the game or a menu: once the page the chain opened is no longer on
    /// top (it closed, or a screen replaced it), the next page opens.
    pub(in crate::frontend) fn end_game_frame(&mut self, host: &mut Host, env: &mut Env) {
        if self.end_game.is_none() {
            return;
        }
        if self.screens.top() == self.end_game_shown.as_deref() {
            return;
        }
        self.end_game_next(host, env);
    }

    /// Runs the chain up to the next page the port shows, and opens it. The safe house menu is the last page: once it is up the
    /// chain is over, and the menu's own answers go on.
    fn end_game_next(&mut self, host: &mut Host, env: &mut Env) {
        let Some(game) = self.end_game.as_mut() else { return };
        let Some(page) = game.step() else {
            log::info!("career: the end game is over");
            self.end_game = None;
            self.end_game_shown = None;
            return;
        };
        let name = screen_of(page);
        if page == EndGamePage::MainMenuSub {
            self.end_game = None;
            self.end_game_shown = None;
            // Credits Back already went to the safe house menu: it stays up as it is.
            if self.screens.top() == Some(name) {
                return;
            }
            self.in_safe_house = true;
            let args = Args { safehouse: true, ..Args::default() };
            let commands = self.screens.open(name, args, true, env);
            self.apply(commands, host, env);
            return;
        }
        let commands = self.screens.open(name, Args::default(), true, env);
        self.end_game_shown = Some(name.to_owned());
        self.apply(commands, host, env);
    }
}

/// The screens the end game shows, in order: `GameWon.fng` first, then the chain's pages (credits, the rap sheet pages,
/// the safe house menu).
#[cfg(test)]
pub(super) fn screens_in_order() -> Vec<&'static str> {
    let mut context = FrontEndContext::CAREER;
    let mut game = EndGame::start(&mut context);
    let mut shown = vec![screen::GAME_WON];
    while let Some(page) = game.step() {
        shown.push(screen_of(page));
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_end_game_chain_includes_the_win_screen_then_the_credits_the_rap_sheet_pages_and_the_safe_house_menu() {
        assert_eq!(
            screens_in_order(),
            [
                screen::GAME_WON,
                screen::CREDITS,
                screen::RAP_SHEET_LOGIN_ENDGAME,
                screen::RAP_SHEET_LOGIN2_ENDGAME,
                screen::RAP_SHEET_MAIN_ENDGAME,
                screen::MAIN_MENU_SUB,
            ]
        );
    }

    #[test]
    fn every_page_of_the_chain_has_a_screen_logic_that_closes_it() {
        for name in screens_in_order() {
            assert!(
                crate::frontend::career_screens::end_game_page::is_end_game_page(name) || name == screen::MAIN_MENU_SUB,
                "{name} has no end-game logic"
            );
        }
    }
}
