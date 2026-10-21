//! The end game: once the final race is won and its results are answered, the front end runs the end-game chain over the
//! frozen game (`docs/specs/story-flow-2.md` section 5.4). The steps are [`EndGame`] (`rapsheet::session`). The port has
//! the credits and the safe house menu; `GameWon.fng` and the `_ENDGAME` rap sheet pages are not in it, so those steps are
//! logged and skipped.

use nfsmw_career::rapsheet::{EndGame, EndGamePage};
use nfsmw_career::safehouse::FrontEndContext;

use super::*;
use crate::frontend::ids::screen;

/// The screen a page of the chain opens in this port. `None` when the port has none (the rap sheet end-game pages).
pub(super) fn screen_of(page: EndGamePage) -> Option<&'static str> {
    match page {
        EndGamePage::Credits => Some(screen::CREDITS),
        EndGamePage::MainMenuSub => Some(screen::MAIN_MENU_SUB),
        EndGamePage::LoginEndgame | EndGamePage::Login2Endgame | EndGamePage::MainEndgame => None,
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
        self.end_game_next(host, env);
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
        loop {
            let Some(game) = self.end_game.as_mut() else { return };
            let Some(page) = game.step() else {
                log::info!("career: the end game is over");
                self.end_game = None;
                self.end_game_shown = None;
                return;
            };
            let Some(name) = screen_of(page) else {
                log::info!("career: the end game page {page:?} has no screen in this port; skipped");
                continue;
            };
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
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_end_game_shows_the_credits_then_the_safe_house_menu() {
        let mut context = FrontEndContext::CAREER;
        let mut game = EndGame::start(&mut context);
        let mut shown = Vec::new();
        while let Some(page) = game.step() {
            shown.extend(screen_of(page));
        }
        assert_eq!(shown, [screen::CREDITS, screen::MAIN_MENU_SUB], "the rap sheet pages are not in the port");
    }

    #[test]
    fn the_rap_sheet_end_game_pages_have_no_screen() {
        assert_eq!(screen_of(EndGamePage::LoginEndgame), None);
        assert_eq!(screen_of(EndGamePage::Login2Endgame), None);
        assert_eq!(screen_of(EndGamePage::MainEndgame), None);
    }
}
