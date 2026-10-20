//! Which logic runs which package (the original `ScreenFactoryData`, docs/specs/frontend-menus.md section 1).

use super::career_screens::rap_sheet;
use super::confirm::{Alert, AlertLogic};
use super::controls_screen::ControlsLogic;
use super::credits::CreditsLogic;
use super::icon_menu::{IconMenu, Kind};
use super::ids::screen;
use super::jukebox::JukeboxLogic;
use super::logic::{Args, Plain, ScreenLogic};
use super::splash::Splash;
use super::track_options::TrackOptions;
use super::track_select::TrackSelect;
use super::widget_menu::WidgetMenu;
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
    if is(screen::MAIN_MENU) {
        return Box::new(IconMenu::new(Kind::Main, args));
    }
    if is(screen::MAIN_MENU_SUB) && args.safehouse {
        return super::career_screens::safehouse_menu::make(args);
    }
    if let Some(logic) = super::career_screens::make(&name, args) {
        return logic;
    }
    if is(screen::MAIN_MENU_SUB) {
        return Box::new(IconMenu::new(Kind::Categories { pause: false, mods: mod_rows }, args));
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
    if is(screen::EA_TRAX_JUKEBOX) {
        return Box::new(JukeboxLogic::new());
    }
    if is(screen::POST_RACE_RESULTS) {
        return Box::new(super::post_race::PostRaceLogic::new(crate::viewer::RaceReport::sample()));
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
    if is(screen::MAIN_MENU_SUB) {
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
