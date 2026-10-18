//! The rap sheet (`docs/specs/career-rapsheet.md`): the login pair and the main page here, the pages behind the main
//! page in `pages.rs`. The screens open from the safe house menu's Rap Sheet item, which exists only when the career
//! has the rap sheet (settings flag bit 4). The rules are in `nfsmw_career::rapsheet`.

mod login;
mod main_page;
mod pages;

use nfsmw_career::rapsheet::{Page, rap_sheet_unlocked};

use crate::career::CareerHandle;
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command, ScreenLogic};

/// `RapSheetLogin2.fng`, the second login page (not in `ids::screen`).
pub const LOGIN2: &str = "RapSheetLogin2.fng";
/// `RapSheetRS.fng`, the summary page with the police verdict (not in `ids::screen`).
pub const SUMMARY: &str = "RapSheetRS.fng";
/// `RapSheetTEP.fng`, the top 5 pursuits (not in `ids::screen`).
pub const TOP_PURSUITS: &str = "RapSheetTEP.fng";
/// `RapSheetPD.fng`, the pursuit detail (not in `ids::screen`).
pub const PURSUIT_DETAIL: &str = "RapSheetPD.fng";

/// Every rap sheet screen file, lower case, as the factory passes the names.
const SCREENS: [&str; 11] = [
    "RapSheetLogin.fng",
    LOGIN2,
    "RapSheetMain.fng",
    SUMMARY,
    "RapSheetUS.fng",
    "RapSheetVD.fng",
    "RapSheetCTS.fng",
    TOP_PURSUITS,
    PURSUIT_DETAIL,
    "RapSheetRankings.fng",
    "RapSheetRankingsDetail.fng",
];

/// The logic of the rap sheet screen `name` (lower case), or `None` when `name` is no rap sheet screen or the career
/// has no rap sheet yet.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    if !is_rap_sheet_screen(name) {
        return None;
    }
    if !unlocked() {
        return None;
    }
    if name == screen::RAP_SHEET_LOGIN.to_ascii_lowercase() {
        return Some(Box::new(login::LoginScreen::first()));
    }
    if name == LOGIN2.to_ascii_lowercase() {
        return Some(Box::new(login::LoginScreen::second()));
    }
    if name == screen::RAP_SHEET_MAIN.to_ascii_lowercase() {
        return Some(Box::new(main_page::MainPage::default()));
    }
    pages::make(name, args)
}

/// Whether `name` (lower case) is one of the rap sheet's screens.
pub fn is_rap_sheet_screen(name: &str) -> bool {
    SCREENS.iter().any(|screen| name == screen.to_ascii_lowercase())
}

/// The gate: the career is being played and its settings have the rap sheet.
fn unlocked() -> bool {
    CareerHandle::shared().with(|career| rap_sheet_unlocked(&career.settings)).unwrap_or(false)
}

/// The package file of a rap sheet page (`Page::SafeHouseMenu` is the safe house menu).
pub fn page_file(page: Page) -> &'static str {
    match page {
        Page::Login => screen::RAP_SHEET_LOGIN,
        Page::Login2 => LOGIN2,
        Page::Main => screen::RAP_SHEET_MAIN,
        Page::Summary => SUMMARY,
        Page::Vehicles => screen::RAP_SHEET_VD,
        Page::Infractions => screen::RAP_SHEET_US,
        Page::CostToState => screen::RAP_SHEET_CTS,
        Page::TopPursuits => TOP_PURSUITS,
        Page::PursuitDetail(_) => PURSUIT_DETAIL,
        Page::Rankings => screen::RAP_SHEET_RANKINGS,
        Page::RankingDetail(_) => screen::RAP_SHEET_RANKINGS_DETAIL,
        Page::SafeHouseMenu => screen::MAIN_MENU_SUB,
    }
}

/// The command that opens a page from the rap sheet. Every screen of the rap sheet is opened from the safe house, so
/// the arguments carry `safehouse`; the safe house menu then gets its own look back.
pub fn open(page: Page) -> Command {
    let args = Args { safehouse: true, ..Args::default() };
    Command::Switch(page_file(page).to_owned(), args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rap_sheet_screens_are_recognised_in_lower_case() {
        assert!(is_rap_sheet_screen("rapsheetlogin.fng"));
        assert!(is_rap_sheet_screen("rapsheetrankingsdetail.fng"));
        assert!(!is_rap_sheet_screen("mainmenu_sub.fng"));
        assert!(!is_rap_sheet_screen("sms_mailboxes.fng"));
    }

    #[test]
    fn every_page_has_a_rap_sheet_file() {
        let pages = [
            Page::Login,
            Page::Login2,
            Page::Main,
            Page::Summary,
            Page::Vehicles,
            Page::Infractions,
            Page::CostToState,
            Page::TopPursuits,
            Page::PursuitDetail(0),
            Page::Rankings,
            Page::RankingDetail(0),
        ];
        for page in pages {
            assert!(is_rap_sheet_screen(&page_file(page).to_ascii_lowercase()), "{page:?}");
        }
    }

    #[test]
    fn the_safe_house_menu_is_the_back_target_file() {
        assert_eq!(page_file(Page::SafeHouseMenu), screen::MAIN_MENU_SUB);
    }

    #[test]
    fn opening_a_page_carries_the_safe_house_arguments() {
        match open(Page::Vehicles) {
            Command::Switch(file, args) => {
                assert_eq!(file, screen::RAP_SHEET_VD);
                assert!(args.safehouse);
            }
            other => panic!("unexpected command {other:?}"),
        }
    }
}
