//! Which logic runs which package (the original `ScreenFactoryData`, docs/specs/frontend-menus.md section 1).

use super::career_screens::car_select::CareerCarSelectLogic as CareerCarSelect;
use super::career_screens::rap_sheet;
use super::confirm::{Alert, AlertLogic};
use super::controls_screen::ControlsLogic;
use super::credits::CreditsLogic;
use super::custom_race::car_select::CarSelect as CustomCarSelect;
use super::custom_race::press_start::PressStart;
use super::custom_race::showcase::Showcase;
use super::custom_race::track_options::TrackOptions;
use super::custom_race::track_select::TrackSelect;
use super::icon_menu::{IconMenu, Kind};
use super::ids::screen;
use super::jukebox::JukeboxLogic;
use super::loading_tips::LoadingTips;
use super::logic::{Args, Plain, ScreenLogic, Sub};
use super::look::{CarLook, Look};
use super::quick_play::QuickPlay;
use super::splash::Splash;
use super::widget_menu::WidgetMenu;
use crate::career::CareerHandle;
use crate::ui::Text;
use crate::ui::theme::ElementId;

/// The logic for the package called `name` (its file name, any case). A package without logic of its own runs
/// as it is.
pub fn make(name: &str, args: Args, mod_rows: bool) -> Box<dyn ScreenLogic> {
    let name = name.to_ascii_lowercase();
    let is = |file: &str| name == file.to_ascii_lowercase();
    if is(screen::SPLASH) || is("WS_MW_LS_Splash.fng") {
        return Box::new(Splash::default());
    }
    if is(screen::LOADING_TIPS) {
        return Box::new(LoadingTips::new());
    }
    if is(screen::MAIN_MENU) {
        return Box::new(IconMenu::new(Kind::Main, args));
    }
    if is(screen::MAIN_MENU_SUB) && args.safehouse {
        return super::career_screens::safehouse_menu::make(args);
    }
    if is(screen::CAR_SELECT) && args.custom {
        return Box::new(CustomCarSelect::new(args));
    }
    if is(screen::CAR_SELECT) && args.coop {
        return Box::new(CareerCarSelect::coop());
    }
    if is(screen::PRESS_START) {
        return Box::new(PressStart::new(args));
    }
    if is(screen::SHOWCASE) {
        return Box::new(Showcase::new(args));
    }
    if let Some(logic) = super::career_screens::make(&name, args) {
        return logic;
    }
    if is(screen::MAIN_MENU_SUB) && args.sub == Sub::Career {
        return super::career_screens::career_menu::make(args);
    }
    if is(screen::MAIN_MENU_SUB) && args.sub == Sub::QuickRace {
        return Box::new(IconMenu::new(Kind::QuickRace, args));
    }
    if is(screen::MAIN_MENU_SUB) && args.sub == Sub::CustomRace {
        return Box::new(IconMenu::new(Kind::CustomRace, args));
    }
    if is(screen::MAIN_MENU_SUB) {
        return Box::new(IconMenu::new(Kind::Categories { pause: false, mods: mod_rows }, args));
    }
    if is(screen::MC_PROFILE_MANAGER) {
        return Box::new(IconMenu::new(Kind::AliasManager, args));
    }
    if is(screen::PAUSE_MENU) {
        let kind = if args.options { Kind::Categories { pause: true, mods: mod_rows } } else { Kind::Pause };
        return Box::new(IconMenu::new(kind, args));
    }
    if is(screen::OPTIONS) || is(screen::PAUSE_OPTIONS) {
        return Box::new(WidgetMenu::new(args));
    }
    // A dialog opened by name (`view-screen`) shows a notice; confirmations open it with their own logic.
    if is(screen::DIALOG) {
        return Box::new(AlertLogic::new(Alert::new(Text::Custom("Load complete.".into()))));
    }
    if is(screen::CONTROLS) || is(screen::PAUSE_CONTROLS) {
        return Box::new(ControlsLogic::new(&name, args));
    }
    if is(screen::TRACK_SELECT) {
        return Box::new(TrackSelect::default());
    }
    if is(screen::TRACK_OPTIONS) {
        return Box::new(TrackOptions::default());
    }
    if is(screen::CHALLENGE_SERIES) {
        return Box::new(super::challenge_series::ChallengeSeries::new());
    }
    if is(screen::QUICK_RACE_BRIEF) {
        return Box::new(QuickPlay::new());
    }
    if is(screen::EA_TRAX_JUKEBOX) {
        return Box::new(JukeboxLogic::new());
    }
    if is(screen::POST_RACE_RESULTS) {
        return Box::new(super::post_race::PostRaceLogic::new(crate::viewer::RaceReport::sample()));
    }
    if is(screen::POST_RACE_PURSUIT) {
        return Box::new(super::pursuit_results::PursuitResultsLogic::new(crate::viewer::PursuitReport::sample()));
    }
    if is(screen::PHOTO) {
        let logic = super::photo::PhotoLogic::for_race(
            crate::viewer::PhotoReport::sample(),
            crate::viewer::RaceReport::sample(),
        );
        return Box::new(logic);
    }
    if is(screen::MILESTONE_REWARDS) {
        return Box::new(super::rewards::RewardsLogic::new(crate::viewer::RewardsReport::sample()));
    }
    if is(screen::CREDITS) {
        return Box::new(CreditsLogic::default());
    }
    Box::new(Plain)
}

/// The menu slot the package called `name` is drawn for, or `None` when no slot takes it (the splash, the track options,
/// the loading screen). It tests the same names and `Args` as [`make`], in the same order, so a
/// package is never drawn by two slots. The screen's logic does not matter: a career screen with no career still has
/// its slot (`docs/specs/ui-themes-menus.md`, section 3).
pub fn slot_for(name: &str, args: &Args) -> Option<ElementId> {
    let name = name.to_ascii_lowercase();
    let is = |file: &str| name == file.to_ascii_lowercase();
    if is(screen::MAIN_MENU) {
        return Some(ElementId::MainMenu);
    }
    if is(screen::MAIN_MENU_SUB) && args.safehouse {
        return Some(ElementId::SafeHouseMenu);
    }
    if let Some(slot) = career_slot(&name) {
        return Some(slot);
    }
    if is(screen::MAIN_MENU_SUB) || is(screen::MC_PROFILE_MANAGER) {
        return Some(ElementId::Categories);
    }
    if is(screen::PAUSE_MENU) {
        return Some(if args.options { ElementId::Categories } else { ElementId::PauseMenu });
    }
    if is(screen::OPTIONS) || is(screen::PAUSE_OPTIONS) {
        return Some(ElementId::OptionTable);
    }
    if is(screen::TRACK_SELECT) {
        return Some(ElementId::TrackSelect);
    }
    if is(screen::DIALOG) {
        return Some(ElementId::Dialog);
    }
    if is(screen::CONTROLS) || is(screen::PAUSE_CONTROLS) {
        return Some(ElementId::ControlsList);
    }
    if is(screen::EA_TRAX_JUKEBOX) {
        return Some(ElementId::Jukebox);
    }
    if is(screen::CREDITS) {
        return Some(ElementId::Credits);
    }
    None
}

/// The backdrop the package called `name` wants behind it, or `None` when it keeps the one there: the dialogs, the lists and
/// the screens that open over others. Like [`slot_for`] it reads the file name and the `Args` only (`look.rs`).
pub fn look_for(name: &str, args: &Args) -> Option<Look> {
    let name = name.to_ascii_lowercase();
    let is = |file: &str| name == file.to_ascii_lowercase();
    if is(screen::MAIN_MENU) || is(screen::MC_PROFILE_MANAGER) {
        return Some(Look::Main);
    }
    if is(screen::MAIN_MENU_SUB) && args.safehouse {
        return Some(Look::SafeHouse(CarLook::of_profile(&CareerHandle::shared())));
    }
    if is(screen::MAIN_MENU_SUB) && args.sub == Sub::Career {
        return Some(Look::Car(CarLook::of_profile(&CareerHandle::shared())));
    }
    if is(screen::MAIN_MENU_SUB) && !args.pause {
        return Some(Look::Main);
    }
    // The custom race's car select and Showcase name the car they show once the cars were read (`ScreenLogic::look`).
    if (is(screen::CAR_SELECT) && args.custom) || is(screen::SHOWCASE) {
        return Some(Look::Car(CarLook::default_car()));
    }
    let safehouse_screen = career_slot(&name).is_some() && !is(screen::MC_LIST);
    safehouse_screen.then(|| Look::SafeHouse(CarLook::of_profile(&CareerHandle::shared())))
}

/// The pose collection the package called `name` asks the camera for (`FE_ATTRIB.bin`, class `frontend`; the collection
/// names are the screens' file stems with the front end's context in front, `menu-backdrop-looks.md` section 4.2), or
/// `None` when the screen keeps the camera where it is: the dialogs, the lists, the screens whose pose is not read.
/// Like [`look_for`] it reads the file name and the `Args` only.
pub fn pose_for(name: &str, args: &Args) -> Option<&'static str> {
    let name = name.to_ascii_lowercase();
    let is = |file: &str| name == file.to_ascii_lowercase();
    if is(screen::MAIN_MENU) {
        return Some("mainmenu");
    }
    if is(screen::MAIN_MENU_SUB) {
        return match (args.pause, args.safehouse, args.sub) {
            (true, _, _) => None,
            // The safe house menu is the career's sub menu; the career menu in the crib is the career manager's.
            (_, true, _) => Some("career_mainmenu_sub"),
            (_, _, Sub::Career) => Some("career_manager"),
            (_, _, Sub::QuickRace) => Some("quickracemain_mainmenu_sub"),
            (_, _, Sub::CustomRace) => Some("quickrace_mainmenu_sub"),
            (_, _, Sub::Options) => Some("options_mainmenu_sub"),
        };
    }
    let poses = [
        (screen::OPTIONS, "options_options"),
        (screen::CONTROLS, "options_ui_optionscontroller"),
        (screen::CREDITS, "options_credits"),
        (screen::EA_TRAX_JUKEBOX, "options_ea_trax_jukebox"),
        (screen::QUICK_RACE_BRIEF, "quickracemain_quick_race_brief"),
        (screen::CHALLENGE_SERIES, "quickracemain_challengeseries"),
        (screen::TRACK_SELECT, "quickrace_track_select"),
        (screen::TRACK_OPTIONS, "quickrace_track_options"),
        (screen::MC_PROFILE_MANAGER, "mc_profilemanager"),
        (screen::SAFE_HOUSE_RIVAL_CHALLENGE, "career_safehouserivalchallenge"),
    ];
    poses.iter().find(|(file, _)| is(file)).map(|(_, pose)| *pose)
}

/// The slot of a career screen, by its lower-case file name.
fn career_slot(name: &str) -> Option<ElementId> {
    let is = |file: &str| name == file.to_ascii_lowercase();
    if is(screen::SAFE_HOUSE_RIVAL_CHALLENGE) || is(screen::SAFE_HOUSE_RIVAL_BIO) {
        return Some(ElementId::Blacklist);
    }
    if is(screen::CAR_SELECT) {
        return Some(ElementId::CarLot);
    }
    if rap_sheet::is_rap_sheet_screen(name) {
        return Some(ElementId::RapSheet);
    }
    if is(screen::SAFEHOUSE_REGION_UNLOCK) {
        return Some(ElementId::RegionUnlock);
    }
    if is(screen::MC_MAIN) || is(screen::MC_LIST) {
        return Some(ElementId::MemoryCard);
    }
    if is(screen::SMS_MAILBOXES) || is(screen::SMS_MESSAGE) {
        return Some(ElementId::Sms);
    }
    None
}

#[cfg(test)]
#[path = "factory_tests.rs"]
mod tests;
