//! Which logic runs which package (the original `ScreenFactoryData`, docs/specs/frontend-menus.md section 1).

use super::icon_menu::{IconMenu, Kind};
use super::ids::screen;
use super::logic::{Args, Plain, ScreenLogic};
use super::splash::Splash;
use super::widget_menu::WidgetMenu;

/// The logic for the package called `name` (its file name, any case). A package without logic of its own runs
/// as it is.
pub fn make(name: &str, args: Args) -> Box<dyn ScreenLogic> {
    let name = name.to_ascii_lowercase();
    let is = |file: &str| name == file.to_ascii_lowercase();
    if is(screen::SPLASH) || is("WS_MW_LS_Splash.fng") {
        return Box::new(Splash::default());
    }
    if is(screen::MAIN_MENU) {
        return Box::new(IconMenu::new(Kind::Main, args));
    }
    if is(screen::MAIN_MENU_SUB) {
        return Box::new(IconMenu::new(Kind::Categories { pause: false }, args));
    }
    if is(screen::PAUSE_MENU) {
        let kind = if args.options { Kind::Categories { pause: true } } else { Kind::Pause };
        return Box::new(IconMenu::new(kind, args));
    }
    if is(screen::OPTIONS) || is(screen::PAUSE_OPTIONS) {
        return Box::new(WidgetMenu::new(args));
    }
    Box::new(Plain)
}
