//! The rap sheet screen graph, the login pair, the remembered selection and the end-game chain
//! (`docs/specs/career-rapsheet.md` sections 1 to 3, `docs/specs/career-rapsheet-2.md` section 3).

use crate::safehouse::FrontEndContext;
use crate::settings::{CareerSettings, Flags};

/// Login animation timings, in milliseconds from the start of the page.
pub mod timing {
    /// Both login pages lock input at the start and enable it at this time.
    pub const INPUT_ENABLED_MS: u32 = 1_200;
    /// The leave animation after Accept.
    pub const LEAVE_ANIMATION_MS: u32 = 600;
    /// `Login`: the leave animation starts.
    pub const LOGIN_LEAVE_MS: u32 = 10_480;
    /// `Login`: `EXIT_COMPLETE` (11.08 s, "11.1 s" in the screen graph).
    pub const LOGIN_EXIT_COMPLETE_MS: u32 = 11_080;
    /// `Login2`: the leave animation starts (5.98 to 6.0 s).
    pub const LOGIN2_LEAVE_MS: u32 = 5_980;
    /// `Login2`: `EXIT_COMPLETE` (6.6 s).
    pub const LOGIN2_EXIT_COMPLETE_MS: u32 = 6_600;
    /// `Login2_ENDGAME`: `EXIT_COMPLETE` (6.58 s).
    pub const LOGIN2_ENDGAME_EXIT_COMPLETE_MS: u32 = 6_580;
}

/// The entry gate: the safe house menu item exists only when settings flag bit 4 is set. The screens never test
/// it again.
pub fn rap_sheet_unlocked(settings: &CareerSettings) -> bool {
    settings.flags.contains(Flags::HAS_RAP_SHEET)
}

/// A page of the rap sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// `RapSheetLogin`.
    Login,
    /// `RapSheetLogin2`.
    Login2,
    /// `RapSheetMain`.
    Main,
    /// `RapSheetRS`, the summary (Main button 1).
    Summary,
    /// `RapSheetVD`, the vehicle database (button 2).
    Vehicles,
    /// `RapSheetUS`, the infractions (button 3).
    Infractions,
    /// `RapSheetCTS`, the cost to state (button 4).
    CostToState,
    /// `RapSheetTEP`, the top 5 pursuits (button 5).
    TopPursuits,
    /// `RapSheetPD` with the entry index 0 to 4.
    PursuitDetail(u8),
    /// `RapSheetRankings` (button 6).
    Rankings,
    /// `RapSheetRankingsDetail` with the statistic index.
    RankingDetail(u8),
    /// Back in `MainMenu_Sub` (the safe house menu); context bit 9 is cleared.
    SafeHouseMenu,
}

/// The page a Main button 1 to 6 opens; other numbers (Back, a leave without a press) go to the safe house menu.
pub const fn main_button_page(button: u8) -> Page {
    match button {
        1 => Page::Summary,
        2 => Page::Vehicles,
        3 => Page::Infractions,
        4 => Page::CostToState,
        5 => Page::TopPursuits,
        6 => Page::Rankings,
        _ => Page::SafeHouseMenu,
    }
}

/// Where Back goes, for the pages whose Back is a plain page change. The Summary, Vehicles, Infractions and Cost
/// pages leave to Main on any leave. Login pages go through [`LoginPage`].
pub const fn back_target(page: Page) -> Page {
    match page {
        Page::Summary | Page::Vehicles | Page::Infractions | Page::CostToState | Page::TopPursuits | Page::Rankings => {
            Page::Main
        }
        Page::PursuitDetail(_) => Page::TopPursuits,
        Page::RankingDetail(_) => Page::Rankings,
        Page::Main | Page::Login | Page::Login2 | Page::SafeHouseMenu => Page::SafeHouseMenu,
    }
}

/// Accept on the top 5 entry `n` (1 to 5) opens the detail with the argument `n - 1`.
pub const fn top_entry_page(n: u8) -> Page {
    Page::PursuitDetail(n.saturating_sub(1))
}

/// Accept on a rankings category opens the detail with the statistic index.
pub const fn ranking_category_page(stat: u8) -> Page {
    Page::RankingDetail(stat)
}

/// Selecting the safe house menu item: context bit 9 is set and the login page opens.
pub fn enter_rap_sheet(context: &mut FrontEndContext) -> Page {
    context.insert(FrontEndContext::RAP_SHEET);
    Page::Login
}

/// Leaving to the safe house menu clears context bit 9.
pub fn leave_to_safe_house(context: &mut FrontEndContext) -> Page {
    context.remove(FrontEndContext::RAP_SHEET);
    Page::SafeHouseMenu
}

/// The mode of a login page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginMode {
    /// `Login`, animation running.
    Login,
    /// `Login2`, animation running.
    Login2,
    /// Accept was pressed (mode 3).
    Skipped,
}

/// The login page class: Accept skips, Back flags a return, and the end of the leave animation decides the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginPage {
    /// The mode.
    pub mode: LoginMode,
    /// The Back flag.
    pub back: bool,
}

impl LoginPage {
    /// A fresh `Login` (mode 0).
    pub const fn login() -> LoginPage {
        LoginPage { mode: LoginMode::Login, back: false }
    }

    /// A fresh `Login2` (mode 2).
    pub const fn login2() -> LoginPage {
        LoginPage { mode: LoginMode::Login2, back: false }
    }

    /// Accept (or a mouse release): the mode becomes 3 and a UI sound plays, `0x67` for `Login`, `0x68` for
    /// `Login2`. Returns the sound id (none when already skipped).
    pub fn accept(&mut self) -> Option<u8> {
        let sound = match self.mode {
            LoginMode::Login => Some(0x67),
            LoginMode::Login2 => Some(0x68),
            LoginMode::Skipped => None,
        };
        self.mode = LoginMode::Skipped;
        sound
    }

    /// Back sets the flag.
    pub fn back(&mut self) {
        self.back = true;
    }

    /// `EXIT_COMPLETE`: with the Back flag the safe house menu opens and context bit 9 is cleared; else after an
    /// Accept (mode 3) or in `Login2` the Main page opens; else (`Login` ran out) `Login2` opens.
    pub fn exit_complete(&self, context: &mut FrontEndContext) -> Page {
        if self.back {
            return leave_to_safe_house(context);
        }
        match self.mode {
            LoginMode::Skipped | LoginMode::Login2 => Page::Main,
            LoginMode::Login => Page::Login2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> FrontEndContext {
        FrontEndContext(FrontEndContext::CAREER.0 | FrontEndContext::RAP_SHEET.0)
    }

    #[test]
    fn the_item_needs_flag_bit_4() {
        let mut s = CareerSettings::new_career();
        assert!(!rap_sheet_unlocked(&s));
        s.flags.insert(Flags::HAS_RAP_SHEET);
        assert!(rap_sheet_unlocked(&s));
    }

    #[test]
    fn entering_sets_context_bit_9_and_opens_the_login() {
        let mut c = FrontEndContext::CAREER;
        assert_eq!(enter_rap_sheet(&mut c), Page::Login);
        assert!(c.contains(FrontEndContext::RAP_SHEET));
        assert_eq!(leave_to_safe_house(&mut c), Page::SafeHouseMenu);
        assert!(!c.contains(FrontEndContext::RAP_SHEET));
    }

    #[test]
    fn the_main_buttons_open_the_six_pages() {
        let pages: Vec<Page> = (1..=6).map(main_button_page).collect();
        assert_eq!(
            pages,
            [Page::Summary, Page::Vehicles, Page::Infractions, Page::CostToState, Page::TopPursuits, Page::Rankings]
        );
        assert_eq!(main_button_page(0), Page::SafeHouseMenu);
        assert_eq!(main_button_page(7), Page::SafeHouseMenu);
    }

    #[test]
    fn leaves_follow_the_screen_graph() {
        for page in
            [Page::Summary, Page::Vehicles, Page::Infractions, Page::CostToState, Page::TopPursuits, Page::Rankings]
        {
            assert_eq!(back_target(page), Page::Main);
        }
        assert_eq!(back_target(Page::PursuitDetail(2)), Page::TopPursuits);
        assert_eq!(back_target(Page::RankingDetail(7)), Page::Rankings);
        assert_eq!(back_target(Page::Main), Page::SafeHouseMenu);
        assert_eq!(top_entry_page(1), Page::PursuitDetail(0));
        assert_eq!(top_entry_page(5), Page::PursuitDetail(4));
        assert_eq!(ranking_category_page(9), Page::RankingDetail(9));
    }

    #[test]
    fn a_full_login_run_shows_login2_then_main() {
        let mut c = ctx();
        let login = LoginPage::login();
        assert_eq!(login.exit_complete(&mut c), Page::Login2);
        let login2 = LoginPage::login2();
        assert_eq!(login2.exit_complete(&mut c), Page::Main);
        assert!(c.contains(FrontEndContext::RAP_SHEET));
    }

    #[test]
    fn accept_skips_straight_to_main_with_the_documented_sounds() {
        let mut c = ctx();
        let mut login = LoginPage::login();
        assert_eq!(login.accept(), Some(0x67));
        assert_eq!(login.exit_complete(&mut c), Page::Main);
        let mut login2 = LoginPage::login2();
        assert_eq!(login2.accept(), Some(0x68));
        assert_eq!(login2.accept(), None);
        assert_eq!(login2.exit_complete(&mut c), Page::Main);
    }

    #[test]
    fn back_returns_to_the_safe_house_and_clears_the_bit() {
        for mut page in [LoginPage::login(), LoginPage::login2()] {
            let mut c = ctx();
            page.back();
            assert_eq!(page.exit_complete(&mut c), Page::SafeHouseMenu);
            assert!(!c.contains(FrontEndContext::RAP_SHEET));
        }
    }

    #[test]
    fn the_login_timings_are_the_documented_ones() {
        assert_eq!(timing::INPUT_ENABLED_MS, 1200);
        assert_eq!(timing::LEAVE_ANIMATION_MS, 600);
        assert_eq!((timing::LOGIN_LEAVE_MS, timing::LOGIN_EXIT_COMPLETE_MS), (10_480, 11_080));
        assert_eq!((timing::LOGIN2_LEAVE_MS, timing::LOGIN2_EXIT_COMPLETE_MS), (5_980, 6_600));
        assert_eq!(timing::LOGIN2_ENDGAME_EXIT_COMPLETE_MS, 6_580);
        assert_eq!(timing::LOGIN_LEAVE_MS + timing::LEAVE_ANIMATION_MS, timing::LOGIN_EXIT_COMPLETE_MS);
    }
}
