//! Which look draws the menus, as the screens ask it: whether a theme lays the menus out as lists from the entries the
//! screens publish ([`Cx::list_look`]), and, for the list screens beyond the icon menus, whether the look in use draws
//! that screen ([`Cx::screen_list_look`]).
//!
//! Two looks draw lists: Remastered and Custom (`ui/theme/remastered/menu`, `ui/theme/custom/menu`). Both draw the main
//! menu, the option categories, the pause menu, the safe house, the option pages and the dialogs, which ask
//! [`Cx::list_look`]. The other list screens are named by a [`ListScreen`]: Custom draws the key list, the jukebox, the
//! credits and the Blacklist (PR 11b), and leaves the marker cards and the rap sheet pages to their FEng packages, which
//! Remastered alone lays out as lists.

use super::Cx;
use crate::settings::UiStyle;

/// A list screen beyond the icon menus, the option pages and the dialogs, as [`Cx::screen_list_look`] asks for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListScreen {
    /// The key list (`ControlsList`).
    Controls,
    /// The EA Trax playlist (`Jukebox`).
    Jukebox,
    /// The credits roll (`Credits`).
    Credits,
    /// The Blacklist's flow: the list of rivals and a rival's bio (both on the `Blacklist` slot), the safe house's way
    /// into it, and the reputation overview between them. The overview has no slot of its own, but it is where a list
    /// of rivals leads and where the boss race is started from, so it takes the list's input whenever the Blacklist is a
    /// list: otherwise the challenge could not be reached.
    Blacklist,
    /// The marker cards after a boss race (`SafehouseMarkers.fng`).
    Markers,
    /// A rival's rap sheet pages (`career_screens/rep_sheets`).
    RepSheets,
}

impl ListScreen {
    /// Every list screen, for the tests' truth table.
    #[cfg(test)]
    pub const ALL: [ListScreen; 6] = [
        ListScreen::Controls,
        ListScreen::Jukebox,
        ListScreen::Credits,
        ListScreen::Blacklist,
        ListScreen::Markers,
        ListScreen::RepSheets,
    ];

    /// Whether the Custom look draws this screen as a list.
    fn custom_draws(self) -> bool {
        match self {
            ListScreen::Controls | ListScreen::Jukebox | ListScreen::Credits | ListScreen::Blacklist => true,
            ListScreen::Markers | ListScreen::RepSheets => false,
        }
    }
}

/// Whether `style` draws the menus as lists, when no mod theme took over (`mod_theme` false).
pub fn list_look_of(style: UiStyle, mod_theme: bool) -> bool {
    if mod_theme {
        return false;
    }
    matches!(style, UiStyle::Remastered | UiStyle::Custom)
}

/// Whether `style` draws `screen` as a list, when no mod theme took over (`mod_theme` false): Remastered draws every
/// list screen, Custom the ones it has components for.
pub fn screen_list_look_of(screen: ListScreen, style: UiStyle, mod_theme: bool) -> bool {
    if !list_look_of(style, mod_theme) {
        return false;
    }
    match style {
        UiStyle::Remastered => true,
        UiStyle::Custom => screen.custom_draws(),
        UiStyle::Original => false,
    }
}

impl Cx<'_> {
    /// Whether a mod theme took the menus over (the Mods page's theme choice).
    fn mod_theme(&self) -> bool {
        self.mods.is_some_and(|mods| mods.ui_theme_choice().is_some())
    }

    /// Whether a list look (Remastered or Custom) draws the menus: the screen then publishes its entries as a list
    /// (`ScreenLogic::list`) and is driven as one, and the theme draws it.
    pub fn list_look(&self) -> bool {
        list_look_of(self.settings.ui_style, self.mod_theme())
    }

    /// Whether the look in use draws `screen` as a list. A screen the look does not draw keeps its FEng package.
    pub fn screen_list_look(&self, screen: ListScreen) -> bool {
        screen_list_look_of(screen, self.settings.ui_style, self.mod_theme())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_looks_are_remastered_and_custom_unless_a_mod_theme_took_over() {
        let cases = [
            (UiStyle::Original, false, false),
            (UiStyle::Remastered, false, true),
            (UiStyle::Custom, false, true),
            (UiStyle::Original, true, false),
            (UiStyle::Remastered, true, false),
            (UiStyle::Custom, true, false),
        ];
        for (style, mod_theme, list) in cases {
            assert_eq!(list_look_of(style, mod_theme), list, "{style:?}, mod theme {mod_theme}: list look");
        }
    }

    #[test]
    fn every_style_is_covered() {
        for style in UiStyle::ALL {
            let expected = style != UiStyle::Original;
            assert_eq!(list_look_of(style, false), expected, "{style:?}");
        }
    }

    #[test]
    fn each_list_screen_follows_the_looks_that_draw_it() {
        use ListScreen::*;
        // Screen, then whether Original, Remastered and Custom draw it as a list with no mod theme.
        let table = [
            (Controls, [false, true, true]),
            (Jukebox, [false, true, true]),
            (Credits, [false, true, true]),
            (Blacklist, [false, true, true]),
            (Markers, [false, true, false]),
            (RepSheets, [false, true, false]),
        ];
        assert_eq!(table.len(), ListScreen::ALL.len(), "every screen is in the table");
        for (screen, drawn) in table {
            for (style, expected) in [UiStyle::Original, UiStyle::Remastered, UiStyle::Custom].into_iter().zip(drawn) {
                assert_eq!(screen_list_look_of(screen, style, false), expected, "{screen:?} under {style:?}");
                assert!(!screen_list_look_of(screen, style, true), "{screen:?} under a mod theme over {style:?}");
            }
        }
    }

    #[test]
    fn remastered_draws_every_list_screen_as_it_did() {
        // Before PR 11b these screens asked whether the look was Remastered; under Remastered they still all answer yes,
        // and under Original or a mod theme no.
        for screen in ListScreen::ALL {
            assert!(screen_list_look_of(screen, UiStyle::Remastered, false), "{screen:?}");
            assert!(!screen_list_look_of(screen, UiStyle::Original, false), "{screen:?}");
        }
    }
}
